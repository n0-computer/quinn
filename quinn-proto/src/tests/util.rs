use std::{
    cmp,
    collections::{HashMap, HashSet, VecDeque},
    env,
    io::{self, Write},
    mem,
    net::{Ipv6Addr, SocketAddr, UdpSocket},
    num::{NonZeroU32, NonZeroUsize},
    ops::RangeFrom,
    str,
    sync::{Arc, LazyLock, Mutex},
};

use assert_matches::assert_matches;
use bytes::BytesMut;
use rand::{SeedableRng, rngs::StdRng};
use rustls::{
    KeyLogFile,
    client::WebPkiServerVerifier,
    pki_types::{CertificateDer, PrivateKeyDer},
};
use tracing::{debug, info_span, trace};

use super::crypto::rustls::{QuicClientConfig, QuicServerConfig, configured_provider};
use super::*;
use crate::{Duration, Instant, congestion::Controller};

pub(super) const DEFAULT_MTU: usize = 1452;

pub(super) struct Pair {
    pub(super) server: TestEndpoint,
    pub(super) client: TestEndpoint,
    /// Start time
    epoch: Instant,
    /// Current time
    pub(super) time: Instant,
    /// Simulates the maximum size allowed for UDP payloads by the link (packets exceeding this size will be dropped)
    pub(super) mtu: usize,
    /// Simulates explicit congestion notification
    pub(super) congestion_experienced: bool,
    // One-way
    pub(super) latency: Duration,
    /// Number of spin bit flips
    pub(super) spins: u64,
    /// The routing table used for resolving addresses observed for incoming packets
    /// and determining whether they should get lost.
    pub(super) routes: Option<RoutingTable>,
    last_spin: bool,
}

impl Pair {
    /// Creates an endpoint pair that'll run deterministically with hardcoded addresses.
    pub(super) fn seeded(seed: [u8; 32]) -> Self {
        let mut rng = StdRng::from_seed(seed);
        let mut client_seed = [0u8; 32];
        let mut server_seed = [0u8; 32];
        rng.fill_bytes(&mut client_seed);
        rng.fill_bytes(&mut server_seed);

        let mut cfg = server_config();
        let mut transport = TransportConfig::default();
        transport.deterministic_packet_numbers(true);
        cfg.transport = Arc::new(transport);

        let mut client_config = EndpointConfig::default();
        let mut server_config = EndpointConfig::default();
        client_config.rng_seed(Some(client_seed));
        server_config.rng_seed(Some(server_seed));

        let server = Endpoint::new(Arc::new(client_config), Some(Arc::new(cfg)), true);
        let client = Endpoint::new(Arc::new(server_config), None, true);

        let server_addr = SocketAddr::new(Ipv6Addr::LOCALHOST.into(), 4433);
        let client_addr = SocketAddr::new(Ipv6Addr::LOCALHOST.into(), 44433);
        let now = Instant::now();
        Self {
            server: TestEndpoint::new(server, server_addr),
            client: TestEndpoint::new(client, client_addr),
            epoch: now,
            time: now,
            mtu: DEFAULT_MTU,
            latency: Duration::from_millis(1),
            spins: 0,
            last_spin: false,
            congestion_experienced: false,
            routes: None,
        }
    }

    pub(super) fn default_with_deterministic_pns() -> Self {
        let mut cfg = server_config();
        let mut transport = TransportConfig::default();
        transport.deterministic_packet_numbers(true);
        cfg.transport = Arc::new(transport);
        Self::new(Default::default(), cfg)
    }

    pub(super) fn new(endpoint_config: Arc<EndpointConfig>, server_config: ServerConfig) -> Self {
        let server = Endpoint::new(endpoint_config.clone(), Some(Arc::new(server_config)), true);
        let client = Endpoint::new(endpoint_config, None, true);

        Self::new_from_endpoint(client, server)
    }

    pub(super) fn new_from_endpoint(client: Endpoint, server: Endpoint) -> Self {
        let server_addr = SocketAddr::new(
            Ipv6Addr::LOCALHOST.into(),
            SERVER_PORTS.lock().unwrap().next().unwrap(),
        );
        let client_addr = SocketAddr::new(
            Ipv6Addr::LOCALHOST.into(),
            CLIENT_PORTS.lock().unwrap().next().unwrap(),
        );
        let now = Instant::now();
        Self {
            server: TestEndpoint::new(server, server_addr),
            client: TestEndpoint::new(client, client_addr),
            epoch: now,
            time: now,
            mtu: DEFAULT_MTU,
            latency: Duration::ZERO,
            spins: 0,
            last_spin: false,
            congestion_experienced: false,
            routes: None,
        }
    }

    /// Returns whether the connection is not idle
    pub(super) fn step(&mut self) -> bool {
        self.blackhole_step(false, false)
    }

    /// Advance time until both connections are idle
    pub(super) fn drive(&mut self) {
        while self.step() {}
    }

    /// Advance time until both connections are idle, or after 100 steps have been executed
    ///
    /// Returns true if the amount of steps exceeds the bounds, because the connections never became
    /// idle
    pub(super) fn drive_bounded(&mut self, iters: usize) -> bool {
        for _ in 0..iters {
            if !self.step() {
                return false;
            }
        }

        true
    }

