use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll, ready};
use std::time::Duration;

use proto::{
    ClosePathError, ClosedPath, ConnectionError, PathError, PathEvent, PathId, PathStatus, VarInt,
};
use tokio::sync::{oneshot, watch};
use tokio::task::AbortHandle;
use tokio_stream::{Stream, wrappers::WatchStream};

use crate::connection::ConnectionRef;

/// Future produced by [`crate::Connection::open_path`]
pub struct OpenPath(OpenPathInner);

enum OpenPathInner {
    /// Opening a path in underway
    ///
    /// This migth fail later on.
    Ongoing {
        opened: watch::Receiver<Option<Result<(), PathError>>>,
        path_id: PathId,
        conn: ConnectionRef,
    },
    /// Opening a path failed immediately
    Rejected {
        /// The error that occurred
        err: PathError,
    },
    /// The path is already open
    Ready {
        path_id: PathId,
        conn: ConnectionRef,
    },
}

impl OpenPath {
    pub(crate) fn new(
        path_id: PathId,
        opened: watch::Receiver<Option<Result<(), PathError>>>,
        conn: ConnectionRef,
    ) -> Self {
        Self(OpenPathInner::Ongoing {
            opened,
            path_id,
            conn,
        })
    }

    pub(crate) fn ready(path_id: PathId, conn: ConnectionRef) -> Self {
        Self(OpenPathInner::Ready { path_id, conn })
    }

    pub(crate) fn rejected(err: PathError) -> Self {
        Self(OpenPathInner::Rejected { err })
    }
}

impl Future for OpenPath {
    type Output = Result<Path, PathError>;
    fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.get_mut().0 {
            OpenPathInner::Ongoing {
                ref mut opened,
                path_id,
                ref mut conn,
            } => {
                let mut fut = std::pin::pin!(opened.wait_for(|v| v.is_some()));
                fut.as_mut().poll(ctx).map(|_| {
                    Ok(Path {
                        id: path_id,
                        conn: conn.clone(),
                    })
                })
            }
            OpenPathInner::Ready {
                path_id,
                ref mut conn,
            } => Poll::Ready(Ok(Path {
                id: path_id,
                conn: conn.clone(),
            })),
            OpenPathInner::Rejected { err } => Poll::Ready(Err(err)),
        }
    }
}

/// An open (Multi)Path
pub struct Path {
    pub(crate) id: PathId,
    pub(crate) conn: ConnectionRef,
}

impl Path {
    /// The [`PathId`] of this path.
    pub fn id(&self) -> PathId {
        self.id
    }

    /// The current local [`PathStatus`] of this path.
    pub fn status(&self) -> Result<PathStatus, ClosedPath> {
        self.conn
            .state
            .lock("path status")
            .inner
            .path_status(self.id)
    }

    /// Closes this path
    ///
    /// The passed in `error_code` is sent to the remote.
    /// The future will resolve to the `error_code` received from the remote.
    pub fn close(&self, error_code: VarInt) -> Result<ClosePath, ClosePathError> {
        let (on_path_close_send, on_path_close_recv) = oneshot::channel();
        {
            let mut state = self.conn.state.lock("close_path");
            state
                .inner
                .close_path(crate::Instant::now(), self.id, error_code)?;
            state.close_path.insert(self.id, on_path_close_send);
        }

        Ok(ClosePath {
            closed: on_path_close_recv,
        })
    }

    /// Sets the keep_alive_interval for a specific path
    ///
    /// See [`TransportConfig::default_path_keep_alive_interval`] for details.
    ///
    /// Returns the previous value of the setting.
    ///
    /// [`TransportConfig::default_path_keep_alive_interval`]: crate::TransportConfig::default_path_keep_alive_interval
    pub fn set_max_idle_timeout(
        &self,
        timeout: Option<Duration>,
    ) -> Result<Option<Duration>, ClosedPath> {
        let mut state = self.conn.state.lock("path_set_max_idle_timeout");
        state.inner.set_path_max_idle_timeout(self.id, timeout)
    }

    /// Sets the keep_alive_interval for a specific path
    ///
    /// See [`TransportConfig::default_path_keep_alive_interval`] for details.
    ///
    /// Returns the previous value of the setting.
    ///
    /// [`TransportConfig::default_path_keep_alive_interval`]: crate::TransportConfig::default_path_keep_alive_interval
    pub fn set_keep_alive_interval(
        &self,
        interval: Option<Duration>,
    ) -> Result<Option<Duration>, ClosedPath> {
        let mut state = self.conn.state.lock("path_set_keep_alive_interval");
        state.inner.set_path_keep_alive_interval(self.id, interval)
    }

    /// Track changes on our external address as reported by the peer.
    ///
    /// If the address-discovery extension is not negotiated, the stream will never return.
    pub fn observed_external_addr(&self) -> Result<AddressDiscovery, ClosedPath> {
        let state = self.conn.state.lock("per_path_observed_address");
        let mut path_events = state.path_events.subscribe();
        let path_id = self.id;
        let initial_value = state.inner.path_observed_address(path_id)?;
        // if the dummy value is used, it will be ignored
        let (tx, rx) = watch::channel(
            initial_value.unwrap_or_else(|| SocketAddr::new([0, 0, 0, 0].into(), 0)),
        );
        let filter = async move {
            loop {
                match path_events.recv().await {
                    Ok(PathEvent::ObservedAddr { id, addr: observed }) if id == path_id => {
                        tx.send_if_modified(|addr| {
                            let old = std::mem::replace(addr, observed);
                            old != *addr
                        });
                    }
                    Ok(PathEvent::Abandoned { id, .. }) if id == path_id => {
                        // If the path is closed, terminate the stream
                        break;
                    }
                    Ok(_) => {
                        // ignore any other event
                    }
                    Err(_) => {
                        // A lagged error should never happen since this (detached) task is
                        // constantly reading from the channel. Therefore, if an error does happen,
                        // the stream can terminate
                        break;
                    }
                }
            }
        };

        let watcher = if initial_value.is_some() {
            WatchStream::new(rx)
        } else {
            WatchStream::from_changes(rx)
        };

        let filter_future = tokio::spawn(filter).abort_handle();
        Ok(AddressDiscovery {
            watcher,
            filter_future,
        })
    }
}

/// Future produced by [`Path::close`]
pub struct ClosePath {
    closed: oneshot::Receiver<VarInt>,
}

impl Future for ClosePath {
    type Output = Result<VarInt, ConnectionError>;
    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        // TODO: thread through errors
        let res = ready!(Pin::new(&mut self.closed).poll(ctx));
        match res {
            Ok(code) => Poll::Ready(Ok(code)),
            Err(_err) => todo!(), // TODO: appropriate error
        }
    }
}

/// Stream produced by [`Path::observed_external_addr`]
///
/// Will always return the most recently seen observed address by the remote over this path. If the
/// extension is not negotiated, this stream will never return. If the path is abandoned, the
/// stream will end.
pub struct AddressDiscovery {
    watcher: WatchStream<SocketAddr>,
    filter_future: AbortHandle,
}

impl Stream for AddressDiscovery {
    type Item = SocketAddr;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.filter_future.is_finished() {
            return Poll::Ready(None);
        }
        Pin::new(&mut self.watcher).poll_next(cx)
    }
}

impl Drop for AddressDiscovery {
    fn drop(&mut self) {
        self.filter_future.abort();
    }
}
