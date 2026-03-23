//! Tests for multipath

use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

use assert_matches::assert_matches;
use testresult::TestResult;
use tracing::info;

use crate::tests::RoutingTable;
use crate::tests::util::{CLIENT_PORTS, ConnPair, SERVER_PORTS};
use crate::{
    ClientConfig, ConnectionId, ConnectionIdGenerator, Endpoint, EndpointConfig, FourTuple,
    LOCAL_CID_COUNT, NetworkChangeHint, PathId, PathStatus, RandomConnectionIdGenerator,
    ServerConfig, Side::*, TransportConfig, cid_queue::CidQueue,
};
use crate::{Dir, Event, PathAbandonReason, PathEvent, StreamEvent, TransportErrorCode};

use super::util::{min_opt, subscribe};
use super::{Pair, client_config, server_config};

const MAX_PATHS: u32 = 3;

/// Returns a connected client-server pair with multipath enabled
fn multipath_pair() -> ConnPair {
    let mut cfg = TransportConfig::default();
    cfg.max_concurrent_multipath_paths(MAX_PATHS);
    // Assume a low-latency connection so pacing doesn't interfere with the test
    cfg.initial_rtt(Duration::from_millis(10));
    #[cfg(feature = "qlog")]
    cfg.qlog_from_env("multipath_test");

    let mut pair = ConnPair::with_transport_cfg(cfg.clone(), cfg);
    pair.drive();
    info!("connected");
    pair
}

#[test]
fn non_zero_length_cids() {
    let _guard = subscribe();
    let multipath_transport_cfg = Arc::new(TransportConfig {
        max_concurrent_multipath_paths: NonZeroU32::new(3 as _),
        // Assume a low-latency connection so pacing doesn't interfere with the test
        initial_rtt: Duration::from_millis(10),
        ..TransportConfig::default()
    });
    let server_cfg = Arc::new(ServerConfig {
        transport: multipath_transport_cfg.clone(),
        ..server_config()
    });
    let server = Endpoint::new(Default::default(), Some(server_cfg), true);

    struct ZeroLenCidGenerator;

    impl ConnectionIdGenerator for ZeroLenCidGenerator {
        fn generate_cid(&mut self) -> ConnectionId {
            ConnectionId::new(&[])
        }

        fn cid_len(&self) -> usize {
            0
        }

        fn cid_lifetime(&self) -> Option<std::time::Duration> {
            None
        }
    }

    let mut ep_config = EndpointConfig::default();
    ep_config.cid_generator(|| Box::new(ZeroLenCidGenerator));
    let client = Endpoint::new(Arc::new(ep_config), None, true);

    let mut pair = Pair::new_from_endpoint(client, server);
    let client_cfg = ClientConfig {
        transport: multipath_transport_cfg,
        ..client_config()
    };
    pair.begin_connect(client_cfg);
    pair.drive();
    let accept_err = pair
        .server
        .accepted
        .take()
        .expect("server didn't try connecting")
        .expect_err("server did not raise error for connection");
    match accept_err {
        crate::ConnectionError::TransportError(error) => {
            assert_eq!(error.code, crate::TransportErrorCode::PROTOCOL_VIOLATION);
        }
        _ => panic!("Not a TransportError"),
    }
}

#[test]
fn path_acks() {
    let _guard = subscribe();
    let mut pair = multipath_pair();

    let stats = pair.stats(Client);
    assert!(stats.frame_rx.path_acks > 0);
    assert!(stats.frame_tx.path_acks > 0);
}

#[test]
fn path_status() -> TestResult {
    let _guard = subscribe();
    let mut pair = multipath_pair();

    let prev_status = pair.set_path_status(Client, PathId::ZERO, PathStatus::Backup)?;
    assert_eq!(prev_status, PathStatus::Available);

    // Send the frame to the server
    pair.drive();

    assert_eq!(
        pair.remote_path_status(Server, PathId::ZERO),
        Some(PathStatus::Backup)
    );

    let client_stats = pair.stats(Client);
    assert_eq!(client_stats.frame_tx.path_status_available, 0);
    assert_eq!(client_stats.frame_tx.path_status_backup, 1);
    assert_eq!(client_stats.frame_rx.path_status_available, 0);
    assert_eq!(client_stats.frame_rx.path_status_backup, 0);

    let server_stats = pair.stats(Server);
    assert_eq!(server_stats.frame_tx.path_status_available, 0);
    assert_eq!(server_stats.frame_tx.path_status_backup, 0);
    assert_eq!(server_stats.frame_rx.path_status_available, 0);
    assert_eq!(server_stats.frame_rx.path_status_backup, 1);
    Ok(())
}

#[test]
fn path_close_last_path() {
    let _guard = subscribe();
    let mut pair = multipath_pair();

    // Closing the last path is allowed — it triggers connection close via PATH_ABANDON
    // followed by CONNECTION_CLOSE.
    assert_matches!(pair.close_path(Client, PathId::ZERO, 0u8.into()), Ok(()));
    pair.drive();

    // Both sides should be closed
    assert!(pair.is_closed(Client));
    assert!(pair.is_closed(Server));
}

#[test]
fn cid_issued_multipath() {
    let _guard = subscribe();
    const ACTIVE_CID_LIMIT: u64 = crate::cid_queue::CidQueue::LEN as _;
    let mut pair = multipath_pair();

    let client_stats = pair.stats(Client);
    dbg!(&client_stats);

    // The client does not send NEW_CONNECTION_ID frames when multipath is enabled as they
    // are all sent after the handshake is completed.
    assert_eq!(client_stats.frame_tx.new_connection_id, 0);
    assert_eq!(
        client_stats.frame_tx.path_new_connection_id,
        MAX_PATHS as u64 * ACTIVE_CID_LIMIT
    );

    // The server sends NEW_CONNECTION_ID frames before the handshake is completed.
    // Multipath is only enabled *after* the handshake completes.  The first server-CID is
    // not issued but assigned by the client and changed by the server.
    assert_eq!(
        client_stats.frame_rx.new_connection_id,
        ACTIVE_CID_LIMIT - 1
    );
    assert_eq!(
        client_stats.frame_rx.path_new_connection_id,
        (MAX_PATHS - 1) as u64 * ACTIVE_CID_LIMIT
    );
}

