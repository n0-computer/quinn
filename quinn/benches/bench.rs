use std::{
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
    task::{Poll, ready},
    thread,
};

use bytes::Bytes;
use quinn::AsyncUdpSocket;
use tokio::sync::mpsc;
use tokio_util::sync::PollSender;
use udp::{EcnCodepoint, Transmit};

use bencher::{Bencher, benchmark_group, benchmark_main};
use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer};
use tokio::runtime::{Builder, Runtime};
use tracing::error_span;
use tracing_futures::Instrument as _;

use iroh_quinn::{self as quinn, Endpoint, TokioRuntime};

pub struct TestAddr(pub u8);

impl From<TestAddr> for SocketAddr {
    fn from(TestAddr(id): TestAddr) -> Self {
        ([1, 1, 1, id], 42u16).into()
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

    fn receive_into(
        &self,
        buf: &mut std::io::IoSliceMut<'_>,
        meta: &mut udp::RecvMeta,
    ) -> std::io::Result<()> {
        buf[..self.contents.len()].copy_from_slice(&self.contents);
        meta.addr = self.src_ip;
        meta.dst_ip = Some(self.destination.ip());
        meta.len = self.contents.len();
        meta.stride = self.segment_size.unwrap_or(self.contents.len());
        meta.ecn = self.ecn;
        Ok(())
    }
}

#[derive(Debug)]
pub struct VirtualSocket {
    pub addr: SocketAddr,
    sender: mpsc::Sender<OwnedTransmit>,
    receiver: mpsc::Receiver<OwnedTransmit>,
    max_transmit_segments: usize,
    max_receive_segments: usize,
}

#[derive(Debug)]
pub struct VirtualSocketSender {
    addr: SocketAddr,
    sender: PollSender<OwnedTransmit>,
    max_transmit_segments: usize,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub max_transmit_segments: usize,
    pub max_receive_segments: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            max_transmit_segments: 32,
            max_receive_segments: 32,
        }
    }
}

impl VirtualSocket {
    pub fn pair(addr0: impl Into<SocketAddr>, addr1: impl Into<SocketAddr>, options: Options) -> (Self, Self) {
        let zero_to_one = mpsc::channel(150); // Default UDP buffer size in linux is ~213KB, which would hold about 150 datagrams
        let one_to_zero = mpsc::channel(150); // Default UDP buffer size in linux is ~213KB, which would hold about 150 datagrams
        (
            Self {
                addr: addr0.into(),
                sender: zero_to_one.0,
                receiver: one_to_zero.1,
                max_transmit_segments: options.max_transmit_segments,
                max_receive_segments: options.max_receive_segments,
            },
            Self {
                addr: addr1.into(),
                sender: one_to_zero.0,
                receiver: zero_to_one.1,
                max_transmit_segments: options.max_transmit_segments,
                max_receive_segments: options.max_receive_segments,
            },
        )
    }

    pub fn new(
        addr: impl Into<SocketAddr>,
        sender: mpsc::Sender<OwnedTransmit>,
        receiver: mpsc::Receiver<OwnedTransmit>,
        options: Options,
    ) -> Self {
        Self {
            addr: addr.into(),
            sender,
            receiver,
            max_transmit_segments: options.max_transmit_segments,
            max_receive_segments: options.max_receive_segments,
        }
    }

    pub async fn receive_datagram(&mut self) -> std::io::Result<(SocketAddr, Bytes)> {
        use std::io::IoSliceMut;

        let mut buf = [0u8; 1200];
        let mut bufs = [IoSliceMut::new(&mut buf)];
        let mut meta = [udp::RecvMeta::default()];

        let num_datagrams =
            std::future::poll_fn(|cx| self.poll_recv(cx, &mut bufs, &mut meta)).await?;

        debug_assert_eq!(num_datagrams, 1); // we don't support GSO/GRO(?)

        Ok((meta[0].addr, Bytes::copy_from_slice(&buf[..meta[0].len])))
    }

    pub async fn send_datagram(
        &self,
        destination: impl Into<SocketAddr>,
        contents: Bytes,
    ) -> std::io::Result<()> {
        let transmit = OwnedTransmit {
            contents,
            destination: destination.into(),
            ecn: None,
            segment_size: None,
            src_ip: self.addr,
        };

        let mut socket_sender = self.create_sender();
        std::future::poll_fn(|cx| {
            socket_sender
                .as_mut()
                .poll_send(&transmit.as_quinn_transmit(), cx)
        })
        .await?;

        Ok(())
    }
}

impl quinn::UdpSender for VirtualSocketSender {
    fn max_transmit_segments(&self) -> usize {
        self.max_transmit_segments
    }

    fn poll_send(
        mut self: Pin<&mut Self>,
        transmit: &Transmit,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        if let Err(_closed) = std::task::ready!(self.as_mut().sender.poll_reserve(cx)) {
            return Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "virtual socket closed when sending",
            )));
        }

        let addr = self.addr;
        if let Err(_closed) = self
            .as_mut()
            .sender
            .send_item(OwnedTransmit::new(addr, transmit))
        {
            return Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "virtual socket closed when sending",
            )));
        }

        Poll::Ready(Ok(()))
    }
}

impl AsyncUdpSocket for VirtualSocket {
    fn create_sender(&self) -> Pin<Box<dyn iroh_quinn::UdpSender>> {
        Box::pin(VirtualSocketSender {
            addr: self.addr,
            sender: PollSender::new(self.sender.clone()),
            max_transmit_segments: self.max_transmit_segments,
        })
    }

    fn max_receive_segments(&self) -> usize {
        self.max_receive_segments
    }