    pub(super) fn drive_client(&mut self) {
        let span = info_span!("client");
        let _guard = span.enter();
        self.client.drive(self.time);
        for (packet, buffer) in self.client.outbound.drain(..) {
            let packet_size = packet_size(&packet, &buffer);
            if packet_size > self.mtu {
                info!(packet_size, "dropping packet (max size exceeded)");
                continue;
            }
            if buffer[0] & packet::LONG_HEADER_FORM == 0 {
                let spin = buffer[0] & packet::SPIN_BIT != 0;
                self.spins += (spin == self.last_spin) as u64;
                self.last_spin = spin;
            }
            if let Some(ref socket) = self.client.socket {
                socket.send_to(&buffer, packet.destination).unwrap();
            }
            let client_addr = match &self.routes {
                Some(table) => table.resolve_client_to_server(packet.destination),
                None => (self.server.addr == packet.destination).then_some(self.client.addr),
            };
            if let Some(client_addr) = client_addr {
                let ecn = set_congestion_experienced(packet.ecn, self.congestion_experienced);
                self.server.inbound.push_back(Inbound {
                    recv_time: self.time + self.latency,
                    ecn,
                    packet: buffer.as_ref().into(),
                    remote: client_addr,
                    dst_ip: Some(packet.destination.ip()),
                });
            } else {
                debug!(?packet.destination, "no route from server to client for packet");
            }
        }
    }

    pub(super) fn drive_server(&mut self) {
        let span = info_span!("server");
        let _guard = span.enter();
        self.server.drive(self.time);
        for (packet, buffer) in self.server.outbound.drain(..) {
            let packet_size = packet_size(&packet, &buffer);
            if packet_size > self.mtu {
                info!(packet_size, "dropping packet (max size exceeded)");
                continue;
            }
            if let Some(ref socket) = self.server.socket {
                socket.send_to(&buffer, packet.destination).unwrap();
            }
            let server_addr = match &self.routes {
                Some(table) => table.resolve_server_to_client(packet.destination),
                None => (self.client.addr == packet.destination).then_some(self.server.addr),
            };
            if let Some(server_addr) = server_addr {
                let ecn = set_congestion_experienced(packet.ecn, self.congestion_experienced);
                self.client.inbound.push_back(Inbound {
                    recv_time: self.time + self.latency,
                    ecn,
                    packet: buffer.as_ref().into(),
                    remote: server_addr,
                    dst_ip: Some(packet.destination.ip()),
                });
            } else {
                debug!(?packet.destination, "no route from server to client for packet");
            }
        }
    }

    /// Drive both endpoints once, optionally preventing them from receiving traffic.
    ///
    /// Returns `false` if the connection is idle after the step.
    pub(super) fn blackhole_step(
        &mut self,
        server_blackhole: bool,
        client_blackhole: bool,
    ) -> bool {
        self.drive_client();
        if server_blackhole {
            self.server.inbound.clear();
        }
        self.drive_server();
        if client_blackhole {
            self.client.inbound.clear();
        }
        if self.client.is_idle() && self.server.is_idle() {
            return false;
        }

        self.advance_time()
    }

    pub(super) fn advance_time(&mut self) -> bool {
        let client_t = self.client.next_wakeup();
        let server_t = self.server.next_wakeup();
        match min_opt(client_t, server_t) {
            Some(t) if Some(t) == client_t => {
                if t != self.time {
                    self.time = self.time.max(t);
                    trace!("advancing to {:?} for client", self.time - self.epoch);
                }
                true
            }
            Some(t) if Some(t) == server_t => {
                if t != self.time {
                    self.time = self.time.max(t);
                    trace!("advancing to {:?} for server", self.time - self.epoch);
                }
                true
            }
            Some(_) => unreachable!(),
            None => false,
        }
    }

    pub(super) fn connect(&mut self) -> (ConnectionHandle, ConnectionHandle) {
        self.connect_with(client_config())
    }

    pub(super) fn connect_with(
        &mut self,
        config: ClientConfig,
    ) -> (ConnectionHandle, ConnectionHandle) {
        info!("connecting");
        let client_ch = self.begin_connect(config);
        self.drive();
        let server_ch = self.server.assert_accept();
        self.finish_connect(client_ch, server_ch);
        (client_ch, server_ch)
    }

    /// Just start connecting the client
    pub(super) fn begin_connect(&mut self, config: ClientConfig) -> ConnectionHandle {
        let span = info_span!("client");
        let _guard = span.enter();
        let (client_ch, client_conn) = self
            .client
            .connect(self.time, config, self.server.addr, "localhost")
            .unwrap();
        self.client.connections.insert(client_ch, client_conn);
        client_ch
    }

    fn finish_connect(&mut self, client_ch: ConnectionHandle, server_ch: ConnectionHandle) {
        assert_matches!(
            self.client_conn_mut(client_ch).poll(),
            Some(Event::HandshakeDataReady)
        );
        assert_matches!(
            self.client_conn_mut(client_ch).poll(),
            Some(Event::Connected)
        );
        assert_matches!(
            self.server_conn_mut(server_ch).poll(),
            Some(Event::HandshakeDataReady)
        );
        assert_matches!(
            self.server_conn_mut(server_ch).poll(),
            Some(Event::HandshakeConfirmed)
        );
        assert_matches!(
            self.server_conn_mut(server_ch).poll(),
            Some(Event::Connected)
        );
        assert_matches!(
            self.client_conn_mut(client_ch).poll(),
            Some(Event::HandshakeConfirmed)
        );
    }

    pub(super) fn client_conn_mut(&mut self, ch: ConnectionHandle) -> &mut Connection {
        self.client.connections.get_mut(&ch).unwrap()
    }