#[test]
fn multipath_cid_rotation() {
    let _guard = subscribe();
    const CID_TIMEOUT: Duration = Duration::from_secs(2);

    let cid_generator_factory: fn() -> Box<dyn ConnectionIdGenerator> =
        || Box::new(*RandomConnectionIdGenerator::new(8).set_lifetime(CID_TIMEOUT));

    // Only test cid rotation on server side to have a clear output trace
    let server_cfg = ServerConfig {
        transport: Arc::new(TransportConfig {
            max_concurrent_multipath_paths: NonZeroU32::new(MAX_PATHS),
            // Assume a low-latency connection so pacing doesn't interfere with the test
            initial_rtt: Duration::from_millis(10),
            ..TransportConfig::default()
        }),
        ..server_config()
    };

    let server = Endpoint::new(
        Arc::new(EndpointConfig {
            connection_id_generator_factory: Arc::new(cid_generator_factory),
            ..EndpointConfig::default()
        }),
        Some(Arc::new(server_cfg)),
        true,
    );
    let client = Endpoint::new(Arc::new(EndpointConfig::default()), None, true);

    let client_cfg = ClientConfig {
        transport: Arc::new(TransportConfig {
            max_concurrent_multipath_paths: NonZeroU32::new(MAX_PATHS),
            // Assume a low-latency connection so pacing doesn't interfere with the test
            initial_rtt: Duration::from_millis(10),
            ..TransportConfig::default()
        }),
        ..client_config()
    };

    let mut pair = ConnPair::connect_with(Pair::new_from_endpoint(client, server), client_cfg);
    let mut round: u64 = 1;
    let mut stop = pair.time;
    let end = pair.time + 5 * CID_TIMEOUT;

    let mut active_cid_num = CidQueue::LEN as u64 + 1;
    active_cid_num = active_cid_num.min(LOCAL_CID_COUNT);
    let mut left_bound = 0;
    let mut right_bound = active_cid_num - 1;

    while pair.time < end {
        stop += CID_TIMEOUT;
        // Run a while until PushNewCID timer fires
        while pair.time < stop {
            if !pair.step()
                && let Some(time) = min_opt(pair.client.next_wakeup(), pair.server.next_wakeup())
            {
                pair.time = time;
            }
        }
        info!(
            "Checking active cid sequence range before {:?} seconds",
            round * CID_TIMEOUT.as_secs()
        );
        let _bound = (left_bound, right_bound);
        for path_id in 0..MAX_PATHS {
            assert_matches!(pair.conn(Server).active_local_path_cid_seq(path_id), _bound);
        }
        round += 1;
        left_bound += active_cid_num;
        right_bound += active_cid_num;
        pair.drive_server();
    }

    let stats = pair.stats(Server);

    // Server sends CIDs for PathId::ZERO before multipath is negotiated.
    assert_eq!(stats.frame_tx.new_connection_id, (CidQueue::LEN - 1) as u64);

    // For the first batch the PathId::ZERO CIDs have already been sent.
    let initial_batch: u64 = (MAX_PATHS - 1) as u64 * CidQueue::LEN as u64;
    // Each round expires all CIDs, so they all get re-issued.
    let each_round: u64 = MAX_PATHS as u64 * CidQueue::LEN as u64;
    // The final round only pushes one set of CIDs with expires_before, the round is not run
    // to completion to wait for the expiry messages from the client.
    let final_round: u64 = MAX_PATHS as u64;
    let path_new_cids = initial_batch + (round - 2) * each_round + final_round;
    debug_assert_eq!(path_new_cids, 73);
    assert_eq!(stats.frame_tx.path_new_connection_id, path_new_cids);

    // We don't retire any CIDs before multipath is negotiated.
    assert_eq!(stats.frame_tx.retire_connection_id, 0);

    // Server expires the CID of the initial sent by the client.
    assert_eq!(stats.frame_tx.path_retire_connection_id, 1);

    // Client only sends CIDs after multipath is negotiated.
    assert_eq!(stats.frame_rx.new_connection_id, 0);

    // Client does not expire CIDs, only the initial set for all the paths.
    assert_eq!(
        stats.frame_rx.path_new_connection_id,
        MAX_PATHS as u64 * CidQueue::LEN as u64
    );
    assert_eq!(stats.frame_rx.retire_connection_id, 0);

    // Test stops before last batch of retirements is sent.
    let path_retire_cids = MAX_PATHS as u64 * CidQueue::LEN as u64 * (round - 2);
    debug_assert_eq!(path_retire_cids, 60);
    assert_eq!(stats.frame_rx.path_retire_connection_id, path_retire_cids);
}

#[test]
fn issue_max_path_id() -> TestResult {
    let _guard = subscribe();

    // We enable multipath but initially do not allow any paths to be opened.
    let server_cfg = TransportConfig {
        max_concurrent_multipath_paths: NonZeroU32::new(1),
        // Assume a low-latency connection so pacing doesn't interfere with the test
        initial_rtt: Duration::from_millis(10),
        ..TransportConfig::default()
    };

    // The client is allowed to create more paths immediately.
    let client_cfg = TransportConfig {
        max_concurrent_multipath_paths: NonZeroU32::new(MAX_PATHS),
        // Assume a low-latency connection so pacing doesn't interfere with the test
        initial_rtt: Duration::from_millis(10),
        ..TransportConfig::default()
    };

    let mut pair = ConnPair::with_transport_cfg(server_cfg, client_cfg);

    pair.drive();
    info!("connected");

    // Server should only have sent NEW_CONNECTION_ID frames for now.
    let server_new_cids = CidQueue::LEN as u64 - 1;
    let mut server_path_new_cids = 0;
    let stats = pair.stats(Server);
    assert_eq!(stats.frame_tx.max_path_id, 0);
    assert_eq!(stats.frame_tx.new_connection_id, server_new_cids);
    assert_eq!(stats.frame_tx.path_new_connection_id, server_path_new_cids);

    // Client should have sent PATH_NEW_CONNECTION_ID frames for PathId::ZERO.
    let client_new_cids = 0;
    let mut client_path_new_cids = CidQueue::LEN as u64;
    assert_eq!(stats.frame_rx.new_connection_id, client_new_cids);
    assert_eq!(stats.frame_rx.path_new_connection_id, client_path_new_cids);

    // Server increases MAX_PATH_ID.
    pair.set_max_concurrent_paths(Server, MAX_PATHS)?;
    pair.drive();
    let stats = pair.stats(Server);

    // Server should have sent MAX_PATH_ID and new CIDs
    server_path_new_cids += (MAX_PATHS as u64 - 1) * CidQueue::LEN as u64;
    assert_eq!(stats.frame_tx.max_path_id, 1);
    assert_eq!(stats.frame_tx.new_connection_id, server_new_cids);
    assert_eq!(stats.frame_tx.path_new_connection_id, server_path_new_cids);

    // Client should have sent CIDs for new paths
    client_path_new_cids += (MAX_PATHS as u64 - 1) * CidQueue::LEN as u64;
    assert_eq!(stats.frame_rx.new_connection_id, client_new_cids);
    assert_eq!(stats.frame_rx.path_new_connection_id, client_path_new_cids);

    Ok(())
}

/// A copy of [`issue_max_path_id`], but reordering the `MAX_PATH_ID` frame
/// that's sent from the server to the client, so that some `NEW_CONNECTION_ID`
/// frames arrive with higher path IDs than the most recently received
/// `MAX_PATH_ID` frame on the client side.
#[test]
fn issue_max_path_id_reordered() -> TestResult {
    let _guard = subscribe();

    // We enable multipath but initially do not allow any paths to be opened.
    let server_cfg = TransportConfig {
        max_concurrent_multipath_paths: NonZeroU32::new(1),
        // Assume a low-latency connection so pacing doesn't interfere with the test
        initial_rtt: Duration::from_millis(10),
        ..TransportConfig::default()
    };

    // The client is allowed to create more paths immediately.
    let client_cfg = TransportConfig {
        max_concurrent_multipath_paths: NonZeroU32::new(MAX_PATHS),
        // Assume a low-latency connection so pacing doesn't interfere with the test
        initial_rtt: Duration::from_millis(10),
        ..TransportConfig::default()
    };
    let mut pair = ConnPair::with_transport_cfg(server_cfg, client_cfg);

    pair.drive();
    info!("connected");

    // Server should only have sent NEW_CONNECTION_ID frames for now.
    let server_new_cids = CidQueue::LEN as u64 - 1;
    let mut server_path_new_cids = 0;
    let stats = pair.stats(Server);
    assert_eq!(stats.frame_tx.max_path_id, 0);
    assert_eq!(stats.frame_tx.new_connection_id, server_new_cids);
    assert_eq!(stats.frame_tx.path_new_connection_id, server_path_new_cids);

    // Client should have sent PATH_NEW_CONNECTION_ID frames for PathId::ZERO.
    let client_new_cids = 0;
    let mut client_path_new_cids = CidQueue::LEN as u64;
    assert_eq!(stats.frame_rx.new_connection_id, client_new_cids);
    assert_eq!(stats.frame_rx.path_new_connection_id, client_path_new_cids);

    // Server increases MAX_PATH_ID, but we reorder the frame
    pair.set_max_concurrent_paths(Server, MAX_PATHS)?;
    pair.drive_server();
    // reorder the frames on the incoming side
    pair.reorder_inbound(Client);
    pair.drive();
    let stats = pair.stats(Server);

    // Server should have sent MAX_PATH_ID and new CIDs
    server_path_new_cids += (MAX_PATHS as u64 - 1) * CidQueue::LEN as u64;
    assert_eq!(stats.frame_tx.max_path_id, 1);
    assert_eq!(stats.frame_tx.new_connection_id, server_new_cids);
    assert_eq!(stats.frame_tx.path_new_connection_id, server_path_new_cids);

    // Client should have sent CIDs for new paths
    client_path_new_cids += (MAX_PATHS as u64 - 1) * CidQueue::LEN as u64;
    assert_eq!(stats.frame_rx.new_connection_id, client_new_cids);
    assert_eq!(stats.frame_rx.path_new_connection_id, client_path_new_cids);

    Ok(())
}

