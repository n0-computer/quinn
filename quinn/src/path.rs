use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, ready};
use std::time::Duration;

use proto::{
    ClosePathError, ClosedPath, ConnectionError, PathError, PathEvent, PathId, PathStats,
    PathStatus, SetPathStatusError, TransportErrorCode, VarInt,
};
use tokio::sync::{oneshot, watch};
use tokio_stream::{Stream, wrappers::WatchStream};

use crate::connection::ConnectionRef;
use crate::{Runtime, WeakConnectionHandle};

/// Future produced by [`crate::Connection::open_path`]
pub struct OpenPath(OpenPathInner);

enum OpenPathInner {
    /// Opening a path in underway
    ///
    /// This might fail later on.
    Ongoing {
        opened: WatchStream<Result<(), PathError>>,
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
        opened: watch::Receiver<Result<(), PathError>>,
        conn: ConnectionRef,
    ) -> Self {
        Self(OpenPathInner::Ongoing {
            opened: WatchStream::from_changes(opened),
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

    /// Returns the path ID of the new path being opened.
    ///
    /// If an error occurred before a path ID was allocated, `None` is returned.  In this
    /// case the future is ready and polling it will immediately yield the error.
    ///
    /// The returned value remains the same for the entire lifetime of this future.
    pub fn path_id(&self) -> Option<PathId> {
        match self.0 {
            OpenPathInner::Ongoing { path_id, .. } => Some(path_id),
            OpenPathInner::Rejected { .. } => None,
            OpenPathInner::Ready { path_id, .. } => Some(path_id),
        }
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
            } => match ready!(Pin::new(opened).poll_next(ctx)) {
                Some(value) => {
                    Poll::Ready(value.map(|_| Path::new_unchecked(conn.clone(), path_id)))
                }
                None => {
                    // This only happens if receiving a notification change failed, this means the
                    // sender was dropped. This generally should not happen so we use a transient
                    // error
                    Poll::Ready(Err(PathError::ValidationFailed))
                }
            },
            OpenPathInner::Ready {
                path_id,
                ref mut conn,
            } => Poll::Ready(Ok(Path::new_unchecked(conn.clone(), path_id))),
            OpenPathInner::Rejected { err } => Poll::Ready(Err(err)),
        }
    }
}

/// An open network transmission within a multipath-enabled connection.
///
/// As long as a [`Path`] or [`WeakPathHandle`] is alive, it is ensured that the [`PathStats`] for this path
/// are not dropped even after the path is abandoned.
#[derive(Debug)]
pub struct Path {
    id: PathId,
    conn: ConnectionRef,
}

impl Clone for Path {
    fn clone(&self) -> Self {
        self.conn
            .state
            .lock("Path::clone")
            .increment_path_refs(self.id);
        Self {
            id: self.id,
            conn: self.conn.clone(),
        }
    }
}

impl Drop for Path {
    fn drop(&mut self) {
        let mut state = self.conn.state.lock("Path::drop");
        state.decrement_path_refs(self.id);
    }
}

impl Path {
    /// Returns a [`Path`] for a path id, after checking that the path is not closed.
    pub(crate) fn new(conn: &ConnectionRef, id: PathId) -> Option<Self> {
        {
            let mut state = conn.state.lock("Path::new");
            // TODO(flub): Using this to know if the path still exists is... hacky.
            state.inner.path_status(id).ok()?;
            state.increment_path_refs(id);
        }
        Some(Self {
            id,
            conn: conn.clone(),
        })
    }

    /// Returns a [`Path`] for a path id without checking if the path exists or is closed.
    fn new_unchecked(conn: ConnectionRef, id: PathId) -> Self {
        conn.state
            .lock("Path::new_unchecked")
            .increment_path_refs(id);
        Self { id, conn }
    }

    /// Returns a [`WeakPathHandle`] for this path.
    ///
    /// Holding a [`WeakPathHandle`] does not keep a connection alive, but ensures that the
    /// path's stats are not dropped until the underlying connection is dropped, even if the
    /// path is abandoned.
    pub fn weak_handle(&self) -> WeakPathHandle {
        self.conn
            .state
            .lock("Path::weak_handle")
            .increment_path_refs(self.id);
        WeakPathHandle {
            id: self.id,
            conn: self.conn.weak_handle(),
        }
    }

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

    /// Sets the [`PathStatus`] of this path.
    pub fn set_status(&self, status: PathStatus) -> Result<(), SetPathStatusError> {
        self.conn
            .state
            .lock("set path status")
            .inner
            .set_path_status(self.id, status)?;
        Ok(())
    }