    pub(super) fn client_streams(&mut self, ch: ConnectionHandle) -> Streams<'_> {
        self.client_conn_mut(ch).streams()
    }

    pub(super) fn client_send(&mut self, ch: ConnectionHandle, s: StreamId) -> SendStream<'_> {
        self.client_conn_mut(ch).send_stream(s)
    }

    pub(super) fn client_recv(&mut self, ch: ConnectionHandle, s: StreamId) -> RecvStream<'_> {
        self.client_conn_mut(ch).recv_stream(s)
    }

    pub(super) fn client_datagrams(&mut self, ch: ConnectionHandle) -> Datagrams<'_> {
        self.client_conn_mut(ch).datagrams()
    }

    pub(super) fn server_conn_mut(&mut self, ch: ConnectionHandle) -> &mut Connection {
        self.server.connections.get_mut(&ch).unwrap()
    }

    pub(super) fn server_streams(&mut self, ch: ConnectionHandle) -> Streams<'_> {
        self.server_conn_mut(ch).streams()
    }

    pub(super) fn server_send(&mut self, ch: ConnectionHandle, s: StreamId) -> SendStream<'_> {
        self.server_conn_mut(ch).send_stream(s)
    }

    pub(super) fn server_recv(&mut self, ch: ConnectionHandle, s: StreamId) -> RecvStream<'_> {
        self.server_conn_mut(ch).recv_stream(s)
    }

    pub(super) fn server_datagrams(&mut self, ch: ConnectionHandle) -> Datagrams<'_> {
        self.server_conn_mut(ch).datagrams()
    }

    pub(super) fn addrs_to_server(&self) -> FourTuple {
        FourTuple {
            remote: self.server.addr,
            local_ip: Some(self.client.addr.ip()),
        }
    }

    pub(super) fn addrs_to_client(&self) -> FourTuple {
        FourTuple {
            remote: self.client.addr,
            local_ip: Some(self.server.addr.ip()),
        }
    }
}

/// Wrapper to a [`Pair`] which keeps handles to the client and server connections.
#[derive(derive_more::Deref, derive_more::DerefMut)]
pub(super) struct ConnPair {
    #[deref]
    #[deref_mut]
    pair: Pair,
    client_ch: ConnectionHandle,
    server_ch: ConnectionHandle,
}

impl ConnPair {
    pub(super) fn connect_with(mut pair: Pair, client_cfg: ClientConfig) -> Self {
        let (client_ch, server_ch) = pair.connect_with(client_cfg);
        Self {
            pair,
            client_ch,
            server_ch,
        }
    }

    /// Creates a [`ConnPair`] with the default [`EndpointConfig`] and given `server_cfg` and
    /// `client_cfg`.
    pub(super) fn with_default_endpoint(
        server_cfg: ServerConfig,
        client_cfg: ClientConfig,
    ) -> Self {
        let pair = Pair::new(Default::default(), server_cfg);
        Self::connect_with(pair, client_cfg)
    }

    /// Creates a [`ConnPair`] using the default [`EndpointConfig`] and configurations for the
    /// server and client as defined by [`server_config`] and [`client_config`], setting the
    /// [`TransportConfig`] given for each.
    pub(super) fn with_transport_cfg(
        server_transport: TransportConfig,
        client_transport: TransportConfig,
    ) -> Self {
        let server_cfg = ServerConfig {
            transport: Arc::new(server_transport),
            ..server_config()
        };
        let client_cfg = ClientConfig {
            transport: Arc::new(client_transport),
            ..client_config()
        };
        Self::with_default_endpoint(server_cfg, client_cfg)
    }

    pub(super) fn conn(&self, side: Side) -> &Connection {
        match side {
            Side::Client => self.pair.client.connections.get(&self.client_ch).unwrap(),
            Side::Server => self.pair.server.connections.get(&self.server_ch).unwrap(),
        }
    }

    pub(super) fn conn_mut(&mut self, side: Side) -> &mut Connection {
        match side {
            Side::Client => self
                .pair
                .client
                .connections
                .get_mut(&self.client_ch)
                .unwrap(),
            Side::Server => self
                .pair
                .server
                .connections
                .get_mut(&self.server_ch)
                .unwrap(),
        }
    }

    pub(super) fn poll_timeout(&mut self, side: Side) -> Option<Instant> {
        self.conn_mut(side).poll_timeout()
    }

    pub(super) fn poll(&mut self, side: Side) -> Option<Event> {
        self.conn_mut(side).poll()
    }

    pub(super) fn poll_endpoint_events(&mut self, side: Side) -> Option<EndpointEvent> {
        self.conn_mut(side).poll_endpoint_events()
    }

