// patchbay only runs on linux
#![cfg(target_os = "linux")]
// Only compile these tests when the patchbay_tests cfg is enabled.
// Run with: RUSTFLAGS="--cfg patchbay_tests" cargo test -p noq --features rustls-ring --test netsim -- --test-threads=1
#![cfg(patchbay_tests)]
//! Network simulation tests using patchbay.
//!
//! These tests exercise holepunching and path migration through realistic
//! network topologies backed by Linux network namespaces.

use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use patchbay::{Lab, LabOpts, OutDir, RouterPreset};
use proto::{PathEvent, n0_nat_traversal};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use testdir::testdir;
use tokio::sync::oneshot;

#[ctor::ctor]
unsafe fn init() {
    unsafe { patchbay::init_userns_for_ctor() };
}

// ── Helpers ────────────────────────────────────────────────────────────

fn subscribe() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "noq=debug,info".parse().unwrap()),
        )
        .with_test_writer()
        .try_init()
        .ok();
}

/// Create a self-signed cert + matching client/server configs with NAT traversal enabled.
struct CryptoConfig {
    server_config: noq::ServerConfig,
    client_config: noq::ClientConfig,
}

impl CryptoConfig {
    fn new() -> Self {
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let key = PrivateKeyDer::Pkcs8(cert.signing_key.serialize_der().into());
        let cert_der: CertificateDer<'static> = cert.cert.into();

        let mut transport = noq::TransportConfig::default();
        // Enable NAT traversal (which also enables multipath)
        transport.set_max_remote_nat_traversal_addresses(8);

        let transport = Arc::new(transport);

        let mut server_config =
            noq::ServerConfig::with_single_cert(vec![cert_der.clone()], key).unwrap();
        server_config.transport_config(transport.clone());

        let mut roots = rustls::RootCertStore::empty();
        roots.add(cert_der).unwrap();
        let mut client_config = noq::ClientConfig::with_root_certificates(Arc::new(roots)).unwrap();
        client_config.transport_config(transport);

        Self {
            server_config,
            client_config,
        }
    }
}

/// Send a ping over a bidirectional stream and read back the echo.
async fn ping(conn: &noq::Connection) -> Result<()> {
    let (mut send, mut recv) = conn.open_bi().await?;
    send.write_all(b"ping").await?;
    send.finish()?;
    let data = recv.read_to_end(64).await?;
    anyhow::ensure!(data == b"ping", "unexpected echo payload");
    Ok(())
}

