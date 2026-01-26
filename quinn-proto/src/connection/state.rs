use bytes::Bytes;
use tracing::trace;

use crate::frame::Close;
use crate::{ApplicationClose, ConnectionClose, ConnectionError, TransportError, TransportErrorCode};

#[allow(unreachable_pub)] // fuzzing only
#[derive(Debug, Clone)]
pub struct State {
    /// Nested [`InnerState`] to enforce all state transitions are done in this module.
    inner: InnerState,
}

impl State {
    pub(super) fn as_handshake_mut(&mut self) -> Option<&mut Handshake> {
        if let InnerState::Handshake(ref mut hs) = self.inner {
            Some(hs)
        } else {
            None
        }
    }

    pub(super) fn as_handshake(&self) -> Option<&Handshake> {
        if let InnerState::Handshake(ref hs) = self.inner {
            Some(hs)
        } else {
            None
        }
    }

    pub(super) fn as_closed(&self) -> Option<&CloseReason> {
        if let InnerState::Closed {
            ref remote_reason, ..
        } = self.inner
        {
            Some(remote_reason)
        } else {
            None
        }
    }

    #[allow(unreachable_pub)] // fuzzing only
    #[cfg(any(test, fuzzing))]
    pub fn established() -> Self {
        Self {
            inner: InnerState::Established,
        }
    }

    pub(super) fn handshake(hs: Handshake) -> Self {
        Self {
            inner: InnerState::Handshake(hs),
        }
    }

    pub(super) fn move_to_handshake(&mut self, hs: Handshake) {
        self.inner = InnerState::Handshake(hs);
        trace!("connection state: handshake");
    }

    pub(super) fn move_to_established(&mut self) {
        self.inner = InnerState::Established;
        trace!("connection state: established");
    }

    /// Moves to the drained state.
    ///
    /// Panics if the state was already drained.
    pub(super) fn move_to_drained(&mut self, error: Option<ConnectionError>) {
        let (error, is_local) = if let Some(error) = error {
            (Some(error), false)
        } else {
            let error = match &mut self.inner {
                InnerState::Draining { error, .. } => error.take(),
                InnerState::Drained { .. } => panic!("invalid state transition drained -> drained"),
                InnerState::Closed { error_read, .. } if *error_read => None,
                InnerState::Closed { remote_reason, .. } => {
                    let error = match remote_reason.clone().into() {
                        ConnectionError::ConnectionClosed(close) => {
                            if close.error_code == TransportErrorCode::PROTOCOL_VIOLATION {
                                ConnectionError::TransportError(TransportError::new(
                                    close.error_code,
                                    std::string::String::from_utf8_lossy(&close.reason[..])
                                        .to_string(),
                                ))
                            } else {
                                ConnectionError::ConnectionClosed(close)
                            }
                        }
                        e => e,
                    };
                    Some(error)
                }
                InnerState::Handshake(_) | InnerState::Established => None,
            };
            (error, self.is_local_close())
        };
        self.inner = InnerState::Drained { error, is_local };
        trace!("connection state: drained");
    }

    /// Moves to a draining state.
    ///
    /// Panics if the state is already draining or drained.
    pub(super) fn move_to_draining(&mut self, error: Option<ConnectionError>) {
        assert!(
            matches!(
                self.inner,
                InnerState::Handshake(_) | InnerState::Established | InnerState::Closed { .. }
            ),
            "invalid state transition {:?} -> draining",
            self.as_type()
        );
        let is_local = self.is_local_close();
        self.inner = InnerState::Draining { error, is_local };
        trace!("connection state: draining");
    }

    fn is_local_close(&self) -> bool {
        match self.inner {
            InnerState::Handshake(_) => false,
            InnerState::Established => false,
            InnerState::Closed { is_local, .. } => is_local,
            InnerState::Draining { is_local, .. } => is_local,
            InnerState::Drained { is_local, .. } => is_local,
        }
    }

