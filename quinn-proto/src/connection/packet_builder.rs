use bytes::{BufMut, Bytes};
use rand::Rng;
use tracing::{debug, trace, trace_span};

use super::{Connection, PathId, SentFrames, TransmitBuf, spaces::SentPacket};
use crate::{
    ConnectionId, FrameStats, Instant, MIN_INITIAL_SIZE, TransportError, TransportErrorCode,
    coding::Encodable,
    connection::{ConnectionSide, qlog::QlogSentPacket, spaces::Retransmits},
    frame::{self, Close, EncodableFrame},
    packet::{FIXED_BIT, Header, InitialHeader, LongType, PacketNumber, PartialEncode, SpaceId},
};

/// QUIC packet builder
///
/// This allows building QUIC packets: it takes care of writing the header, allows writing
/// frames and on [`PacketBuilder::finish`] (or [`PacketBuilder::finish_and_track`]) it
/// encrypts the packet so it is ready to be sent on the wire.
///
/// The builder manages the write buffer into which the packet is written, and directly
/// implements [`BufMut`] to write frames into the packet.
pub(super) struct PacketBuilder<'a, 'b> {
    pub(super) buf: &'a mut TransmitBuf<'b>,
    pub(super) space: SpaceId,
    path: PathId,
    pub(super) partial_encode: PartialEncode,
    pub(super) ack_eliciting: bool,
    pub(super) packet_number: u64,
    /// Is this packet allowed to be coalesced?
    pub(super) can_coalesce: bool,
    /// Smallest absolute position in the associated buffer that must be occupied by this packet's
    /// frames
    pub(super) min_size: usize,
    pub(super) tag_len: usize,
    pub(super) _span: tracing::span::EnteredSpan,
    qlog: QlogSentPacket,
    sent_frames: SentFrames,
}

