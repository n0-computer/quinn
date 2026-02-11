use std::mem;
use std::ops::{Index, IndexMut};

use tracing::{debug, trace};

use crate::connection::assembler::Assembler;
use crate::crypto::{self, HeaderKey, KeyPair, Keys, PacketKey};
use crate::packet::{Packet, PartialDecode, SpaceId};
use crate::token::ResetToken;
use rand::Rng;

use crate::{ConnectionId, Instant, Side};
use crate::{RESET_TOKEN_SIZE, TransportError};

use super::PathId;
use super::spaces::PacketSpace;

/// Perform key updates this many packets before the AEAD confidentiality limit.
///
/// Chosen arbitrarily, intended to be large enough to prevent spurious connection loss.
const KEY_UPDATE_MARGIN: u64 = 10_000;

pub(super) struct UnprotectHeaderResult {
    /// The packet with the now unprotected header (`None` in the case of stateless reset packets
    /// that fail to be decoded)
    pub(super) packet: Option<Packet>,
    /// Whether the packet was a stateless reset packet
    pub(super) stateless_reset: bool,
}

pub(super) struct DecryptPacketResult {
    /// The packet number
    pub(super) number: u64,
    /// Whether a locally initiated key update has been acknowledged by the peer
    pub(super) outgoing_key_update_acked: bool,
    /// Whether the peer has initiated a key update
    pub(super) incoming_key_update: bool,
}

pub(super) struct PrevCrypto {
    /// The keys used for the previous key phase, temporarily retained to decrypt packets sent by
    /// the peer prior to its own key update.
    pub(super) crypto: KeyPair<Box<dyn PacketKey>>,
    /// The incoming packet that ends the interval for which these keys are applicable, and the time
    /// of its receipt.
    ///
    /// Incoming packets should be decrypted using these keys iff this is `None` or their packet
    /// number is lower. `None` indicates that we have not yet received a packet using newer keys,
    /// which implies that the update was locally initiated.
    pub(super) end_packet: Option<(u64, Instant)>,
    /// Whether the following key phase is from a remotely initiated update that we haven't acked
    pub(super) update_unacked: bool,
}

pub(super) struct ZeroRttCrypto {
    pub(super) header: Box<dyn HeaderKey>,
    pub(super) packet: Box<dyn PacketKey>,
}

impl ZeroRttCrypto {
    fn keys(&self) -> (&dyn HeaderKey, &dyn PacketKey) {
        (self.header.as_ref(), self.packet.as_ref())
    }
}

/// Consolidated crypto state for a connection.
///
/// This struct groups all cryptographic state together, including:
/// - The TLS session
/// - Per-space keys and crypto streams
/// - Key update state (prev/next keys)
/// - 0-RTT state
pub(super) struct CryptoState {
    /// Per encryption level crypto data (Initial, Handshake, Data).
    pub(super) spaces: [CryptoSpace; 3],
    /// The TLS session.
    pub(super) session: Box<dyn crypto::Session>,

    /*
     * 0-RTT related fields
     */
    /// Whether 0-RTT was accepted.
    pub(super) accepted_0rtt: bool,
    /// Whether or not 0-RTT was enabled during the handshake. Does not imply acceptance.
    pub(super) zero_rtt_enabled: bool,
    /// 0-RTT crypto state, cleared when no longer needed.
    pub(super) zero_rtt_crypto: Option<ZeroRttCrypto>,

    /*
     * State to manage 1-RTT key updates
     */
    /// 1-RTT keys to be used for the next key update.
    ///
    /// These are generated in advance to prevent timing attacks and/or DoS by third-party
    /// attackers spoofing key updates.
    pub(super) next_crypto: Option<KeyPair<Box<dyn PacketKey>>>,
    /// 1-RTT keys used prior to a key update.
    pub(super) prev_crypto: Option<PrevCrypto>,
    /// Current key phase, toggled on each 1-RTT key update.
    pub(super) key_phase: bool,
    /// How many packets are in the current key phase. Used only for `Data` space.
    pub(super) key_phase_size: u64,
}

