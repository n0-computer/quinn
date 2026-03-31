use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4};

use bytes::Bytes;
use test_strategy::Arbitrary;
use tracing::{debug, error, info, trace};

use crate::{
    ClientConfig, Connection, ConnectionHandle, Dir, FourTuple, PathId, PathStatus, Side, StreamId,
    tests::{Pair, TestEndpoint},
};

#[derive(Debug, Clone, Copy, Arbitrary)]
pub(super) enum TestOp {
    /// Finish the started connection attempt by driving the connection and accepting it on the server side.
    ///
    /// If `drive_fully` is set to `true`, then this will call `pair.drive()` before and after to make
    /// sure the connection drove to completion.
    FinishConnect { drive_fully: bool },
    /// Drive the endpoint on the given `side`, processing all pending I/O.
    Drive { side: Side },
    /// Advance the simulated time forward, unless both endpoints are idle.
    AdvanceTime,
    /// Drop all pending inbound packets for the endpoint on the given `side`.
    DropInbound { side: Side },
    /// Move the first inbound packet to the back of the queue, simulating reordering.
    ReorderInbound { side: Side },
    /// Force a TLS key update on the connection belonging to `side`.
    ForceKeyUpdate { side: Side },
    /// Simulate a passive address migration by changing the address at `addr_idx` in the routing
    /// table for `side`.
    PassiveMigration {
        side: Side,
        /// Index into the routing table's address list to migrate.
        #[strategy(0..3usize)]
        addr_idx: usize,
    },
    /// Open a new network path from `side` to the remote's address at `addr_idx`.
    OpenPath {
        side: Side,
        /// Initial status to assign to the newly opened path.
        status: PathStatus,
        /// Index used to look up the remote address from the routing table.
        #[strategy(0..3usize)]
        addr_idx: usize,
    },
    /// Close the path at `path_idx` on the connection belonging to `side`.
    ClosePath {
        side: Side,
        /// Index into the connection's list of path IDs.
        #[strategy(0..3usize)]
        path_idx: usize,
        /// Application-level error code sent with the path closure.
        error_code: u32,
    },
    /// Update the status of an existing path on the connection belonging to `side`.
    PathSetStatus {
        side: Side,
        /// Index into the connection's list of path IDs.
        #[strategy(0..3usize)]
        path_idx: usize,
        /// New status to assign to the path.
        status: PathStatus,
    },
    /// Perform a stream-level operation on the connection belonging to `side`.
    StreamOp { side: Side, stream_op: StreamOp },
    /// Send a datagram.
    SendDatagram {
        side: Side,
        #[strategy(0..2000usize)]
        size: usize,
        drop: bool,
    },
    /// Read all datagrams on given `side`.
    ReadDatagrams { side: Side },
    /// Close the connection belonging to `side`.
    CloseConn {
        side: Side,
        /// Application-level error code sent with the connection close.
        error_code: u32,
    },
    /// Register a NAT traversal address for `side`'s own address at `addr_idx`.
    AddHpAddr {
        side: Side,
        /// Index used to look up the address from the routing table.
        #[strategy(0..3usize)]
        addr_idx: usize,
    },
    /// Initiate a NAT traversal round on the connection belonging to `side`.
    InitiateHpRound { side: Side },
}