    pub(super) fn streams(&mut self, side: Side) -> Streams<'_> {
        self.conn_mut(side).streams()
    }

    pub(super) fn recv_stream(&mut self, side: Side, id: StreamId) -> RecvStream<'_> {
        self.conn_mut(side).recv_stream(id)
    }

    pub(super) fn send_stream(&mut self, side: Side, id: StreamId) -> SendStream<'_> {
        self.conn_mut(side).send_stream(id)
    }

    pub(super) fn open_path_ensure(
        &mut self,
        side: Side,
        network_path: FourTuple,
        initial_status: PathStatus,
    ) -> Result<(PathId, bool), PathError> {
        let now = self.pair.time;
        self.conn_mut(side)
            .open_path_ensure(network_path, initial_status, now)
    }

    pub(super) fn open_path(
        &mut self,
        side: Side,
        network_path: FourTuple,
        initial_status: PathStatus,
    ) -> Result<PathId, PathError> {
        let now = self.pair.time;
        self.conn_mut(side)
            .open_path(network_path, initial_status, now)
    }

    pub(super) fn close_path(
        &mut self,
        side: Side,
        path_id: PathId,
        error_code: VarInt,
    ) -> Result<(), ClosePathError> {
        let now = self.pair.time;
        self.conn_mut(side).close_path(now, path_id, error_code)
    }

    pub(super) fn paths(&self, side: Side) -> Vec<PathId> {
        self.conn(side).paths()
    }

    pub(super) fn path_status(
        &self,
        side: Side,
        path_id: PathId,
    ) -> Result<PathStatus, ClosedPath> {
        self.conn(side).path_status(path_id)
    }

    pub(super) fn network_path(
        &self,
        side: Side,
        path_id: PathId,
    ) -> Result<FourTuple, ClosedPath> {
        self.conn(side).network_path(path_id)
    }

    pub(super) fn set_path_status(
        &mut self,
        side: Side,
        path_id: PathId,
        status: PathStatus,
    ) -> Result<PathStatus, SetPathStatusError> {
        self.conn_mut(side).set_path_status(path_id, status)
    }

    pub(super) fn remote_path_status(&self, side: Side, path_id: PathId) -> Option<PathStatus> {
        self.conn(side).remote_path_status(path_id)
    }

    pub(super) fn set_path_max_idle_timeout(
        &mut self,
        side: Side,
        path_id: PathId,
        timeout: Option<Duration>,
    ) -> Result<Option<Duration>, ClosedPath> {
        self.conn_mut(side)
            .set_path_max_idle_timeout(path_id, timeout)
    }

    pub(super) fn set_path_keep_alive_interval(
        &mut self,
        side: Side,
        path_id: PathId,
        interval: Option<Duration>,
    ) -> Result<Option<Duration>, ClosedPath> {
        self.conn_mut(side)
            .set_path_keep_alive_interval(path_id, interval)
    }

    pub(super) fn poll_transmit(
        &mut self,
        side: Side,
        max_datagrams: NonZeroUsize,
        buf: &mut Vec<u8>,
    ) -> Option<Transmit> {
        let now = self.pair.time;
        self.conn_mut(side).poll_transmit(now, max_datagrams, buf)
    }

    pub(super) fn handle_event(&mut self, side: Side, event: ConnectionEvent) {
        self.conn_mut(side).handle_event(event)
    }

    pub(super) fn handle_timeout(&mut self, side: Side, now: Instant) {
        self.conn_mut(side).handle_timeout(now)
    }

    pub(super) fn close(&mut self, side: Side, now: Instant, error_code: VarInt, reason: Bytes) {
        self.conn_mut(side).close(now, error_code, reason)
    }

    pub(super) fn datagrams(&mut self, side: Side) -> Datagrams<'_> {
        self.conn_mut(side).datagrams()
    }

    pub(super) fn stats(&mut self, side: Side) -> ConnectionStats {
        self.conn_mut(side).stats()
    }

    pub(super) fn path_stats(&mut self, side: Side, path_id: PathId) -> Option<PathStats> {
        self.conn_mut(side).path_stats(path_id)
    }

    pub(super) fn ping(&mut self, side: Side) {
        self.conn_mut(side).ping()
    }

    pub(super) fn ping_path(&mut self, side: Side, path: PathId) -> Result<(), ClosedPath> {
        self.conn_mut(side).ping_path(path)
    }

    pub(super) fn force_key_update(&mut self, side: Side) {
        self.conn_mut(side).force_key_update()
    }

    pub(super) fn crypto_session(&self, side: Side) -> &dyn crypto::Session {
        self.conn(side).crypto_session()
    }

    pub(super) fn is_handshaking(&self, side: Side) -> bool {
        self.conn(side).is_handshaking()
    }

    pub(super) fn is_closed(&self, side: Side) -> bool {
        self.conn(side).is_closed()
    }

    pub(super) fn is_drained(&self, side: Side) -> bool {
        self.conn(side).is_drained()
    }

    pub(super) fn accepted_0rtt(&self, side: Side) -> bool {
        self.conn(side).accepted_0rtt()
    }

    pub(super) fn has_0rtt(&self, side: Side) -> bool {
        self.conn(side).has_0rtt()
    }

    pub(super) fn has_pending_retransmits(&self, side: Side) -> bool {
        self.conn(side).has_pending_retransmits()
    }

    pub(super) fn path_observed_address(
        &self,
        side: Side,
        path_id: PathId,
    ) -> Result<Option<SocketAddr>, ClosedPath> {
        self.conn(side).path_observed_address(path_id)
    }

    pub(super) fn rtt(&self, side: Side, path_id: PathId) -> Option<Duration> {
        self.conn(side).rtt(path_id)
    }

    pub(super) fn congestion_state(&self, side: Side, path_id: PathId) -> Option<&dyn Controller> {
        self.conn(side).congestion_state(path_id)
    }

    pub(super) fn set_max_concurrent_streams(&mut self, side: Side, dir: Dir, count: VarInt) {
        self.conn_mut(side).set_max_concurrent_streams(dir, count)
    }

    #[track_caller]
    pub(super) fn set_max_concurrent_paths(
        &mut self,
        side: Side,
        count: u32,
    ) -> Result<(), MultipathNotNegotiated> {
        let now = self.pair.time;
        let count = NonZeroU32::new(count).unwrap();
        self.conn_mut(side).set_max_concurrent_paths(now, count)
    }

    pub(super) fn max_concurrent_streams(&self, side: Side, dir: Dir) -> u64 {
        self.conn(side).max_concurrent_streams(dir)
    }

    pub(super) fn set_send_window(&mut self, side: Side, send_window: u64) {
        self.conn_mut(side).set_send_window(send_window)
    }

    pub(super) fn set_receive_window(&mut self, side: Side, receive_window: VarInt) {
        self.conn_mut(side).set_receive_window(receive_window)
    }

    #[track_caller]
    pub(super) fn reorder_inbound(&mut self, side: Side) {
        let inbound = match side {
            Side::Client => &mut self.pair.client.inbound,
            Side::Server => &mut self.pair.server.inbound,
        };
        let p = inbound.pop_front().unwrap();
        inbound.push_back(p);
    }

    pub(super) fn is_multipath_negotiated(&self, side: Side) -> bool {
        self.conn(side).is_multipath_negotiated()
    }

    /// Simulate a passive migration by assigning a new port to the address.
    #[track_caller]
    pub(super) fn passive_migration(&mut self, side: Side) -> SocketAddr {
        let address = match side {
            Side::Client => &mut self.pair.client.addr,
            Side::Server => &mut self.pair.server.addr,
        };

        let new_port = address.port().checked_add(1).unwrap();
        address.set_port(new_port);
        *address
    }

    pub(super) fn current_mtu(&self, side: Side) -> u16 {
        self.conn(side).current_mtu()
    }

    pub(super) fn add_nat_traversal_address(
        &mut self,
        side: Side,
        address: SocketAddr,
    ) -> Result<(), n0_nat_traversal::Error> {
        self.conn_mut(side).add_nat_traversal_address(address)
    }

    pub(super) fn remove_nat_traversal_address(
        &mut self,
        side: Side,
        address: SocketAddr,
    ) -> Result<(), n0_nat_traversal::Error> {
        self.conn_mut(side).remove_nat_traversal_address(address)
    }

    pub(super) fn get_local_nat_traversal_addresses(
        &self,
        side: Side,
    ) -> Result<Vec<SocketAddr>, n0_nat_traversal::Error> {
        self.conn(side).get_local_nat_traversal_addresses()
    }

    pub(super) fn get_remote_nat_traversal_addresses(
        &self,
        side: Side,
    ) -> Result<Vec<SocketAddr>, n0_nat_traversal::Error> {
        self.conn(side).get_remote_nat_traversal_addresses()
    }

    pub(super) fn initiate_nat_traversal_round(
        &mut self,
        side: Side,
    ) -> Result<Vec<SocketAddr>, n0_nat_traversal::Error> {
        let now = self.pair.time;
        self.conn_mut(side).initiate_nat_traversal_round(now)
    }

    pub(crate) fn handle_network_change(
        &mut self,
        side: Side,
        hint: Option<&dyn NetworkChangeHint>,
    ) {
        let now = self.pair.time;
        self.conn_mut(side).handle_network_change(hint, now);
    }
}