impl CryptoState {
    pub(super) fn new(
        session: Box<dyn crypto::Session>,
        init_cid: ConnectionId,
        side: Side,
        rng: &mut impl Rng,
    ) -> Self {
        let initial_keys = session.initial_keys(init_cid, side);
        let initial_space = CryptoSpace {
            keys: Some(initial_keys),
            ..Default::default()
        };
        Self {
            spaces: [initial_space, Default::default(), Default::default()],
            session,
            next_crypto: None,
            prev_crypto: None,
            accepted_0rtt: false,
            zero_rtt_enabled: false,
            zero_rtt_crypto: None,
            key_phase: false,
            // A small initial key phase size ensures peers that don't handle key updates correctly
            // fail sooner rather than later. It's okay for both peers to do this, as the first one
            // to perform an update will reset the other's key phase size in `update_keys`, and a
            // simultaneous key update by both is just like a regular key update with a really fast
            // response. Inspired by quic-go's similar behavior of performing the first key update
            // at the 100th short-header packet.
            key_phase_size: rng.random_range(10..1000),
        }
    }

    /// Removes header protection of a packet, or returns `None` if the packet was dropped.
    pub(super) fn unprotect_header(
        &self,
        partial_decode: PartialDecode,
        stateless_reset_token: Option<ResetToken>,
    ) -> Option<UnprotectHeaderResult> {
        let encryption_level = partial_decode.encryption_level();
        let header_crypto = match encryption_level {
            Some(level) => match self.remote_crypto(level) {
                Some(crypto) => Some(crypto.0),
                None => {
                    let bytes = partial_decode.len();
                    debug!(?encryption_level, bytes, "dropping unexpected packet");
                    return None;
                }
            },
            // Unprotected packet
            None => None,
        };

        let packet = partial_decode.data();
        let stateless_reset = packet.len() >= RESET_TOKEN_SIZE + 5
            && stateless_reset_token.as_deref() == Some(&packet[packet.len() - RESET_TOKEN_SIZE..]);

        match partial_decode.finish(header_crypto) {
            Ok(packet) => Some(UnprotectHeaderResult {
                packet: Some(packet),
                stateless_reset,
            }),
            Err(_) if stateless_reset => Some(UnprotectHeaderResult {
                packet: None,
                stateless_reset: true,
            }),
            Err(e) => {
                trace!("unable to complete packet decoding: {}", e);
                None
            }
        }
    }