impl<'a, 'b> PacketBuilder<'a, 'b> {
    /// Write a new packet header to `buffer` and determine the packet's properties
    ///
    /// Marks the connection drained and returns `None` if the confidentiality limit would be
    /// violated.
    pub(super) fn new(
        now: Instant,
        space_id: SpaceId,
        path_id: PathId,
        dst_cid: ConnectionId,
        buffer: &'a mut TransmitBuf<'b>,
        ack_eliciting: bool,
        conn: &mut Connection,
    ) -> Option<Self>
    where
        'b: 'a,
    {
        let mut qlog = QlogSentPacket::default();

        let version = conn.version;
        // Initiate key update if we're approaching the confidentiality limit
        let sent_with_keys = conn.spaces[space_id].sent_with_keys();
        if space_id == SpaceId::Data {
            if sent_with_keys >= conn.key_phase_size {
                debug!("routine key update due to phase exhaustion");
                conn.force_key_update();
            }
        } else {
            let confidentiality_limit = conn.spaces[space_id]
                .crypto
                .as_ref()
                .map_or_else(
                    || &conn.zero_rtt_crypto.as_ref().unwrap().packet,
                    |keys| &keys.packet.local,
                )
                .confidentiality_limit();
            if sent_with_keys.saturating_add(1) == confidentiality_limit {
                // We still have time to attempt a graceful close
                conn.close_inner(
                    now,
                    Close::Connection(frame::ConnectionClose {
                        error_code: TransportErrorCode::AEAD_LIMIT_REACHED,
                        frame_type: frame::MaybeFrame::None,
                        reason: Bytes::from_static(b"confidentiality limit reached"),
                    }),
                )
            } else if sent_with_keys > confidentiality_limit {
                // Confidentiality limited violated and there's nothing we can do
                conn.kill(
                    TransportError::AEAD_LIMIT_REACHED("confidentiality limit reached").into(),
                );
                return None;
            }
        }

        let space = &mut conn.spaces[space_id];
        let packet_number = space.for_path(path_id).get_tx_number(&mut conn.rng);
        let span = trace_span!("send", space = ?space_id, pn = packet_number, %path_id).entered();

        let number = PacketNumber::new(
            packet_number,
            space.for_path(path_id).largest_acked_packet.unwrap_or(0),
        );
        let header = match space_id {
            SpaceId::Data if space.crypto.is_some() => Header::Short {
                dst_cid,
                number,
                spin: if conn.spin_enabled {
                    conn.spin
                } else {
                    conn.rng.random()
                },
                key_phase: conn.key_phase,
            },
            SpaceId::Data => Header::Long {
                ty: LongType::ZeroRtt,
                src_cid: conn.handshake_cid,
                dst_cid,
                number,
                version,
            },
            SpaceId::Handshake => Header::Long {
                ty: LongType::Handshake,
                src_cid: conn.handshake_cid,
                dst_cid,
                number,
                version,
            },
            SpaceId::Initial => Header::Initial(InitialHeader {
                src_cid: conn.handshake_cid,
                dst_cid,
                token: match &conn.side {
                    ConnectionSide::Client { token, .. } => token.clone(),
                    ConnectionSide::Server { .. } => Bytes::new(),
                },
                number,
                version,
            }),
        };

        let partial_encode = header.encode(buffer);
        if conn.peer_params.grease_quic_bit && conn.rng.random() {
            buffer.as_mut_slice()[partial_encode.start] ^= FIXED_BIT;
        }

        let (sample_size, tag_len) = if let Some(ref crypto) = space.crypto {
            (
                crypto.header.local.sample_size(),
                crypto.packet.local.tag_len(),
            )
        } else if space_id == SpaceId::Data {
            let zero_rtt = conn.zero_rtt_crypto.as_ref().unwrap();
            (zero_rtt.header.sample_size(), zero_rtt.packet.tag_len())
        } else {
            unreachable!();
        };

        // Each packet must be large enough for header protection sampling, i.e. the combined
        // lengths of the encoded packet number and protected payload must be at least 4 bytes
        // longer than the sample required for header protection. Further, each packet should be at
        // least tag_len + 6 bytes larger than the destination CID on incoming packets so that the
        // peer may send stateless resets that are indistinguishable from regular traffic.

        // pn_len + payload_len + tag_len >= sample_size + 4
        // payload_len >= sample_size + 4 - pn_len - tag_len
        let min_size = Ord::max(
            buffer.len() + (sample_size + 4).saturating_sub(number.len() + tag_len),
            partial_encode.start + dst_cid.len() + 6,
        );
        let max_size = buffer.datagram_max_offset() - tag_len;
        debug_assert!(max_size >= min_size);

        qlog.header(
            &header,
            Some(packet_number),
            space_id,
            space_id == SpaceId::Data && conn.spaces[SpaceId::Data].crypto.is_none(),
            path_id,
        );

        Some(Self {
            buf: buffer,
            space: space_id,
            path: path_id,
            partial_encode,
            packet_number,
            can_coalesce: header.can_coalesce(),
            min_size,
            tag_len,
            ack_eliciting,
            qlog,
            sent_frames: SentFrames::default(),
            _span: span,
        })
    }

    #[cfg(test)]
    pub(crate) fn simple_data_buf(buf: &'a mut TransmitBuf<'b>) -> Self {
        Self {
            buf,
            space: SpaceId::Data,
            path: PathId::ZERO,
            partial_encode: PartialEncode::no_header(),
            ack_eliciting: true,
            packet_number: 0,
            can_coalesce: true,
            min_size: 0,
            tag_len: 0,
            _span: trace_span!("test").entered(),
            qlog: QlogSentPacket::default(),
            sent_frames: SentFrames::default(),
        }
    }

    /// Append the minimum amount of padding to the packet such that, after encryption, the
    /// enclosing datagram will occupy at least `min_size` bytes
    pub(super) fn pad_to(&mut self, min_size: u16) {
        // The datagram might already have a larger minimum size than the caller is requesting, if
        // e.g. we're coalescing packets and have populated more than `min_size` bytes with packets
        // already.
        self.min_size = Ord::max(
            self.min_size,
            self.buf.datagram_start_offset() + (min_size as usize) - self.tag_len,
        );
    }

