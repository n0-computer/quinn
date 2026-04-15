use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use proptest::{
    collection::vec,
    prelude::{Strategy, any},
    prop_assert,
};
use test_strategy::proptest;
use tracing::error;

use crate::{
    ClientConfig, Connection, ConnectionClose, ConnectionError, Event, MtuDiscoveryConfig,
    PathStatus, Side, TransportConfig, TransportErrorCode,
    tests::{
        Pair, RoutingTable, client_config,
        random_interaction::{Establishment, TestOp, run_random_interaction},
        server_config, subscribe,
    },
};

// These TransportConfig constants are designed to match iroh for now.
const MAX_MULTIPATH_PATHS: u32 = 13;
const MAX_QNT_ADDRS: u8 = 12;
const PATH_MAX_IDLE_TIMEOUT: Duration = Duration::from_secs(15);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

const CLIENT_PORT: u16 = 44433;
const SERVER_PORT: u16 = 4433;

const CLIENT_ADDRS: [SocketAddr; 3] = [
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
const SERVER_ADDRS: [SocketAddr; 3] = [
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

/// Struct for generating random pair setups.
///
/// Compared to randomly generating e.g. the `TransportConfig` on both sides,
/// this has several advantages:
/// - On a proptest failure, it is easy to see which minimal setup the proptest fails with.
/// - The definition of the setup itself is concise, e.g. when copying it into regression tests.
/// - We can be more precise/smaller in the "search space" and have more efficient shrinking,
///   see [`Seed`] or [`RoutingSetup`].
#[derive(Debug, test_strategy::Arbitrary)]
struct PairSetup {
    seed: Seed,
    extensions: Extensions,
    mtud_enabled: bool,
    routing_setup: RoutingSetup,
}

/// Extensions to enable or not enable in the proptests.
#[derive(Debug, test_strategy::Arbitrary)]
enum Extensions {
    None,
    MultipathOnly,
    QntAndMultipath,
}

/// Categories of routing setups used for proptests.
///
/// The advantage of using this is very efficient shrinking: The first attempt at shrinking the
/// routing setup will be to reduce the routing setup to nothing or a simple symmetric one.
#[derive(Debug, test_strategy::Arbitrary)]
enum RoutingSetup {
    /// Set [`Pair::routes`] to `None`
    None,
    /// Use [`RoutingTable::simple_symmetric`] with the default [`CLIENT_ADDRS`] and [`SERVER_ADDRS`].
    SimpleSymmetric,
    /// Use given generated routing table.
    Complex(#[strategy(routing_table())] RoutingTable),
}

/// Which seed to use in the test setup.
///
/// This structure has an advantage over a simple `[u8; 32]`, because on one hand, we don't want
/// to waste too much time shrinking the seed itself (reducing individual values inside the array),
/// but also we don't want to disable shrinking altogether: when the seed shrinks to zero, this
/// helps us understand that the seed is likely irrelevant to the test failure.
///
/// This struct achieves the best of both worlds: If the seed is generated as `Generated(some_seed)`,
/// shrinking will try `Zeroes` once, and if that fails, fall back to using the generated seed
/// and avoid doing any further shrinking of `some_seed`.
#[derive(Debug, test_strategy::Arbitrary)]
enum Seed {
    /// The zero seed.
    ///
    /// If a test generates the zero seed, then it's likely that the seed doesn't have
    /// any effect on the test failure.
    Zeroes,
    /// A specific generated seed.
    Generated(#[strategy(any::<[u8; 32]>().no_shrink())] [u8; 32]),
}

impl PairSetup {
    fn run(self, prefix: &'static str) -> (Pair, ClientConfig) {
        let mut pair = Pair::seeded(self.seed.into_slice());

        // Initialize the transport config

        let mut transport = TransportConfig::default();
        // Set the qlog prefix, if the feature is enabled
        #[cfg(feature = "qlog")]
        transport.qlog_from_env(prefix);
        #[cfg(not(feature = "qlog"))]
        let _ = prefix;

        if self.extensions.is_multipath_enabled() {
            // enable multipath
            transport.max_concurrent_multipath_paths(MAX_MULTIPATH_PATHS);
            transport.default_path_max_idle_timeout(Some(PATH_MAX_IDLE_TIMEOUT));
            transport.default_path_keep_alive_interval(Some(HEARTBEAT_INTERVAL));
        }

        if self.extensions.is_qnt_enabled() {
            // enable QNT:
            transport.set_max_remote_nat_traversal_addresses(MAX_QNT_ADDRS);
        }

        // enable/disable MTUD
        transport.mtu_discovery_config(self.mtud_enabled.then_some(MtuDiscoveryConfig::default()));

        // Initialize the server config

        let mut server_cfg = server_config();
        server_cfg.transport = Arc::new(transport.clone());
        pair.server
            .endpoint
            .set_server_config(Some(Arc::new(server_cfg)));

        // Initialize the client config

        let mut client_cfg = client_config();
        client_cfg.transport = Arc::new(transport);

        // Add routing, if enabled

        match self.routing_setup {
            RoutingSetup::None => {
                pair.routes = None;
            }
            RoutingSetup::SimpleSymmetric => {
                let routes = RoutingTable::simple_symmetric(CLIENT_ADDRS, SERVER_ADDRS);
                pair.client.addr = routes.client_addr(0).unwrap();
                pair.server.addr = routes.server_addr(0).unwrap();
                pair.routes = Some(routes);
            }
            RoutingSetup::Complex(routes) => {
                pair.client.addr = routes.client_addr(0).unwrap();
                pair.server.addr = routes.server_addr(0).unwrap();
                pair.routes = Some(routes);
            }
        }

        (pair, client_cfg)
    }
}

impl Extensions {
    fn is_multipath_enabled(&self) -> bool {
        matches!(self, Self::MultipathOnly | Self::QntAndMultipath)
    }

    fn is_qnt_enabled(&self) -> bool {
        matches!(self, Self::QntAndMultipath)
    }
}

impl Seed {
    fn into_slice(self) -> [u8; 32] {
        match self {
            Self::Zeroes => [0u8; 32],
            Self::Generated(generated) => generated,
        }
    }
}

#[proptest(cases = 256)]
fn random_interaction(
    setup: PairSetup,
    establishment: Establishment,
    #[strategy(vec(any::<TestOp>(), 0..100))] interactions: Vec<TestOp>,
) {
    let (mut pair, client_config) = setup.run("random_interaction");
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, client_config, establishment);

    prop_assert!(!pair.drive_bounded(1000), "connection never became idle");
    prop_assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    if let Some(server_ch) = server_ch {
        prop_assert!(allowed_error(poll_to_close(
            pair.server_conn_mut(server_ch)
        )));
    }
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

/// All outgoing links go to first destination interface.
///
/// Client and server have multiple interfaces, but all outgoing links go to the first
/// interface of defined for the peer.
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
            &err.reason == "last path abandoned, no new path opened"
        }
        Some(ConnectionError::ConnectionClosed(ConnectionClose { error_code, .. })) => {
            *error_code != TransportErrorCode::PROTOCOL_VIOLATION
        }
        _ => true,
    };
    if !allowed {
        error!(
            ?err,
            "Got an error that's unexpected in noq <-> noq interaction"
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
    let setup = PairSetup {
        seed: Seed::Generated([
            60, 116, 60, 165, 136, 238, 239, 131, 14, 159, 221, 16, 80, 60, 30, 15, 15, 69, 133,
            33, 89, 203, 28, 107, 123, 117, 6, 54, 215, 244, 47, 1,
        ]),
        extensions: Extensions::MultipathOnly,
        mtud_enabled: true,
        routing_setup: RoutingSetup::Complex(old_routing_table()),
    };
    let interactions = vec![
        TestOp::OpenPath {
            side: Side::Client,
            status: PathStatus::Available,
            addr_idx: 0,
        },
        TestOp::ClosePath {
            side: Side::Client,
            path_idx: 0,
            error_code: 0,
        },
        TestOp::Drive { side: Side::Client },
        TestOp::AdvanceTime,
        TestOp::Drive { side: Side::Server },
        TestOp::DropInbound { side: Side::Client },
    ];

    let _guard = subscribe();
    let (mut pair, client_config) = setup.run(prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, client_config, Establishment::Full);

    assert!(!pair.drive_bounded(1000), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    if let Some(server_ch) = server_ch {
        assert!(allowed_error(poll_to_close(
            pair.server_conn_mut(server_ch)
        )));
    }
}

#[test]
fn regression_invalid_key() {
    let prefix = "regression_invalid_key";
    let setup = PairSetup {
        seed: Seed::Generated([
            41, 24, 232, 72, 136, 73, 31, 115, 14, 101, 61, 219, 30, 168, 130, 122, 120, 238, 6,
            130, 117, 84, 250, 190, 50, 237, 14, 167, 60, 5, 140, 149,
        ]),
        extensions: Extensions::MultipathOnly,
        mtud_enabled: true,
        routing_setup: RoutingSetup::Complex(old_routing_table()),
    };
    let interactions = vec![
        TestOp::OpenPath {
            side: Side::Client,
            status: PathStatus::Available,
            addr_idx: 0,
        },
        TestOp::AdvanceTime,
        TestOp::Drive { side: Side::Client },
        TestOp::OpenPath {
            side: Side::Client,
            status: PathStatus::Available,
            addr_idx: 0,
        },
    ];

    let _guard = subscribe();
    let (mut pair, client_config) = setup.run(prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, client_config, Establishment::Full);

    assert!(!pair.drive_bounded(1000), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    if let Some(server_ch) = server_ch {
        assert!(allowed_error(poll_to_close(
            pair.server_conn_mut(server_ch)
        )));
    }
}

/// Regression test for the "invalid key" panic in `noq-proto::Endpoint::handle_event`.
///
/// This test establishes this situation:
/// - There's a `Connection` in the `Drained` state (using close connection & advance time).
/// - We try to generate another `EndpointEvent` from the connection via closing the path.
///
/// Noq has an invariant that the last endpoint event allowed is `EndpointEventInner::Drained`,
/// but this invariant was violated in `Connection::close_path_inner`, as that can be called
/// on drained connections via an API.
///
/// We fixed this bug by short-circuting in `close_path_inner` if the connection is drained.
#[test]
fn regression_invalid_key2() {
    let prefix = "regression_invalid_key2";
    let setup = PairSetup {
        seed: Seed::Zeroes,
        extensions: Extensions::MultipathOnly,
        mtud_enabled: true,
        routing_setup: RoutingSetup::SimpleSymmetric,
    };
    let interactions = vec![
        TestOp::CloseConn {
            side: Side::Client,
            error_code: 0,
        },
        TestOp::AdvanceTime,
        TestOp::Drive { side: Side::Client },
        TestOp::OpenPath {
            side: Side::Client,
            status: PathStatus::Available,
            addr_idx: 0,
        },
        TestOp::ClosePath {
            side: Side::Client,
            path_idx: 0,
            error_code: 0,
        },
    ];

    let _guard = subscribe();
    let (mut pair, client_config) = setup.run(prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, client_config, Establishment::Full);

    assert!(!pair.drive_bounded(1000), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    if let Some(server_ch) = server_ch {
        assert!(allowed_error(poll_to_close(
            pair.server_conn_mut(server_ch)
        )));
    }
}

#[test]
fn regression_key_update_error() {
    let prefix = "regression_key_update_error";
    let setup = PairSetup {
        seed: Seed::Generated([
            68, 93, 15, 237, 88, 31, 93, 255, 246, 51, 203, 224, 20, 124, 107, 163, 143, 43, 193,
            187, 208, 54, 158, 239, 190, 82, 198, 62, 91, 51, 53, 226,
        ]),
        extensions: Extensions::MultipathOnly,
        mtud_enabled: true,
        routing_setup: RoutingSetup::Complex(old_routing_table()),
    };
    let interactions = vec![
        TestOp::OpenPath {
            side: Side::Client,
            status: PathStatus::Available,
            addr_idx: 0,
        },
        TestOp::Drive { side: Side::Client },
        TestOp::ForceKeyUpdate { side: Side::Server },
    ];

    let _guard = subscribe();
    let (mut pair, client_config) = setup.run(prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, client_config, Establishment::Full);

    assert!(!pair.drive_bounded(1000), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    if let Some(server_ch) = server_ch {
        assert!(allowed_error(poll_to_close(
            pair.server_conn_mut(server_ch)
        )));
    }
}

#[test]
fn regression_never_idle() {
    let prefix = "regression_never_idle";
    let setup = PairSetup {
        seed: Seed::Zeroes,
        extensions: Extensions::MultipathOnly,
        mtud_enabled: true,
        routing_setup: RoutingSetup::Complex(old_routing_table()),
    };
    let interactions = vec![
        TestOp::OpenPath {
            side: Side::Client,
            status: PathStatus::Available,
            addr_idx: 1,
        },
        TestOp::PathSetStatus {
            side: Side::Server,
            path_idx: 0,
            status: PathStatus::Backup,
        },
        TestOp::ClosePath {
            side: Side::Client,
            path_idx: 0,
            error_code: 0,
        },
    ];

    let _guard = subscribe();
    let (mut pair, client_config) = setup.run(prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, client_config, Establishment::Full);

    assert!(!pair.drive_bounded(1000), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    if let Some(server_ch) = server_ch {
        assert!(allowed_error(poll_to_close(
            pair.server_conn_mut(server_ch)
        )));
    }
}

#[test]
fn regression_never_idle2() {
    let prefix = "regression_never_idle2";
    let setup = PairSetup {
        seed: Seed::Zeroes,
        extensions: Extensions::MultipathOnly,
        mtud_enabled: true,
        routing_setup: RoutingSetup::Complex(old_routing_table()),
    };
    let interactions = vec![
        TestOp::OpenPath {
            side: Side::Client,
            status: PathStatus::Backup,
            addr_idx: 1,
        },
        TestOp::ClosePath {
            side: Side::Client,
            path_idx: 0,
            error_code: 0,
        },
        TestOp::Drive { side: Side::Client },
        TestOp::DropInbound { side: Side::Server },
        TestOp::PathSetStatus {
            side: Side::Client,
            path_idx: 0,
            status: PathStatus::Available,
        },
    ];

    let _guard = subscribe();
    let (mut pair, client_config) = setup.run(prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, client_config, Establishment::Full);

    // We needed to increase the bounds. It eventually times out.
    assert!(!pair.drive_bounded(1000), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    if let Some(server_ch) = server_ch {
        assert!(allowed_error(poll_to_close(
            pair.server_conn_mut(server_ch)
        )));
    }
}

#[test]
fn regression_packet_number_space_missing() {
    let prefix = "regression_packet_number_space_missing";
    let setup = PairSetup {
        seed: Seed::Zeroes,
        extensions: Extensions::MultipathOnly,
        mtud_enabled: true,
        routing_setup: RoutingSetup::SimpleSymmetric,
    };
    let interactions = vec![
        TestOp::OpenPath {
            side: Side::Client,
            status: PathStatus::Backup,
            addr_idx: 0,
        },
        TestOp::OpenPath {
            side: Side::Client,
            status: PathStatus::Backup,
            addr_idx: 0,
        },
        TestOp::Drive { side: Side::Client },
        TestOp::DropInbound { side: Side::Server },
        TestOp::ClosePath {
            side: Side::Client,
            path_idx: 0,
            error_code: 0,
        },
    ];

    let _guard = subscribe();
    let (mut pair, client_config) = setup.run(prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, client_config, Establishment::Full);

    assert!(!pair.drive_bounded(1000), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    if let Some(server_ch) = server_ch {
        assert!(allowed_error(poll_to_close(
            pair.server_conn_mut(server_ch)
        )));
    }
}

#[test]
fn regression_peer_failed_to_respond_with_path_abandon() {
    let prefix = "regression_peer_failed_to_respond_with_path_abandon";
    let setup = PairSetup {
        seed: Seed::Zeroes,
        extensions: Extensions::MultipathOnly,
        mtud_enabled: true,
        routing_setup: RoutingSetup::Complex(old_routing_table()),
    };
    let interactions = vec![
        TestOp::OpenPath {
            side: Side::Client,
            status: PathStatus::Available,
            addr_idx: 1,
        },
        TestOp::ClosePath {
            side: Side::Client,
            path_idx: 0,
            error_code: 0,
        },
    ];

    let _guard = subscribe();
    let (mut pair, client_config) = setup.run(prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, client_config, Establishment::Full);

    assert!(!pair.drive_bounded(1000), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    if let Some(server_ch) = server_ch {
        assert!(allowed_error(poll_to_close(
            pair.server_conn_mut(server_ch)
        )));
    }
}

#[test]
fn regression_peer_failed_to_respond_with_path_abandon2() {
    let prefix = "regression_peer_failed_to_respond_with_path_abandon2";
    let setup = PairSetup {
        seed: Seed::Zeroes,
        extensions: Extensions::MultipathOnly,
        mtud_enabled: true,
        routing_setup: RoutingSetup::SimpleSymmetric,
    };
    let interactions = vec![
        TestOp::OpenPath {
            side: Side::Client,
            status: PathStatus::Available,
            addr_idx: 0,
        },
        TestOp::Drive { side: Side::Client },
        TestOp::CloseConn {
            side: Side::Server,
            error_code: 0,
        },
        TestOp::DropInbound { side: Side::Server },
        TestOp::AdvanceTime,
        TestOp::Drive { side: Side::Server },
        TestOp::ClosePath {
            side: Side::Client,
            path_idx: 0,
            error_code: 0,
        },
        TestOp::Drive { side: Side::Server },
        TestOp::DropInbound { side: Side::Client },
    ];

    let _guard = subscribe();
    let (mut pair, client_config) = setup.run(prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, client_config, Establishment::Full);

    assert!(!pair.drive_bounded(1000), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    if let Some(server_ch) = server_ch {
        assert!(allowed_error(poll_to_close(
            pair.server_conn_mut(server_ch)
        )));
    }
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
    let setup = PairSetup {
        seed: Seed::Zeroes,
        extensions: Extensions::MultipathOnly,
        mtud_enabled: true,
        routing_setup: RoutingSetup::Complex(RoutingTable::from_routes(
            vec![("[::ffff:1.1.1.0]:44433".parse().unwrap(), 0)],
            vec![
                ("[::ffff:2.2.2.0]:4433".parse().unwrap(), 0),
                ("[::ffff:2.2.2.1]:4433".parse().unwrap(), 0),
            ],
        )),
    };
    let interactions = vec![
        TestOp::OpenPath {
            side: Side::Client,
            status: PathStatus::Available,
            addr_idx: 1,
        },
        TestOp::Drive { side: Side::Client },
        TestOp::AdvanceTime,
        TestOp::Drive { side: Side::Server },
        TestOp::OpenPath {
            side: Side::Client,
            status: PathStatus::Available,
            addr_idx: 1,
        },
        TestOp::ClosePath {
            side: Side::Server,
            path_idx: 0,
            error_code: 0,
        },
    ];

    let _guard = subscribe();
    let (mut pair, client_config) = setup.run(prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, client_config, Establishment::Full);

    assert!(!pair.drive_bounded(1000), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    if let Some(server_ch) = server_ch {
        assert!(allowed_error(poll_to_close(
            pair.server_conn_mut(server_ch)
        )));
    }
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
    let setup = PairSetup {
        seed: Seed::Zeroes,
        extensions: Extensions::MultipathOnly,
        mtud_enabled: true,
        routing_setup: RoutingSetup::SimpleSymmetric,
    };
    let interactions = vec![
        TestOp::CloseConn {
            side: Side::Server,
            error_code: 0,
        },
        TestOp::Drive { side: Side::Server },
        TestOp::DropInbound { side: Side::Client },
        TestOp::OpenPath {
            side: Side::Client,
            status: PathStatus::Available,
            addr_idx: 0,
        },
        TestOp::ClosePath {
            side: Side::Client,
            path_idx: 0,
            error_code: 0,
        },
        TestOp::AdvanceTime,
    ];

    let _guard = subscribe();
    let (mut pair, client_config) = setup.run(prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, client_config, Establishment::Full);

    assert!(!pair.drive_bounded(1000), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    if let Some(server_ch) = server_ch {
        assert!(allowed_error(poll_to_close(
            pair.server_conn_mut(server_ch)
        )));
    }
}

#[test]
fn regression_frame_encoding_error() {
    let prefix = "regression_frame_encoding_error";
    let setup = PairSetup {
        seed: Seed::Zeroes,
        extensions: Extensions::MultipathOnly,
        mtud_enabled: true,
        routing_setup: RoutingSetup::SimpleSymmetric,
    };
    let interactions = vec![
        TestOp::OpenPath {
            side: Side::Client,
            status: PathStatus::Available,
            addr_idx: 1,
        },
        TestOp::OpenPath {
            side: Side::Client,
            status: PathStatus::Available,
            addr_idx: 0,
        },
        TestOp::ClosePath {
            side: Side::Client,
            path_idx: 0,
            error_code: 0,
        },
    ];

    let _guard = subscribe();
    let (mut pair, client_config) = setup.run(prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, client_config, Establishment::Full);

    assert!(!pair.drive_bounded(1000), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    if let Some(server_ch) = server_ch {
        assert!(allowed_error(poll_to_close(
            pair.server_conn_mut(server_ch)
        )));
    }
}

#[test]
fn regression_there_should_be_at_least_one_path() {
    let prefix = "regression_there_should_be_at_least_one_path";
    let setup = PairSetup {
        seed: Seed::Zeroes,
        extensions: Extensions::MultipathOnly,
        mtud_enabled: true,
        routing_setup: RoutingSetup::SimpleSymmetric,
    };
    let interactions = vec![
        TestOp::PassiveMigration {
            side: Side::Client,
            addr_idx: 0,
        },
        TestOp::CloseConn {
            side: Side::Client,
            error_code: 0,
        },
    ];

    let _guard = subscribe();
    let (mut pair, client_config) = setup.run(prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, client_config, Establishment::Full);

    assert!(!pair.drive_bounded(1000), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    if let Some(server_ch) = server_ch {
        assert!(allowed_error(poll_to_close(
            pair.server_conn_mut(server_ch)
        )));
    }
}

/// This test will loop forever, unless the loss detection timer is allowed to back off
/// infinitely beyond the idle timeout.
///
/// The situation this test creates is one where there are two paths between client and
/// server, and path 0 is fully broken, while path 1 is fully working.
///
/// There is one packet with acknowledgements on path 0 that is sent but never delivered,
/// and the path will be broken forever due to the passive migration.
///
/// This will cause sending tail-loss probes (in practice that's an IMMEDIATE_ACK frame)
/// on path 1 forever, or at least until the loss detection timer is bigger than the
/// idle timeout, at which point the whole connection times out instead of resending.
///
/// Changes to the backoff behavior that make the loss detection timer not explode beyond
/// the idle timeout will make this test fail.
///
/// We fix this test by fixing the test setup: If we allow path 0 to time out on its own
/// with its own idle timeout, then the path closes and we finally stop resending tail
/// loss probes.
#[test]
fn regression_conn_never_idle5() {
    let prefix = "regression_conn_never_idle5";
    let setup = PairSetup {
        seed: Seed::Zeroes,
        extensions: Extensions::MultipathOnly,
        mtud_enabled: true,
        routing_setup: RoutingSetup::SimpleSymmetric,
    };
    let interactions = vec![
        TestOp::PassiveMigration {
            side: Side::Server,
            addr_idx: 0,
        },
        TestOp::OpenPath {
            side: Side::Client,
            status: PathStatus::Available,
            addr_idx: 0,
        },
    ];

    let _guard = subscribe();
    let (mut pair, client_config) = setup.run(prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, client_config, Establishment::Full);

    assert!(!pair.drive_bounded(1000), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    if let Some(server_ch) = server_ch {
        assert!(allowed_error(poll_to_close(
            pair.server_conn_mut(server_ch)
        )));
    }
}

/// Yet another regression with PATH_ABANDON "not being answered" by our peer.
///
/// This test ended up severing the connection in an interesting way: It generates
/// a passive migration just before closing the path. The passive migration breaks
/// path 0.
/// That path used the network path (1.1.1.0, 2.2.2.0), but was changed to
/// (1.1.1.1, 2.2.2.0) by a middlebox without the server noticing, since the client
/// never ended up sending anything on that path.
/// This means that path 0 is effectively broken on the server side, but the server
/// still has path 0 verified.
/// When the server then wants to abandon path 1, it chooses path 0 as the path to
/// send the path abandon on, even though that path is "doomed forever".
/// No retransmits make it through, and the client keeps using path 1, thus eventually
/// the server thinks the client ignored the PATH_ABANDON frame, although the client
/// just never *received* that frame.
///
/// We fixed this issue by not generating protocol violation errors anymore.
/// It's generally hard/impossible(?) to decide whether a PATH_ABANDON frame not
/// arriving means the client is not protocol compliant or just under bad network.
///
/// To prevent memory accumulation due to malicious client behavior, we now delay
/// sending MAX_PATH_ID until the client reciprocated the PATH_ABANDON instead.
#[test]
fn regression_peer_ignored_path_abandon() {
    let prefix = "regression_peer_ignored_path_abandon";
    let setup = PairSetup {
        seed: Seed::Zeroes,
        extensions: Extensions::MultipathOnly,
        mtud_enabled: true,
        routing_setup: RoutingSetup::SimpleSymmetric,
    };
    let interactions = vec![
        TestOp::OpenPath {
            side: Side::Client,
            status: PathStatus::Available,
            addr_idx: 0,
        },
        TestOp::Drive { side: Side::Client },
        TestOp::PathSetStatus {
            side: Side::Client,
            path_idx: 0,
            status: PathStatus::Backup,
        },
        TestOp::AdvanceTime,
        TestOp::Drive { side: Side::Server },
        TestOp::PassiveMigration {
            side: Side::Client,
            addr_idx: 0,
        },
        TestOp::ClosePath {
            side: Side::Server,
            path_idx: 1,
            error_code: 0,
        },
    ];

    let _guard = subscribe();
    let (mut pair, client_config) = setup.run(prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, client_config, Establishment::Full);

    assert!(!pair.drive_bounded(1000), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    if let Some(server_ch) = server_ch {
        assert!(allowed_error(poll_to_close(
            pair.server_conn_mut(server_ch)
        )));
    }
}

/// A regression test that used to put noq into a state of sending PATH_CHALLENGE
/// from the client side indefinitely.
///
/// The test uses passive migrations to establish a situation in which the client
/// expects the server to respond to a PATH_CHALLENGE that was sent on the interface
/// 1.1.1.1, but the server cannot respond to it anymore, because the client was
/// involuntarily migrated to path 1.1.1.2.
///
/// Here's a log line that shows the client skipping the path response due to it
/// being delivered "on the wrong network path" (after migration).
///
/// > DEBUG client:pkt{path_id=1}:recv{space=Data pn=3}:frame{ty=PATH_RESPONSE}:
/// > noq_proto::connection: 4704:
/// > ignoring invalid PATH_RESPONSE
/// >  response=PATH_RESPONSE(ece9dc07f89ded7e)
/// >  network_path=(local: ::ffff:1.1.1.2, remote: [::ffff:2.2.2.0]:4433)
/// >  expected=(local: ::ffff:1.1.1.1, remote: [::ffff:2.2.2.0]:4433)
///
/// The client will then never clear out the PATH_CHALLENGE from the "pending"
/// challenges, and so it will never fully clear the path challenge timer.
///
/// This issue was fixed by making sure to clear out challenges that were probing
/// 4-tuples that are different from the current network path.
#[test]
fn regression_never_idle4() {
    let prefix = "regression_never_idle4";
    let setup = PairSetup {
        seed: Seed::Zeroes,
        extensions: Extensions::MultipathOnly,
        mtud_enabled: true,
        routing_setup: RoutingSetup::Complex(RoutingTable::from_routes(
            vec![
                ("[::ffff:1.1.1.0]:44433".parse().unwrap(), 0),
                ("[::ffff:1.1.1.1]:44433".parse().unwrap(), 0),
            ],
            vec![("[::ffff:2.2.2.0]:4433".parse().unwrap(), 0)],
        )),
    };
    let interactions = vec![
        // Open path 1 with the same remote address as path 0
        TestOp::OpenPath {
            side: Side::Client,
            status: PathStatus::Backup,
            addr_idx: 0,
        },
        // Sets path 0 to backup, but generally just sends *something*
        TestOp::PathSetStatus {
            side: Side::Client,
            path_idx: 0,
            status: PathStatus::Backup,
        },
        // Sends the two packets (opening path 1 & setting path status on path 0)
        TestOp::Drive { side: Side::Client },
        // But loses those two packets
        TestOp::DropInbound { side: Side::Server },
        // Client's interface 0 now migrates from 1.1.1.0 to 1.1.1.1
        TestOp::PassiveMigration {
            side: Side::Client,
            addr_idx: 0,
        },
        TestOp::AdvanceTime,
        // Client closes path 0, path 1 is now the only remaining path for the client.
        // It will now always choose path 1 to send, even though it's not validated.
        TestOp::ClosePath {
            side: Side::Client,
            path_idx: 0,
            error_code: 0,
        },
        // Send out the packet containing the PATH_ABANDON
        TestOp::Drive { side: Side::Client },
        // Migrate the first interface from 1.1.1.1 to 1.1.1.2 now.
        TestOp::PassiveMigration {
            side: Side::Client,
            addr_idx: 0,
        },
    ];

    let _guard = subscribe();
    let (mut pair, client_config) = setup.run(prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, client_config, Establishment::Full);

    assert!(!pair.drive_bounded(1000), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    if let Some(server_ch) = server_ch {
        assert!(allowed_error(poll_to_close(
            pair.server_conn_mut(server_ch)
        )));
    }
}

/// This test reproduced an infinite loop in loss detection.
///
/// After `pair.drive_bounded(4539)` this still didn't finish driving.
/// With `pair.drive_bounded(4540)` the `drive_bounded` call never finishes
/// due to an infinite LossDetection timer loop:
/// - The loss detection timer would fire
/// - Upon detecting loss, it would re-set the loss detection timer to `now` (0ms delay) again
/// - Within a single `pair.step()` it would go back to the loss detection timer firing
///   (it expects timers to fire in the future eventually.)
///
/// The reason this tight loop existed was because the loss detection timer is relative to the
/// `time_of_last_ack_eliciting_packet`. If this becomes too old (and we cap the PTO duration to
/// a maximum), then this will always be in the past, thus causing timers to be set to the past.
///
/// Usually, the timer would fire and then cause something to happen, e.g. we send
/// a tail-loss probe.
/// But in this case it didn't because the tail-loss probe was "scheduled" for path 1,
/// which in this example is already abandoned by the time the loss detection timer fired.
///
/// This caused the conditions for the loss detection timer to never be cleared, as in-flight
/// packets would sit in that path indefinitely.
///
/// The fix was to only schedule loss detection timers for tail-loss probes when the path
/// is not yet abandoned.
#[test]
fn regression_infinite_loop() {
    let prefix = "regression_infinite_loop";
    let setup = PairSetup {
        seed: Seed::Zeroes,
        extensions: Extensions::MultipathOnly,
        mtud_enabled: true,
        routing_setup: RoutingSetup::SimpleSymmetric,
    };
    let interactions = vec![
        TestOp::OpenPath {
            side: Side::Client,
            status: PathStatus::Available,
            addr_idx: 0,
        },
        TestOp::OpenPath {
            side: Side::Client,
            status: PathStatus::Available,
            addr_idx: 1,
        },
        TestOp::PassiveMigration {
            side: Side::Server,
            addr_idx: 0,
        },
    ];

    let _guard = subscribe();
    let (mut pair, client_config) = setup.run(prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, client_config, Establishment::Full);

    // This bug originally occurred at exactly 4540 iterations.
    // At 4539 it still finishes (but fails the assertion).
    // At 4540 it the `drive_bounded` call never returns.
    assert!(!pair.drive_bounded(1000), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    if let Some(server_ch) = server_ch {
        assert!(allowed_error(poll_to_close(
            pair.server_conn_mut(server_ch)
        )));
    }
}

/// This test reproduced a situation in which a QNT-enabled connection sends path challenges indefinitely.
///
/// In this test setup, we enable QNT, call the required functions for adding addresses to holepunch,
/// and then eventually initiate the first holepunching round.
/// Before that, we also trigger a passive migration on the server side, effectively severing the connection
/// in the server -> client direction on path 0 (the only path at that time), because all packets are
/// rejected on the client side as coming from the wrong address.
///
/// What follows is that the server sends PATH_CHALLENGEs for path 0 (as that's what we've added as the
/// "holepunching address"), and initiating the holepunching means that we reuse existing paths if we
/// already have one on the required address, but we do *revalidate* them (triggering new PATH_CHALLENGEs).
///
/// However, in this code path, we didn't have anything that would prevent re-validated path challenges
/// to ever be stopped, so this revalidation would keep the connection busy in the path challenge sent ->
/// path challenge lost -> path challenge sent loop.
///
/// We fixed this bug by introducing another `OpenState::Revalidating`, and arming the `PathOpenFailed`
/// timer when we start revalidating a path.
#[test]
fn regression_qnt_revalidating_path_forever() {
    let prefix = "regression_qnt_revalidating_path_forever";
    let setup = PairSetup {
        seed: Seed::Zeroes,
        extensions: Extensions::QntAndMultipath,
        mtud_enabled: true,
        routing_setup: RoutingSetup::SimpleSymmetric,
    };
    let interactions = vec![
        TestOp::AddHpAddr {
            side: Side::Server,
            addr_idx: 0,
        },
        TestOp::Drive { side: Side::Server },
        TestOp::AdvanceTime,
        TestOp::Drive { side: Side::Client },
        TestOp::PassiveMigration {
            side: Side::Server,
            addr_idx: 0,
        },
        TestOp::AddHpAddr {
            side: Side::Client,
            addr_idx: 0,
        },
        TestOp::InitiateHpRound { side: Side::Client },
    ];

    let _guard = subscribe();
    let (mut pair, client_config) = setup.run(prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, client_config, Establishment::Full);

    assert!(!pair.drive_bounded(1000), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    if let Some(server_ch) = server_ch {
        assert!(allowed_error(poll_to_close(
            pair.server_conn_mut(server_ch)
        )));
    }
}

#[test]
fn regression_1() {
    let prefix = "regression_1";
    let setup = PairSetup {
        seed: Seed::Zeroes,
        extensions: Extensions::MultipathOnly,
        mtud_enabled: true,
        routing_setup: RoutingSetup::SimpleSymmetric,
    };
    let interactions = vec![
        TestOp::PassiveMigration {
            side: Side::Server,
            addr_idx: 0,
        },
        TestOp::SendDatagram {
            side: Side::Client,
            size: 0,
            drop: false,
        },
        TestOp::DriveBothToIdle,
        TestOp::FinishConnect,
        TestOp::DriveBothToIdle,
        TestOp::OpenPath {
            side: Side::Client,
            status: PathStatus::Available,
            addr_idx: 0,
        },
    ];

    let _guard = subscribe();
    let (mut pair, client_config) = setup.run(prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, client_config, Establishment::Full);

    assert!(!pair.drive_bounded(1000), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    if let Some(server_ch) = server_ch {
        assert!(allowed_error(poll_to_close(
            pair.server_conn_mut(server_ch)
        )));
    }
}

#[test]
fn regression_2() {
    let prefix = "regression_2";
    let setup = PairSetup {
        seed: Seed::Zeroes,
        extensions: Extensions::MultipathOnly,
        mtud_enabled: true,
        routing_setup: RoutingSetup::SimpleSymmetric,
    };
    let interactions = vec![
        TestOp::SendDatagram {
            side: Side::Client,
            size: 0,
            drop: false,
        },
        TestOp::PassiveMigration {
            side: Side::Server,
            addr_idx: 0,
        },
        TestOp::DriveBothToIdle,
        TestOp::FinishConnect,
        TestOp::DriveBothToIdle,
        TestOp::OpenPath {
            side: Side::Client,
            status: PathStatus::Available,
            addr_idx: 0,
        },
    ];

    let _guard = subscribe();
    let (mut pair, client_config) = setup.run(prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, client_config, Establishment::Full);

    assert!(!pair.drive_bounded(1000), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    if let Some(server_ch) = server_ch {
        assert!(allowed_error(poll_to_close(
            pair.server_conn_mut(server_ch)
        )));
    }
}

#[test]
fn regression_3() {
    let prefix = "regression_3";
    let setup = PairSetup {
        seed: Seed::Zeroes,
        extensions: Extensions::MultipathOnly,
        mtud_enabled: true,
        routing_setup: RoutingSetup::SimpleSymmetric,
    };
    let establishment = Establishment::BeforeHandshake;
    let interactions = vec![
        TestOp::DriveBothToIdle,
        TestOp::PassiveMigration {
            side: Side::Server,
            addr_idx: 0,
        },
        TestOp::OpenPath {
            side: Side::Client,
            status: PathStatus::Available,
            addr_idx: 2,
        },
        TestOp::PassiveMigration {
            side: Side::Server,
            addr_idx: 2,
        },
        TestOp::DriveBothToIdle,
        TestOp::OpenPath {
            side: Side::Client,
            status: PathStatus::Available,
            addr_idx: 0,
        },
    ];

    let _guard = subscribe();
    let (mut pair, client_config) = setup.run(prefix);
    let (client_ch, server_ch) =
        run_random_interaction(&mut pair, interactions, client_config, establishment);

    assert!(!pair.drive_bounded(1000), "connection never became idle");
    assert!(allowed_error(poll_to_close(
        pair.client_conn_mut(client_ch)
    )));
    if let Some(server_ch) = server_ch {
        assert!(allowed_error(poll_to_close(
            pair.server_conn_mut(server_ch)
        )));
    }
}
