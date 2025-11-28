use bytes::Bytes;

use crate::frame::Close;
use crate::{ConnectionError, TransportErrorCode};

#[allow(unreachable_pub)] // fuzzing only
#[derive(Debug, Clone)]
pub struct State {
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

    pub(super) fn as_closed(&self) -> Option<&Close> {
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
    #[cfg(test)]
    pub fn established() -> Self {
        Self {
            inner: InnerState::Established,
        }
    }

    pub(super) fn handshake(handshake: Handshake) -> Self {
        Self {
            inner: InnerState::Handshake(handshake),
        }
    }

    pub(super) fn move_to_handshake(&mut self, hs: Handshake) {
        self.inner = InnerState::Handshake(hs);
    }

    pub(super) fn move_to_established(&mut self) {
        self.inner = InnerState::Established;
    }

    pub(super) fn move_to_drained(&mut self, error: Option<ConnectionError>) {
        dbg!(&self, &error);
        let (error, is_local) = if let Some(error) = error {
            (Some(error), false)
        } else {
            let error = match &mut self.inner {
                InnerState::Draining { error, .. } => error.take(),
                InnerState::Drained { .. } => panic!("invalid state transition drained -> drained"),
                InnerState::Closed {
                    remote_reason,
                    error_read,
                    ..
                } => {
                    if *error_read {
                        None
                    } else {
                        *error_read = true;
                        let error = match remote_reason.clone().into() {
                            ConnectionError::ConnectionClosed(close) => {
                                if close.error_code == TransportErrorCode::PROTOCOL_VIOLATION {
                                    ConnectionError::TransportError(close.error_code.into())
                                } else {
                                    ConnectionError::ConnectionClosed(close)
                                }
                            }
                            e => e,
                        };
                        Some(error)
                    }
                }
                InnerState::Handshake(_) | InnerState::Established => None,
            };
            (error, self.is_local_close())
        };
        self.inner = InnerState::Drained { error, is_local };
    }

    pub(super) fn moved_to_draining(&mut self, error: Option<ConnectionError>) {
        dbg!(&self, &error);
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

    pub(super) fn move_to_closed<R: Into<Close>>(&mut self, reason: R) {
        assert!(
            matches!(
                self.inner,
                InnerState::Handshake(_) | InnerState::Established
            ),
            "invalid state transition {:?} -> closed",
            self.as_type()
        );
        self.inner = InnerState::Closed {
            error_read: false,
            remote_reason: reason.into(),
            is_local: false,
        };
    }

    pub(super) fn move_to_closed_local<R: Into<Close>>(&mut self, reason: R) {
        assert!(
            matches!(
                self.inner,
                InnerState::Handshake(_) | InnerState::Established
            ),
            "invalid state transition {:?} -> closed (local)",
            self.as_type()
        );
        self.inner = InnerState::Closed {
            error_read: false,
            remote_reason: reason.into(),
            is_local: true,
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

#[allow(unreachable_pub)] // fuzzing only
#[derive(Debug, Clone)]
pub enum StateType {
    Handshake,
    Established,
    Closed,
    Draining,
    Drained,
}

#[derive(Debug, Clone)]
enum InnerState {
    Handshake(Handshake),
    Established,
    // TODO: should this be split into `ClosedLocal` and `ClosedRemote`?
    Closed {
        /// The reason the remote closed the connection, or the reason we are sending to the remote.
        remote_reason: Close,
        /// Set to true if we closed the connection locally
        is_local: bool,
        /// Did we read this as error already?
        error_read: bool,
    },
    Draining {
        /// Why the connection was lost, if it has been
        error: Option<ConnectionError>,
        is_local: bool,
    },
    /// Waiting for application to call close so we can dispose of the resources
    Drained {
        /// Why the connection was lost, if it has been
        error: Option<ConnectionError>,
        is_local: bool,
    },
}

#[allow(unreachable_pub)] // fuzzing only
#[derive(Debug, Clone)]
pub struct Handshake {
    /// Whether the remote CID has been set by the peer yet
    ///
    /// Always set for servers
    pub(super) rem_cid_set: bool,
    /// Stateless retry token received in the first Initial by a server.
    ///
    /// Must be present in every Initial. Always empty for clients.
    pub(super) expected_token: Bytes,
    /// First cryptographic message
    ///
    /// Only set for clients
    pub(super) client_hello: Option<Bytes>,
    /// Whether the server address is allowed to migrate
    ///
    /// We allow the server to migrate during the handshake as long as we have not
    /// received an authenticated handshake packet: it can send a response from a
    /// different address than we sent the initial to.  This allows us to send the
    /// initial packet over multiple paths - by means of an IPv6 ULA address that copies
    /// the packets sent to it to multiple destinations - and accept one response.
    ///
    /// This is only ever set to true if for a client which hasn't yet received an
    /// authenticated handshake packet.  It is set back to false in
    /// [`Connection::on_packet_authenticated`].
    ///
    /// THIS IS NOT RFC 9000 COMPLIANT!  A server is not allowed to migrate addresses,
    /// other than using the preferred-address transport parameter.
    pub(super) allow_server_migration: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// This makes sure that the assumption of error set if drained holds up.
    #[test]
    fn test_always_error_if_drained() {
        let mut state = State {
            inner: InnerState::Draining {
                error: Some(ConnectionError::Reset),
                is_local: true,
            },
        };
        state.move_to_drained(None);
        assert_eq!(state.take_error(), Some(ConnectionError::Reset));
    }
}