impl Default for Pair {
    fn default() -> Self {
        Self::new(Default::default(), server_config())
    }
}

pub(super) struct TestEndpoint {
    pub(super) endpoint: Endpoint,
    pub(super) addr: SocketAddr,
    socket: Option<UdpSocket>,
    timeout: Option<Instant>,
    pub(super) outbound: VecDeque<(Transmit, Bytes)>,
    delayed: VecDeque<(Transmit, Bytes)>,
    pub(super) inbound: VecDeque<Inbound>,
    pub(super) accepted: Option<Result<ConnectionHandle, ConnectionError>>,
    pub(super) connections: HashMap<ConnectionHandle, Connection>,
    drained_connections: HashSet<ConnectionHandle>,
    conn_events: HashMap<ConnectionHandle, VecDeque<ConnectionEvent>>,
    pub(super) captured_packets: Vec<Vec<u8>>,
    pub(super) capture_inbound_packets: bool,
    pub(super) handle_incoming: Box<dyn FnMut(&Incoming) -> IncomingConnectionBehavior>,
    pub(super) waiting_incoming: Vec<Incoming>,
}

pub(super) struct Inbound {
    pub(super) recv_time: Instant,
    pub(super) ecn: Option<EcnCodepoint>,
    pub(super) packet: BytesMut,
    pub(super) remote: SocketAddr,
    pub(super) dst_ip: Option<IpAddr>,
}

#[derive(Debug, Copy, Clone)]
pub(super) enum IncomingConnectionBehavior {
    Accept,
    Reject,
    Retry,
    Wait,
}

pub(super) fn validate_incoming(incoming: &Incoming) -> IncomingConnectionBehavior {
    if incoming.remote_address_validated() {
        IncomingConnectionBehavior::Accept
    } else {
        IncomingConnectionBehavior::Retry
    }
}

impl TestEndpoint {
    fn new(endpoint: Endpoint, addr: SocketAddr) -> Self {
        let socket = if env::var_os("SSLKEYLOGFILE").is_some() {
            let socket = UdpSocket::bind(addr).expect("failed to bind UDP socket");
            socket
                .set_read_timeout(Some(Duration::from_millis(10)))
                .unwrap();
            Some(socket)
        } else {
            None
        };
        Self {
            endpoint,
            addr,
            socket,
            timeout: None,
            outbound: VecDeque::new(),
            delayed: VecDeque::new(),
            inbound: VecDeque::new(),
            accepted: None,
            connections: HashMap::default(),
            drained_connections: HashSet::default(),
            conn_events: HashMap::default(),
            captured_packets: Vec::new(),
            capture_inbound_packets: false,
            handle_incoming: Box::new(|_| IncomingConnectionBehavior::Accept),
            waiting_incoming: Vec::new(),
        }
    }

    pub(super) fn drive(&mut self, now: Instant) {
        self.drive_incoming(now);
        self.drive_outgoing(now);
    }

