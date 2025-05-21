use std::{
    fmt::Debug,
    future::Future,
    io,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Instant,
};

use tokio::{
    io::Interest,
    time::{sleep_until, Sleep},
};

use super::{AsyncTimer, AsyncUdpSocket, Runtime};

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

    fn wrap_udp_socket(&self, sock: std::net::UdpSocket) -> io::Result<Arc<dyn AsyncUdpSocket>> {
        Ok(Arc::new(UdpSocket {
            inner: udp::UdpSocketState::new((&sock).into())?,
            io: tokio::net::UdpSocket::from_std(sock)?,
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
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
        Future::poll(self, cx)
    }
}

#[derive(Debug)]
struct UdpSocket {
    io: tokio::net::UdpSocket,
    inner: udp::UdpSocketState,
}

pin_project_lite::pin_project! {
    struct UdpSender {
        #[pin]
        fut: Option<Pin<Box<dyn Future<Output = io::Result<()>> + Send + Sync + 'static>>>,
        inner: Arc<UdpSocket>,
    }
}

impl Debug for UdpSender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("UdpSender")
    }
}

impl super::UdpSender for UdpSender {
    fn poll_send(
        self: Pin<&mut Self>,
        transmit: &udp::Transmit,
        cx: &mut Context,
    ) -> Poll<io::Result<()>> {
        let mut this = self.project();
        loop {
            if this.fut.is_none() {
                this.fut.set(Some(Box::pin({
                    let socket = this.inner.clone();
                    async move { socket.io.writable().await }
                })));
            }
            // We're forced to `unwrap` here because `Fut` may be `!Unpin`, which means we can't safely
            // obtain an `&mut Fut` after storing it in `self.fut` when `self` is already behind `Pin`,
            // and if we didn't store it then we wouldn't be able to keep it alive between
            // `poll_writable` calls.
            let result = ready!(this.fut.as_mut().as_pin_mut().unwrap().poll(cx));

            // Polling an arbitrary `Future` after it becomes ready is a logic error, so arrange for
            // a new `Future` to be created on the next call.
            this.fut.set(None);

            // If .writable() fails, propagate the error
            result?;

            let socket = &this.inner;
            let result = socket.io.try_io(Interest::WRITABLE, || {
                socket.inner.send((&socket.io).into(), transmit)
            });

            match result {
                // We thought the socket was writable, but it wasn't, then retry so that either another
                // `writable().await` call determines that the socket is indeed not writable and
                // registers us for a wakeup, or the send succeeds if this really was just a
                // transient failure.
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                // In all other cases, either propagate the error or we're Ok
                _ => return Poll::Ready(result),
            }
        }
    }

    fn max_transmit_segments(&self) -> usize {
        self.inner.inner.max_gso_segments()
    }

    fn try_send(self: Pin<&mut Self>, transmit: &udp::Transmit) -> io::Result<()> {
        let socket = &self.inner;
        socket.io.try_io(Interest::WRITABLE, || {
            socket.inner.send((&socket.io).into(), transmit)
        })
    }
}

impl AsyncUdpSocket for UdpSocket {
    fn create_sender(self: Arc<Self>) -> Pin<Box<dyn super::UdpSender>> {
        // TODO(matheus23): There's probably a way to get rid of the double-boxing here (and the box inside UdpSender)
        Box::pin(UdpSender {
            fut: None,
            inner: self,
        })
    }

    fn poll_recv(
        &self,
        cx: &mut Context,
        bufs: &mut [std::io::IoSliceMut<'_>],
        meta: &mut [udp::RecvMeta],
    ) -> Poll<io::Result<usize>> {
        loop {
            ready!(self.io.poll_recv_ready(cx))?;
            if let Ok(res) = self.io.try_io(Interest::READABLE, || {
                self.inner.recv((&self.io).into(), bufs, meta)
            }) {
                return Poll::Ready(Ok(res));
            }
        }
    }

    fn local_addr(&self) -> io::Result<std::net::SocketAddr> {
        self.io.local_addr()
    }

    fn may_fragment(&self) -> bool {
        self.inner.may_fragment()
    }

    fn max_receive_segments(&self) -> usize {
        self.inner.gro_segments()
    }
}
