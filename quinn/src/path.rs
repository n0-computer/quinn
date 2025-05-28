use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use proto::{ConnectionError, PathId, PathStatus};
use tokio::sync::oneshot;

use crate::connection::ConnectionRef;

/// Future produced by [`Connection::open_path`]
pub struct OpenPath {
    opened: oneshot::Receiver<()>,
    path_id: PathId,
    conn: ConnectionRef,
}

impl OpenPath {
    pub(crate) fn new(path_id: PathId, opened: oneshot::Receiver<()>, conn: ConnectionRef) -> Self {
        Self {
            opened,
            path_id,
            conn,
        }
    }
}

impl Future for OpenPath {
    type Output = Result<Path, ConnectionError>;
    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        // TODO: thread through errors
        Pin::new(&mut self.opened).poll(ctx).map(|_| {
            Ok(Path {
                id: self.path_id,
                conn: self.conn.clone(),
            })
        })
    }
}

/// An open (Multi)Path
pub struct Path {
    id: PathId,
    conn: ConnectionRef,
}

impl Path {
    /// The [`PathId`] of this path.
    pub fn id(&self) -> PathId {
        self.id
    }

    /// The current [`PathStatus`] of this path.
    pub fn status(&self) -> PathStatus {
        self.conn
            .state
            .lock("path status")
            .inner
            .path_status(self.id)
    }
}
