use std::{
    collections::{BTreeMap, VecDeque},
    net::SocketAddr,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

use bytes::Bytes;
use proto::{ClientConfig, EndpointConfig, ServerConfig};
use quinn::{AsyncUdpSocket, Endpoint, Runtime as _};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use udp::{EcnCodepoint, Transmit};

#[derive(Debug)]
struct VirtualSocket {
    addr: SocketAddr,
    net: Arc<VirtualNet>,
}

#[derive(Debug, Default)]
struct VirtualNet {
    sockets: BTreeMap<SocketAddr, Mutex<VirtualSocketState>>,
}

#[derive(Debug, Default)]
struct VirtualSocketState {
    datagrams: VecDeque<OwnedTransmit>,
    wiretap: Option<UnboundedSender<OwnedTransmit>>,
    wakers: Vec<Waker>,
    paused: bool,
}

impl VirtualNet {
    fn add_socket(&mut self, id: u8) -> SocketAddr {
        let addr = SocketAddr::from(([192, 168, 0, id], 1234u16));
        self.sockets.insert(addr, Default::default());
        addr
    }
}

impl VirtualSocket {
    fn new(net: &Arc<VirtualNet>, addr: SocketAddr) -> Arc<Self> {
        Arc::new(Self {
            net: Arc::clone(net),
            addr,
        })
    }

    fn wiretap(&self) -> UnboundedReceiver<OwnedTransmit> {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        self.socket_state().wiretap = Some(sender);
        receiver
    }

    fn set_paused(&self, paused: bool) {
        let mut socket_state = self.socket_state();

        socket_state.paused = paused;
        if !paused {
            while let Some(waker) = socket_state.wakers.pop() {
                waker.wake();
            }
        }
    }

    fn socket_state(&self) -> std::sync::MutexGuard<'_, VirtualSocketState> {
        self.net
            .sockets
            .get(&self.addr)
            .expect("socket missing")
            .lock()
            .expect("poisoned")
    }
}

#[derive(Debug, Clone)]
pub struct OwnedTransmit {
    pub destination: SocketAddr,
    pub ecn: Option<EcnCodepoint>,
    pub contents: Bytes,
    pub segment_size: Option<usize>,
    pub src_ip: SocketAddr,
}

impl OwnedTransmit {
    fn new(src: SocketAddr, t: &udp::Transmit) -> Self {
        Self {
            destination: t.destination,
            ecn: t.ecn.clone(),
            contents: Bytes::copy_from_slice(t.contents),
            segment_size: t.segment_size.clone(),
            src_ip: SocketAddr::new(t.src_ip.unwrap_or(src.ip()), src.port()),
        }
    }

    fn as_quinn_transmit(&self) -> Transmit<'_> {
        Transmit {
            destination: self.destination,
            ecn: self.ecn,
            contents: self.contents.as_ref(),
            segment_size: self.segment_size.clone(),
            src_ip: Some(self.src_ip.ip()),
        }
    }
}

impl AsyncUdpSocket for VirtualSocket {
    fn create_io_poller(self: Arc<Self>) -> Pin<Box<dyn quinn::UdpPoller>> {
        #[derive(Debug)]
        struct UdpAlwaysReady;

        impl quinn::UdpPoller for UdpAlwaysReady {
            fn poll_writable(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<std::io::Result<()>> {
                Poll::Ready(Ok(()))
            }
        }

        Box::pin(UdpAlwaysReady)
    }

    fn try_send(&self, transmit: &udp::Transmit) -> std::io::Result<()> {
        // If there's no socket, then there's no point to ever send, since nobody could ever receive
        let Some(send) = self.net.sockets.get(&transmit.destination) else {
            return Ok(());
        };

        let mut socket_state = send.lock().expect("poisoned");
        let transmit = OwnedTransmit::new(self.addr, transmit);
        socket_state.datagrams.push_back(transmit.clone());
        if let Some(sender) = &socket_state.wiretap {
            if sender.send(transmit).is_err() {
                socket_state.wiretap = None;
            }
        }
        if !socket_state.paused {
            while let Some(waker) = socket_state.wakers.pop() {
                waker.wake();
            }
        }
        Ok(())
    }

    fn poll_recv(
        &self,
        cx: &mut Context,
        bufs: &mut [std::io::IoSliceMut<'_>],
        meta: &mut [udp::RecvMeta],
    ) -> Poll<std::io::Result<usize>> {
        let mut socket_state = self.socket_state();

        if socket_state.paused {
            socket_state.wakers.push(cx.waker().clone());
            return Poll::Pending;
        }

        let mut num_msgs = 0;
        while let Some(t) = socket_state.datagrams.pop_front() {
            if bufs.len() <= num_msgs || meta.len() <= num_msgs {
                break;
            }

            let buf = &mut bufs[num_msgs];
            let meta = &mut meta[num_msgs];

            if buf.len() >= t.contents.len() {
                tracing::debug!(
                    "Received from {:?} to {:?}: {} bytes",
                    t.src_ip,
                    t.destination,
                    t.contents.len()
                );
                buf[..t.contents.len()].copy_from_slice(&t.contents);
                meta.addr = t.src_ip;
                meta.dst_ip = Some(t.destination.ip());
                meta.len = t.contents.len();
                meta.stride = t.contents.len();
                meta.ecn = t.ecn;
                num_msgs += 1;
            }
        }

        if num_msgs > 0 {
            Poll::Ready(Ok(num_msgs))
        } else {
            socket_state.wakers.push(cx.waker().clone());
            Poll::Pending
        }
    }

    fn local_addr(&self) -> std::io::Result<std::net::SocketAddr> {
        Ok(self.addr)
    }
}

#[tokio::test]
async fn test_connect_with_virtual_socket() {
    tracing_subscriber::fmt::init();
    let runtime = Arc::new(quinn::TokioRuntime);

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