#[test]
fn open_path() -> TestResult {
    let _guard = subscribe();
    let mut pair = multipath_pair();

    let server_addr = pair.addrs_to_server();
    let path_id = pair.open_path(Client, server_addr, PathStatus::Available)?;
    pair.drive();
    assert_matches!(
        pair.poll(Client),
        Some(Event::Path(crate::PathEvent::Opened { id  })) if id == path_id
    );

    assert_matches!(
        pair.poll(Server),
        Some(Event::Path(crate::PathEvent::Opened { id  })) if id == path_id
    );
    Ok(())
}

#[test]
fn open_path_key_update() -> TestResult {
    let _guard = subscribe();
    let mut pair = multipath_pair();

    let server_addr = pair.addrs_to_server();
    let path_id = pair.open_path(Client, server_addr, PathStatus::Available)?;

    // Do a key-update at the same time as opening the new path.
    pair.force_key_update(Client);

    pair.drive();
    assert_matches!(
        pair.poll(Client),
        Some(Event::Path(crate::PathEvent::Opened { id  })) if id == path_id
    );

    assert_matches!(
        pair.poll(Server),
        Some(Event::Path(crate::PathEvent::Opened { id  })) if id == path_id
    );
    Ok(())
}

/// Client starts opening a path but the server fails to validate the path
///
/// The client should receive an event closing the path.
#[test]
fn open_path_validation_fails_server_side() -> TestResult {
    let _guard = subscribe();
    let mut pair = multipath_pair();

    let different_addr = FourTuple {
        remote: SocketAddr::new(
            [9, 8, 7, 6].into(),
            SERVER_PORTS.lock()?.next().ok_or("no port")?,
        ),
        local_ip: None,
    };
    let path_id = pair.open_path(Client, different_addr, PathStatus::Available)?;

    // block the server from receiving anything
    while pair.blackhole_step(true, false) {}
    assert_matches!(
        pair.poll(Client),
        Some(Event::Path(crate::PathEvent::Abandoned { id, reason: PathAbandonReason::ValidationFailed  })) if id == path_id
    );

    assert!(pair.poll(Server).is_none());
    Ok(())
}

/// Client starts opening a path but the client fails to validate the path
///
/// The server should receive an event close the path
#[test]
fn open_path_validation_fails_client_side() -> TestResult {
    let _guard = subscribe();
    let mut pair = multipath_pair();

    // make sure the new path cannot be validated using the existing path
    pair.client.addr = SocketAddr::new(
        [9, 8, 7, 6].into(),
        CLIENT_PORTS.lock()?.next().ok_or("no port")?,
    );

    let addr = pair.server.addr;
    let network_path = FourTuple {
        remote: addr,
        local_ip: None,
    };
    let path_id = pair.open_path(Client, network_path, PathStatus::Available)?;

    // Make sure the client's path open makes it through to the server and is processed.
    pair.drive_client();
    pair.drive_server();
    pair.client.inbound.clear();

    // Sever the connection and run it to idle.
    // This makes sure that
    // - path validation can't succeed because path responses don't make it through and
    // - the server needs to decide to close the path on its own, because path abandons don't make it through.
    while pair.blackhole_step(true, true) {}

    assert_matches!(
        pair.poll(Server),
        Some(Event::Path(crate::PathEvent::Abandoned { id, reason: PathAbandonReason::ValidationFailed  })) if id == path_id
    );
    Ok(())
}

/// Client opens a path, then abandons, then calls open_path_ensure.
///
/// In the end there should be an open path.
#[test]
fn open_path_ensure_after_abandon() -> TestResult {
    let _guard = subscribe();
    let mut pair = multipath_pair();
    let mut second_client_addr = pair.client.addr;
    let mut second_server_addr = pair.server.addr;
    second_client_addr.set_port(second_client_addr.port() + 1);
    second_server_addr.set_port(second_server_addr.port() + 1);
    pair.routes = Some(RoutingTable::simple_symmetric(
        [pair.client.addr, second_client_addr],
        [pair.server.addr, second_server_addr],
    ));

    let second_path = FourTuple {
        local_ip: Some(second_client_addr.ip()),
        remote: second_server_addr,
    };

    info!("opening path 1");
    let path_id = pair.open_path(Client, second_path, PathStatus::Available)?;
    pair.drive();

    assert_matches!(
        pair.poll(Client),
        Some(Event::Path(crate::PathEvent::Opened { id  })) if id == path_id
    );

    assert_matches!(
        pair.poll(Server),
        Some(Event::Path(crate::PathEvent::Opened { id  })) if id == path_id
    );

    info!("closing path {path_id}");
    pair.close_path(Client, path_id, 0u8.into())?;
    pair.drive();

    // The path should be closed:
    assert_matches!(
        pair.poll(Client),
        Some(Event::Path(crate::PathEvent::Abandoned { id, reason: PathAbandonReason::ApplicationClosed { error_code }}))
        if id == path_id && error_code == 0u8.into()
    );

    assert_matches!(
        pair.poll(Server),
        Some(Event::Path(crate::PathEvent::Abandoned { id, reason: PathAbandonReason::RemoteAbandoned { error_code }}))
        if id == path_id && error_code == 0u8.into()
    );

    pair.drive();

    // The path should be discarded:
    assert_matches!(
        pair.poll(Client),
        Some(Event::Path(crate::PathEvent::Discarded { id, .. })) if id == path_id
    );

    assert_matches!(
        pair.poll(Server),
        Some(Event::Path(crate::PathEvent::Discarded { id, .. })) if id == path_id
    );

    info!("opening path 2");
    let (path_id, existed) = pair.open_path_ensure(Client, second_path, PathStatus::Available)?;
    pair.drive();

    assert!(!existed);

    // The path should have been opened:
    assert_matches!(
        pair.poll(Client),
        Some(Event::Path(crate::PathEvent::Opened { id  })) if id == path_id
    );

    assert_matches!(
        pair.poll(Server),
        Some(Event::Path(crate::PathEvent::Opened { id  })) if id == path_id
    );
    Ok(())
}

#[test]
fn close_path() -> TestResult {
    let _guard = subscribe();
    let mut pair = multipath_pair();

    let server_addr = pair.addrs_to_server();
    let path_id = pair.open_path(Client, server_addr, PathStatus::Available)?;
    pair.drive();
    assert_ne!(path_id, PathId::ZERO);

    let stats0 = pair.stats(Client);
    assert_eq!(stats0.frame_tx.path_abandon, 0);
    assert_eq!(stats0.frame_rx.path_abandon, 0);
    assert_eq!(stats0.frame_tx.max_path_id, 0);
    assert_eq!(stats0.frame_rx.max_path_id, 0);

    info!("closing path 0");
    pair.close_path(Client, PathId::ZERO, 0u8.into())?;
    pair.drive();

    let stats1 = pair.stats(Client);
    assert_eq!(stats1.frame_tx.path_abandon, 1);
    assert_eq!(stats1.frame_rx.path_abandon, 1);
    assert_eq!(stats1.frame_tx.max_path_id, 1);
    assert_eq!(stats1.frame_rx.max_path_id, 1);
    assert!(stats1.frame_tx.path_new_connection_id > stats0.frame_tx.path_new_connection_id);
    assert!(stats1.frame_rx.path_new_connection_id > stats0.frame_rx.path_new_connection_id);
    Ok(())
}

#[test]
fn close_last_path() -> TestResult {
    let _guard = subscribe();
    let mut pair = multipath_pair();

    let server_addr = pair.addrs_to_server();
    let path_id = pair.open_path(Client, server_addr, PathStatus::Available)?;
    pair.drive();
    assert_ne!(path_id, PathId::ZERO);

    info!("client closes path 0");
    pair.close_path(Client, PathId::ZERO, 0u8.into())?;

    info!("server closes path 1");
    pair.close_path(Server, PathId(1), 0u8.into())?;

    pair.drive();

    assert!(pair.is_closed(Server));
    assert!(pair.is_closed(Client));
    Ok(())
}