    /// Enters the Closing connection state, due to changes in the [`Connection`] state.
    ///
    /// This is the closing state from
    /// <https://www.rfc-editor.org/rfc/rfc9000.html#section-10.2.1> due to the local side
    /// having initiated immediate close.
    ///
    /// Crucially, this is to be used when internal state changes result in initiating an
    /// immediate close. The resulting error will be surfaced as a [`ConnectionLost`] event
    /// in [`Connection::poll`].
    ///
    /// # Panics
    ///
    /// Panics if the state is later than established.
    ///
    /// [`ConnectionLost`]: crate::Event::ConnectionLost
    /// [`Connection::poll`]: super::Connection::poll
    pub(super) fn move_to_closed<R: Into<CloseReason>>(&mut self, reason: R) {
        assert!(
            matches!(
                self.inner,
                InnerState::Handshake(_) | InnerState::Established | InnerState::Closed { .. }
            ),
            "invalid state transition {:?} -> closed",
            self.as_type()
        );
        let remote_reason = reason.into();
        let is_local = false;
        trace!(?remote_reason, ?is_local, "connection state: closed");
        self.inner = InnerState::Closed {
            error_read: false,
            remote_reason,
            is_local,
        };
    }

    /// Enters the Closing connection state, initiated by explicit API calls.
    ///
    /// This is the closing state from
    /// <https://www.rfc-editor.org/rfc/rfc9000.html#section-10.2.1> due to the local side
    /// having initiated immediate close.
    ///
    /// Crucially, this is to be used when immediate close is entered due to an API
    /// being called. It means the close will NOT surface as a [`ConnectionLost`] event in
    /// [`Connection::poll`].
    ///
    /// See [`Self::move_to_closed`] for when the internal state changes resulted in
    /// initiating an immediate close.
    ///
    /// # Panics
    ///
    /// Panics if the state is later than established.
    ///
    /// [`ConnectionLost`]: crate::Event::ConnectionLost
    /// [`Connection::poll`]: super::Connection::poll
    pub(super) fn move_to_closed_local<R: Into<CloseReason>>(&mut self, reason: R) {
        assert!(
            matches!(
                self.inner,
                InnerState::Handshake(_) | InnerState::Established | InnerState::Closed { .. }
            ),
            "invalid state transition {:?} -> closed (local)",
            self.as_type()
        );
        let remote_reason = reason.into();
        let is_local = true;
        trace!(?remote_reason, ?is_local, "connection state: closed");
        self.inner = InnerState::Closed {
            error_read: false,
            remote_reason,
            is_local,
        };
    }

    pub(super) fn is_handshake(&self) -> bool {
        matches!(self.inner, InnerState::Handshake(_))
    }

    pub(super) fn is_established(&self) -> bool {
        matches!(self.inner, InnerState::Established)
    }

    pub(super) fn is_closed(&self) -> bool {
        matches!(
            self.inner,
            InnerState::Closed { .. } | InnerState::Draining { .. } | InnerState::Drained { .. }
        )
    }

    pub(super) fn is_drained(&self) -> bool {
        matches!(self.inner, InnerState::Drained { .. })
    }

    pub(super) fn take_error(&mut self) -> Option<ConnectionError> {
        match &mut self.inner {
            InnerState::Draining { error, is_local } => {
                if !*is_local {
                    error.take()
                } else {
                    None
                }
            }
            InnerState::Drained { error, is_local } => {
                if !*is_local {
                    error.take()
                } else {
                    None
                }
            }
            InnerState::Closed {
                remote_reason,
                is_local: local_reason,
                error_read,
            } => {
                if *error_read {
                    None
                } else {
                    *error_read = true;
                    if *local_reason {
                        None
                    } else {
                        Some(remote_reason.clone().into())
                    }
                }
            }
            InnerState::Handshake(_) | InnerState::Established => None,
        }
    }

    pub(super) fn as_type(&self) -> StateType {
        match self.inner {
            InnerState::Handshake(_) => StateType::Handshake,
            InnerState::Established => StateType::Established,
            InnerState::Closed { .. } => StateType::Closed,
            InnerState::Draining { .. } => StateType::Draining,
            InnerState::Drained { .. } => StateType::Drained,
        }
    }
}

/// The state a [`Connection`] can be in.
///
/// [`Connection`]: super::Connection
#[derive(Debug, Clone)]
pub(super) enum StateType {
    /// Before the handshake is *confirmed*.
    Handshake,
    /// Once the handshake is *confirmed*.
    Established,
    /// The connection is closed, waiting for remote to confirm close.
    ///
    /// Specifically <https://www.rfc-editor.org/rfc/rfc9000.html#section-10.2.1>.
    ///
    /// So the side that initiates an immediate close will stay in this state while it is
    /// waiting for the remote to also send a CONNECTION_CLOSE. The side that receives a
    /// connection close will skip straight to [`StateType::Draining`].
    Closed,
    /// The connection is draining, giving time to gracefully discard any in-flight packets.
    ///
    /// Specifically <https://www.rfc-editor.org/rfc/rfc9000.html#section-10.2.2>.
    ///
    /// See [`StateType::Closed`] above for more details.
    Draining,
    /// The connection is drained, waiting for the application to drop us.
    ///
    /// This is a terminal state in which the connection does nothing and can never do
    /// anything again. Waiting for the application to drop the [`Connection`] struct.
    ///
    /// [`Connection`]: super::Connection
    Drained,
}

