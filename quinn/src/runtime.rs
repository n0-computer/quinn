use std::{
    fmt::Debug,
    future::Future,
    io::{self, IoSliceMut},
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use udp::{RecvMeta, Transmit};

use crate::Instant;

/// Abstracts I/O and timer operations for runtime independence
pub trait Runtime: Send + Sync + Debug + 'static {
    /// Construct a timer that will expire at `i`
    fn new_timer(&self, i: Instant) -> Pin<Box<dyn AsyncTimer>>;
    /// Drive `future` to completion in the background
    fn spawn(&self, future: Pin<Box<dyn Future<Output = ()> + Send>>);
    /// Convert `t` into the socket type used by this runtime
    #[cfg(not(wasm_browser))]
    fn wrap_udp_socket(&self, t: std::net::UdpSocket) -> io::Result<Box<dyn AsyncUdpSocket>>;
    /// Look up the current time
    ///
    /// Allows simulating the flow of time for testing.
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// Abstract implementation of an async timer for runtime independence
pub trait AsyncTimer: Send + Debug + 'static {
    /// Update the timer to expire at `i`
    fn reset(self: Pin<&mut Self>, i: Instant);
    /// Check whether the timer has expired, and register to be woken if not
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<()>;
}

/// Abstract implementation of a UDP socket for runtime independence
pub trait AsyncUdpSocket: Send + Sync + Debug + 'static {
    /// Create a [`UdpPoller`] that can register a single task for write-readiness notifications
    ///
    /// A `poll_send` method on a single object can usually store only one [`Waker`] at a time,
    /// i.e. allow at most one caller to wait for an event. This method allows any number of
    /// interested tasks to construct their own [`UdpPoller`] object. They can all then wait for the
    /// same event and be notified concurrently, because each [`UdpPoller`] can store a separate
    /// [`Waker`].
    ///
    /// [`Waker`]: std::task::Waker
    fn create_sender(&self) -> Pin<Box<dyn UdpSender>>;

    /// Receive UDP datagrams, or register to be woken if receiving may succeed in the future
    fn poll_recv(
        &mut self,
        cx: &mut Context,
        bufs: &mut [IoSliceMut<'_>],
        meta: &mut [RecvMeta],
    ) -> Poll<io::Result<usize>>;

    /// Look up the local IP address and port used by this socket
    fn local_addr(&self) -> io::Result<SocketAddr>;

    /// Maximum number of datagrams that might be described by a single [`RecvMeta`]
    fn max_receive_segments(&self) -> usize {
        1
    }

    /// Whether datagrams might get fragmented into multiple parts
    ///
    /// Sockets should prevent this for best performance. See e.g. the `IPV6_DONTFRAG` socket
    /// option.
    fn may_fragment(&self) -> bool {
        true
    }
}

/// An object polled to detect when an associated [`AsyncUdpSocket`] is writable
///
/// Any number of `UdpPoller`s may exist for a single [`AsyncUdpSocket`]. Each `UdpPoller` is
/// responsible for notifying at most one task when that socket becomes writable.
pub trait UdpSender: Send + Sync + Debug + 'static {
    /// Check whether the associated socket is likely to be writable
    ///
    /// Must be called after [`AsyncUdpSocket::try_send`] returns [`io::ErrorKind::WouldBlock`] to
    /// register the task associated with `cx` to be woken when a send should be attempted
    /// again. Unlike in [`Future::poll`], a [`UdpPoller`] may be reused indefinitely no matter how
    /// many times `poll_writable` returns [`Poll::Ready`].
    ///
    /// // TODO(matheus23): Fix weird documentation merge
    ///
    /// Send UDP datagrams from `transmits`, or return `WouldBlock` and clear the underlying
    /// socket's readiness, or return an I/O error
    ///
    /// If this returns [`io::ErrorKind::WouldBlock`], [`UdpPoller::poll_writable`] must be called
    /// to register the calling task to be woken when a send should be attempted again.
    fn poll_send(
        self: Pin<&mut Self>,
        transmit: &Transmit,
        cx: &mut Context,
    ) -> Poll<io::Result<()>>;

    /// Maximum number of datagrams that a [`Transmit`] may encode
    fn max_transmit_segments(&self) -> usize {
        1
    }

    /// TODO(matheus23): Docs
    /// Last ditch/best effort of sending a transmit.
    /// Used by the endpoint for resets / close frames when dropped, etc.
    fn try_send(self: Pin<&mut Self>, transmit: &Transmit) -> io::Result<()>;
}