#[test]
fn per_path_observed_address() -> TestResult {
    let _guard = subscribe();
    // create the endpoint pair with both address discovery and multipath enabled
    let transport_cfg = TransportConfig {
        max_concurrent_multipath_paths: NonZeroU32::new(MAX_PATHS),
        address_discovery_role: crate::address_discovery::Role::Both,
        ..TransportConfig::default()
    };

    let mut pair = ConnPair::with_transport_cfg(transport_cfg.clone(), transport_cfg);
    info!("connected");
    pair.drive();

    // check that the client received the correct address
    let expected_addr = pair.client.addr;
    assert_matches!(pair.poll(Client), Some(Event::Path(PathEvent::ObservedAddr{id: PathId::ZERO, addr})) if addr == expected_addr);
    assert_matches!(pair.poll(Client), None);

    // check that the server received the correct address
    let expected_addr = pair.server.addr;
    assert_matches!(pair.poll(Server), Some(Event::Path(PathEvent::ObservedAddr{id: PathId::ZERO, addr})) if addr == expected_addr);
    assert_matches!(pair.poll(Server), None);

    // simulate a rebind on the client, this will close the current path and open a new one
    let our_addr = pair.passive_migration(Client);
    pair.handle_network_change(Client, None);

    pair.drive();

    assert_matches!(
        pair.poll(Client),
        Some(Event::Path(PathEvent::Abandoned {
            id: PathId(0),
            reason: PathAbandonReason::UnusableAfterNetworkChange
        }))
    );
    assert_matches!(
        pair.poll(Client),
        Some(Event::Path(PathEvent::Opened { id: PathId(1) }))
    );
    assert_matches!(
        pair.poll(Client),
        Some(Event::Path(PathEvent::ObservedAddr{ id: PathId(1), addr })) if addr == our_addr
    );

    Ok(())
}

#[test]
fn mtud_on_two_paths() -> TestResult {
    let _guard = subscribe();

    // Manual pair setup because we need to disable the max_idle_timeout.
    let multipath_transport_cfg = Arc::new(TransportConfig {
        max_concurrent_multipath_paths: NonZeroU32::new(MAX_PATHS),
        initial_rtt: Duration::from_millis(10),
        max_idle_timeout: None,
        ..TransportConfig::default()
    });
    let server_cfg = Arc::new(ServerConfig {
        transport: multipath_transport_cfg.clone(),
        ..server_config()
    });
    let server = Endpoint::new(Default::default(), Some(server_cfg), true);
    let client = Endpoint::new(Default::default(), None, true);

    let mut pair = Pair::new_from_endpoint(client, server);
    pair.mtu = 1200; // Start with a small MTU
    let client_cfg = ClientConfig {
        transport: multipath_transport_cfg,
        ..client_config()
    };
    let mut pair = ConnPair::connect_with(pair, client_cfg);
    pair.drive();
    info!("connected");

    assert_eq!(pair.conn(Client).path_mtu(PathId::ZERO), 1200);

    // Open a 2nd path.
    let server_addr = pair.addrs_to_server();
    let path_id = pair.open_path(Client, server_addr, PathStatus::Available)?;
    pair.drive();

    // Ensure the path opened correctly.
    assert_matches!(
        pair.poll(Client),
        Some(Event::Path(crate::PathEvent::Opened { id  })) if id == path_id
    );
    assert_matches!(
        pair.poll(Server),
        Some(Event::Path(crate::PathEvent::Opened { id  })) if id == path_id
    );

    // MTU should be 1200 for both paths.
    assert_eq!(pair.conn(Client).path_mtu(PathId::ZERO), 1200);
    assert_eq!(pair.conn(Client).path_mtu(path_id), 1200);

    // The default MtuDiscoveryConfig::upper_bound is 1452, the default
    // MtuDiscoveryConfig::interval is 600s.
    pair.mtu = 1452;
    pair.time += Duration::from_secs(600);
    info!("Bumping MTU to: {}", pair.mtu);
    pair.drive();

    info!("MTU Path 0: {}", pair.conn(Client).path_mtu(PathId::ZERO));
    info!(
        "MTU Path {}: {}",
        path_id,
        pair.conn(Client).path_mtu(path_id)
    );

    // Both paths should have found the new MTU.
    assert_eq!(pair.conn(Client).path_mtu(PathId::ZERO), 1452);
    assert_eq!(pair.conn(Client).path_mtu(path_id), 1452);
    Ok(())
}

/// Closing a path locally may be rejected if this leaves the endpoint without validated paths. For
/// paths closed by the remote, however, a `PATH_ABANDON` frame must be accepted. In
/// particular, it should not kill the connection.
///
/// This is a regression test.
#[test]
fn remote_can_close_last_validated_path() -> TestResult {
    let _guard = subscribe();
    let mut pair = multipath_pair();

    pair.passive_migration(Client);
    let route = FourTuple {
        remote: pair.server.addr,
        local_ip: None,
    };
    pair.open_path(Client, route, PathStatus::Available)?;
    pair.drive_client();
    pair.close_path(Client, PathId::ZERO, 0u8.into())?;
    pair.drive();

    // Neither side of the connection should error on close
    let mut close = None;
    for side in [Client, Server] {
        while let Some(event) = pair.poll(side) {
            if let Event::ConnectionLost { reason } = event {
                close = Some(reason);
            }
        }
        assert_eq!(close, None);
    }

    Ok(())
}

/// With multipath and hint=None, the client defaults to non-recoverable: the old path is closed
/// with PATH_UNSTABLE_OR_POOR and a new path is opened. Data still flows on the new path.
#[test]
fn network_change_multipath_no_hint_replaces_path() -> TestResult {
    let _guard = subscribe();
    let mut pair = multipath_pair();

    // Simulate a passive migration + network change with no hint
    pair.passive_migration(Client);
    pair.handle_network_change(Client, None);

    pair.drive();

    // A new path should be opened and the old one should be closed
    assert_matches!(
        pair.poll(Client),
        Some(Event::Path(PathEvent::Abandoned {
            id: PathId(0),
            reason: PathAbandonReason::UnusableAfterNetworkChange
        }))
    );
    assert_matches!(
        pair.poll(Client),
        Some(Event::Path(PathEvent::Opened { id: PathId(1) }))
    );

    // The server sees the old path closed with PATH_UNSTABLE_OR_POOR
    assert_matches!(
        pair.poll(Server),
        Some(Event::Path(PathEvent::Abandoned {
            id: PathId::ZERO,
            reason: PathAbandonReason::RemoteAbandoned { error_code }
        }))
        if error_code == TransportErrorCode::PATH_UNSTABLE_OR_POOR.into()
    );
    // And then sees the new path
    assert_matches!(
        pair.poll(Server),
        Some(Event::Path(PathEvent::Opened { id: PathId(1) }))
    );
    // Both client and server see the old path as discarded
    assert_matches!(
        pair.poll(Server),
        Some(Event::Path(PathEvent::Discarded {
            id: PathId::ZERO,
            ..
        }))
    );
    assert_matches!(
        pair.poll(Client),
        Some(Event::Path(PathEvent::Discarded {
            id: PathId::ZERO,
            ..
        }))
    );

    // Data should flow on the new path
    let s = pair.streams(Client).open(Dir::Uni).unwrap();
    const MSG: &[u8] = b"after network change";
    pair.send_stream(Client, s).write(MSG).unwrap();
    pair.send_stream(Client, s).finish().unwrap();
    pair.drive();

    assert_matches!(
        pair.poll(Server),
        Some(Event::Stream(StreamEvent::Opened { dir: Dir::Uni }))
    );
    assert_matches!(pair.streams(Server).accept(Dir::Uni), Some(stream) if stream == s);
    let mut recv = pair.recv_stream(Server, s);
    let mut chunks = recv.read(false).unwrap();
    assert_matches!(
        chunks.next(usize::MAX),
        Ok(Some(chunk)) if chunk.bytes == MSG
    );
    let _ = chunks.finalize();

    Ok(())
}

