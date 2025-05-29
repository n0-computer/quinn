use std::sync::Arc;

use iroh_quinn::{
    virtualnet::{switch::Switch, TestAddr},
    Endpoint, Runtime as _,
};
use proto::{ClientConfig, EndpointConfig, ServerConfig};

#[tokio::test]
async fn test_connect_with_virtual_socket() {
    tracing_subscriber::fmt::init();
    let runtime = Arc::new(iroh_quinn::TokioRuntime);

    // Virtual sockets setup
    let switch = Switch::new();
    let server_socket = switch.connect_socket(TestAddr(42)).await;
    let client_socket = switch.connect_socket(TestAddr(111)).await;
    let server_addr = server_socket.addr;

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
        Box::new(server_socket),
        runtime.clone(),
    )
    .unwrap();
    let mut client_ep = Endpoint::new_with_abstract_socket(
        EndpointConfig::default(),
        None,
        Box::new(client_socket),
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

    // server_socket.set_paused(true);
    // let mut wiretap = server_socket.wiretap();

    // let (conn, to_resend) = tokio::join!(
    //     async {
    // Connect from the client
    let conn = client_ep
        .connect(server_addr, "localhost")
        .unwrap()
        .await
        .unwrap();
    //         conn
    //     },
    //     async {
    //         // Grab the last
    //         let mut to_resend = wiretap.recv().await.unwrap();
    //         to_resend.src_ip = SocketAddr::new([192, 168, 0, 133].into(), 1234);
    //         server_socket
    //             .try_send(&to_resend.as_quinn_transmit())
    //             .expect("couldn't resend packet");
    //         server_socket.set_paused(false);
    //         to_resend
    //     }
    // );

    conn.closed().await;

    client_ep.close(1u32.into(), b"endpoint closed");
    server_ep.close(1u32.into(), b"endpoint closed");

    // client_socket
    //     .try_send(&to_resend.as_quinn_transmit())
    //     .expect("couldn't resend packet");

    client_ep.wait_idle().await;
    server_ep.wait_idle().await;
}