/// Accept bidi streams and echo them back. Run as a background task.
async fn echo_server(conn: noq::Connection) -> Result<()> {
    loop {
        let (mut send, mut recv) = match conn.accept_bi().await {
            Ok(pair) => pair,
            Err(
                noq::ConnectionError::ApplicationClosed(_) | noq::ConnectionError::LocallyClosed,
            ) => {
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        };
        let data = recv.read_to_end(1024).await?;
        send.write_all(&data).await?;
        send.finish()?;
    }
}

/// Wait for a PathEvent::Opened on the connection's path_events channel.
async fn wait_path_opened(conn: &noq::Connection, timeout: Duration) -> Result<noq::PathId> {
    let mut path_events = conn.path_events();
    tokio::time::timeout(timeout, async {
        loop {
            match path_events.recv().await {
                Ok(PathEvent::Opened { id }) => return Ok(id),
                Ok(evt) => {
                    tracing::debug!(?evt, "path event (waiting for Opened)");
                    continue;
                }
                Err(e) => anyhow::bail!("path events channel error: {e}"),
            }
        }
    })
    .await
    .context("timeout waiting for path to open")?
}

/// Wait for a PathEvent::Opened whose remote address satisfies `predicate`.
async fn wait_path_opened_matching(
    conn: &noq::Connection,
    timeout: Duration,
    predicate: fn(SocketAddr) -> bool,
) -> Result<noq::PathId> {
    let mut path_events = conn.path_events();
    tokio::time::timeout(timeout, async {
        loop {
            match path_events.recv().await {
                Ok(PathEvent::Opened { id }) => {
                    if let Some(path) = conn.path(id) {
                        if let Ok(addr) = path.remote_address() {
                            if predicate(addr) {
                                tracing::info!(?addr, ?id, "matching path opened");
                                return Ok(id);
                            }
                        }
                    }
                    tracing::debug!(?id, "path opened but does not match predicate");
                }
                Ok(_) => continue,
                Err(e) => anyhow::bail!("path events channel error: {e}"),
            }
        }
    })
    .await
    .context("timeout waiting for matching path")?
}

// ── Tests ──────────────────────────────────────────────────────────────

/// Two peers on separate public networks. The server has two interfaces
/// on different subnets. The initial QUIC connection is established via one
/// subnet, then the NAT traversal protocol opens a second path via the other.
///
/// This tests the core NAT traversal protocol (ADD_ADDRESS, REACH_OUT,
/// multipath PATH_CHALLENGE/RESPONSE) through a realistic patchbay topology.
#[tokio::test]
async fn holepunch() -> Result<()> {
    subscribe();

    let mut opts = LabOpts::default().outdir(OutDir::Exact(testdir!()));
    if let Some(name) = std::thread::current().name() {
        opts = opts.label(name);
    }
    let lab = Lab::with_opts(opts).await?;
    let guard = lab.test_guard();

    let public1 = lab
        .add_router("public1")
        .preset(RouterPreset::Public)
        .build()
        .await?;
    let public2 = lab
        .add_router("public2")
        .preset(RouterPreset::Public)
        .build()
        .await?;

    // Server has two interfaces on different public networks
    let server_dev = lab
        .add_device("server")
        .iface("eth0", public1.id(), None)
        .iface("eth1", public2.id(), None)
        .build()
        .await?;

    // Client on public1 — shares a subnet with server's eth0
    let client_dev = lab
        .add_device("client")
        .iface("eth0", public1.id(), None)
        .build()
        .await?;

    let server_ip_eth0 = server_dev
        .iface("eth0")
        .context("eth0")?
        .ip()
        .context("no ip on eth0")?;
    let server_ip_eth1 = server_dev
        .iface("eth1")
        .context("eth1")?
        .ip()
        .context("no ip on eth1")?;

    let crypto = CryptoConfig::new();
    let (ready_tx, ready_rx) = oneshot::channel();

    // ── Server ──
    let server_handle = {
        let sc = crypto.server_config.clone();
        let cc = crypto.client_config.clone();
        server_dev.spawn(async move |_dev| -> Result<()> {
            let socket =
                std::net::UdpSocket::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 4433)))?;
            let ep = noq::Endpoint::new(
                noq::EndpointConfig::default(),
                Some(sc),
                socket,
                Arc::new(noq::TokioRuntime),
            )?;
            ep.set_default_client_config(cc);
            ready_tx.send(()).ok();

            let conn = ep.accept().await.context("accept")?.await?;

            // Advertise the SECOND interface's IP for NAT traversal.
            // This creates a genuinely new path (different remote IP for the client).
            conn.add_nat_traversal_address(SocketAddr::from((server_ip_eth1, 4433)))?;

            echo_server(conn).await
        })?
    };

    ready_rx.await?;

    // ── Client ──
    let client_handle = {
        let sc = crypto.server_config.clone();
        let cc = crypto.client_config.clone();
        client_dev.spawn(async move |dev| -> Result<()> {
            let socket = std::net::UdpSocket::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)))?;
            let ep = noq::Endpoint::new(
                noq::EndpointConfig::default(),
                Some(sc),
                socket,
                Arc::new(noq::TokioRuntime),
            )?;
            ep.set_default_client_config(cc);

            // Initial connection via server's eth0 (same subnet)
            let conn = ep
                .connect(SocketAddr::from((server_ip_eth0, 4433)), "localhost")?
                .await?;

            // Subscribe early so we don't miss ADD_ADDRESS frames
            let mut nat_updates = conn.nat_traversal_updates();

            ping(&conn).await.context("initial ping")?;
            tracing::info!("initial ping OK");

            // Wait for server to advertise its second address
            let addr = tokio::time::timeout(Duration::from_secs(10), async {
                loop {
                    match nat_updates.recv().await {
                        Ok(n0_nat_traversal::Event::AddressAdded(addr)) => return Ok(addr),
                        Ok(_) => continue,
                        Err(e) => anyhow::bail!("channel error: {e}"),
                    }
                }
            })
            .await
            .context("timeout waiting for server address")??;
            tracing::info!(?addr, "received server NAT traversal address");

            // Advertise our address
            let local_ip = dev.ip().context("client ip")?;
            let local_port = ep.local_addr()?.port();
            conn.add_nat_traversal_address(SocketAddr::from((local_ip, local_port)))?;

            // Initiate NAT traversal — should open a path to the server's second IP
            let probed = conn.initiate_nat_traversal_round()?;
            tracing::info!(?probed, "initiated NAT traversal round");

            // Wait for the new path to open
            let path_id = wait_path_opened(&conn, Duration::from_secs(15)).await?;
            tracing::info!(?path_id, "second path opened via NAT traversal");

            ping(&conn).await.context("ping on second path")?;
            tracing::info!("ping on second path OK");

            conn.close(0u32.into(), b"done");
            Ok(())
        })?
    };

    client_handle.await?.context("client task")?;
    let _ = server_handle.await;
    guard.ok();
    Ok(())
}