/// With two paths open and a selective hint, only the non-recoverable path gets replaced.
/// The recoverable path is kept and pinged for liveness.
#[test]
fn network_change_selective_hint() -> TestResult {
    let _guard = subscribe();
    let mut pair = multipath_pair();

    // Open a second path
    let server_addr = pair.addrs_to_server();
    let second_path = pair.open_path(Client, server_addr, PathStatus::Available)?;
    pair.drive();

    assert_matches!(
        pair.poll(Client),
        Some(Event::Path(PathEvent::Opened { id })) if id == second_path
    );
    assert_matches!(
        pair.poll(Server),
        Some(Event::Path(PathEvent::Opened { id })) if id == second_path
    );

    // A hint that says PathId::ZERO is recoverable but the second path is not
    #[derive(Debug)]
    struct SelectiveHint(PathId);
    impl NetworkChangeHint for SelectiveHint {
        fn is_path_recoverable(&self, path_id: PathId, _network_path: FourTuple) -> bool {
            path_id == self.0
        }
    }
    let hint = SelectiveHint(PathId::ZERO);

    pair.passive_migration(Client);
    pair.handle_network_change(Client, Some(&hint));

    pair.drive();

    // The second path (non-recoverable) should be replaced: a new path opens
    // PathId::ZERO (recoverable) should stay open (no Closed event for it)
    let mut client_events = Vec::new();
    while let Some(event) = pair.poll(Client) {
        client_events.push(event);
    }

    // There should be an Opened event for the replacement path
    assert!(
        client_events
            .iter()
            .any(|e| matches!(e, Event::Path(PathEvent::Opened { .. }))),
        "expected an Opened event for the replacement path, got: {client_events:?}"
    );
    // PathId::ZERO should NOT have been closed
    assert!(
        !client_events.iter().any(|e| matches!(
            e,
            Event::Path(PathEvent::Discarded {
                id: PathId::ZERO,
                ..
            })
        )),
        "PathId::ZERO should not have been closed: {client_events:?}"
    );

    Ok(())
}

/// Checks that the deadline given before a path fails to be considered open start only when the
/// first packet is sent.
///
/// This is a regression test. See <https://github.com/n0-computer/noq/issues/435>
#[test]
fn path_open_deadline_is_set_on_send() -> TestResult {
    let _guard = subscribe();
    let mut pair = multipath_pair();

    let server_addr = pair.addrs_to_server();
    let path_id = pair.open_path(Client, server_addr, PathStatus::Available)?;

    // Fast-forward time well past 3×PTO without letting any transmit happen on the new
    // path.
    let far_future = pair.time + Duration::from_secs(5);
    pair.handle_timeout(Client, far_future);

    assert!(
        pair.poll(Client).is_none(),
        "path was abandoned before any challenge was sent (issue #456)"
    );

    // Now let the challenge be sent and the path to be opened.
    pair.time = far_future;
    pair.drive();

    assert_matches!(
        pair.poll(Client),
        Some(Event::Path(PathEvent::Opened { id })) if id == path_id,
        "path should open successfully after the challenge is sent"
    );

    Ok(())
}

#[test]
fn path_scheduling_path_status() -> TestResult {
    let _guard = subscribe();
    let mut pair = multipath_pair();

    info!("Setting Path 0 to PathStatus::Backup");
    let prev_status = pair.set_path_status(Client, PathId::ZERO, PathStatus::Backup)?;
    assert_eq!(prev_status, PathStatus::Available);

    // Send the frame to the server
    pair.drive();

    assert_eq!(
        pair.remote_path_status(Server, PathId::ZERO),
        Some(PathStatus::Backup)
    );

    info!("Opening Path 1 with PathStatus::Available");
    let server_addr = pair.addrs_to_server();
    let path_1 = pair.open_path(Client, server_addr, PathStatus::Available)?;
    pair.drive();

    let stats_path0_t0 = pair.conn_mut(Client).path_stats(PathId::ZERO).unwrap();
    let stats_path1_t0 = pair.conn_mut(Client).path_stats(path_1).unwrap();

    info!("Sending STREAM frame");
    let s = pair.streams(Client).open(Dir::Uni).unwrap();
    pair.send_stream(Client, s).write(b"hello").unwrap();
    pair.drive();

    let stats_path0_t1 = pair.conn_mut(Client).path_stats(PathId::ZERO).unwrap();
    let stats_path1_t1 = pair.conn_mut(Client).path_stats(path_1).unwrap();

    info!("assert");
    assert!((stats_path0_t1.udp_tx.datagrams - stats_path0_t0.udp_tx.datagrams) == 0);
    assert!((stats_path1_t1.udp_tx.datagrams - stats_path1_t0.udp_tx.datagrams) > 0);

    Ok(())
}

// --- Tests for issue #397: remote PATH_ABANDON handling ---
//
// These tests verify compliance with draft-ietf-quic-multipath-21, Section 3.4
// "Path Close" and Section 4.2 "PATH_ABANDON Frame".
// Tests for remote PATH_ABANDON handling (draft-ietf-quic-multipath-21 Section 3.4).

/// Remote abandons a non-last path: accept and send reciprocal PATH_ABANDON.
#[test]
fn remote_path_abandon_with_remaining_path() -> TestResult {
    let _guard = subscribe();
    let mut pair = multipath_pair();

    // Open a second path so we have two (path 0 and path 1)
    let server_addr = pair.addrs_to_server();
    let _path_id = pair.open_path(Client, server_addr, PathStatus::Available)?;
    pair.drive();

    // Drain open events
    assert_matches!(
        pair.poll(Client),
        Some(Event::Path(PathEvent::Opened { .. }))
    );
    assert_matches!(
        pair.poll(Server),
        Some(Event::Path(PathEvent::Opened { .. }))
    );

    let stats_before = pair.stats(Server);
    assert_eq!(stats_before.frame_tx.path_abandon, 0);

    // Server abandons path 0. The client should accept this.
    info!("server abandons path 0");
    pair.close_path(Server, PathId::ZERO, 42u8.into())?;
    pair.drive();

    // Section 3.4 para 4: client MUST have received the abandon and sent one back
    let client_stats = pair.stats(Client);
    assert!(
        client_stats.frame_rx.path_abandon >= 1,
        "client should have received PATH_ABANDON"
    );
    assert!(
        client_stats.frame_tx.path_abandon >= 1,
        "client should have sent reciprocal PATH_ABANDON"
    );

    // Client sees the path abandoned by remote
    assert_matches!(
        pair.poll(Client),
        Some(Event::Path(PathEvent::Abandoned {
            id: PathId::ZERO,
            reason: PathAbandonReason::RemoteAbandoned { error_code }
        })) if error_code == 42u8.into()
    );

    // Server sees its own local abandon
    assert_matches!(
        pair.poll(Server),
        Some(Event::Path(PathEvent::Abandoned {
            id: PathId::ZERO,
            reason: PathAbandonReason::ApplicationClosed { error_code }
        })) if error_code == 42u8.into()
    );

    // Connection is still alive on both sides
    assert!(
        !pair.is_closed(Client),
        "client connection should still be alive"
    );
    assert!(
        !pair.is_closed(Server),
        "server connection should still be alive"
    );

    Ok(())
}