    /// Writes a frame into the underlying buffer.
    ///
    /// It will also:
    /// - Track the frame so that it's registered with the path once [`Self::finish_and_track`] is
    ///   called.
    /// - Register the sent frame with the given [`FrameStats`].
    /// - If the qlog feature is enabled, register the frame.
    /// - Log the frame.
    pub(super) fn write_frame<'c>(
        &mut self,
        frame: impl Into<EncodableFrame<'c>>,
        stats: &mut FrameStats,
    ) {
        self.write_frame_with_log_msg(frame, stats, None);
    }

    /// Writes a frame into the underlying buffer.
    ///
    /// It will also:
    /// - Track the frame so that it's registered with the path once [`Self::finish_and_track`] is
    ///   called.
    /// - Register the sent frame with the given [`FrameStats`].
    /// - If the qlog feature is enabled, register the frame.
    /// - Log the frame. If a `msg` is given, this will be added to the log.
    pub(super) fn write_frame_with_log_msg<'c>(
        &mut self,
        frame: impl Into<EncodableFrame<'c>>,
        stats: &mut FrameStats,
        msg: Option<&'static str>,
    ) {
        let frame = frame.into();
        frame.encode(&mut self.frame_space_mut());
        stats.record(frame.get_type());
        self.qlog.record(&frame);
        match msg {
            Some(msg) => trace!(%frame, msg),
            None => trace!(%frame),
        }
        self.sent_frames.record_sent_frame(frame);
    }

    /// Returns a writable buffer limited to the remaining frame space
    ///
    /// The [`BufMut::remaining_mut`] call on the returned buffer indicates the amount of
    /// space available to write QUIC frames into.
    // In rust 1.82 we can use `-> impl BufMut + use<'_, 'a, 'b>`
    pub(super) fn frame_space_mut(&mut self) -> bytes::buf::Limit<&mut TransmitBuf<'b>> {
        self.buf.limit(self.frame_space_remaining())
    }

    pub(super) fn sent_frames(&self) -> &SentFrames {
        &self.sent_frames
    }

    pub(super) fn finish_and_track(
        mut self,
        now: Instant,
        conn: &mut Connection,
        path_id: PathId,
        pad_datagram: PadDatagram,
    ) {
        match pad_datagram {
            PadDatagram::No => (),
            PadDatagram::ToSize(size) => self.pad_to(size),
            PadDatagram::ToSegmentSize => self.pad_to(self.buf.segment_size() as u16),
            PadDatagram::ToMinMtu => self.pad_to(MIN_INITIAL_SIZE),
        }
        let ack_eliciting = self.ack_eliciting;
        let packet_number = self.packet_number;
        let space_id = self.space;
        let (size, padded, sent) = self.finish(conn, now);

        let size = match padded || ack_eliciting {
            true => size as u16,
            false => 0,
        };

        let packet = SentPacket {
            path_generation: conn.paths.get_mut(&path_id).unwrap().data.generation(),
            largest_acked: sent.largest_acked,
            time_sent: now,
            size,
            ack_eliciting,
            retransmits: sent.retransmits,
            stream_frames: sent.stream_frames,
        };

        conn.paths.get_mut(&path_id).unwrap().data.sent(
            packet_number,
            packet,
            conn.spaces[space_id].for_path(path_id),
        );
        conn.reset_keep_alive(path_id, now);
        if size != 0 {
            if ack_eliciting {
                conn.spaces[space_id]
                    .for_path(path_id)
                    .time_of_last_ack_eliciting_packet = Some(now);
                if conn.permit_idle_reset {
                    conn.reset_idle_timeout(now, space_id, path_id);
                }
                conn.permit_idle_reset = false;
            }
            conn.set_loss_detection_timer(now, path_id);
            conn.path_data_mut(path_id).pacing.on_transmit(size);
        }
    }

    /// Encrypt packet, returning the length of the packet and whether padding was added
    pub(super) fn finish(
        mut self,
        conn: &mut Connection,
        now: Instant,
    ) -> (usize, bool, SentFrames) {
        debug_assert!(
            self.buf.len() <= self.buf.datagram_max_offset() - self.tag_len,
            "packet exceeds maximum size"
        );
        let pad = self.buf.len() < self.min_size;
        if pad {
            let padding = self.min_size - self.buf.len();
            trace!("PADDING * {}", padding);
            self.buf.put_bytes(0, padding);
            self.qlog.frame_padding(padding);
        }

        let space = &conn.spaces[self.space];
        let (header_crypto, packet_crypto) = if let Some(ref crypto) = space.crypto {
            (&*crypto.header.local, &*crypto.packet.local)
        } else if self.space == SpaceId::Data {
            let zero_rtt = conn.zero_rtt_crypto.as_ref().unwrap();
            (&*zero_rtt.header, &*zero_rtt.packet)
        } else {
            unreachable!("tried to send {:?} packet without keys", self.space);
        };

        debug_assert_eq!(
            packet_crypto.tag_len(),
            self.tag_len,
            "Mismatching crypto tag len"
        );

        self.buf.put_bytes(0, packet_crypto.tag_len());
        let encode_start = self.partial_encode.start;
        let packet_buf = &mut self.buf.as_mut_slice()[encode_start..];
        // for packet protection, PathId::ZERO and no path are equivalent.
        self.partial_encode.finish(
            packet_buf,
            header_crypto,
            Some((self.packet_number, self.path, packet_crypto)),
        );

        let packet_len = self.buf.len() - encode_start;
        trace!(size = %packet_len, "wrote packet");
        self.qlog.finalize(packet_len);
        conn.qlog.emit_packet_sent(self.qlog, now);
        (packet_len, pad, self.sent_frames)
    }

    /// The number of additional bytes the current packet would take up if it was finished now
    ///
    /// This will include any padding which is required to make the size large enough to be
    /// encrypted correctly.
    pub(super) fn predict_packet_end(&self) -> usize {
        self.buf.len().max(self.min_size) + self.tag_len - self.buf.len()
    }

    /// Returns the remaining space in the packet that can be taken up by QUIC frames
    ///
    /// This leaves space in the datagram for the cryptographic tag that needs to be written
    /// when the packet is finished.
    pub(super) fn frame_space_remaining(&self) -> usize {
        let max_offset = self.buf.datagram_max_offset() - self.tag_len;
        max_offset.saturating_sub(self.buf.len())
    }

    pub(crate) fn require_padding(&mut self) {
        self.sent_frames.requires_padding = true;
    }

    pub(crate) fn retransmits_mut(&mut self) -> &mut Retransmits {
        self.sent_frames.retransmits_mut()
    }
}

