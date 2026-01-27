use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use proptest::{
    collection::vec,
    prelude::{Strategy, any},
    prop_assert,
};
use test_strategy::proptest;
use tracing::error;

use crate::{
    Connection, ConnectionClose, ConnectionError, Event, PathStatus, Side, TransportConfig,
    TransportErrorCode,
    tests::{
        Pair, RoutingTable,
        random_interaction::{TestOp, run_random_interaction},
        server_config, subscribe,
    },
};

const MAX_PATHS: u32 = 3;
const CLIENT_PORT: u16 = 44433;
const SERVER_PORT: u16 = 4433;

const CLIENT_ADDRS: [SocketAddr; MAX_PATHS as usize] = [
    SocketAddr::new(
        IpAddr::V6(Ipv4Addr::new(1, 1, 1, 0).to_ipv6_mapped()),
        CLIENT_PORT,
    ),
    SocketAddr::new(
        IpAddr::V6(Ipv4Addr::new(1, 1, 1, 1).to_ipv6_mapped()),
        CLIENT_PORT,
    ),
    SocketAddr::new(
        IpAddr::V6(Ipv4Addr::new(1, 1, 1, 2).to_ipv6_mapped()),
        CLIENT_PORT,
    ),
];
const SERVER_ADDRS: [SocketAddr; MAX_PATHS as usize] = [
    SocketAddr::new(
        IpAddr::V6(Ipv4Addr::new(2, 2, 2, 0).to_ipv6_mapped()),
        SERVER_PORT,
    ),
    SocketAddr::new(
        IpAddr::V6(Ipv4Addr::new(2, 2, 2, 1).to_ipv6_mapped()),
        SERVER_PORT,
    ),
    SocketAddr::new(
        IpAddr::V6(Ipv4Addr::new(2, 2, 2, 2).to_ipv6_mapped()),
        SERVER_PORT,
    ),
];

fn setup_deterministic_with_multipath(
    seed: [u8; 32],
    routes: RoutingTable,
    qlog_prefix: &'static str,
) -> Pair {
    let mut pair = Pair::seeded(seed);

    let mut cfg = server_config();
    let transport = multipath_transport_config(qlog_prefix);
    cfg.transport = Arc::new(transport);
    pair.server.endpoint.set_server_config(Some(Arc::new(cfg)));

    pair.client.addr = routes.client_addr(0).unwrap();
    pair.server.addr = routes.server_addr(0).unwrap();
    pair.routes = Some(routes);
    pair
}

fn multipath_transport_config(qlog_prefix: &'static str) -> TransportConfig {
    let mut cfg = TransportConfig::default();
    // enable multipath
    cfg.max_concurrent_multipath_paths(MAX_PATHS);
    // cfg.mtu_discovery_config(None);
    #[cfg(feature = "qlog")]
    cfg.qlog_from_env(qlog_prefix);
    #[cfg(not(feature = "qlog"))]
    let _ = qlog_prefix;
    cfg
}

#[proptest(cases = 256)]
fn random_interaction(
    #[strategy(any::<[u8; 32]>().no_shrink())] seed: [u8; 32],
    #[strategy(vec(any::<TestOp>(), 0..100))] interactions: Vec<TestOp>,
) {
    let prefix = "random_interaction";
    let mut pair = Pair::seeded(seed);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, multipath_transport_config(prefix));

    prop_assert!(!pair.drive_bounded(1000), "connection never became idle");
    prop_assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    prop_assert!(allowed_error(poll_to_close(
        pair.server_conn_mut(server_ch)
    )));
}

#[proptest(cases = 256)]
fn random_interaction_with_multipath_simple_routing(
    #[strategy(any::<[u8; 32]>().no_shrink())] seed: [u8; 32],
    #[strategy(vec(any::<TestOp>(), 0..100))] interactions: Vec<TestOp>,
) {
    let prefix = "random_interaction_with_multipath_simple_routing";
    let routes = RoutingTable::simple_symmetric(CLIENT_ADDRS, SERVER_ADDRS);
    let mut pair = setup_deterministic_with_multipath(seed, routes, prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, multipath_transport_config(prefix));

    prop_assert!(!pair.drive_bounded(1000), "connection never became idle");
    prop_assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    prop_assert!(allowed_error(poll_to_close(
        pair.server_conn_mut(server_ch)
    )));
}