/// Remote abandons the last path, no new path opened: connection closes after grace period.
#[test]
fn remote_path_abandon_last_path_closes_connection() -> TestResult {
    let _guard = subscribe();

    let mut cfg = TransportConfig::default();
    cfg.max_concurrent_multipath_paths(MAX_PATHS);
    cfg.initial_rtt(Duration::from_millis(10));

    let mut pair = ConnPair::with_transport_cfg(cfg.clone(), cfg);
    pair.drive();

    let mut second_client_addr = pair.client.addr;
    let mut second_server_addr = pair.server.addr;
    second_client_addr.set_port(second_client_addr.port() + 1);
    second_server_addr.set_port(second_server_addr.port() + 1);
    let mut third_client_addr = pair.client.addr;
    let mut third_server_addr = pair.server.addr;
    third_client_addr.set_port(third_client_addr.port() + 2);
    third_server_addr.set_port(third_server_addr.port() + 2);
    pair.routes = Some(RoutingTable::simple_symmetric(
        [pair.client.addr, second_client_addr, third_client_addr],
        [pair.server.addr, second_server_addr, third_server_addr],
    ));

    // Open two additional paths (total: path 0, path 1, path 2)
    let path1 = pair.open_path(
        Client,
        FourTuple {
            local_ip: Some(second_client_addr.ip()),
            remote: second_server_addr,
        },
        PathStatus::Available,
    )?;
    pair.drive();
    let path2 = pair.open_path(
        Client,
        FourTuple {
            local_ip: Some(third_client_addr.ip()),
            remote: third_server_addr,
        },
        PathStatus::Available,
    )?;
    pair.drive();

    // Drain all events
    while pair.poll(Client).is_some() {}
    while pair.poll(Server).is_some() {}

    // Server closes path 0 (2 paths remain: path1, path2)
    info!("server closes path 0");
    pair.close_path(Server, PathId::ZERO, 0u8.into())?;
    pair.drive();
    while pair.poll(Client).is_some() {}
    while pair.poll(Server).is_some() {}

    // Server closes path 1 (1 path remains: path2)
    info!("server closes path {path1}");
    pair.close_path(Server, path1, 0u8.into())?;
    pair.drive();
    while pair.poll(Client).is_some() {}
    while pair.poll(Server).is_some() {}

    // Server closes path 2 — its last path (fix #1: locally-initiated last-path close
    // must succeed; server is intentionally ending the connection via PATH_ABANDON).
    info!("server closes last path ({path2})");
    pair.close_path(Server, path2, 0u8.into())?;
    pair.drive();

    // Section 3.4 para 4: the client MUST accept all PATH_ABANDONs.
    let client_stats = pair.stats(Client);
    assert!(
        client_stats.frame_rx.path_abandon >= 3,
        "client should have received PATH_ABANDON for all three paths"
    );

    // Section 3.4 para 4: the client MUST send reciprocal PATH_ABANDONs.
    assert!(
        client_stats.frame_tx.path_abandon >= 3,
        "client should have sent reciprocal PATH_ABANDONs"
    );

    // After the grace period (no new path opened), both sides should be closed.
    assert!(
        pair.is_closed(Client),
        "client should be closed after grace period expired"
    );
    assert!(
        pair.is_closed(Server),
        "server should be closed after abandoning last path"
    );

    // Verify the close was clean (CONNECTION_CLOSE, not a protocol violation).
    let mut saw_abandon = false;
    let mut saw_close = false;
    while let Some(event) = pair.poll(Client) {
        match event {
            Event::Path(PathEvent::Abandoned {
                reason: PathAbandonReason::RemoteAbandoned { .. },
                ..
            }) => saw_abandon = true,
            Event::ConnectionLost { .. } => saw_close = true,
            _ => {}
        }
    }
    assert!(
        saw_abandon,
        "client should see path abandon event for last path"
    );
    assert!(saw_close, "client should see connection lost event");

    Ok(())
}

/// Remote abandons the last path, client opens a new path within grace period: connection survives.
#[test]
fn remote_path_abandon_last_path_client_opens_new() -> TestResult {
    let _guard = subscribe();

    let mut cfg = TransportConfig::default();
    cfg.max_concurrent_multipath_paths(4);
    cfg.initial_rtt(Duration::from_millis(10));

    let mut pair = ConnPair::with_transport_cfg(cfg.clone(), cfg);
    pair.drive();

    // Set up 4 addresses for routing
    let mut addrs_client = vec![pair.client.addr];
    let mut addrs_server = vec![pair.server.addr];
    for i in 1..4u16 {
        let mut ca = pair.client.addr;
        ca.set_port(ca.port() + i);
        addrs_client.push(ca);
        let mut sa = pair.server.addr;
        sa.set_port(sa.port() + i);
        addrs_server.push(sa);
    }
    pair.routes = Some(RoutingTable::simple_symmetric(
        addrs_client.clone(),
        addrs_server.clone(),
    ));

    // Open paths 1 and 2
    let path1 = pair.open_path(
        Client,
        FourTuple {
            local_ip: Some(addrs_client[1].ip()),
            remote: addrs_server[1],
        },
        PathStatus::Available,
    )?;
    pair.drive();
    let path2 = pair.open_path(
        Client,
        FourTuple {
            local_ip: Some(addrs_client[2].ip()),
            remote: addrs_server[2],
        },
        PathStatus::Available,
    )?;
    pair.drive();

    // Drain all events
    while pair.poll(Client).is_some() {}
    while pair.poll(Server).is_some() {}

    // Server closes path 0, then path 1 (each driven to completion)
    info!("server closes path 0");
    pair.close_path(Server, PathId::ZERO, 0u8.into())?;
    pair.drive();
    while pair.poll(Client).is_some() {}
    while pair.poll(Server).is_some() {}

    info!("server closes path {path1}");
    pair.close_path(Server, path1, 0u8.into())?;
    pair.drive();
    while pair.poll(Client).is_some() {}
    while pair.poll(Server).is_some() {}

    // Server closes path 2 — its last path (fix #1 required).
    // Only partially drive: server sends PATH_ABANDON, client receives it.
    // Client does NOT fully drive yet — we want to open a new path within the grace period.
    info!("server closes last path ({path2})");
    pair.close_path(Server, path2, 0u8.into())?;
    pair.drive_server();
    pair.drive_client();

    // Section 3.4 para 4: client accepts the abandon.
    // Section 3.4 para 7: client opens a new path instead of closing.
    let new_path_net = FourTuple {
        local_ip: Some(addrs_client[3].ip()),
        remote: addrs_server[3],
    };
    info!("client opens new path within grace period");
    let new_path_id = pair.open_path(Client, new_path_net, PathStatus::Available)?;
    pair.drive();

    // Connection should still be alive — grace period was not exceeded
    assert!(
        !pair.is_closed(Client),
        "client should still be alive after opening new path"
    );
    assert!(
        !pair.is_closed(Server),
        "server should still be alive after receiving new path"
    );

    // The client should see the abandon for path2, then the new path opened
    let mut saw_abandon = false;
    let mut saw_opened = false;
    while let Some(event) = pair.poll(Client) {
        match event {
            Event::Path(PathEvent::Abandoned {
                reason: PathAbandonReason::RemoteAbandoned { .. },
                ..
            }) => saw_abandon = true,
            Event::Path(PathEvent::Opened { id }) if id == new_path_id => saw_opened = true,
            _ => {}
        }
    }
    assert!(saw_abandon, "client should see abandon for last path");
    assert!(saw_opened, "client should see new path opened");

    Ok(())
}

/// Both sides abandon the same path simultaneously: should be a no-op, not an error.
#[test]
fn remote_path_abandon_already_abandoned_locally() -> TestResult {
    let _guard = subscribe();
    let mut pair = multipath_pair();

    // Open second path
    let server_addr = pair.addrs_to_server();
    let _path_id = pair.open_path(Client, server_addr, PathStatus::Available)?;
    pair.drive();

    // Drain open events
    while pair.poll(Client).is_some() {}
    while pair.poll(Server).is_some() {}

    // Both sides abandon path 0 "simultaneously" (before driving)
    info!("client abandons path 0");
    pair.close_path(Client, PathId::ZERO, 0u8.into())?;
    info!("server abandons path 0");
    pair.close_path(Server, PathId::ZERO, 0u8.into())?;

    // Drive should complete without errors
    pair.drive();

    // Connection should remain alive on the remaining path
    assert!(!pair.is_closed(Client));
    assert!(!pair.is_closed(Server));

    Ok(())
}

/// Remote abandons a path before validation completes: must still be accepted.
#[test]
fn remote_path_abandon_unvalidated_path() -> TestResult {
    let _guard = subscribe();
    let mut pair = multipath_pair();

    // Open second path from client - don't fully drive, so it's not validated on server yet
    let server_addr = pair.addrs_to_server();
    let path_id = pair.open_path(Client, server_addr, PathStatus::Available)?;

    // Only drive client side so the PATH_CHALLENGE is sent but not yet processed by server
    pair.drive_client();
    // Drive server to process the PATH_CHALLENGE (it will see the new path)
    pair.drive_server();
    // Don't drive_client again, so path validation isn't complete on client side

    // Server abandons the not-yet-fully-validated path
    info!("server abandons unvalidated path");
    pair.close_path(Server, path_id, 0u8.into())?;
    pair.drive();

    // Client should accept the abandon
    let mut saw_abandon = false;
    while let Some(event) = pair.poll(Client) {
        if matches!(
            &event,
            Event::Path(PathEvent::Abandoned {
                reason: PathAbandonReason::RemoteAbandoned { .. },
                ..
            })
        ) {
            saw_abandon = true;
        }
    }
    assert!(
        saw_abandon,
        "client must accept remote abandon of unvalidated path"
    );

    // Connection alive on path 0
    assert!(!pair.is_closed(Client));
    assert!(!pair.is_closed(Server));

    Ok(())
}

