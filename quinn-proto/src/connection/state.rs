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

    pub(super) fn as_closed(&self) -> Option<&Closed> {
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

    pub(super) fn to_handshake(&mut self, hs: Handshake) {
        self.inner = InnerState::Handshake(hs);
    }

    pub(super) fn to_established(&mut self) {
        self.inner = InnerState::Established;
    }

    pub(super) fn to_drained(&mut self, error: Option<ConnectionError>) {
        let error = if let Some(error) = error {
            Some(error)
        } else {
            match &mut self.inner {
                InnerState::Draining { error } => error.take(),
                InnerState::Drained { error } => error.take(),
                InnerState::Closed {
                    remote_reason,
                    local_reason,
                    error_read,
                } => {
                    if *error_read {
                        None
                    } else {
                        *error_read = true;
                        if *local_reason {
                            Some(ConnectionError::LocallyClosed)
                        } else {
                            let error = match remote_reason.clone().reason.into() {
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
                }
                InnerState::Handshake(_) | InnerState::Established => None,
            }
        };
        self.inner = InnerState::Drained { error };
    }

    pub(super) fn to_draining(&mut self, error: ConnectionError) {
        self.inner = InnerState::Draining { error: Some(error) };
    }

    pub(super) fn to_draining_clean(&mut self) {
        let mut error = None;
        if let InnerState::Closed { local_reason, .. } = &self.inner {
            if *local_reason {
                error.replace(ConnectionError::LocallyClosed);
            }
        }
        self.inner = InnerState::Draining { error };
    }

    pub(super) fn to_closed<R: Into<Close>>(&mut self, reason: R) {
        self.inner = InnerState::Closed {
            error_read: false,
            remote_reason: Closed {
                reason: reason.into(),
            },
            local_reason: false,
        };
    }

    pub(super) fn to_closed_locally<R: Into<Close>>(&mut self, reason: R) {
        self.inner = InnerState::Closed {
            error_read: false,
            remote_reason: Closed {
                reason: reason.into(),
            },
            local_reason: true,
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
            InnerState::Draining { error } => error.take(),
            InnerState::Drained { error } => error.take(),
            InnerState::Closed {
                remote_reason,
                local_reason,
                error_read,
            } => {
                if *error_read {
                    None
                } else {
                    *error_read = true;
                    if *local_reason {
                        Some(ConnectionError::LocallyClosed)
                    } else {
                        Some(remote_reason.clone().reason.into())
                    }
                }
            }
            InnerState::Handshake(_) | InnerState::Established => None,
        }
    }

    pub(super) fn as_typ(&self) -> StateTyp {
        match self.inner {
            InnerState::Handshake(_) => StateTyp::Handshake,
            InnerState::Established => StateTyp::Established,
            InnerState::Closed { .. } => StateTyp::Closed,
            InnerState::Draining { .. } => StateTyp::Draining,
            InnerState::Drained { .. } => StateTyp::Drained,
        }
    }
}

#[allow(unreachable_pub)] // fuzzing only
#[derive(Debug, Clone)]
pub enum StateTyp {
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
    Closed {
        remote_reason: Closed,
        local_reason: bool,
        error_read: bool,
    },
    Draining {
        /// Why the connection was lost, if it has been
        error: Option<ConnectionError>,
    },
    /// Waiting for application to call close so we can dispose of the resources
    Drained {
        /// Why the connection was lost, if it has been
        error: Option<ConnectionError>,
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

#[allow(unreachable_pub)] // fuzzing only
#[derive(Debug, Clone)]
pub struct Closed {
    pub(super) reason: Close,
}