    /// Decrypts a packet's body in-place.
    pub(super) fn decrypt_packet_body(
        &self,
        packet: &mut Packet,
        path_id: PathId,
        spaces: &[PacketSpace; 3],
    ) -> Result<Option<DecryptPacketResult>, Option<TransportError>> {
        let conn_key_phase = self.key_phase;
        if !packet.header.is_protected() {
            // Unprotected packets also don't have packet numbers
            return Ok(None);
        }
        let space = packet.header.space();

        if path_id != PathId::ZERO && space != SpaceId::Data {
            // do not try to decrypt illegal multipath packets
            return Err(Some(TransportError::PROTOCOL_VIOLATION(
                "multipath packet on non Data packet number space",
            )));
        }
        // Packets that do not belong to known path ids are valid as long as they can be decrypted.
        // If we didn't have a path, that's for the purposes of this function equivalent to not
        // having received packets on that path yet. So both of these cases are represented by `None`.
        let rx_packet = spaces[space as usize]
            .path_space(path_id)
            .and_then(|s| s.rx_packet);
        let number = packet
            .header
            .number()
            .ok_or(None)?
            .expand(rx_packet.map(|n| n + 1).unwrap_or_default());
        let packet_key_phase = packet.header.key_phase();

        let mut crypto_update = false;
        let crypto = if packet.header.is_0rtt() {
            let (_, packet) = self.remote_crypto(EncryptionLevel::ZeroRtt).unwrap();
            packet
        } else if packet_key_phase == conn_key_phase || space != SpaceId::Data {
            let (_, packet) = self.remote_crypto(space.encryption_level()).unwrap();
            packet
        } else if let Some(prev) = self.prev_crypto.as_ref().and_then(|crypto| {
            // If this packet comes prior to acknowledgment of the key update by the peer,
            if crypto.end_packet.is_none_or(|(pn, _)| number < pn) {
                // use the previous keys.
                Some(crypto)
            } else {
                // Otherwise, this must be a remotely-initiated key update, so fall through to the
                // final case.
                None
            }
        }) {
            &*prev.crypto.remote
        } else {
            // We're in the Data space with a key phase mismatch and either there is no locally
            // initiated key update or the locally initiated key update was acknowledged by a
            // lower-numbered packet. The key phase mismatch must therefore represent a new
            // remotely-initiated key update.
            crypto_update = true;
            &*self.next_crypto.as_ref().unwrap().remote
        };

        crypto
            .decrypt(path_id, number, &packet.header_data, &mut packet.payload)
            .map_err(|_| {
                trace!("decryption failed with packet number {}", number);
                None
            })?;

        if !packet.reserved_bits_valid() {
            return Err(Some(TransportError::PROTOCOL_VIOLATION(
                "reserved bits set",
            )));
        }

        let mut outgoing_key_update_acked = false;
        if let Some(ref prev) = self.prev_crypto
            && prev.end_packet.is_none()
            && packet_key_phase == conn_key_phase
        {
            outgoing_key_update_acked = true;
        }

        if crypto_update {
            // Validate incoming key update
            // If `rx_packet` is `None`, then either the path is entirely new, or we haven't received
            // any packets on this path yet. In that case, having the first packet be a crypto update
            // is fine.
            let invalid_packet_number = rx_packet.is_some_and(|rx_packet| number <= rx_packet);
            if invalid_packet_number || self.prev_crypto.as_ref().is_some_and(|x| x.update_unacked)
            {
                trace!(?number, ?rx_packet, %path_id, "crypto update failed");
                return Err(Some(TransportError::KEY_UPDATE_ERROR("")));
            }
        }

        Ok(Some(DecryptPacketResult {
            number,
            outgoing_key_update_acked,
            incoming_key_update: crypto_update,
        }))
    }

    /// Check if keys are available for the given encryption level.
    pub(super) fn has_keys(&self, level: EncryptionLevel) -> bool {
        match level {
            EncryptionLevel::Initial => self.spaces[0].keys.is_some(),
            EncryptionLevel::ZeroRtt => self.zero_rtt_crypto.is_some(),
            EncryptionLevel::Handshake => self.spaces[1].keys.is_some(),
            EncryptionLevel::OneRtt => self.spaces[2].keys.is_some(),
        }
    }

    /// Discard temporary key state (0-RTT and previous keys).
    pub(super) fn discard_temporary_keys(&mut self) {
        self.zero_rtt_crypto = None;
        self.prev_crypto = None;
    }

    /// Enable 0-RTT crypto with the given keys.
    pub(super) fn enable_zero_rtt(
        &mut self,
        header: Box<dyn HeaderKey>,
        packet: Box<dyn PacketKey>,
    ) {
        self.zero_rtt_enabled = true;
        self.zero_rtt_crypto = Some(ZeroRttCrypto { header, packet });
    }

    /// Discard 0-RTT crypto keys.
    pub(super) fn discard_zero_rtt(&mut self) {
        self.zero_rtt_crypto = None;
    }

    /// Get the integrity limit for the given space's local packet keys.
    pub(super) fn integrity_limit(&self, space: SpaceKind) -> Option<u64> {
        let keys = self.spaces[space].keys.as_ref()?;
        Some(keys.packet.local.integrity_limit())
    }