    /// Returns the [`PathStats`] for this path.
    pub fn stats(&self) -> PathStats {
        // The `expect` is safe:
        // - `Path` can only be created for non-closed paths.
        // - `Path` and its clones or `WeakPathHandle`s all increment the connection state's `path_ref`
        //   reference counter
        // - As long as a path is not abandoned, its stats are available from `proto::Connection`
        // - If a path is abandoned, the `crate::Connection` stores the final stats as long as
        //   the path's refcount is not 0
        // - Therefore, we always get stats here.
        self.conn
            .state
            .lock("Path::stats")
            .path_stats(self.id)
            .expect("either path stats or discarded path stats are always set as long as Path is not dropped")
    }

    /// Closes this path
    ///
    /// The future will resolve when all the path state is dropped.  This only happens after
    /// the remote has confirmed the path as closed **and** after an additional timeout to
    /// give any in-flight packets the time to arrive.
    pub fn close(&self) -> Result<ClosePath, ClosePathError> {
        let (on_path_close_send, on_path_close_recv) = oneshot::channel();
        {
            let mut state = self.conn.state.lock("close_path");
            state.inner.close_path(
                crate::Instant::now(),
                self.id,
                TransportErrorCode::APPLICATION_ABANDON_PATH.into(),
            )?;
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
        let path_events = state.path_events.subscribe();
        let initial_value = state.inner.path_observed_address(self.id)?;
        Ok(AddressDiscovery::new(
            self.id,
            path_events,
            initial_value,
            state.runtime.clone(),
        ))
    }

    /// The peer's UDP address for this path.
    pub fn remote_address(&self) -> Result<SocketAddr, ClosedPath> {
        let state = self.conn.state.lock("per_path_remote_address");
        Ok(state.inner.network_path(self.id)?.remote)
    }

    /// Ping the remote endpoint over this path.
    pub fn ping(&self) -> Result<(), ClosedPath> {
        let mut state = self.conn.state.lock("ping");
        state.inner.ping_path(self.id)
    }
}

/// Weak handle for a [`Path`] that does not keep the connection alive.
///
/// As long as a [`WeakPathHandle`] for a path exists, that path's final stats will not be dropped even if
/// the path was abandoned.
///
/// The [`WeakPathHandle`] can be upgraded to a [`Path`] as long as its [`Connection`] has not been dropped.
///
/// [`Connection`]: crate::Connection
#[derive(Debug)]
pub struct WeakPathHandle {
    id: PathId,
    conn: WeakConnectionHandle,
}

impl Clone for WeakPathHandle {
    fn clone(&self) -> Self {
        if let Some(conn) = self.conn.upgrade_to_ref() {
            conn.state
                .lock("WeakPathHandle::clone")
                .increment_path_refs(self.id);
        }
        Self {
            id: self.id,
            conn: self.conn.clone(),
        }
    }
}

impl PartialEq for WeakPathHandle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.conn.ptr_eq(&other.conn)
    }
}

impl Eq for WeakPathHandle {}

impl Drop for WeakPathHandle {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.upgrade_to_ref() {
            conn.state
                .lock("WeakPathHandle::drop")
                .decrement_path_refs(self.id);
        }
    }
}

impl WeakPathHandle {
    /// Returns the [`PathId`] of this path.
    pub fn id(&self) -> PathId {
        self.id
    }

    /// Upgrades to a [`Path`].
    ///
    /// Returns `None` if the connection was dropped.
    pub fn upgrade(&self) -> Option<Path> {
        let conn = self.conn.upgrade_to_ref()?;
        Some(Path::new_unchecked(conn, self.id))
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
/// This will always return the external address most recently reported by the remote over this
/// path. If the extension is not negotiated, this stream will never return.
// TODO(@divma): provide a way to check if the extension is negotiated.
pub struct AddressDiscovery {
    watcher: WatchStream<SocketAddr>,
}

impl AddressDiscovery {
    pub(super) fn new(
        path_id: PathId,
        mut path_events: tokio::sync::broadcast::Receiver<PathEvent>,
        initial_value: Option<SocketAddr>,
        runtime: Arc<dyn Runtime>,
    ) -> Self {
        let (tx, rx) = watch::channel(initial_value.unwrap_or_else(||
                // if the dummy value is used, it will be ignored
                SocketAddr::new([0, 0, 0, 0].into(), 0)));
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

        runtime.spawn(Box::pin(filter));
        // TODO(@divma): check if there's a way to ensure the future ends. AbortHandle is not an
        // option
        Self { watcher }
    }
}

impl Stream for AddressDiscovery {
    type Item = SocketAddr;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.watcher).poll_next(cx)
    }
}