    fn poll_recv(
        &mut self,
        cx: &mut std::task::Context<'_>,
        bufs: &mut [std::io::IoSliceMut<'_>],
        meta: &mut [udp::RecvMeta],
    ) -> Poll<std::io::Result<usize>> {
        let mut transmits = Vec::new();
        let recvd = ready!(self.receiver.poll_recv_many(cx, &mut transmits, bufs.len()));
        if recvd == 0 {
            return Poll::Pending;
            // return Poll::Ready(Err(std::io::Error::new(
            //     std::io::ErrorKind::BrokenPipe,
            //     "virtual socket closed when receiving",
            // )));
        }

        for ((t, buf), meta) in transmits
            .into_iter()
            .zip(bufs.iter_mut())
            .zip(meta.iter_mut())
        {
            if buf.len() >= t.contents.len() {
                // tracing::debug!(
                //     "Received from {:?} to {:?}: {} bytes",
                //     t.src_ip,
                //     t.destination,
                //     t.contents.len()
                // );
                t.receive_into(buf, meta)?;
            }
        }

        Poll::Ready(Ok(recvd))
    }

    fn local_addr(&self) -> std::io::Result<std::net::SocketAddr> {
        Ok(self.addr)
    }
}

benchmark_group!(
    benches,
    large_data_1_stream,
    large_data_10_streams,
    small_data_1_stream,
    small_data_100_streams
);
benchmark_main!(benches);

fn large_data_1_stream(bench: &mut Bencher) {
    send_data(bench, LARGE_DATA, 1);
}

fn large_data_10_streams(bench: &mut Bencher) {
    send_data(bench, LARGE_DATA, 10);
}

fn small_data_1_stream(bench: &mut Bencher) {
    send_data(bench, SMALL_DATA, 1);
}

fn small_data_100_streams(bench: &mut Bencher) {
    send_data(bench, SMALL_DATA, 100);
}

fn send_data(bench: &mut Bencher, data: &'static [u8], concurrent_streams: usize) {
    let _ = tracing_subscriber::fmt::try_init();

    let (client_sock, server_sock) = VirtualSocket::pair(TestAddr(0), TestAddr(1), Options::default());
    let ctx = Context::new();
    let (addr, thread) = ctx.spawn_server(server_sock);
    let (endpoint, client, runtime) = ctx.make_client(client_sock, addr);
    let client = Arc::new(client);

    bench.bytes = (data.len() as u64) * (concurrent_streams as u64);
    bench.iter(|| {
        let mut handles = Vec::new();

        for _ in 0..concurrent_streams {
            let client = client.clone();
            handles.push(runtime.spawn(async move {
                let mut stream = client.open_uni().await.unwrap();
                stream.write_all(data).await.unwrap();
                stream.finish().unwrap();
                // Wait for stream to close
                _ = stream.stopped().await;
            }));
        }

        runtime.block_on(async {
            for handle in handles {
                handle.await.unwrap();
            }
        });
    });
    drop(client);
    runtime.block_on(endpoint.wait_idle());
    thread.join().unwrap()
}

struct Context {
    server_config: quinn::ServerConfig,
    client_config: quinn::ClientConfig,
}

impl Context {
    fn new() -> Self {
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let key = PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der());
        let cert = CertificateDer::from(cert.cert);

        let mut server_config =
            quinn::ServerConfig::with_single_cert(vec![cert.clone()], key.into()).unwrap();
        let transport_config = Arc::get_mut(&mut server_config.transport).unwrap();
        transport_config.max_concurrent_uni_streams(1024_u16.into());

        let mut roots = rustls::RootCertStore::empty();
        roots.add(cert).unwrap();

        Self {
            server_config,
            client_config: quinn::ClientConfig::with_root_certificates(Arc::new(roots)).unwrap(),
        }
    }

    pub fn spawn_server(&self, sock: VirtualSocket) -> (SocketAddr, thread::JoinHandle<()>) {
        let addr = sock.local_addr().unwrap();
        let config = self.server_config.clone();
        let handle = thread::spawn(move || {
            let runtime = rt();
            let endpoint = {
                let _guard = runtime.enter();
                Endpoint::new_with_abstract_socket(
                    Default::default(),
                    Some(config),
                    Box::new(sock),
                    Arc::new(TokioRuntime),
                )
                .unwrap()
            };
            let handle = runtime.spawn(
                async move {
                    let connection = endpoint
                        .accept()
                        .await
                        .expect("accept")
                        .await
                        .expect("connect");

                    while let Ok(mut stream) = connection.accept_uni().await {
                        tokio::spawn(async move {
                            while stream
                                .read_chunk(usize::MAX, false)
                                .await
                                .unwrap()
                                .is_some()
                            {}
                        });
                    }
                }
                .instrument(error_span!("server")),
            );
            runtime.block_on(handle).unwrap();
        });
        (addr, handle)
    }

    pub fn make_client(
        &self,
        sock: VirtualSocket,
        server_addr: SocketAddr,
    ) -> (quinn::Endpoint, quinn::Connection, Runtime) {
        let runtime = rt();
        let endpoint = {
            let _guard = runtime.enter();
            Endpoint::new_with_abstract_socket(
                Default::default(),
                None,
                Box::new(sock),
                Arc::new(TokioRuntime),
            )
            .unwrap()
        };
        let connection = runtime
            .block_on(async {
                endpoint
                    .connect_with(self.client_config.clone(), server_addr, "localhost")
                    .unwrap()
                    .instrument(error_span!("client"))
                    .await
            })
            .unwrap();
        (endpoint, connection, runtime)
    }
}

fn rt() -> Runtime {
    Builder::new_multi_thread().enable_all().build().unwrap()
}

const LARGE_DATA: &[u8] = &[0xAB; 1024 * 1024];

const SMALL_DATA: &[u8] = &[0xAB; 1];