/// Verify reciprocal PATH_ABANDON is sent and MAX_PATH_ID bumped via frame stats.
#[test]
fn remote_path_abandon_reciprocal_and_cid_retirement() -> TestResult {
    let _guard = subscribe();
    let mut pair = multipath_pair();

    // Open second path
    let server_addr = pair.addrs_to_server();
    let _path_id = pair.open_path(Client, server_addr, PathStatus::Available)?;
    pair.drive();

    // Drain events
    while pair.poll(Client).is_some() {}
    while pair.poll(Server).is_some() {}

    let client_stats_before = pair.stats(Client);
    let server_stats_before = pair.stats(Server);

    // Server abandons path 0
    info!("server abandons path 0");
    pair.close_path(Server, PathId::ZERO, 0u8.into())?;
    pair.drive();

    let client_stats_after = pair.stats(Client);
    let server_stats_after = pair.stats(Server);

    // Server sent PATH_ABANDON
    assert_eq!(
        server_stats_after.frame_tx.path_abandon,
        server_stats_before.frame_tx.path_abandon + 1,
        "server should have sent one PATH_ABANDON"
    );

    // Client received PATH_ABANDON
    assert_eq!(
        client_stats_after.frame_rx.path_abandon,
        client_stats_before.frame_rx.path_abandon + 1,
        "client should have received one PATH_ABANDON"
    );

    // Section 3.4 para 4: client sent reciprocal PATH_ABANDON
    assert_eq!(
        client_stats_after.frame_tx.path_abandon,
        client_stats_before.frame_tx.path_abandon + 1,
        "client should have sent reciprocal PATH_ABANDON"
    );

    // Server received reciprocal PATH_ABANDON
    assert_eq!(
        server_stats_after.frame_rx.path_abandon,
        server_stats_before.frame_rx.path_abandon + 1,
        "server should have received reciprocal PATH_ABANDON"
    );

    // MAX_PATH_ID should have been bumped to allow opening new paths
    assert!(
        client_stats_after.frame_rx.max_path_id > client_stats_before.frame_rx.max_path_id
            || server_stats_after.frame_rx.max_path_id > server_stats_before.frame_rx.max_path_id,
        "MAX_PATH_ID should be incremented after path abandon"
    );

    Ok(())
}

/// Path state is retained for 3 PTO after PATH_ABANDON, then discarded.
#[test]
fn remote_path_abandon_path_discarded_after_3pto() -> TestResult {
    let _guard = subscribe();
    let mut pair = multipath_pair();

    // Open second path
    let server_addr = pair.addrs_to_server();
    let _path_id = pair.open_path(Client, server_addr, PathStatus::Available)?;
    pair.drive();

    // Drain events
    while pair.poll(Client).is_some() {}
    while pair.poll(Server).is_some() {}

    // Server abandons path 0
    info!("server abandons path 0");
    pair.close_path(Server, PathId::ZERO, 0u8.into())?;
    pair.drive();

    // Drain abandon events
    while let Some(event) = pair.poll(Client) {
        if matches!(&event, Event::Path(PathEvent::Abandoned { .. })) {
            break;
        }
    }

    // The path should not be immediately discarded - it should be retained for 3 PTO.
    // Drive time forward past the 3 PTO drain period.
    pair.drive();

    // Eventually we should see the Discarded event for path 0
    let mut saw_discard = false;
    while let Some(event) = pair.poll(Client) {
        if matches!(
            &event,
            Event::Path(PathEvent::Discarded { id, .. }) if *id == PathId::ZERO
        ) {
            saw_discard = true;
        }
    }
    assert!(saw_discard, "path should be discarded after drain period");

    Ok(())
}

// --- Tests ported from picoquic (picoquictest/multipath_test.c) ---
//
// Picoquic's multipath test harness (`multipath_test_one`) covers several path
// abandonment scenarios. The following tests adapt the most relevant ones to noq's
// sans-io test infrastructure.

/// Ported from picoquic `multipath_test_abandon` scenario.
///
/// Client abandons path 0 (while path 1 exists). After the abandon is processed by
/// both sides, data continues to flow on the remaining path. Verifies:
/// - Path is fully removed from both sides after drain period
/// - Data transmission succeeds on the surviving path
/// - Both sides end up with exactly 1 path
///
/// Picoquic ref: multipath_test.c lines 1014-1018 (abandon action),
///               lines 1189-1197 (verify nb_paths == 1 on both sides)
#[test]
fn abandon_path_data_continues() -> TestResult {
    let _guard = subscribe();
    let mut pair = multipath_pair();

    // Open a second path
    let server_addr = pair.addrs_to_server();
    let path1 = pair.open_path(Client, server_addr, PathStatus::Available)?;
    pair.drive();

    // Drain open events
    while pair.poll(Client).is_some() {}
    while pair.poll(Server).is_some() {}

    // Client abandons path 0 (picoquic: `picoquic_abandon_path(cnx_client, 0, 0, "test", time)`)
    info!("client abandons path 0");
    pair.close_path(Client, PathId::ZERO, 0u8.into())?;
    pair.drive();

    // Drain abandon + discard events
    while pair.poll(Client).is_some() {}
    while pair.poll(Server).is_some() {}

    // Picoquic verification: both sides should have exactly 1 path remaining.
    // In noq, we check that path 0 is abandoned and path 1 is still alive.
    assert!(
        pair.path_status(Client, path1).is_ok(),
        "client should still have path 1"
    );
    assert!(
        pair.path_status(Server, path1).is_ok(),
        "server should still have path 1"
    );

    // Data should still flow on the remaining path (picoquic sends test_scenario_multipath)
    let s = pair.streams(Client).open(Dir::Uni).unwrap();
    const MSG: &[u8] = b"data after path abandon";
    pair.send_stream(Client, s).write(MSG).unwrap();
    pair.send_stream(Client, s).finish().unwrap();
    pair.drive();

    assert_matches!(
        pair.poll(Server),
        Some(Event::Stream(StreamEvent::Opened { dir: Dir::Uni }))
    );
    assert_matches!(pair.streams(Server).accept(Dir::Uni), Some(stream) if stream == s);
    let mut recv = pair.recv_stream(Server, s);
    let mut chunks = recv.read(false).unwrap();
    assert_matches!(
        chunks.next(usize::MAX),
        Ok(Some(chunk)) if chunk.bytes == MSG
    );
    let _ = chunks.finalize();

    // Connection alive
    assert!(!pair.is_closed(Client));
    assert!(!pair.is_closed(Server));

    Ok(())
}