    /// Get local (sending) crypto keys for the given space.
    ///
    /// Returns header and packet keys used for encrypting outgoing packets. For `SpaceKind::Data`,
    /// returns 1-RTT keys if available, otherwise 0-RTT keys.
    pub(super) fn local_crypto(
        &self,
        space: SpaceKind,
    ) -> Option<(&dyn HeaderKey, &dyn PacketKey)> {
        let keys = self.spaces[space].keys.as_ref().map(Keys::local);
        if keys.is_none() && space == SpaceKind::Data {
            return self.zero_rtt_crypto.as_ref().map(ZeroRttCrypto::keys);
        }

        keys
    }

    /// Get remote (receiving) crypto keys for the given encryption level.
    ///
    /// Returns header and packet keys used for decrypting incoming packets.
    pub(super) fn remote_crypto(
        &self,
        level: EncryptionLevel,
    ) -> Option<(&dyn HeaderKey, &dyn PacketKey)> {
        match level {
            EncryptionLevel::Initial => self.spaces[0].keys.as_ref().map(Keys::remote),
            EncryptionLevel::Handshake => self.spaces[1].keys.as_ref().map(Keys::remote),
            EncryptionLevel::OneRtt => self.spaces[2].keys.as_ref().map(Keys::remote),
            // 0-RTT uses the same keys for both directions
            EncryptionLevel::ZeroRtt => self.zero_rtt_crypto.as_ref().map(ZeroRttCrypto::keys),
        }
    }

    /// Perform a 1-RTT key update.
    ///
    /// Generates the next set of keys, rotates current keys into previous, and installs the new
    /// keys. Updates `key_phase` and `key_phase_size` accordingly.
    ///
    /// PANICS: If 1rtt keys are missing.
    pub(super) fn update_keys(&mut self, end_packet: Option<(u64, Instant)>, remote: bool) {
        let new = self
            .session
            .next_1rtt_keys()
            .expect("only called for `Data` packets");
        let confidentiality_limit = new.local.confidentiality_limit();
        let old = mem::replace(
            &mut self.spaces[SpaceKind::Data]
                .keys
                .as_mut()
                .unwrap() // safe because update_keys() can only be triggered by short packets
                .packet,
            mem::replace(self.next_crypto.as_mut().unwrap(), new),
        );
        self.prev_crypto = Some(PrevCrypto {
            crypto: old,
            end_packet,
            update_unacked: remote,
        });

        self.key_phase_size = confidentiality_limit.saturating_sub(KEY_UPDATE_MARGIN);
        self.key_phase = !self.key_phase;
    }
}

/// Per space kind cryptographic state.
#[derive(Default)]
pub(super) struct CryptoSpace {
    /// Packet protection keys for this space.
    pub(super) keys: Option<Keys>,
    /// Incoming cryptographic handshake stream.
    pub(super) crypto_stream: Assembler,
    /// Current offset of outgoing cryptographic handshake stream.
    pub(super) crypto_offset: u64,
    /// Number of packets encrypted with the current set of keys.
    pub(super) sent_with_keys: u64,
}

/// QUIC packet protection levels (RFC 9001).
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) enum EncryptionLevel {
    /// Initial packets (client and server).
    Initial,
    /// Early data (0-RTT), client only.
    ZeroRtt,
    /// Handshake packets.
    Handshake,
    /// Application data (1-RTT).
    OneRtt,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) enum SpaceKind {
    /// Initial packets (client and server).
    Initial = 0,
    /// Handshake packets.
    Handshake = 1,
    /// Data (1-RTT and 0-RTT)
    Data = 2,
}

impl From<SpaceKind> for crate::packet::SpaceId {
    fn from(kind: SpaceKind) -> Self {
        match kind {
            SpaceKind::Initial => Self::Initial,
            SpaceKind::Handshake => Self::Handshake,
            SpaceKind::Data => Self::Data,
        }
    }
}

impl IndexMut<SpaceKind> for [CryptoSpace; 3] {
    fn index_mut(&mut self, index: SpaceKind) -> &mut Self::Output {
        &mut self[index as usize]
    }
}

impl Index<SpaceKind> for [CryptoSpace; 3] {
    type Output = CryptoSpace;

    fn index(&self, index: SpaceKind) -> &Self::Output {
        &self[index as usize]
    }
}
