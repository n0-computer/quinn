//! Newtype wrappers around broadcast receivers for connection events.

use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll};

use proto::PathEvent;
use proto::n0_nat_traversal;
use thiserror::Error;
use tokio::sync::broadcast;
use tokio_stream::wrappers::{BroadcastStream, errors::BroadcastStreamRecvError};
use tokio_stream::Stream;

/// The receiver lagged too far behind. Attempting to receive again will
/// return the oldest message still retained by the channel.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("channel lagged by {0}")]
pub struct Lagged(pub u64);

/// A stream of [`PathEvent`]s for all paths in a connection.
pub struct PathEvents {
    inner: BroadcastStream<PathEvent>,
}

impl PathEvents {
    pub(crate) fn new(rx: broadcast::Receiver<PathEvent>) -> Self {
        Self {
            inner: BroadcastStream::new(rx),
        }
    }
}

impl Stream for PathEvents {
    type Item = Result<PathEvent, Lagged>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner)
            .poll_next(cx)
            .map(|opt| opt.map(|res| res.map_err(|BroadcastStreamRecvError::Lagged(n)| Lagged(n))))
    }
}

impl fmt::Debug for PathEvents {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PathEvents").finish()
    }
}

/// A stream of NAT traversal updates for a connection.
pub struct NatTraversalUpdates {
    inner: BroadcastStream<n0_nat_traversal::Event>,
}

impl NatTraversalUpdates {
    pub(crate) fn new(rx: broadcast::Receiver<n0_nat_traversal::Event>) -> Self {
        Self {
            inner: BroadcastStream::new(rx),
        }
    }
}

impl Stream for NatTraversalUpdates {
    type Item = Result<n0_nat_traversal::Event, Lagged>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner)
            .poll_next(cx)
            .map(|opt| opt.map(|res| res.map_err(|BroadcastStreamRecvError::Lagged(n)| Lagged(n))))
    }
}

impl fmt::Debug for NatTraversalUpdates {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NatTraversalUpdates").finish()
    }
}