    pub(super) fn drive_incoming(&mut self, now: Instant) {
        if let Some(ref socket) = self.socket {
            loop {
                let mut buf = [0; 8192];
                if socket.recv_from(&mut buf).is_err() {
                    break;
                }
            }
        }
        let buffer_size = self.endpoint.config().get_max_udp_payload_size() as usize;
        let mut buf = Vec::with_capacity(buffer_size);

        while self.inbound.front().is_some_and(|x| x.recv_time <= now) {
            let Inbound {
                recv_time,
                ecn,
                packet,
                remote,
                dst_ip,
            } = self.inbound.pop_front().unwrap();
            let network_path = FourTuple {
                remote,
                local_ip: dst_ip,
            };
            if let Some(event) =
                self.endpoint
                    .handle(recv_time, network_path, ecn, packet, &mut buf)
            {
                match event {
                    DatagramEvent::NewConnection(incoming) => {
                        match (self.handle_incoming)(&incoming) {
                            IncomingConnectionBehavior::Accept => {
                                let _ = self.try_accept(incoming, now);
                            }
                            IncomingConnectionBehavior::Reject => {
                                self.reject(incoming);
                            }
                            IncomingConnectionBehavior::Retry => {
                                self.retry(incoming);
                            }
                            IncomingConnectionBehavior::Wait => {
                                self.waiting_incoming.push(incoming);
                            }
                        }
                    }
                    DatagramEvent::ConnectionEvent(ch, event) => {
                        if self.capture_inbound_packets {
                            let packet = self.connections[&ch].decode_packet(&event);
                            self.captured_packets.extend(packet);
                        }

                        self.conn_events.entry(ch).or_default().push_back(event);
                    }
                    DatagramEvent::Response(transmit) => {
                        let size = transmit.size;
                        self.outbound.extend(split_transmit(transmit, &buf[..size]));
                        buf.clear();
                    }
                }
            }
        }
    }

    pub(super) fn drive_outgoing(&mut self, now: Instant) {
        let buffer_size = self.endpoint.config().get_max_udp_payload_size() as usize;
        let mut buf = Vec::with_capacity(buffer_size);

        loop {
            let mut endpoint_events: Vec<(ConnectionHandle, EndpointEvent)> = vec![];
            for (ch, conn) in self.connections.iter_mut() {
                if self.timeout.is_some_and(|x| x <= now) {
                    self.timeout = None;
                    conn.handle_timeout(now);
                }

                for (_, mut events) in self.conn_events.drain() {
                    for event in events.drain(..) {
                        conn.handle_event(event);
                    }
                }

                while let Some(event) = conn.poll_endpoint_events() {
                    endpoint_events.push((*ch, event));
                }
                while let Some(transmit) = conn.poll_transmit(now, MAX_DATAGRAMS, &mut buf) {
                    let size = transmit.size;
                    self.outbound.extend(split_transmit(transmit, &buf[..size]));
                    buf.clear();
                }
                self.timeout = conn.poll_timeout();
            }

            if endpoint_events.is_empty() {
                break;
            }

            for (ch, event) in endpoint_events {
                if !event.is_drained() && self.drained_connections.contains(&ch) {
                    // Calling self.endpoint.handle_event with a drained connection panics.
                    // For some reason, some tests rely on the fact that the drained event is handled twice?
                    continue;
                }
                if event.is_drained() {
                    self.drained_connections.insert(ch);
                }
                if let Some(event) = self.handle_event(ch, event)
                    && let Some(conn) = self.connections.get_mut(&ch)
                {
                    conn.handle_event(event);
                }
            }
        }
    }

    pub(super) fn next_wakeup(&self) -> Option<Instant> {
        let next_inbound = self.inbound.front().map(|x| x.recv_time);
        min_opt(self.timeout, next_inbound)
    }

    pub(super) fn is_idle(&self) -> bool {
        self.connections.values().all(|x| x.is_idle())
    }

    pub(super) fn delay_outbound(&mut self) {
        assert!(self.delayed.is_empty());
        mem::swap(&mut self.delayed, &mut self.outbound);
    }

    pub(super) fn finish_delay(&mut self) {
        self.outbound.extend(self.delayed.drain(..));
    }

    pub(super) fn try_accept(
        &mut self,
        incoming: Incoming,
        now: Instant,
    ) -> Result<ConnectionHandle, ConnectionError> {
        let mut buf = Vec::new();
        match self.endpoint.accept(incoming, now, &mut buf, None) {
            Ok((ch, conn)) => {
                self.connections.insert(ch, conn);
                self.accepted = Some(Ok(ch));
                Ok(ch)
            }
            Err(error) => {
                if let Some(transmit) = error.response {
                    let size = transmit.size;
                    self.outbound.extend(split_transmit(transmit, &buf[..size]));
                }
                self.accepted = Some(Err(error.cause.clone()));
                Err(error.cause)
            }
        }
    }

    pub(super) fn retry(&mut self, incoming: Incoming) {
        let mut buf = Vec::new();
        let transmit = self.endpoint.retry(incoming, &mut buf).unwrap();
        let size = transmit.size;
        self.outbound.extend(split_transmit(transmit, &buf[..size]));
    }

    pub(super) fn reject(&mut self, incoming: Incoming) {
        let mut buf = Vec::new();
        let transmit = self.endpoint.refuse(incoming, &mut buf);
        let size = transmit.size;
        self.outbound.extend(split_transmit(transmit, &buf[..size]));
    }

    #[track_caller]
    pub(super) fn assert_accept(&mut self) -> ConnectionHandle {
        self.accepted
            .take()
            .expect("server didn't try connecting")
            .expect("server experienced error connecting")
    }

    #[track_caller]
    pub(super) fn assert_accept_error(&mut self) -> ConnectionError {
        self.accepted
            .take()
            .expect("server didn't try connecting")
            .expect_err("server did unexpectedly connect without error")
    }

