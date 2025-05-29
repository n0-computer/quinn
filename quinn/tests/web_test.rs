use std::{
    collections::{BTreeMap, VecDeque},
    net::SocketAddr,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{ready, Context, Poll, Waker},
};

use bytes::Bytes;
use iroh_quinn::{AsyncUdpSocket, Endpoint, Runtime as _};
use proto::{ClientConfig, EndpointConfig, ServerConfig};
use tokio::sync::mpsc;
use tokio_util::{sync::PollSender, task::AbortOnDropHandle};
use udp::{EcnCodepoint, Transmit};

#[tokio::test]
async fn test_connect_with_virtual_socket() {
    tracing_subscriber::fmt::init();
    let runtime = Arc::new(iroh_quinn::TokioRuntime);

    // Virtual sockets setup
    let mut net = VirtualNet::default();
    let sock_a = net.add_socket(42);
    let sock_b = net.add_socket(111);
    let net = Arc::new(net);
    let server_socket = VirtualSocket::new(&net, sock_a);
    let client_socket = VirtualSocket::new(&net, sock_b);

    // Cert setup
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let key = rustls::pki_types::PrivateKeyDer::Pkcs8(cert.key_pair.serialize_der().into());
    let mut roots = rustls::RootCertStore::empty();
    roots.add(cert.cert.der().clone()).unwrap();

    let server_config = ServerConfig::with_single_cert(vec![cert.cert.der().clone()], key).unwrap();
    let client_config = ClientConfig::with_root_certificates(Arc::new(roots)).unwrap();

    let server_ep = Endpoint::new_with_abstract_socket(
        EndpointConfig::default(),
        Some(server_config),
        server_socket.clone(),
        runtime.clone(),
    )
    .unwrap();
    let mut client_ep = Endpoint::new_with_abstract_socket(
        EndpointConfig::default(),
        None,
        client_socket.clone(),
        runtime.clone(),
    )
    .unwrap();
    client_ep.set_default_client_config(client_config);

    // Run server task
    runtime.spawn(Box::pin({
        let server_ep = server_ep.clone();
        async move {
            // Simple echo loop
            while let Some(incoming) = server_ep.accept().await {
                let conn = incoming.accept().unwrap().await.unwrap();
                conn.close(0u32.into(), b"bye!");
            }
        }
    }));

    server_socket.set_paused(true);
    let mut wiretap = server_socket.wiretap();

    let (conn, to_resend) = tokio::join!(
        async {
            // Connect from the client
            let conn = client_ep
                .connect(server_socket.addr, "localhost")
                .unwrap()
                .await
                .unwrap();
            conn
        },
        async {
            // Grab the last
            let mut to_resend = wiretap.recv().await.unwrap();
            to_resend.src_ip = SocketAddr::new([192, 168, 0, 133].into(), 1234);
            server_socket
                .try_send(&to_resend.as_quinn_transmit())
                .expect("couldn't resend packet");
            server_socket.set_paused(false);
            to_resend
        }
    );

    conn.closed().await;

    client_ep.close(1u32.into(), b"endpoint closed");
    server_ep.close(1u32.into(), b"endpoint closed");

    client_socket
        .try_send(&to_resend.as_quinn_transmit())
        .expect("couldn't resend packet");

    client_ep.wait_idle().await;
    server_ep.wait_idle().await;
}
