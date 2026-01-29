use std::{
    fmt::Debug,
    future::Future,
    io::{self, IoSliceMut},
    num::NonZeroUsize,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, ready},
    time::Instant,
};

use tokio::{
    io::Interest,
    time::{Sleep, sleep_until},
};

use super::{AsyncTimer, AsyncUdpSocket, Runtime, UdpSenderHelper, UdpSenderHelperSocket};

/// A Quinn runtime for Tokio
#[derive(Debug)]
pub struct TokioRuntime;

impl Runtime for TokioRuntime {
    fn new_timer(&self, t: Instant) -> Pin<Box<dyn AsyncTimer>> {
        Box::pin(sleep_until(t.into()))
    }

    fn spawn(&self, future: Pin<Box<dyn Future<Output = ()> + Send>>) {
        tokio::spawn(future);
    }

    fn wrap_udp_socket(&self, sock: std::net::UdpSocket) -> io::Result<Box<dyn AsyncUdpSocket>> {
        Ok(Box::new(UdpSocket {
            send: UdpSocketSend {
                inner: Arc::new(udp::UdpSocketState::new((&sock).into())?),
                io: Arc::new(tokio::net::UdpSocket::from_std(sock)?),
            },
            recv_buf: Vec::new(),
        }))
    }

    fn now(&self) -> Instant {
        tokio::time::Instant::now().into_std()
    }
}

impl AsyncTimer for Sleep {
    fn reset(self: Pin<&mut Self>, t: Instant) {
        Self::reset(self, t.into())
    }
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        Future::poll(self, cx)
    }
}

/// The parts of a UDP socket needed for sending
///
/// This is separated from UdpSocket so that senders can clone just what they need
/// without carrying the receive buffer.
#[derive(Debug, Clone)]
struct UdpSocketSend {
    io: Arc<tokio::net::UdpSocket>,
    inner: Arc<udp::UdpSocketState>,
}

#[derive(Debug)]
struct UdpSocket {
    send: UdpSocketSend,
    recv_buf: Vec<u8>,
}

impl UdpSenderHelperSocket for UdpSocketSend {
    fn max_transmit_segments(&self) -> NonZeroUsize {
        self.inner.max_gso_segments()
    }

    fn try_send(&self, transmit: &udp::Transmit<'_>) -> io::Result<()> {
        self.io.try_io(Interest::WRITABLE, || {
            self.inner.send((&self.io).into(), transmit)
        })
    }
}

impl AsyncUdpSocket for UdpSocket {
    fn create_sender(&self) -> Pin<Box<dyn super::UdpSender>> {
        let core = self.send.clone();
        Box::pin(UdpSenderHelper::new(core, |socket: &UdpSocketSend| {
            let socket = socket.clone();
            async move { socket.io.writable().await }
        }))
    }

    fn poll_recv(
        &mut self,
        cx: &mut Context<'_>,
        bufs: &mut [std::io::IoSliceMut<'_>],
        meta: &mut [udp::RecvMeta],
    ) -> Poll<io::Result<usize>> {
        loop {
            ready!(self.send.io.poll_recv_ready(cx))?;
            if let Ok(res) = self.send.io.try_io(Interest::READABLE, || {
                self.send.inner.recv((&self.send.io).into(), bufs, meta)
            }) {
                return Poll::Ready(Ok(res));
            }
        }
    }

    fn poll_recv_datagrams(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<udp::ReceivedDatagrams>> {
        // Ensure buffer is sized for GRO coalescing
        // Use 1500 (typical Ethernet MTU) as max payload size
        const MAX_PAYLOAD_SIZE: usize = 1500;
        let gro_segments = self.send.inner.gro_segments().get();
        let buf_size = MAX_PAYLOAD_SIZE * gro_segments;
        let total_size = buf_size * udp::BATCH_SIZE;
        if self.recv_buf.len() < total_size {
            self.recv_buf.resize(total_size, 0);
        }

        loop {
            ready!(self.send.io.poll_recv_ready(cx))?;

            // Prepare IoSliceMut array
            let mut bufs: [IoSliceMut<'_>; udp::BATCH_SIZE] =
                std::array::from_fn(|_| IoSliceMut::new(&mut []));
            for (i, chunk) in self
                .recv_buf
                .chunks_mut(buf_size)
                .enumerate()
                .take(udp::BATCH_SIZE)
            {
                bufs[i] = IoSliceMut::new(chunk);
            }
            let mut metas = [udp::RecvMeta::default(); udp::BATCH_SIZE];

            if let Ok(msg_count) = self.send.io.try_io(Interest::READABLE, || {
                self.send
                    .inner
                    .recv((&self.send.io).into(), &mut bufs, &mut metas)
            }) {
                // Convert to ReceivedDatagrams, splitting by stride
                let mut result = udp::ReceivedDatagrams::new();
                for (meta, buf) in metas.iter().zip(bufs.iter()).take(msg_count) {
                    let mut offset = 0;
                    while offset < meta.len {
                        let stride = meta.stride.min(meta.len - offset);
                        let data = buf[offset..offset + stride].to_vec();
                        result.push(udp::ReceivedDatagram {
                            data,
                            remote: meta.addr,
                            local_ip: meta.dst_ip,
                            ecn: meta.ecn,
                        });
                        offset += stride;
                    }
                }
                return Poll::Ready(Ok(result));
            }
        }
    }

    fn local_addr(&self) -> io::Result<std::net::SocketAddr> {
        self.send.io.local_addr()
    }

    fn may_fragment(&self) -> bool {
        self.send.inner.may_fragment()
    }

    fn max_receive_segments(&self) -> NonZeroUsize {
        self.send.inner.gro_segments()
    }
}