fn routing_table() -> impl Strategy<Value = RoutingTable> {
    (vec(0..=5usize, 0..=4), vec(0..=5usize, 0..=4)).prop_map(|(client_offsets, server_offsets)| {
        let mut client_addr = SocketAddr::new(
            IpAddr::V6(Ipv4Addr::new(1, 1, 1, 0).to_ipv6_mapped()),
            CLIENT_PORT,
        );
        let mut server_addr = SocketAddr::new(
            IpAddr::V6(Ipv4Addr::new(2, 2, 2, 0).to_ipv6_mapped()),
            SERVER_PORT,
        );
        let mut client_routes = vec![(client_addr, 0)];
        let mut server_routes = vec![(server_addr, 0)];
        for (idx, &offset) in client_offsets.iter().enumerate() {
            let other_idx = idx.saturating_sub(offset);
            let server_idx = other_idx.clamp(0, server_offsets.len());
            client_addr.set_ip(IpAddr::V6(
                Ipv4Addr::new(1, 1, 1, idx as u8 + 1).to_ipv6_mapped(),
            ));
            client_routes.push((client_addr, server_idx));
        }
        for (idx, &offset) in server_offsets.iter().enumerate() {
            let other_idx = idx.saturating_sub(offset);
            let client_idx = other_idx.clamp(0, client_offsets.len());
            server_addr.set_ip(IpAddr::V6(
                Ipv4Addr::new(2, 2, 2, idx as u8 + 1).to_ipv6_mapped(),
            ));
            server_routes.push((server_addr, client_idx));
        }

        RoutingTable::from_routes(client_routes, server_routes)
    })
}

#[proptest(cases = 256)]
fn random_interaction_with_multipath_complex_routing(
    #[strategy(any::<[u8; 32]>().no_shrink())] seed: [u8; 32],
    #[strategy(vec(any::<TestOp>(), 0..100))] interactions: Vec<TestOp>,
    #[strategy(routing_table())] routes: RoutingTable,
) {
    let prefix = "random_interaction_with_multipath_complex_routing";
    let mut pair = setup_deterministic_with_multipath(seed, routes, prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, multipath_transport_config(prefix));

    prop_assert!(!pair.drive_bounded(1000), "connection never became idle");
    prop_assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    prop_assert!(allowed_error(poll_to_close(
        pair.server_conn_mut(server_ch)
    )));
}

fn old_routing_table() -> RoutingTable {
    let mut routes = RoutingTable::simple_symmetric([CLIENT_ADDRS[0]], [SERVER_ADDRS[0]]);
    for addr in CLIENT_ADDRS.into_iter().skip(1) {
        routes.add_client_route(addr, 0);
    }
    for addr in SERVER_ADDRS.into_iter().skip(1) {
        routes.add_server_route(addr, 0);
    }
    routes
}

/// In proptests, we only allow connection errors that don't indicate erroring out
/// because we think we're working with another implementation that isn't protocol-
/// abiding. If we think that, clearly something is wrong, given we're controlling
/// both ends of the connection.
fn allowed_error(err: Option<ConnectionError>) -> bool {
    let allowed = match &err {
        None => true,
        Some(ConnectionError::TransportError(err)) => {
            // keep in sync with connection/mod.rs
            &err.reason == "last path abandoned by peer"
        }
        Some(ConnectionError::ConnectionClosed(ConnectionClose { error_code, .. })) => {
            *error_code != TransportErrorCode::PROTOCOL_VIOLATION
        }
        _ => true,
    };
    if !allowed {
        error!(
            ?err,
            "Got an error that's unexpected in quinn <-> quinn interaction"
        );
    }
    allowed
}

fn poll_to_close(conn: &mut Connection) -> Option<ConnectionError> {
    let mut close = None;
    while let Some(event) = conn.poll() {
        if let Event::ConnectionLost { reason } = event {
            close = Some(reason);
        }
    }
    close
}