#[derive(Debug, Copy, Clone)]
pub(super) enum PadDatagram {
    /// Do not pad the datagram
    No,
    /// To a specific size
    ToSize(u16),
    /// Pad to the current MTU/segment size
    ///
    /// For the first datagram in a transmit the MTU is the same as the
    /// [`TransmitBuf::segment_size`].
    ToSegmentSize,
    /// Pad to [`MIN_INITIAL_SIZE`], the minimal QUIC MTU of 1200 bytes
    ToMinMtu,
}

impl std::ops::BitOrAssign for PadDatagram {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl std::ops::BitOr for PadDatagram {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::No, rhs) => rhs,
            (Self::ToSize(size), Self::No) => Self::ToSize(size),
            (Self::ToSize(a), Self::ToSize(b)) => Self::ToSize(a.max(b)),
            (Self::ToSize(_), Self::ToSegmentSize) => Self::ToSegmentSize,
            (Self::ToSize(_), Self::ToMinMtu) => Self::ToMinMtu,
            (Self::ToSegmentSize, Self::No) => Self::ToSegmentSize,
            (Self::ToSegmentSize, Self::ToSize(_)) => Self::ToSegmentSize,
            (Self::ToSegmentSize, Self::ToSegmentSize) => Self::ToSegmentSize,
            (Self::ToSegmentSize, Self::ToMinMtu) => Self::ToMinMtu,
            (Self::ToMinMtu, _) => Self::ToMinMtu,
        }
    }
}