pin_project_lite::pin_project! {
    struct UdpSenderHelper<Socket, MakeFut, Fut> {
        socket: Socket,
        make_fut: MakeFut,
        #[pin]
        fut: Option<Fut>,
    }
}

impl<Socket, MakeFut, Fut> Debug for UdpSenderHelper<Socket, MakeFut, Fut> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("UdpSender")
    }
}

impl<Socket, MakeFut, Fut> UdpSenderHelper<Socket, MakeFut, Fut> {
    fn new(inner: Socket, make_fut: MakeFut) -> Self {
        Self {
            socket: inner,
            make_fut,
            fut: None,
        }
    }
}

impl<Socket, MakeFut, Fut> super::UdpSender for UdpSenderHelper<Socket, MakeFut, Fut>
where
    Socket: UdpSenderHelperSocket,
    MakeFut: Fn(&Socket) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = io::Result<()>> + Send + Sync + 'static,
{
    fn poll_send(
        self: Pin<&mut Self>,
        transmit: &udp::Transmit,
        cx: &mut Context,
    ) -> Poll<io::Result<()>> {
        let mut this = self.project();
        loop {
            if this.fut.is_none() {
                this.fut.set(Some((this.make_fut)(&this.socket)));
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

            let result = this.socket.try_send(transmit);

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
        self.socket.max_transmit_segments()
    }

    fn try_send(self: Pin<&mut Self>, transmit: &udp::Transmit) -> io::Result<()> {
        self.socket.try_send(transmit)
    }
}

trait UdpSenderHelperSocket: Send + Sync + 'static {
    fn try_send(&self, transmit: &udp::Transmit) -> io::Result<()>;

    fn max_transmit_segments(&self) -> usize;
}

/// Automatically select an appropriate runtime from those enabled at compile time
///
/// If `runtime-tokio` is enabled and this function is called from within a Tokio runtime context,
/// then `TokioRuntime` is returned. Otherwise, if `runtime-async-std` is enabled, `AsyncStdRuntime`
/// is returned. Otherwise, if `runtime-smol` is enabled, `SmolRuntime` is returned.
/// Otherwise, `None` is returned.
#[allow(clippy::needless_return)] // Be sure we return the right thing
pub fn default_runtime() -> Option<Arc<dyn Runtime>> {
    #[cfg(feature = "runtime-tokio")]
    {
        if ::tokio::runtime::Handle::try_current().is_ok() {
            return Some(Arc::new(TokioRuntime));
        }
    }

    #[cfg(feature = "runtime-async-std")]
    {
        return Some(Arc::new(AsyncStdRuntime));
    }

    #[cfg(all(feature = "runtime-smol", not(feature = "runtime-async-std")))]
    {
        return Some(Arc::new(SmolRuntime));
    }

    #[cfg(not(any(feature = "runtime-async-std", feature = "runtime-smol")))]
    None
}

#[cfg(feature = "runtime-tokio")]
mod tokio;
// Due to MSRV, we must specify `self::` where there's crate/module ambiguity
#[cfg(feature = "runtime-tokio")]
pub use self::tokio::TokioRuntime;

#[cfg(feature = "async-io")]
mod async_io;
// Due to MSRV, we must specify `self::` where there's crate/module ambiguity
#[cfg(feature = "async-io")]
pub use self::async_io::*;
