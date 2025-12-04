use std::{
    net::{IpAddr, Ipv6Addr, SocketAddr},
    num::NonZeroU32,
    sync::Arc,
    time::{Duration, Instant},
};

use proptest::{
    collection::vec,
    prelude::{Strategy, any},
};
use rand::{RngCore, SeedableRng, rngs::StdRng};
use test_strategy::proptest;

use crate::{
    Endpoint, EndpointConfig, TransportConfig,
    tests::{
        DEFAULT_MTU, Pair, RoutingTable, TestEndpoint,
        random_interaction::{PathKind, Side, TestOp, run_random_interaction},
        server_config, subscribe,
    },
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

fn setup_deterministic_with_multipath(seed: [u8; 32], routes: RoutingTable) -> Pair {
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
    let server = TestEndpoint::new(server, SERVER_ADDRS[0]);
    let client = TestEndpoint::new(client, CLIENT_ADDRS[0]);
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
        routes: Some(routes),
    }
}

fn multipath_transport_config() -> TransportConfig {
    let mut transport = TransportConfig::default();
    // enable multipath
    transport.max_concurrent_multipath_paths = NonZeroU32::new(MAX_PATHS);
    transport
}

#[proptest(cases = 256)]
fn random_interaction(
    #[strategy(any::<[u8; 32]>().no_shrink())] seed: [u8; 32],
    #[strategy(vec(any::<TestOp>(), 0..100))] interactions: Vec<TestOp>,
) {
    let mut pair = Pair::default_deterministic(seed);
    run_random_interaction(&mut pair, interactions, multipath_transport_config());

    assert!(!pair.drive_bounded(1000), "connection never became idle");
}

#[proptest(cases = 256)]
fn random_interaction_with_multipath(
    #[strategy(any::<[u8; 32]>().no_shrink())] seed: [u8; 32],
    #[strategy(vec(any::<TestOp>(), 0..100))] interactions: Vec<TestOp>,
) {
    let routes = RoutingTable::simple_symmetric(CLIENT_ADDRS, SERVER_ADDRS);
    let mut pair = setup_deterministic_with_multipath(seed, routes);
    run_random_interaction(&mut pair, interactions, multipath_transport_config());

    assert!(!pair.drive_bounded(1000), "connection never became idle");
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
    let routes = old_routing_table();
    let mut pair = setup_deterministic_with_multipath(seed, routes);
    run_random_interaction(&mut pair, interactions, multipath_transport_config());

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
    let routes = old_routing_table();
    let mut pair = setup_deterministic_with_multipath(seed, routes);
    run_random_interaction(&mut pair, interactions, multipath_transport_config());

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
    let routes = old_routing_table();
    let mut pair = setup_deterministic_with_multipath(seed, routes);
    run_random_interaction(&mut pair, interactions, multipath_transport_config());

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
    let routes = old_routing_table();
    let mut pair = setup_deterministic_with_multipath(seed, routes);
    run_random_interaction(&mut pair, interactions, multipath_transport_config());

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
    let routes = old_routing_table();
    let mut pair = setup_deterministic_with_multipath(seed, routes);
    run_random_interaction(&mut pair, interactions, multipath_transport_config());

    // We needed to increase the bounds. It eventually times out.
    assert!(!pair.drive_bounded(1000), "connection never became idle");
}