/// After establishing a direct connection via NAT traversal, one peer switches
/// network (replug to a different router). The peer then re-establishes
/// connectivity by initiating a new NAT traversal round with updated addresses.
///
/// Variations test different IP version transitions.
async fn switch_uplink_inner(
    path: std::path::PathBuf,
    initial_preset: RouterPreset,
    target_preset: RouterPreset,
    _check_initial: fn(SocketAddr) -> bool,
    check_target: fn(SocketAddr) -> bool,
    label: &str,
) -> Result<()> {
    subscribe();

    let mut opts = LabOpts::default().outdir(OutDir::Exact(path));
    if let Some(name) = std::thread::current().name() {
        opts = opts.label(name);
    }
    let lab = Lab::with_opts(opts).await?;
    let guard = lab.test_guard();

    // Public router for the server (dualstack)
    let public = lab
        .add_router("public")
        .preset(RouterPreset::Public)
        .build()
        .await?;

    // Initial router for the mobile peer
    let router_initial = lab
        .add_router("router-initial")
        .preset(initial_preset)
        .build()
        .await?;

    // Target router the mobile peer will switch to
    let router_target = lab
        .add_router("router-target")
        .preset(target_preset)
        .build()
        .await?;

    // Server device on public network
    let server_dev = lab
        .add_device("server")
        .iface("eth0", public.id(), None)
        .build()
        .await?;

    // Mobile device initially on router_initial
    let mobile_dev = lab
        .add_device("mobile")
        .iface("eth0", router_initial.id(), None)
        .build()
        .await?;

    // Pre-extract router uplink IPs for NAT traversal addresses.
    let initial_uplink_ip = router_initial.uplink_ip();
    let initial_uplink_ip_v6 = router_initial.uplink_ip_v6();
    let target_uplink_ip = router_target.uplink_ip();
    let target_uplink_ip_v6 = router_target.uplink_ip_v6();

    // Send both IPv4 and IPv6 server addresses so the client can pick the right one.
    let (server_addrs_tx, server_addrs_rx) =
        oneshot::channel::<(Option<SocketAddr>, Option<SocketAddr>)>();
    let (connected_tx, connected_rx) = oneshot::channel::<()>();

    let crypto = CryptoConfig::new();

    // Server side
    let server_handle = {
        let server_config = crypto.server_config.clone();
        let client_config = crypto.client_config.clone();
        server_dev.spawn(async move |dev| -> Result<()> {
            let socket =
                std::net::UdpSocket::bind(SocketAddr::from((Ipv6Addr::UNSPECIFIED, 4433)))?;
            let ep = noq::Endpoint::new(
                noq::EndpointConfig::default(),
                Some(server_config),
                socket,
                Arc::new(noq::TokioRuntime),
            )?;
            ep.set_default_client_config(client_config);

            let listen_port = ep.local_addr()?.port();
            let addr_v4 = dev.ip().map(|ip| SocketAddr::from((ip, listen_port)));
            let addr_v6 = dev.ip6().map(|ip| SocketAddr::from((ip, listen_port)));
            server_addrs_tx.send((addr_v4, addr_v6)).ok();

            let conn = ep.accept().await.context("accept")?.await?;

            // Advertise our addresses for NAT traversal (server is public, no NAT)
            if let Some(ip4) = dev.ip() {
                conn.add_nat_traversal_address(SocketAddr::from((ip4, listen_port)))?;
            }
            if let Some(ip6) = dev.ip6() {
                conn.add_nat_traversal_address(SocketAddr::from((ip6, listen_port)))?;
            }

            connected_tx.send(()).ok();

            echo_server(conn).await
        })?
    };

    let (server_addr_v4, server_addr_v6) = server_addrs_rx.await?;
    tracing::info!(
        ?server_addr_v4,
        ?server_addr_v6,
        "[{label}] server listening"
    );

    // Pick server address reachable from the initial network.
    let server_addr = match initial_preset {
        RouterPreset::IspV6 => server_addr_v6.context("server has no ipv6 for IspV6 client")?,
        _ => server_addr_v4
            .or(server_addr_v6)
            .context("server has no address")?,
    };

    // Client (mobile) side
    let mobile_handle = {
        let router_target_id = router_target.id();
        let server_config = crypto.server_config.clone();
        let client_config = crypto.client_config.clone();
        let label = label.to_string();
        mobile_dev.spawn(async move |dev| -> Result<()> {
            let socket = std::net::UdpSocket::bind(SocketAddr::from((Ipv6Addr::UNSPECIFIED, 0)))?;
            let ep = noq::Endpoint::new(
                noq::EndpointConfig::default(),
                Some(server_config),
                socket,
                Arc::new(noq::TokioRuntime),
            )?;
            ep.set_default_client_config(client_config);

            let conn = ep.connect(server_addr, "localhost")?.await?;
            tracing::info!("[{label}] connected to server");

            ping(&conn).await.context("initial ping")?;
            tracing::info!("[{label}] initial ping OK");

            // Wait for server to advertise addresses
            connected_rx.await?;
            tokio::time::sleep(Duration::from_secs(1)).await;

            // Register our NATted public addresses
            let port = ep.local_addr()?.port();
            if let Some(ip4) = initial_uplink_ip {
                conn.add_nat_traversal_address(SocketAddr::from((ip4, port)))?;
            }
            if let Some(ip6) = initial_uplink_ip_v6 {
                conn.add_nat_traversal_address(SocketAddr::from((ip6, port)))?;
            }

            let probed = conn.initiate_nat_traversal_round()?;
            tracing::info!(?probed, "[{label}] first NAT traversal round");

            let path_id = wait_path_opened(&conn, Duration::from_secs(15)).await?;
            tracing::info!(?path_id, "[{label}] direct path opened");

            ping(&conn).await.context("ping on direct path")?;
            tracing::info!("[{label}] direct ping OK");

            // ── Device switch ──
            tracing::info!("[{label}] switching uplink...");
            dev.replug_iface("eth0", router_target_id).await?;
            tokio::time::sleep(Duration::from_secs(1)).await;

            let new_socket =
                std::net::UdpSocket::bind(SocketAddr::from((Ipv6Addr::UNSPECIFIED, 0)))?;
            ep.rebind(new_socket)?;
            ep.handle_network_change(None);

            let new_port = ep.local_addr()?.port();
            if let Some(ip4) = target_uplink_ip {
                conn.add_nat_traversal_address(SocketAddr::from((ip4, new_port)))?;
            }
            if let Some(ip6) = target_uplink_ip_v6 {
                conn.add_nat_traversal_address(SocketAddr::from((ip6, new_port)))?;
            }

            let probed = conn.initiate_nat_traversal_round()?;
            tracing::info!(?probed, "[{label}] NAT traversal round after switch");

            let path_id =
                wait_path_opened_matching(&conn, Duration::from_secs(20), check_target).await?;
            tracing::info!(?path_id, "[{label}] target path established after switch");

            ping(&conn).await.context("ping after device switch")?;
            tracing::info!("[{label}] ping after switch OK");

            conn.close(0u32.into(), b"done");
            Ok(())
        })?
    };

    mobile_handle
        .await?
        .context(format!("[{label}] mobile task"))?;

    let _ = server_handle.await;

    guard.ok();
    Ok(())
}

