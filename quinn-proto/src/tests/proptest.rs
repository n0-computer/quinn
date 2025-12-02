use std::{
    collections::VecDeque,
    net::{IpAddr, Ipv6Addr, SocketAddr},
    num::NonZeroU32,
    sync::Arc,
    time::{Duration, Instant},
};

use proptest::{
    collection::vec,
    prelude::{Strategy, any},
    prop_assert,
};
use rand::{RngCore, SeedableRng, rngs::StdRng};
use test_strategy::{Arbitrary, proptest};
use tracing::{debug, trace};

use crate::{
    Connection, ConnectionHandle, Endpoint, EndpointConfig, PathId, PathStatus, StreamId,
    TransportConfig,
    tests::{DEFAULT_MTU, Pair, TestEndpoint, client_config, server_config, subscribe},
};

const CLIENT_ADDRS: [SocketAddr; MAX_PATHS as usize] = [
    SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 44433u16),
    SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 44434u16),
    SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 44435u16),
];
const SERVER_ADDRS: [SocketAddr; MAX_PATHS as usize] = [
    SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 4433u16),
    SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 4434u16),
    SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 4435u16),
];
const MAX_PATHS: u32 = 3;

#[derive(Debug, Clone, Copy, Arbitrary)]
enum Side {
    Server,
    Client,
}