    #[track_caller]
    pub(super) fn assert_no_accept(&self) {
        assert!(self.accepted.is_none(), "server did unexpectedly connect")
    }
}

impl ::std::ops::Deref for TestEndpoint {
    type Target = Endpoint;
    fn deref(&self) -> &Endpoint {
        &self.endpoint
    }
}

impl ::std::ops::DerefMut for TestEndpoint {
    fn deref_mut(&mut self) -> &mut Endpoint {
        &mut self.endpoint
    }
}

pub(crate) fn subscribe() -> tracing::subscriber::DefaultGuard {
    let builder = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(tracing::Level::TRACE.into())
                .from_env_lossy(),
        )
        .with_line_number(true)
        .with_writer(|| TestWriter);
    // tracing uses std::time to trace time, which panics in wasm.
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    let builder = builder.without_time();
    tracing::subscriber::set_default(builder.finish())
}

struct TestWriter;

impl Write for TestWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        print!(
            "{}",
            str::from_utf8(buf).expect("tried to log invalid UTF-8")
        );
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        io::stdout().flush()
    }
}

pub(super) fn server_config() -> ServerConfig {
    let mut config = ServerConfig::with_crypto(Arc::new(server_crypto()));
    if !cfg!(feature = "bloom") {
        config
            .validation_token
            .sent(2)
            .log(Arc::new(SimpleTokenLog::default()));
    }
    config
}

pub(super) fn server_config_with_cert(
    cert: CertificateDer<'static>,
    key: PrivateKeyDer<'static>,
) -> ServerConfig {
    let mut config = ServerConfig::with_crypto(Arc::new(server_crypto_with_cert(cert, key)));
    config
        .validation_token
        .sent(2)
        .log(Arc::new(SimpleTokenLog::default()));
    config
}

pub(super) fn server_crypto() -> QuicServerConfig {
    server_crypto_inner(None, None)
}

pub(super) fn server_crypto_with_alpn(alpn: Vec<Vec<u8>>) -> QuicServerConfig {
    server_crypto_inner(None, Some(alpn))
}

pub(super) fn server_crypto_with_cert(
    cert: CertificateDer<'static>,
    key: PrivateKeyDer<'static>,
) -> QuicServerConfig {
    server_crypto_inner(Some((cert, key)), None)
}

fn server_crypto_inner(
    identity: Option<(CertificateDer<'static>, PrivateKeyDer<'static>)>,
    alpn: Option<Vec<Vec<u8>>>,
) -> QuicServerConfig {
    let (cert, key) = identity.unwrap_or_else(|| {
        (
            CERTIFIED_KEY.cert.der().clone(),
            PrivateKeyDer::Pkcs8(CERTIFIED_KEY.signing_key.serialize_der().into()),
        )
    });

    let mut config = QuicServerConfig::inner(vec![cert], key).unwrap();
    if let Some(alpn) = alpn {
        config.alpn_protocols = alpn;
    }

    config.try_into().unwrap()
}

pub(super) fn client_config() -> ClientConfig {
    ClientConfig::new(Arc::new(client_crypto()))
}

pub(super) fn client_config_with_deterministic_pns() -> ClientConfig {
    let mut cfg = ClientConfig::new(Arc::new(client_crypto()));
    let mut transport = TransportConfig::default();
    transport.deterministic_packet_numbers(true);
    cfg.transport = Arc::new(transport);
    cfg
}

pub(super) fn client_config_with_certs(certs: Vec<CertificateDer<'static>>) -> ClientConfig {
    ClientConfig::new(Arc::new(client_crypto_inner(Some(certs), None)))
}

pub(super) fn client_crypto() -> QuicClientConfig {
    client_crypto_inner(None, None)
}

pub(super) fn client_crypto_with_alpn(protocols: Vec<Vec<u8>>) -> QuicClientConfig {
    client_crypto_inner(None, Some(protocols))
}

fn client_crypto_inner(
    certs: Option<Vec<CertificateDer<'static>>>,
    alpn: Option<Vec<Vec<u8>>>,
) -> QuicClientConfig {
    let mut roots = rustls::RootCertStore::empty();
    for cert in certs.unwrap_or_else(|| vec![CERTIFIED_KEY.cert.der().clone()]) {
        roots.add(cert).unwrap();
    }

    let mut inner = QuicClientConfig::inner(
        WebPkiServerVerifier::builder_with_provider(Arc::new(roots), configured_provider())
            .build()
            .unwrap(),
    );
    inner.key_log = Arc::new(KeyLogFile::new());
    if let Some(alpn) = alpn {
        inner.alpn_protocols = alpn;
    }

    inner.try_into().unwrap()
}

pub(super) fn min_opt<T: Ord>(x: Option<T>, y: Option<T>) -> Option<T> {
    match (x, y) {
        (Some(x), Some(y)) => Some(cmp::min(x, y)),
        (Some(x), _) => Some(x),
        (_, Some(y)) => Some(y),
        _ => None,
    }
}

/// The maximum of datagrams TestEndpoint will produce via `poll_transmit`
const MAX_DATAGRAMS: NonZeroUsize = NonZeroUsize::new(10).expect("known");

fn split_transmit(transmit: Transmit, buffer: &[u8]) -> Vec<(Transmit, Bytes)> {
    let mut buffer = Bytes::copy_from_slice(buffer);
    let segment_size = match transmit.segment_size {
        Some(segment_size) => segment_size,
        _ => return vec![(transmit, buffer)],
    };

    let mut transmits = Vec::new();
    while !buffer.is_empty() {
        let end = segment_size.min(buffer.len());

        let contents = buffer.split_to(end);
        transmits.push((
            Transmit {
                destination: transmit.destination,
                size: contents.len(),
                ecn: transmit.ecn,
                segment_size: None,
                src_ip: transmit.src_ip,
            },
            contents,
        ));
    }

    transmits
}