#[test]
fn regression_unset_packet_acked() {
    let prefix = "regression_unset_packet_acked";
    let seed: [u8; 32] = [
        60, 116, 60, 165, 136, 238, 239, 131, 14, 159, 221, 16, 80, 60, 30, 15, 15, 69, 133, 33,
        89, 203, 28, 107, 123, 117, 6, 54, 215, 244, 47, 1,
    ];
    let interactions = vec![
        TestOp::OpenPath(Side::Client, PathStatus::Available, 0),
        TestOp::ClosePath(Side::Client, 0, 0),
        TestOp::Drive(Side::Client),
        TestOp::AdvanceTime,
        TestOp::Drive(Side::Server),
        TestOp::DropInbound(Side::Client),
    ];

    let _guard = subscribe();
    let routes = old_routing_table();
    let mut pair = setup_deterministic_with_multipath(seed, routes, prefix);
    #[allow(unused_mut)]
    let mut cfg = TransportConfig::default();
    #[cfg(feature = "qlog")]
    cfg.qlog_from_env(prefix);
    let (client_ch, server_ch) = run_random_interaction(&mut pair, interactions, cfg);

    assert!(!pair.drive_bounded(100), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    assert!(allowed_error(poll_to_close(
        pair.server_conn_mut(server_ch)
    )));
}

#[test]
fn regression_invalid_key() {
    let prefix = "regression_invalid_key";
    let seed = [
        41, 24, 232, 72, 136, 73, 31, 115, 14, 101, 61, 219, 30, 168, 130, 122, 120, 238, 6, 130,
        117, 84, 250, 190, 50, 237, 14, 167, 60, 5, 140, 149,
    ];
    let interactions = vec![
        TestOp::OpenPath(Side::Client, PathStatus::Available, 0),
        TestOp::AdvanceTime,
        TestOp::Drive(Side::Client),
        TestOp::OpenPath(Side::Client, PathStatus::Available, 0),
    ];

    let _guard = subscribe();
    let routes = old_routing_table();
    let mut pair = setup_deterministic_with_multipath(seed, routes, prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, multipath_transport_config(prefix));

    assert!(!pair.drive_bounded(100), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    assert!(allowed_error(poll_to_close(
        pair.server_conn_mut(server_ch)
    )));
}

#[test]
fn regression_key_update_error() {
    let prefix = "regression_key_update_error";
    let seed: [u8; 32] = [
        68, 93, 15, 237, 88, 31, 93, 255, 246, 51, 203, 224, 20, 124, 107, 163, 143, 43, 193, 187,
        208, 54, 158, 239, 190, 82, 198, 62, 91, 51, 53, 226,
    ];
    let interactions = vec![
        TestOp::OpenPath(Side::Client, PathStatus::Available, 0),
        TestOp::Drive(Side::Client),
        TestOp::ForceKeyUpdate(Side::Server),
    ];

    let _guard = subscribe();
    let routes = old_routing_table();
    let mut pair = setup_deterministic_with_multipath(seed, routes, prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, multipath_transport_config(prefix));

    assert!(!pair.drive_bounded(100), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    assert!(allowed_error(poll_to_close(
        pair.server_conn_mut(server_ch)
    )));
}

#[test]
fn regression_never_idle() {
    let prefix = "regression_never_idle";
    let seed = [0u8; 32];
    let interactions = vec![
        TestOp::OpenPath(Side::Client, PathStatus::Available, 1),
        TestOp::PathSetStatus(Side::Server, 0, PathStatus::Backup),
        TestOp::ClosePath(Side::Client, 0, 0),
    ];

    let _guard = subscribe();
    let routes = old_routing_table();
    let mut pair = setup_deterministic_with_multipath(seed, routes, prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, multipath_transport_config(prefix));

    assert!(!pair.drive_bounded(100), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    assert!(allowed_error(poll_to_close(
        pair.server_conn_mut(server_ch)
    )));
}

#[test]
fn regression_never_idle2() {
    let prefix = "regression_never_idle2";
    let seed = [0u8; 32];
    let interactions = vec![
        TestOp::OpenPath(Side::Client, PathStatus::Backup, 1),
        TestOp::ClosePath(Side::Client, 0, 0),
        TestOp::Drive(Side::Client),
        TestOp::DropInbound(Side::Server),
        TestOp::PathSetStatus(Side::Client, 0, PathStatus::Available),
    ];

    let _guard = subscribe();
    let routes = old_routing_table();
    let mut pair = setup_deterministic_with_multipath(seed, routes, prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, multipath_transport_config(prefix));

    // We needed to increase the bounds. It eventually times out.
    assert!(!pair.drive_bounded(1000), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    assert!(allowed_error(poll_to_close(
        pair.server_conn_mut(server_ch)
    )));
}

#[test]
fn regression_packet_number_space_missing() {
    let prefix = "regression_packet_number_space_missing";
    let seed = [0u8; 32];
    let interactions = vec![
        TestOp::OpenPath(Side::Client, PathStatus::Backup, 0),
        TestOp::OpenPath(Side::Client, PathStatus::Backup, 0),
        TestOp::Drive(Side::Client),
        TestOp::DropInbound(Side::Server),
        TestOp::ClosePath(Side::Client, 0, 0),
    ];

    let _guard = subscribe();
    let routes = RoutingTable::simple_symmetric([CLIENT_ADDRS[0]], [SERVER_ADDRS[0]]);
    let mut pair = setup_deterministic_with_multipath(seed, routes, prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, multipath_transport_config(prefix));

    assert!(!pair.drive_bounded(100), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    assert!(allowed_error(poll_to_close(
        pair.server_conn_mut(server_ch)
    )));
}

#[test]
fn regression_peer_failed_to_respond_with_path_abandon() {
    let prefix = "regression_peer_failed_to_respond_with_path_abandon";
    let seed = [0u8; 32];
    let interactions = vec![
        TestOp::OpenPath(Side::Client, PathStatus::Available, 1),
        TestOp::ClosePath(Side::Client, 0, 0),
    ];

    let _guard = subscribe();
    let routes = old_routing_table();
    let mut pair = setup_deterministic_with_multipath(seed, routes, prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, multipath_transport_config(prefix));

    assert!(!pair.drive_bounded(100), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    assert!(allowed_error(poll_to_close(
        pair.server_conn_mut(server_ch)
    )));
}

#[test]
fn regression_peer_failed_to_respond_with_path_abandon2() {
    let prefix = "regression_peer_failed_to_respond_with_path_abandon2";
    let seed = [0u8; 32];
    let interactions = vec![
        TestOp::OpenPath(Side::Client, PathStatus::Available, 0),
        TestOp::Drive(Side::Client),
        TestOp::CloseConn(Side::Server, 0),
        TestOp::DropInbound(Side::Server),
        TestOp::AdvanceTime,
        TestOp::Drive(Side::Server),
        TestOp::ClosePath(Side::Client, 0, 0),
        TestOp::Drive(Side::Server),
        TestOp::DropInbound(Side::Client),
    ];

    let _guard = subscribe();
    let routes = RoutingTable::simple_symmetric(CLIENT_ADDRS, SERVER_ADDRS);
    let mut pair = setup_deterministic_with_multipath(seed, routes, prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, multipath_transport_config(prefix));

    assert!(!pair.drive_bounded(1000), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    assert!(allowed_error(poll_to_close(
        pair.server_conn_mut(server_ch)
    )));
}

/// This test sets up two addresses for the server side:
/// 2.2.2.0 and 2.2.2.1. The client side can send to either
/// and the server side will receive them in both cases.
///
/// Such a situation happens in practice with multiple interfaces
/// in a sending-to-my-own-machine situation, e.g. when you have
/// both a WiFi and bridge(?) or docker interface, where sending
/// to the docker address of yourself results in the kernel
/// translating that to sending via WiFi and the incoming "remote"
/// address that comes in looks like it's been sent via WiFi.
///
/// The test here is slightly simplified in that the sides don't
/// share the same IP address and don't both have the same two
/// interfaces, but the resulting situation is the same:
///
/// - The server sees the remote as 1.1.1.0 and had previously
///   sent and received on that address in this connection and
///   thus considers it valid, while
/// - the client side first sends to 2.2.2.1 but gets the response
///   from the remote 2.2.2.0, thus it fails validation on the
///   client side and it ignores the packet.
///
/// Originally this test produced a "PATH_ABANDON was ignored"
/// error message, but that's secondary to the original problem.
/// The reason it was even possible to produce this error is that
/// we were able to abandon the last open path (path 0) on the
/// server because it incorrectly thought path 1 was fully validated
/// and working (and it was not).
/// Or another way to look at what went wrong would be that the
/// server kept sending PATH_ABANDON on path 1, even though it is
/// a broken path.
#[test]
fn regression_path_validation() {
    let prefix = "regression_path_validation";
    let seed = [0u8; 32];
    let interactions = vec![
        TestOp::OpenPath(Side::Client, PathStatus::Available, 1),
        TestOp::Drive(Side::Client),
        TestOp::AdvanceTime,
        TestOp::Drive(Side::Server),
        TestOp::OpenPath(Side::Client, PathStatus::Available, 1),
        TestOp::ClosePath(Side::Server, 0, 0),
    ];
    let routes = RoutingTable::from_routes(
        vec![("[::ffff:1.1.1.0]:44433".parse().unwrap(), 0)],
        vec![
            ("[::ffff:2.2.2.0]:4433".parse().unwrap(), 0),
            ("[::ffff:2.2.2.1]:4433".parse().unwrap(), 0),
        ],
    );

    let _guard = subscribe();
    let mut pair = setup_deterministic_with_multipath(seed, routes, prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, multipath_transport_config(prefix));

    assert!(!pair.drive_bounded(1000), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    assert!(allowed_error(poll_to_close(
        pair.server_conn_mut(server_ch)
    )));
}

/// This regression test used to fail with the client never becoming idle.
/// It kept sending PATH_CHALLENGEs forever.
///
/// The situation in which that happened was this:
/// 1. The server closes the connection, but the close frame is lost.
/// 2. The client opens another path on the same 4-tuple (thus that path is immediately validated).
/// 3. It immediately closes path 0 afterwards.
///
/// At this point, the server is already fully checked out and not responding anymore.
/// The client however thinks the connection is still ongoing and continues sending (that's fine).
/// However, it never stops sending path challenges, because of a bug where only when the
/// path validation timer times out, the path challenge lost timer was stopped. This means
/// the client would keep re-sending path challenges infinitely (never getting a response,
/// which would also stop the challenge lost timer).
///
/// Correctly stopping the path challenge lost timer fixes this.
#[test]
fn regression_never_idle3() {
    let prefix = "regression_never_idle3";
    let seed = [0u8; 32];
    let interactions = vec![
        TestOp::CloseConn(Side::Server, 0),
        TestOp::Drive(Side::Server),
        TestOp::DropInbound(Side::Client),
        TestOp::OpenPath(Side::Client, PathStatus::Available, 0),
        TestOp::ClosePath(Side::Client, 0, 0),
        TestOp::AdvanceTime,
    ];

    let _guard = subscribe();
    let routes = RoutingTable::simple_symmetric([CLIENT_ADDRS[0]], [SERVER_ADDRS[0]]);
    let mut pair = setup_deterministic_with_multipath(seed, routes, prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, multipath_transport_config(prefix));

    assert!(!pair.drive_bounded(1000), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    assert!(allowed_error(poll_to_close(
        pair.server_conn_mut(server_ch)
    )));
}

#[test]
fn regression_frame_encoding_error() {
    let prefix = "regression_frame_encoding_error";
    let seed = [0u8; 32];
    let interactions = vec![
        TestOp::OpenPath(Side::Client, PathStatus::Available, 1),
        TestOp::OpenPath(Side::Client, PathStatus::Available, 0),
        TestOp::ClosePath(Side::Client, 0, 0),
    ];

    let _guard = subscribe();
    let routes = RoutingTable::simple_symmetric(CLIENT_ADDRS, SERVER_ADDRS);
    let mut pair = setup_deterministic_with_multipath(seed, routes, prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, multipath_transport_config(prefix));

    assert!(!pair.drive_bounded(1000), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    assert!(allowed_error(poll_to_close(
        pair.server_conn_mut(server_ch)
    )));
}

#[test]
fn regression_there_should_be_at_least_one_path() {
    let prefix = "regression_there_should_be_at_least_one_path";
    let seed = [0u8; 32];
    let interactions = vec![
        TestOp::PassiveMigration(Side::Client, 0),
        TestOp::CloseConn(Side::Client, 0),
    ];

    let _guard = subscribe();
    let routes = RoutingTable::simple_symmetric([CLIENT_ADDRS[0]], [SERVER_ADDRS[0]]);
    let mut pair = setup_deterministic_with_multipath(seed, routes, prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, multipath_transport_config(prefix));

    assert!(!pair.drive_bounded(1000), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    assert!(allowed_error(poll_to_close(
        pair.server_conn_mut(server_ch)
    )));
}