fn is_ipv4(addr: SocketAddr) -> bool {
    match addr {
        SocketAddr::V4(_) => true,
        SocketAddr::V6(v6) => v6.ip().to_ipv4_mapped().is_some(),
    }
}

fn is_ipv6(addr: SocketAddr) -> bool {
    match addr {
        SocketAddr::V4(_) => false,
        SocketAddr::V6(v6) => v6.ip().to_ipv4_mapped().is_none(),
    }
}

fn is_any(_addr: SocketAddr) -> bool {
    true
}

/// Re-holepunch after switching from a dualstack Home router to an IPv6-only ISP.
#[tokio::test]
async fn switch_dualstack_to_ipv6() -> Result<()> {
    switch_uplink_inner(
        testdir!(),
        RouterPreset::Home,
        RouterPreset::IspV6,
        is_any,
        is_ipv6,
        "dualstack->ipv6",
    )
    .await
}

/// Re-holepunch after switching from an IPv6-only ISP to a dualstack Home router.
#[tokio::test]
async fn switch_ipv6_to_dualstack() -> Result<()> {
    switch_uplink_inner(
        testdir!(),
        RouterPreset::IspV6,
        RouterPreset::Home,
        is_ipv6,
        is_any,
        "ipv6->dualstack",
    )
    .await
}

/// Re-holepunch after switching from an IPv6-only ISP to an IPv4-capable Home router.
#[tokio::test]
async fn switch_ipv6_to_ipv4() -> Result<()> {
    switch_uplink_inner(
        testdir!(),
        RouterPreset::IspV6,
        RouterPreset::Home,
        is_ipv6,
        is_ipv4,
        "ipv6->ipv4",
    )
    .await
}