/// We *basically* only operate with 3 streams concurrently at the moment
/// (even though more might be opened at a time).
#[derive(Debug, Clone, Copy, Arbitrary)]
pub(super) enum StreamOp {
    Open(Dir),
    Send {
        #[strategy(0..3usize)]
        stream: usize,
        #[strategy(0..10_000usize)]
        num_bytes: usize,
    },
    Finish(#[strategy(0..3usize)] usize),
    Reset(#[strategy(0..3usize)] usize, u32),

    Accept(Dir),
    Receive(#[strategy(0..3usize)] usize, bool),
    Stop(#[strategy(0..3usize)] usize, u32),
}

pub(super) struct State {
    send_streams: Vec<StreamId>,
    recv_streams: Vec<StreamId>,
    handle: Option<ConnectionHandle>,
    side: Side,
}

impl TestOp {
    fn run(self, pair: &mut Pair, client: &mut State, server: &mut State) -> Option<()> {
        let now = pair.time;
        match self {
            Self::FinishConnect { drive_fully } => {
                if drive_fully {
                    pair.drive();
                }

                let accept = pair
                    .server
                    .accepted
                    .take()?
                    .inspect_err(|err| error!(?err, "FinishConnect error accepting connection"))
                    .ok()?;
                server.handle = Some(accept);

                if drive_fully {
                    pair.drive();
                }
            }
            Self::Drive { side: Side::Client } => pair.drive_client(),
            Self::Drive { side: Side::Server } => pair.drive_server(),
            Self::AdvanceTime => {
                // If we advance during idle, we just immediately hit the idle timeout
                if !pair.client.is_idle() || !pair.server.is_idle() {
                    pair.advance_time();
                }
            }
            Self::DropInbound { side: Side::Client } => {
                debug!(len = pair.client.inbound.len(), "dropping inbound");
                pair.client.inbound.clear();
            }
            Self::DropInbound { side: Side::Server } => {
                debug!(len = pair.server.inbound.len(), "dropping inbound");
                pair.server.inbound.clear();
            }
            Self::ReorderInbound { side: Side::Client } => {
                let item = pair.client.inbound.pop_front()?;
                pair.client.inbound.push_back(item);
            }
            Self::ReorderInbound { side: Side::Server } => {
                let item = pair.server.inbound.pop_front()?;
                pair.server.inbound.push_back(item);
            }
            Self::ForceKeyUpdate { side: Side::Client } => client.conn(pair)?.force_key_update(),
            Self::ForceKeyUpdate { side: Side::Server } => server.conn(pair)?.force_key_update(),
            Self::PassiveMigration {
                side: Side::Client,
                addr_idx,
            } => {
                let routes = pair.routes.as_mut()?;
                routes.sim_client_migration(addr_idx, inc_last_addr_octet);
            }
            Self::PassiveMigration {
                side: Side::Server,
                addr_idx,
            } => {
                let routes = pair.routes.as_mut()?;
                routes.sim_server_migration(addr_idx, inc_last_addr_octet);
            }
            Self::OpenPath {
                side,
                status,
                addr_idx,
            } => {
                let routes = pair.routes.as_ref()?;
                let remote = match side {
                    Side::Client => routes.server_addr(addr_idx)?,
                    Side::Server => routes.client_addr(addr_idx)?,
                };
                let state = match side {
                    Side::Client => client,
                    Side::Server => server,
                };
                let conn = state.conn(pair)?;
                let network_path = FourTuple {
                    remote,
                    local_ip: None,
                };
                conn.open_path(network_path, status, now)
                    .inspect_err(|err| error!(?err, "OpenPath failed"))
                    .ok();
            }
            Self::ClosePath {
                side,
                path_idx,
                error_code,
            } => {
                let state = match side {
                    Side::Client => client,
                    Side::Server => server,
                };
                let conn = state.conn(pair)?;
                let path_id = get_path_id(conn, path_idx)?;
                conn.close_path(now, path_id, error_code.into())
                    .inspect_err(|err| error!(?err, "ClosePath failed"))
                    .ok();
            }
            Self::PathSetStatus {
                side,
                path_idx,
                status,
            } => {
                let state = match side {
                    Side::Client => client,
                    Side::Server => server,
                };
                let conn = state.conn(pair)?;
                let path_id = get_path_id(conn, path_idx)?;
                conn.set_path_status(path_id, status)
                    .inspect_err(|err| error!(?err, "PathSetStatus failed"))
                    .ok();
            }
            Self::StreamOp { side, stream_op } => {
                let state = match side {
                    Side::Client => client,
                    Side::Server => server,
                };
                stream_op.run(pair, state);
            }
            Self::SendDatagram { side, size, drop } => {
                let state = match side {
                    Side::Client => client,
                    Side::Server => server,
                };
                let data = vec![42u8; size];
                state
                    .conn(pair)?
                    .datagrams()
                    .send(data.into(), drop)
                    .inspect_err(|err| error!(?err, "DatagramOp failed"))
                    .ok();
            }
            Self::ReadDatagrams { side } => {
                let state = match side {
                    Side::Client => client,
                    Side::Server => server,
                };
                while let Some(data) = state.conn(pair)?.datagrams().recv() {
                    trace!(len = data.len(), "ReadDatagrams read a datagram");
                }
            }
            Self::CloseConn { side, error_code } => {
                let state = match side {
                    Side::Client => client,
                    Side::Server => server,
                };
                let conn = state.conn(pair)?;
                conn.close(now, error_code.into(), Bytes::new());
            }
            Self::AddHpAddr { side, addr_idx } => {
                let routes = pair.routes.as_ref()?;
                let address = match side {
                    Side::Client => routes.client_addr(addr_idx)?,
                    Side::Server => routes.server_addr(addr_idx)?,
                };
                let state = match side {
                    Side::Client => client,
                    Side::Server => server,
                };
                let conn = state.conn(pair)?;
                conn.add_nat_traversal_address(address)
                    .inspect_err(|err| error!(?err, "AddHpAddr failed"))
                    .ok();
            }
            Self::InitiateHpRound { side } => {
                let state = match side {
                    Side::Client => client,
                    Side::Server => server,
                };
                let conn = state.conn(pair)?;
                let addrs = conn
                    .initiate_nat_traversal_round(now)
                    .inspect_err(|err| error!(?err, "InitiateHpRound failed"))
                    .ok()?;
                trace!(?addrs, "initiating NAT Traversal");
            }
        }
        Some(())
    }
}

impl StreamOp {
    fn run(self, pair: &mut Pair, state: &mut State) -> Option<()> {
        let conn = state.conn(pair)?;
        // We generally ignore application-level errors. It's legal to call these APIs, so we do. We don't expect them to work all the time.
        match self {
            Self::Open(kind) => state.send_streams.extend(conn.streams().open(kind)),
            Self::Send { stream, num_bytes } => {
                let stream_id = state.send_streams.get(stream)?;
                let data = vec![0; num_bytes];
                let bytes = conn.send_stream(*stream_id).write(&data).ok()?;
                trace!(attempted_write = %num_bytes, actually_written = %bytes, "random interaction: Wrote stream bytes");
            }
            Self::Finish(stream) => {
                let stream_id = state.send_streams.get(stream)?;
                conn.send_stream(*stream_id).finish().ok();
            }
            Self::Reset(stream, code) => {
                let stream_id = state.send_streams.get(stream)?;
                conn.send_stream(*stream_id).reset(code.into()).ok();
            }
            Self::Accept(kind) => state.recv_streams.extend(conn.streams().accept(kind)),
            Self::Receive(stream, ordered) => {
                let stream_id = state.recv_streams.get(stream)?;
                let mut recv_stream = conn.recv_stream(*stream_id);
                let mut chunks = recv_stream.read(ordered).ok()?;
                let chunk = chunks.next(usize::MAX).ok()??;
                trace!(chunk_len = %chunk.bytes.len(), offset = %chunk.offset, "read from stream");
            }
            Self::Stop(stream, code) => {
                let stream_id = state.recv_streams.get(stream)?;
                conn.recv_stream(*stream_id).stop(code.into()).ok();
            }
        };
        Some(())
    }
}

impl State {
    fn new(side: Side) -> Self {
        Self {
            send_streams: Vec::new(),
            recv_streams: Vec::new(),
            handle: None,
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
        self.endpoint(pair).connections.get_mut(&self.handle?)
    }
}

fn get_path_id(conn: &mut Connection, idx: usize) -> Option<PathId> {
    let paths = conn.paths();
    paths
        .get(idx.clamp(0, paths.len().saturating_sub(1)))
        .copied()
}

fn inc_last_addr_octet(addr: SocketAddr) -> SocketAddr {
    match addr {
        SocketAddr::V4(socket_addr_v4) => {
            let [a, b, c, d] = socket_addr_v4.ip().octets();
            SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::new(a, b, c, d.wrapping_add(1)),
                socket_addr_v4.port(),
            ))
        }
        SocketAddr::V6(mut socket_addr_v6) => {
            let [a, b, c, d, e, f, g, h] = socket_addr_v6.ip().segments();
            socket_addr_v6.set_ip(Ipv6Addr::new(a, b, c, d, e, f, g, h.wrapping_add(1)));
            SocketAddr::V6(socket_addr_v6)
        }
    }
}

pub(super) fn run_random_interaction(
    pair: &mut Pair,
    interactions: Vec<TestOp>,
    client_config: ClientConfig,
) -> (ConnectionHandle, Option<ConnectionHandle>) {
    let mut client = State::new(Side::Client);
    let mut server = State::new(Side::Server);

    let client_handle = pair.begin_connect(client_config);
    client.handle = Some(client_handle);

    for interaction in interactions {
        info!(?interaction, "INTERACTION STEP");
        interaction.run(pair, &mut client, &mut server);
    }

    (client_handle, server.handle)
}
