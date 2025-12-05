use std::{collections::VecDeque, sync::Arc};

use proptest::prelude::Strategy;
use test_strategy::Arbitrary;
use tracing::{debug, trace};

use crate::{
    Connection, ConnectionHandle, Dir, PathId, PathStatus, StreamId, TransportConfig,
    tests::{Pair, TestEndpoint, client_config},
};

#[derive(Debug, Clone, Copy, Arbitrary)]
pub(super) enum Side {
    Server,
    Client,
}

#[derive(Debug, Clone, Copy, Arbitrary)]
pub(super) enum TestOp {
    Drive(Side),
    AdvanceTime,
    DropInbound(Side),
    ReorderInbound(Side),
    ForceKeyUpdate(Side),
    OpenPath(
        Side,
        #[strategy(path_status())] PathStatus,
        #[strategy(0..3usize)] usize,
    ),
    ClosePath(Side, #[strategy(0..3usize)] usize, u32),
    PathSetStatus(
        Side,
        #[strategy(0..3usize)] usize,
        #[strategy(path_status())] PathStatus,
    ),
    StreamOp(Side, StreamOp),
}

fn path_status() -> impl Strategy<Value = PathStatus> {
    (0..=1).prop_map(|n| match n {
        0 => PathStatus::Available,
        1 => PathStatus::Backup,
        _ => unreachable!(),
    })
}

/// We *basically* only operate with 3 streams concurrently at the moment
/// (even though more might be opened at a time).
#[derive(Debug, Clone, Copy, Arbitrary)]
pub(super) enum StreamOp {
    Open(#[strategy(stream_dir())] Dir),
    Send {
        #[strategy(0..3usize)]
        stream: usize,
        #[strategy(0..10_000usize)]
        num_bytes: usize,
    },
    Finish(#[strategy(0..3usize)] usize),
    Reset(#[strategy(0..3usize)] usize, u32),

    Accept(#[strategy(stream_dir())] Dir),
    Receive(#[strategy(0..3usize)] usize, bool),
    Stop(#[strategy(0..3usize)] usize, u32),
}

fn stream_dir() -> impl Strategy<Value = Dir> {
    (0..=1).prop_map(|n| match n {
        0 => Dir::Uni,
        1 => Dir::Bi,
        _ => unreachable!(),
    })
}

pub(super) struct State {
    send_streams: Vec<StreamId>,
    recv_streams: Vec<StreamId>,
    handle: ConnectionHandle,
    side: Side,
}

impl StreamOp {
    fn run(self, pair: &mut Pair, state: &mut State) {
        let Some(conn) = state.conn(pair) else {
            return;
        };
        // We generally ignore application-level errors. It's legal to call these APIs, so we do. We don't expect them to work all the time.
        match self {
            Self::Open(kind) => state.send_streams.extend(conn.streams().open(kind.into())),
            Self::Send { stream, num_bytes } => {
                if let Some(&stream_id) = state.send_streams.get(stream) {
                    let data = vec![0; num_bytes];
                    if let Some(bytes) = conn.send_stream(stream_id).write(&data).ok() {
                        trace!(attempted_write = %num_bytes, actually_written = %bytes, "random interaction: Wrote stream bytes");
                    }
                }
            }
            Self::Finish(stream) => {
                if let Some(&stream_id) = state.send_streams.get(stream) {
                    conn.send_stream(stream_id).finish().ok();
                }
            }
            Self::Reset(stream, code) => {
                if let Some(&stream_id) = state.send_streams.get(stream) {
                    conn.send_stream(stream_id).reset(code.into()).ok();
                }
            }
            Self::Accept(kind) => state
                .recv_streams
                .extend(conn.streams().accept(kind.into())),
            Self::Receive(stream, ordered) => {
                if let Some(&stream_id) = state.recv_streams.get(stream) {
                    if let Some(mut chunks) = conn.recv_stream(stream_id).read(ordered).ok() {
                        if let Ok(Some(chunk)) = chunks.next(usize::MAX) {
                            trace!(chunk_len = %chunk.bytes.len(), offset = %chunk.offset, "read from stream");
                        }
                    }
                }
            }
            Self::Stop(stream, code) => {
                if let Some(&stream_id) = state.recv_streams.get(stream) {
                    conn.recv_stream(stream_id).stop(code.into()).ok();
                }
            }
        };
    }
}

impl State {
    fn new(side: Side, handle: ConnectionHandle) -> Self {
        Self {
            send_streams: Vec::new(),
            recv_streams: Vec::new(),
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
}

fn get_path_id(conn: &mut Connection, idx: usize) -> Option<PathId> {
    let paths = conn.paths();
    paths
        .get(idx.clamp(0, paths.len().saturating_sub(1)))
        .copied()
}

pub(super) fn run_random_interaction(
    pair: &mut Pair,
    interactions: Vec<TestOp>,
    transport_config: TransportConfig,
) {
    let mut client_cfg = client_config();
    client_cfg.transport = Arc::new(transport_config);
    let (client_ch, server_ch) = pair.connect_with(client_cfg);
    pair.drive(); // finish establishing the connection;
    debug!("INTERACTION SETUP FINISHED");
    pair.client_conn_mut(client_ch).panic_on_transport_error();
    pair.server_conn_mut(server_ch).panic_on_transport_error();
    let mut client = State::new(Side::Client, client_ch);
    let mut server = State::new(Side::Server, server_ch);

    for interaction in interactions {
        debug!(?interaction, "INTERACTION STEP");
        interaction.run(pair, &mut client, &mut server);
    }
}

impl TestOp {
    fn run(self: TestOp, pair: &mut Pair, client: &mut State, server: &mut State) {
        let now = pair.time;
        match self {
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
            TestOp::ReorderInbound(Side::Client) => {
                if let Some(item) = pair.client.inbound.pop_front() {
                    pair.client.inbound.push_back(item);
                }
            }
            TestOp::ReorderInbound(Side::Server) => {
                if let Some(item) = pair.server.inbound.pop_front() {
                    pair.server.inbound.push_back(item);
                }
            }
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
            TestOp::OpenPath(side, initial_status, addr) => {
                if let Some(routes) = &pair.routes {
                    if let Some(remote) = match side {
                        Side::Client => routes.client_addr(addr),
                        Side::Server => routes.server_addr(addr),
                    } {
                        let state = match side {
                            Side::Client => client,
                            Side::Server => server,
                        };
                        if let Some(conn) = state.conn(pair) {
                            conn.open_path(remote, initial_status, now).ok();
                        }
                    }
                }
            }
            TestOp::ClosePath(side, path_idx, error_code) => {
                let state = match side {
                    Side::Client => client,
                    Side::Server => server,
                };
                if let Some(conn) = state.conn(pair) {
                    if let Some(path_id) = get_path_id(conn, path_idx) {
                        conn.close_path(now, path_id, error_code.into()).ok();
                    }
                }
            }
            TestOp::PathSetStatus(side, path_idx, status) => {
                let state = match side {
                    Side::Client => client,
                    Side::Server => server,
                };
                if let Some(conn) = state.conn(pair) {
                    if let Some(path_id) = get_path_id(conn, path_idx) {
                        conn.set_path_status(path_id, status).ok();
                    }
                }
            }
            TestOp::StreamOp(side, stream_op) => match side {
                Side::Client => stream_op.run(pair, client),
                Side::Server => stream_op.run(pair, server),
            },
        }
    }
}

fn reorder<T>(vec: &mut VecDeque<T>) {
    if let Some(item) = vec.pop_front() {
        vec.push_back(item);
    }
}