#[derive(Debug, Clone, Copy, Arbitrary)]
enum TestOp {
    Drive(Side),
    AdvanceTime,
    DropInbound(Side),
    ReorderInbound(Side),
    ForceKeyUpdate(Side),
    OpenPath(Side, PathKind, #[strategy(0..3usize)] usize),
    ClosePath(Side, #[strategy(0..MAX_PATHS as usize)] usize, u32),
    PathSetStatus(Side, #[strategy(0..MAX_PATHS as usize)] usize, PathKind),
    StreamOp(Side, StreamOp),
}

#[derive(Debug, Clone, Copy, Arbitrary)]
enum PathKind {
    Available,
    Backup,
}

#[derive(Debug, Clone, Copy, Arbitrary)]
enum StreamKind {
    Uni,
    Bi,
}

impl From<StreamKind> for crate::Dir {
    fn from(value: StreamKind) -> Self {
        match value {
            StreamKind::Bi => crate::Dir::Bi,
            StreamKind::Uni => crate::Dir::Uni,
        }
    }
}

/// We *basically* only operate with 3 streams concurrently at the moment
/// (even though more might be opened at a time).
#[derive(Debug, Clone, Copy, Arbitrary)]
enum StreamOp {
    Open(StreamKind),
    Send {
        #[strategy(0..3usize)]
        stream: usize,
        #[strategy(0..10_000usize)]
        num_bytes: usize,
    },
    Finish(#[strategy(0..3usize)] usize),
    Reset(#[strategy(0..3usize)] usize, u32),

    Accept(StreamKind),
    Receive(#[strategy(0..3usize)] usize, bool),
    Stop(#[strategy(0..3usize)] usize, u32),
}

impl StreamOp {
    fn run(self, pair: &mut Pair, state: &mut State) {
        let Some(conn) = state.conn(pair) else {
            return;
        };
        // We generally ignore application-level errors. It's legal to call these APIs, so we do. We don't expect them to work all the time.
        match self {
            StreamOp::Open(kind) => state.send_streams.extend(conn.streams().open(kind.into())),
            StreamOp::Send { stream, num_bytes } => {
                if let Some(&stream_id) = state.send_streams.get(stream) {
                    let data = vec![0; num_bytes];
                    if let Some(bytes) = conn.send_stream(stream_id).write(&data).ok() {
                        trace!(attempted_write = %num_bytes, actually_written = %bytes, "random interaction: Wrote stream bytes");
                    }
                }
            }
            StreamOp::Finish(stream) => {
                if let Some(&stream_id) = state.send_streams.get(stream) {
                    conn.send_stream(stream_id).finish().ok();
                }
            }
            StreamOp::Reset(stream, code) => {
                if let Some(&stream_id) = state.send_streams.get(stream) {
                    conn.send_stream(stream_id).reset(code.into()).ok();
                }
            }
            StreamOp::Accept(kind) => state
                .recv_streams
                .extend(conn.streams().accept(kind.into())),
            StreamOp::Receive(stream, ordered) => {
                if let Some(&stream_id) = state.recv_streams.get(stream) {
                    if let Some(mut chunks) = conn.recv_stream(stream_id).read(ordered).ok() {
                        if let Ok(Some(chunk)) = chunks.next(usize::MAX) {
                            trace!(chunk_len = %chunk.bytes.len(), offset = %chunk.offset, "read from stream");
                        }
                    }
                }
            }
            StreamOp::Stop(stream, code) => {
                if let Some(&stream_id) = state.recv_streams.get(stream) {
                    conn.recv_stream(stream_id).stop(code.into()).ok();
                }
            }
        };
    }
}

struct State {
    send_streams: Vec<StreamId>,
    recv_streams: Vec<StreamId>,
    path_ids: Vec<PathId>,
    handle: ConnectionHandle,
    side: Side,
}

impl State {
    fn new(side: Side, handle: ConnectionHandle) -> Self {
        Self {
            send_streams: Vec::new(),
            recv_streams: Vec::new(),
            path_ids: vec![PathId::ZERO],
            handle,
            side,
        }
    }

    fn endpoint<'a>(&self, pair: &'a mut Pair) -> &'a mut TestEndpoint {
        match self.side {
            Side::Server => &mut pair.server,
            Side::Client => &mut pair.client,
        }
    }

    fn conn<'a>(&self, pair: &'a mut Pair) -> Option<&'a mut Connection> {
        self.endpoint(pair).connections.get_mut(&self.handle)
    }

    fn path_from_idx(&self, idx: usize) -> Option<&PathId> {
        self.path_ids
            .get(idx.clamp(0, self.path_ids.len().saturating_sub(1)))
    }
}

fn run_random_interaction(pair: &mut Pair, interactions: Vec<TestOp>) {
    let mut client_cfg = client_config();
    client_cfg.transport = Arc::new(multipath_transport_config());
    let (client_ch, server_ch) = pair.connect_with(client_cfg);
    pair.drive(); // finish establishing the connection;
    debug!("INTERACTION SETUP FINISHED");
    pair.client_conn_mut(client_ch).panic_on_transport_error();
    pair.server_conn_mut(server_ch).panic_on_transport_error();
    let mut client = State::new(Side::Client, client_ch);
    let mut server = State::new(Side::Server, server_ch);

    for interaction in interactions {
        debug!(?interaction, "INTERACTION STEP");
        let now = pair.time;
        match interaction {
            TestOp::Drive(Side::Client) => pair.drive_client(),
            TestOp::Drive(Side::Server) => pair.drive_server(),
            TestOp::AdvanceTime => {
                // If we advance during idle, we just immediately hit the idle timeout
                if !pair.client.is_idle() || !pair.server.is_idle() {
                    pair.advance_time();
                }
            }
            TestOp::DropInbound(Side::Client) => {
                debug!(len = pair.client.inbound.len(), "dropping inbound");
                pair.client.inbound.clear();
            }
            TestOp::DropInbound(Side::Server) => {
                debug!(len = pair.server.inbound.len(), "dropping inbound");
                pair.server.inbound.clear();
            }
            TestOp::ReorderInbound(Side::Client) => reorder(&mut pair.client.inbound),
            TestOp::ReorderInbound(Side::Server) => reorder(&mut pair.server.inbound),
            TestOp::ForceKeyUpdate(Side::Client) => {
                if let Some(conn) = client.conn(pair) {
                    conn.force_key_update()
                }
            }
            TestOp::ForceKeyUpdate(Side::Server) => {
                if let Some(conn) = server.conn(pair) {
                    conn.force_key_update()
                }
            }
            TestOp::OpenPath(side, path_kind, addr) => {
                let remote = match side {
                    Side::Client => SERVER_ADDRS[addr],
                    Side::Server => CLIENT_ADDRS[addr],
                };
                let initial_status = match path_kind {
                    PathKind::Available => PathStatus::Available,
                    PathKind::Backup => PathStatus::Backup,
                };
                let state = match side {
                    Side::Client => &mut client,
                    Side::Server => &mut server,
                };
                if let Some(conn) = state.conn(pair) {
                    if let Ok(path_id) = conn.open_path(remote, initial_status, now) {
                        state.path_ids.push(path_id);
                    }
                }
            }
            TestOp::ClosePath(side, path_idx, error_code) => {
                let state = match side {
                    Side::Client => &mut client,
                    Side::Server => &mut server,
                };
                if let Some(conn) = state.conn(pair) {
                    if let Some(&path_id) = state.path_from_idx(path_idx) {
                        conn.close_path(now, path_id, error_code.into()).ok();
                        state.path_ids.retain(|id| *id != path_id);
                    }
                }
            }
            TestOp::PathSetStatus(side, path_idx, status) => {
                let state = match side {
                    Side::Client => &mut client,
                    Side::Server => &mut server,
                };
                let status = match status {
                    PathKind::Available => PathStatus::Available,
                    PathKind::Backup => PathStatus::Backup,
                };
                if let Some(conn) = state.conn(pair) {
                    if let Some(&path_id) = state.path_from_idx(path_idx) {
                        conn.set_path_status(path_id, status).ok();
                    }
                }
            }
            TestOp::StreamOp(side, stream_op) => match side {
                Side::Client => stream_op.run(pair, &mut client),
                Side::Server => stream_op.run(pair, &mut server),
            },
        }

        while let Some(event) = client.conn(pair).and_then(Connection::poll) {
            match event {
                crate::Event::Path(crate::PathEvent::Abandoned { id, .. })
                | crate::Event::Path(crate::PathEvent::Closed { id, .. }) => {
                    client.path_ids.retain(|path_id| *path_id != id);
                }
                _ => {}
            }
        }

        while let Some(event) = server.conn(pair).and_then(Connection::poll) {
            match event {
                crate::Event::Path(crate::PathEvent::Abandoned { id, .. })
                | crate::Event::Path(crate::PathEvent::Closed { id, .. }) => {
                    server.path_ids.retain(|path_id| *path_id != id);
                }
                _ => {}
            }
        }
    }
}

fn reorder<T>(vec: &mut VecDeque<T>) {
    if let Some(item) = vec.pop_front() {
        vec.push_back(item);
    }
}

#[proptest]
fn random_interaction(
    #[strategy(any::<[u8; 32]>().no_shrink())] seed: [u8; 32],
    #[strategy(vec(any::<TestOp>(), 0..100))] interactions: Vec<TestOp>,
) {
    let _guard = subscribe(); // TODO(matheus23): Do this in a way that allows us to discard output from the non-final interaction.
    let mut pair = Pair::default_deterministic(seed);
    run_random_interaction(&mut pair, interactions);

    prop_assert!(!pair.drive_bounded(1000), "connection never became idle");
}

fn setup_deterministic_with_multipath(seed: [u8; 32]) -> Pair {
    let mut rng = StdRng::from_seed(seed);
    let mut client_seed = [0u8; 32];
    let mut server_seed = [0u8; 32];
    rng.fill_bytes(&mut client_seed);
    rng.fill_bytes(&mut server_seed);

    let mut cfg = server_config();
    let transport = multipath_transport_config();
    cfg.transport = Arc::new(transport);

    let mut client_config = EndpointConfig::default();
    let mut server_config = EndpointConfig::default();
    client_config.rng_seed(Some(client_seed));
    server_config.rng_seed(Some(server_seed));

    let server = Endpoint::new(Arc::new(server_config), Some(Arc::new(cfg)), true, None);
    let client = Endpoint::new(Arc::new(client_config), None, true, None);

    let now = Instant::now();
    let mut server = TestEndpoint::new(server, SERVER_ADDRS[0]);
    let mut client = TestEndpoint::new(client, CLIENT_ADDRS[0]);
    server.multipath_addrs = SERVER_ADDRS.into();
    client.multipath_addrs = CLIENT_ADDRS.into();
    Pair {
        server,
        client,
        epoch: now,
        time: now,
        mtu: DEFAULT_MTU,
        latency: Duration::from_millis(1),
        spins: 0,
        last_spin: false,
        congestion_experienced: false,
    }
}

fn multipath_transport_config() -> TransportConfig {
    let mut transport = TransportConfig::default();
    transport.deterministic_packet_numbers(true);
    transport.mtu_discovery_config(None); // TODO(matheus23): Disabled for clearer logs. Need to re-enable!
    // enable multipath
    transport.max_concurrent_multipath_paths = NonZeroU32::new(MAX_PATHS);
    transport
}

#[proptest(cases = 2560)]
fn random_interaction_multipath(
    #[strategy(any::<[u8; 32]>().no_shrink())] seed: [u8; 32],
    #[strategy(vec(any::<TestOp>(), 0..100))] interactions: Vec<TestOp>,
) {
    // let _guard = subscribe(); // TODO(matheus23): Do this in a way that allows us to discard output from the non-final interaction.
    let mut pair = setup_deterministic_with_multipath(seed);
    run_random_interaction(&mut pair, interactions);

    prop_assert!(!pair.drive_bounded(1000), "connection never became idle");
}

#[test]
fn regression_unset_packet_acked() {
    let seed: [u8; 32] = [
        60, 116, 60, 165, 136, 238, 239, 131, 14, 159, 221, 16, 80, 60, 30, 15, 15, 69, 133, 33,
        89, 203, 28, 107, 123, 117, 6, 54, 215, 244, 47, 1,
    ];
    let interactions = vec![
        TestOp::OpenPath(Side::Client, PathKind::Available, 0),
        TestOp::ClosePath(Side::Client, 0, 0),
        TestOp::Drive(Side::Client),
        TestOp::AdvanceTime,
        TestOp::Drive(Side::Server),
        TestOp::DropInbound(Side::Client),
    ];

    let _guard = subscribe();
    let mut pair = setup_deterministic_with_multipath(seed);
    run_random_interaction(&mut pair, interactions);

    assert!(!pair.drive_bounded(100), "connection never became idle");
}

#[test]
fn regression_invalid_key() {
    let seed = [
        41, 24, 232, 72, 136, 73, 31, 115, 14, 101, 61, 219, 30, 168, 130, 122, 120, 238, 6, 130,
        117, 84, 250, 190, 50, 237, 14, 167, 60, 5, 140, 149,
    ];
    let interactions = vec![
        TestOp::OpenPath(Side::Client, PathKind::Available, 0),
        TestOp::AdvanceTime,
        TestOp::Drive(Side::Client),
        TestOp::OpenPath(Side::Client, PathKind::Available, 0),
    ];

    let _guard = subscribe();
    let mut pair = setup_deterministic_with_multipath(seed);
    run_random_interaction(&mut pair, interactions);

    assert!(!pair.drive_bounded(100), "connection never became idle");
}

#[test]
fn regression_key_update_error() {
    let seed: [u8; 32] = [
        68, 93, 15, 237, 88, 31, 93, 255, 246, 51, 203, 224, 20, 124, 107, 163, 143, 43, 193, 187,
        208, 54, 158, 239, 190, 82, 198, 62, 91, 51, 53, 226,
    ];
    let interactions = vec![
        TestOp::OpenPath(Side::Client, PathKind::Available, 0),
        TestOp::Drive(Side::Client),
        TestOp::ForceKeyUpdate(Side::Server),
    ];

    let _guard = subscribe();
    let mut pair = setup_deterministic_with_multipath(seed);
    run_random_interaction(&mut pair, interactions);

    assert!(!pair.drive_bounded(100), "connection never became idle");
}

#[test]
fn regression_never_idle() {
    let seed: [u8; 32] = [
        172, 221, 115, 106, 31, 22, 213, 3, 199, 6, 128, 220, 47, 215, 159, 233, 97, 21, 254, 207,
        48, 180, 255, 97, 33, 29, 11, 76, 219, 138, 87, 57,
    ];
    let interactions = vec![
        TestOp::OpenPath(Side::Client, PathKind::Available, 1),
        TestOp::PathSetStatus(Side::Server, 0, PathKind::Backup),
        TestOp::ClosePath(Side::Client, 0, 0),
    ];

    let _guard = subscribe();
    let mut pair = setup_deterministic_with_multipath(seed);
    run_random_interaction(&mut pair, interactions);

    assert!(!pair.drive_bounded(100), "connection never became idle");
}

#[test]
fn regression_never_idle2() {
    let seed: [u8; 32] = [
        201, 119, 56, 156, 173, 104, 243, 75, 174, 248, 232, 226, 240, 106, 118, 59, 226, 245, 138,
        50, 100, 4, 245, 65, 8, 174, 18, 189, 72, 10, 166, 160,
    ];
    let interactions = vec![
        TestOp::OpenPath(Side::Client, PathKind::Backup, 1),
        TestOp::ClosePath(Side::Client, 0, 0),
        TestOp::Drive(Side::Client),
        TestOp::DropInbound(Side::Server),
        TestOp::PathSetStatus(Side::Client, 0, PathKind::Available),
    ];

    let _guard = subscribe();
    let mut pair = setup_deterministic_with_multipath(seed);
    run_random_interaction(&mut pair, interactions);

    // We needed to increase the bounds. It eventually times out.
    assert!(!pair.drive_bounded(1000), "connection never became idle");
}