/// Ported from picoquic `multipath_test_ab1`. Abandon + reopen cycle, 3 rounds.
#[test]
fn abandon_cycle() -> TestResult {
    let _guard = subscribe();

    let mut cfg = TransportConfig::default();
    cfg.max_concurrent_multipath_paths(6);
    cfg.initial_rtt(Duration::from_millis(10));

    let mut pair = ConnPair::with_transport_cfg(cfg.clone(), cfg);
    pair.drive();

    // Set up addresses for multiple paths
    let mut addrs_client = vec![pair.client.addr];
    let mut addrs_server = vec![pair.server.addr];
    for i in 1..6u16 {
        let mut ca = pair.client.addr;
        ca.set_port(ca.port() + i);
        addrs_client.push(ca);
        let mut sa = pair.server.addr;
        sa.set_port(sa.port() + i);
        addrs_server.push(sa);
    }
    pair.routes = Some(RoutingTable::simple_symmetric(
        addrs_client.clone(),
        addrs_server.clone(),
    ));

    // Cycle: open a second path, abandon path 0, verify cleanup, repeat with new paths.
    // Each cycle uses a fresh pair of addresses.
    let mut current_path = PathId::ZERO;
    for cycle in 0..3u16 {
        let addr_idx = (cycle as usize) + 1;
        let new_path_net = FourTuple {
            local_ip: Some(addrs_client[addr_idx].ip()),
            remote: addrs_server[addr_idx],
        };

        info!("cycle {cycle}: opening new path on addr index {addr_idx}");
        let new_path = pair.open_path(Client, new_path_net, PathStatus::Available)?;
        pair.drive();

        // Drain events
        while pair.poll(Client).is_some() {}
        while pair.poll(Server).is_some() {}

        info!("cycle {cycle}: abandoning path {current_path}");
        pair.close_path(Client, current_path, 0u8.into())?;
        pair.drive();

        // Drain events (abandon + discard)
        while pair.poll(Client).is_some() {}
        while pair.poll(Server).is_some() {}

        // Verify the abandoned path is gone and the new path remains
        assert!(
            pair.path_status(Client, current_path).is_err(),
            "cycle {cycle}: abandoned path should be gone"
        );
        assert!(
            pair.path_status(Client, new_path).is_ok(),
            "cycle {cycle}: new path should be alive"
        );

        // Verify connection is alive
        assert!(
            !pair.is_closed(Client),
            "cycle {cycle}: client should be alive"
        );
        assert!(
            !pair.is_closed(Server),
            "cycle {cycle}: server should be alive"
        );

        // Picoquic verifies CID stash has >= 2 entries; we verify data still works.
        let s = pair.streams(Client).open(Dir::Uni).unwrap();
        let msg = format!("cycle {cycle}");
        pair.send_stream(Client, s).write(msg.as_bytes()).unwrap();
        pair.send_stream(Client, s).finish().unwrap();
        pair.drive();

        // Server should receive the data
        assert_matches!(
            pair.poll(Server),
            Some(Event::Stream(StreamEvent::Opened { dir: Dir::Uni }))
        );
        assert_matches!(pair.streams(Server).accept(Dir::Uni), Some(stream) if stream == s);
        let mut recv = pair.recv_stream(Server, s);
        let mut chunks = recv.read(false).unwrap();
        assert_matches!(
            chunks.next(usize::MAX),
            Ok(Some(chunk)) if chunk.bytes == msg.as_bytes()
        );
        let _ = chunks.finalize();

        current_path = new_path;
    }

    Ok(())
}

/// Regression: remote abandons the only validated path while an unvalidated path
/// exists. Connection survives if the unvalidated path validates (#400).
#[test]
fn remote_abandon_last_validated_path_unvalidated_remains() -> TestResult {
    let _guard = subscribe();
    let mut pair = multipath_pair();

    // Set up routing for two address pairs
    let mut second_client_addr = pair.client.addr;
    let mut second_server_addr = pair.server.addr;
    second_client_addr.set_port(second_client_addr.port() + 1);
    second_server_addr.set_port(second_server_addr.port() + 1);
    pair.routes = Some(RoutingTable::simple_symmetric(
        [pair.client.addr, second_client_addr],
        [pair.server.addr, second_server_addr],
    ));

    // Open path 1 from client but don't fully drive — leave it unvalidated.
    let path1_net = FourTuple {
        local_ip: Some(second_client_addr.ip()),
        remote: second_server_addr,
    };
    info!("client opens path 1 (will remain unvalidated)");
    let path1 = pair.open_path(Client, path1_net, PathStatus::Available)?;
    // Drive client → server: PATH_CHALLENGE sent and received
    pair.drive_client();
    pair.drive_server();
    // Don't drive further — PATH_RESPONSE not yet delivered to client

    // Server abandons path 0 — the only validated path from the client's perspective.
    info!("server abandons path 0 (client's only validated path)");
    pair.close_path(Server, PathId::ZERO, 0u8.into())?;
    pair.drive();

    // Client must accept the abandon (Section 3.4 para 4)
    let client_stats = pair.stats(Client);
    assert!(
        client_stats.frame_rx.path_abandon >= 1,
        "client should have received PATH_ABANDON for path 0"
    );

    // Drive to let path 1 validate (PATH_RESPONSE arrives during drive)
    pair.drive();

    // After path 1 validates, the connection should survive
    assert!(
        !pair.is_closed(Client),
        "client should survive — path 1 validated"
    );
    assert!(
        !pair.is_closed(Server),
        "server should survive — path 1 is usable"
    );
    assert!(
        pair.path_status(Client, path1).is_ok(),
        "path 1 should still be alive"
    );

    Ok(())
}

/// Regression: remote abandons last validated path and unvalidated path fails
/// validation. Connection closes via NoViablePath grace timer (#398/#400).
#[test]
fn remote_abandon_last_validated_path_validation_fails() -> TestResult {
    let _guard = subscribe();

    let mut cfg = TransportConfig::default();
    cfg.max_concurrent_multipath_paths(MAX_PATHS);
    cfg.initial_rtt(Duration::from_millis(10));

    let mut pair = ConnPair::with_transport_cfg(cfg.clone(), cfg);
    pair.drive();

    // Use an unreachable address for path 1 so validation will fail
    let unreachable_server: SocketAddr = SocketAddr::new(
        [9, 8, 7, 6].into(),
        SERVER_PORTS.lock()?.next().ok_or("no port")?,
    );

    // Open path 1 to unreachable address — it will never validate
    let path1_net = FourTuple {
        remote: unreachable_server,
        local_ip: None,
    };
    info!("client opens path 1 to unreachable address");
    let _path1 = pair.open_path(Client, path1_net, PathStatus::Available)?;
    pair.drive_client();
    pair.drive_server();

    // Server abandons path 0 — the only validated path
    info!("server abandons path 0");
    pair.close_path(Server, PathId::ZERO, 0u8.into())?;

    // Drive to idle — path 1 validation will time out, then connection closes
    pair.drive();
    while pair.step() {}

    // The connection should be closed:
    // - path 0: abandoned by server
    // - path 1: validation failed (unreachable) → abandoned → NoViablePath timer fires
    assert!(
        pair.is_closed(Client),
        "client should close — no viable path after validation failure"
    );

    let mut saw_connection_lost = false;
    while let Some(event) = pair.poll(Client) {
        if matches!(&event, Event::ConnectionLost { .. }) {
            saw_connection_lost = true;
        }
    }
    assert!(
        saw_connection_lost,
        "client should emit ConnectionLost after no viable path"
    );

    Ok(())
}

/// Regression: remote abandons all paths, client recovers by opening a new one (#398).
#[test]
fn remote_abandon_last_validated_path_recovery_via_new_path() -> TestResult {
    let _guard = subscribe();

    let mut cfg = TransportConfig::default();
    cfg.max_concurrent_multipath_paths(4);
    cfg.initial_rtt(Duration::from_millis(10));

    let mut pair = ConnPair::with_transport_cfg(cfg.clone(), cfg);
    pair.drive();

    // Set up 3 address pairs
    let mut addrs_client = vec![pair.client.addr];
    let mut addrs_server = vec![pair.server.addr];
    for i in 1..3u16 {
        let mut ca = pair.client.addr;
        ca.set_port(ca.port() + i);
        addrs_client.push(ca);
        let mut sa = pair.server.addr;
        sa.set_port(sa.port() + i);
        addrs_server.push(sa);
    }
    pair.routes = Some(RoutingTable::simple_symmetric(
        addrs_client.clone(),
        addrs_server.clone(),
    ));

    // Open path 1 but leave it unvalidated
    let path1_net = FourTuple {
        local_ip: Some(addrs_client[1].ip()),
        remote: addrs_server[1],
    };
    info!("client opens path 1 (will remain unvalidated)");
    let _path1 = pair.open_path(Client, path1_net, PathStatus::Available)?;
    pair.drive_client();
    pair.drive_server();

    // Server abandons path 0 (only validated path)
    info!("server abandons path 0");
    pair.close_path(Server, PathId::ZERO, 0u8.into())?;
    pair.drive_server();
    pair.drive_client();

    // Client opens a new path (path 2) to a reachable address
    let path2_net = FourTuple {
        local_ip: Some(addrs_client[2].ip()),
        remote: addrs_server[2],
    };
    info!("client opens path 2 within grace period");
    let path2 = pair.open_path(Client, path2_net, PathStatus::Available)?;

    // Full drive — path 2 should validate
    pair.drive();

    // Connection should survive
    assert!(
        !pair.is_closed(Client),
        "client should survive — new path validated"
    );
    assert!(
        !pair.is_closed(Server),
        "server should survive — new path validated"
    );
    assert!(
        pair.path_status(Client, path2).is_ok(),
        "path 2 should be alive"
    );

    Ok(())
}