fn packet_size(transmit: &Transmit, buffer: &Bytes) -> usize {
    if transmit.segment_size.is_some() {
        panic!("This transmit is meant to be split into multiple packets!");
    }

    buffer.len()
}

fn set_congestion_experienced(
    x: Option<EcnCodepoint>,
    congestion_experienced: bool,
) -> Option<EcnCodepoint> {
    x.map(|codepoint| match congestion_experienced {
        true => EcnCodepoint::Ce,
        false => codepoint,
    })
}

pub(crate) static SERVER_PORTS: LazyLock<Mutex<RangeFrom<u16>>> =
    LazyLock::new(|| Mutex::new(4433..));
pub(crate) static CLIENT_PORTS: LazyLock<Mutex<RangeFrom<u16>>> =
    LazyLock::new(|| Mutex::new(44433..));
pub(crate) static CERTIFIED_KEY: LazyLock<rcgen::CertifiedKey<rcgen::KeyPair>> =
    LazyLock::new(|| rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap());

#[derive(Default)]
struct SimpleTokenLog(Mutex<HashSet<u128>>);

impl TokenLog for SimpleTokenLog {
    fn check_and_insert(
        &self,
        nonce: u128,
        _issued: SystemTime,
        _lifetime: Duration,
    ) -> Result<(), TokenReuseError> {
        if self.0.lock().unwrap().insert(nonce) {
            Ok(())
        } else {
            Err(TokenReuseError)
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct RoutingTable {
    client_routes: Vec<(SocketAddr, usize)>,
    server_routes: Vec<(SocketAddr, usize)>,
}

impl RoutingTable {
    pub(super) fn from_routes(
        client_routes: Vec<(SocketAddr, usize)>,
        server_routes: Vec<(SocketAddr, usize)>,
    ) -> Self {
        for (_, idx) in client_routes.iter() {
            assert!(*idx < server_routes.len(), "routing table corrupt");
        }
        for (_, idx) in server_routes.iter() {
            assert!(*idx < client_routes.len(), "routing table corrupt");
        }
        Self {
            client_routes,
            server_routes,
        }
    }

    pub(super) fn simple_symmetric(
        client_addrs: impl IntoIterator<Item = SocketAddr>,
        server_addrs: impl IntoIterator<Item = SocketAddr>,
    ) -> Self {
        let mut client_routes = Vec::new();
        let mut server_routes = Vec::new();

        for (idx, (client_addr, server_addr)) in
            client_addrs.into_iter().zip(server_addrs).enumerate()
        {
            client_routes.push((client_addr, idx));
            server_routes.push((server_addr, idx));
        }

        Self {
            client_routes,
            server_routes,
        }
    }

    /// Returns the client address a server would see on an incoming packet if
    /// sent to given server address.
    ///
    /// Returns none if the packet would not find a route and get lost.
    fn resolve_client_to_server(&self, server_addr: SocketAddr) -> Option<SocketAddr> {
        let (_, client_addr_idx) = self
            .server_routes
            .iter()
            .find(|(addr, _)| *addr == server_addr)?;
        let (client_addr, _) = self.client_routes.get(*client_addr_idx)?;
        Some(*client_addr)
    }

    /// Returns the server address a client would see on an incoming packet if
    /// sent to given client address.
    ///
    /// Returns none if the packet would not find a route and get lost.
    fn resolve_server_to_client(&self, client_addr: SocketAddr) -> Option<SocketAddr> {
        let (_, server_addr_idx) = self
            .client_routes
            .iter()
            .find(|(addr, _)| *addr == client_addr)?;
        let (server_addr, _) = self.server_routes.get(*server_addr_idx)?;
        Some(*server_addr)
    }

    /// Adds a new route from an existing server address (identified by index) to a new client address.
    pub(super) fn add_client_route(&mut self, client_addr: SocketAddr, server_addr_idx: usize) {
        assert!(server_addr_idx < self.server_routes.len());
        self.client_routes.push((client_addr, server_addr_idx));
    }

    /// Adds a new route from an existing client address (identified by index) to a new server address.
    pub(super) fn add_server_route(&mut self, server_addr: SocketAddr, client_addr_idx: usize) {
        assert!(client_addr_idx < self.client_routes.len());
        self.server_routes.push((server_addr, client_addr_idx));
    }

    pub(super) fn client_addr(&self, idx: usize) -> Option<SocketAddr> {
        let (addr, _) = self.client_routes.get(idx)?;
        Some(*addr)
    }

    pub(super) fn server_addr(&self, idx: usize) -> Option<SocketAddr> {
        let (addr, _) = self.server_routes.get(idx)?;
        Some(*addr)
    }

    pub(super) fn sim_client_migration(
        &mut self,
        route_idx: usize,
        modify_fn: impl Fn(SocketAddr) -> SocketAddr,
    ) -> Option<SocketAddr> {
        let route = self.client_routes.get_mut(route_idx)?;
        route.0 = modify_fn(route.0);
        Some(route.0)
    }

    pub(super) fn sim_server_migration(
        &mut self,
        route_idx: usize,
        modify_fn: impl Fn(SocketAddr) -> SocketAddr,
    ) -> Option<SocketAddr> {
        let route = self.server_routes.get_mut(route_idx)?;
        route.0 = modify_fn(route.0);
        Some(route.0)
    }
}