#[derive(Debug, Clone)]
pub(super) enum CloseReason {
    TransportError(TransportError),
    Connection(ConnectionClose),
    Application(ApplicationClose),
}

impl From<TransportError> for CloseReason {
    fn from(x: TransportError) -> Self {
        Self::TransportError(x)
    }
}
impl From<ConnectionClose> for CloseReason {
    fn from(x: ConnectionClose) -> Self {
        Self::Connection(x)
    }
}
impl From<ApplicationClose> for CloseReason {
    fn from(x: ApplicationClose) -> Self {
        Self::Application(x)
    }
}

impl From<Close> for CloseReason {
    fn from(value: Close) -> Self {
        match value {
            Close::Application(reason) => Self::Application(reason),
            Close::Connection(reason) => Self::Connection(reason),
        }
    }
}

impl From<CloseReason> for ConnectionError {
    fn from(value: CloseReason) -> Self {
        match value {
            CloseReason::TransportError(err) => Self::TransportError(err),
            CloseReason::Connection(reason) => Self::ConnectionClosed(reason),
            CloseReason::Application(reason) => Self::ApplicationClosed(reason),
        }
    }
}

impl From<CloseReason> for Close {
    fn from(value: CloseReason) -> Self {
        match value {
            CloseReason::TransportError(err) => Self::Connection(err.into()),
            CloseReason::Connection(reason) => Self::Connection(reason),
            CloseReason::Application(reason) => Self::Application(reason),
        }
    }
}

#[derive(Debug, Clone)]
enum InnerState {
    /// See [`StateType::Handshake`].
    Handshake(Handshake),
    /// See [`StateType::Established`].
    Established,
    /// See [`StateType::Closed`].
    Closed {
        /// The reason the remote closed the connection, or the reason we are sending to the remote.
        remote_reason: CloseReason,
        /// Set to true if we closed the connection locally.
        is_local: bool,
        /// Did we read this as error already?
        error_read: bool,
    },
    /// See [`StateType::Draining`].
    Draining {
        /// Why the connection was lost, if it has been.
        error: Option<ConnectionError>,
        /// Set to true if we closed the connection locally.
        is_local: bool,
    },
    /// See [`StateType::Drained`].
    /// Waiting for application to call close so we can dispose of the resources.
    Drained {
        /// Why the connection was lost, if it has been.
        error: Option<ConnectionError>,
        /// Set to true if we closed the connection locally.
        is_local: bool,
    },
}

#[allow(unreachable_pub)] // fuzzing only
#[derive(Debug, Clone)]
pub struct Handshake {
    /// Whether the remote CID has been set by the peer yet.
    ///
    /// Always set for servers.
    pub(super) remote_cid_set: bool,
    /// Stateless retry token received in the first Initial by a server.
    ///
    /// Must be present in every Initial. Always empty for clients.
    pub(super) expected_token: Bytes,
    /// First cryptographic message.
    ///
    /// Only set for clients.
    pub(super) client_hello: Option<Bytes>,
    /// Whether the server address is allowed to migrate.
    ///
    /// We allow the server to migrate during the handshake as long as we have not
    /// received an authenticated handshake packet: it can send a response from a
    /// different address than we sent the initial to.  This allows us to send the
    /// initial packet over multiple paths - by means of an IPv6 ULA address that copies
    /// the packets sent to it to multiple destinations - and accept one response.
    ///
    /// This is only ever set to true if for a client which hasn't yet received an
    /// authenticated handshake packet.  It is set back to false in
    /// [`super::Connection::on_packet_authenticated`].
    ///
    /// THIS IS NOT RFC 9000 COMPLIANT!  A server is not allowed to migrate addresses,
    /// other than using the preferred-address transport parameter.
    pub(super) allow_server_migration: bool,
}
