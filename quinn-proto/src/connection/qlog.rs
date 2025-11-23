// Function bodies in this module are regularly cfg'd out
#![allow(unused_variables)]

#[cfg(feature = "qlog")]
use qlog::{
    events::{
        Event, EventData,
        quic::{
            PacketHeader, PacketLost, PacketLostTrigger, PacketReceived, PacketSent, PacketType,
            QuicFrame, StreamType,
        },
    },
    streamer::QlogStreamer,
};
use std::net::{IpAddr, SocketAddr};
#[cfg(feature = "qlog")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "qlog")]
use tracing::warn;

use crate::{
    ConnectionId, Instant,
    connection::{PathData, SentPacket},
    packet::SpaceId,
};
#[cfg(feature = "qlog")]
use crate::{
    FrameType,
    packet::{Header, Packet},
};

/// Shareable handle to a single qlog output stream
#[cfg(feature = "qlog")]
#[derive(Clone)]
pub struct QlogStream(pub(crate) Arc<Mutex<QlogStreamer>>);

#[cfg(feature = "qlog")]
impl QlogStream {
    fn emit_event(&self, orig_rem_cid: ConnectionId, event: EventData, now: Instant) {
        // Time will be overwritten by `add_event_with_instant`
        let mut event = Event::with_time(0.0, event);
        event.group_id = Some(orig_rem_cid.to_string());

        let mut qlog_streamer = self.0.lock().unwrap();
        if let Err(e) = qlog_streamer.add_event_with_instant(event, now) {
            warn!("could not emit qlog event: {e}");
        }
    }
}

/// A [`QlogStream`] that may be either dynamically disabled or compiled out entirely
#[derive(Clone, Default)]
pub(crate) struct QlogSink {
    #[cfg(feature = "qlog")]
    stream: Option<QlogStream>,
}

impl QlogSink {
    pub(crate) fn is_enabled(&self) -> bool {
        #[cfg(feature = "qlog")]
        {
            self.stream.is_some()
        }
        #[cfg(not(feature = "qlog"))]
        {
            false
        }
    }

    pub(super) fn emit_connection_started(
        &self,
        now: Instant,
        loc_cid: ConnectionId,
        rem_cid: ConnectionId,
        remote: SocketAddr,
        local_ip: Option<IpAddr>,
    ) {
        #[cfg(feature = "qlog")]
        {
            use qlog::events::connectivity::ConnectionStarted;

            let Some(stream) = self.stream.as_ref() else {
                return;
            };
            // TODO: Review fields. The standard has changed since.
            stream.emit_event(
                rem_cid,
                EventData::ConnectionStarted(ConnectionStarted {
                    ip_version: Some(String::from(match remote.ip() {
                        IpAddr::V4(_) => "v4",
                        IpAddr::V6(_) => "v6",
                    })),
                    src_ip: local_ip.map(|addr| addr.to_string()).unwrap_or_default(),
                    dst_ip: remote.ip().to_string(),
                    protocol: None,
                    src_port: None,
                    dst_port: Some(remote.port()),
                    src_cid: Some(loc_cid.to_string()),
                    dst_cid: Some(rem_cid.to_string()),
                }),
                now,
            );
        }
    }

    pub(super) fn emit_recovery_metrics(
        &self,
        pto_count: u32,
        path: &mut PathData,
        now: Instant,
        orig_rem_cid: ConnectionId,
    ) {
        #[cfg(feature = "qlog")]
        {
            let Some(stream) = self.stream.as_ref() else {
                return;
            };

            let Some(metrics) = path.qlog_recovery_metrics(pto_count) else {
                return;
            };

            stream.emit_event(orig_rem_cid, EventData::MetricsUpdated(metrics), now);
        }
    }

    pub(super) fn emit_packet_lost(
        &self,
        pn: u64,
        info: &SentPacket,
        lost_send_time: Instant,
        space: SpaceId,
        now: Instant,
        orig_rem_cid: ConnectionId,
    ) {
        #[cfg(feature = "qlog")]
        {
            let Some(stream) = self.stream.as_ref() else {
                return;
            };

            let event = PacketLost {
                header: Some(PacketHeader {
                    packet_number: Some(pn),
                    packet_type: packet_type(space, false),
                    length: Some(info.size),
                    ..Default::default()
                }),
                frames: None,
                trigger: Some(match info.time_sent <= lost_send_time {
                    true => PacketLostTrigger::TimeThreshold,
                    false => PacketLostTrigger::ReorderingThreshold,
                }),
            };

            stream.emit_event(orig_rem_cid, EventData::PacketLost(event), now);
        }
    }

    pub(super) fn emit_packet_sent(
        &self,
        orig_rem_cid: ConnectionId,
        packet: QlogPacket,
        now: Instant,
    ) {
        #[cfg(feature = "qlog")]
        {
            let Some(stream) = self.stream.as_ref() else {
                return;
            };
            stream.emit_event(orig_rem_cid, EventData::PacketSent(packet.inner), now);
        }
    }

    pub(super) fn emit_packet_received(
        &self,
        data: QlogRecvPacket,
        now: Instant,
        orig_rem_cid: ConnectionId,
    ) {
        #[cfg(feature = "qlog")]
        {
            let Some(stream) = self.stream.as_ref() else {
                return;
            };

            let event = data.inner;
            stream.emit_event(orig_rem_cid, EventData::PacketReceived(event), now);
        }
    }
}

#[derive(Default)]
pub(crate) struct QlogPacket {
    #[cfg(feature = "qlog")]
    inner: PacketSent,
}

#[cfg(feature = "qlog")]
impl QlogPacket {
    pub(crate) fn header(
        &mut self,
        header: &Header,
        pn: Option<u64>,
        space: SpaceId,
        is_0rtt: bool,
    ) {
        self.inner.header.scid = header.src_cid().map(encode_cid);
        self.inner.header.dcid = Some(encode_cid(header.dst_cid()));
        self.inner.header.packet_number = pn;
        self.inner.header.packet_type = packet_type(space, is_0rtt);
    }

    pub(crate) fn frame(&mut self, frame: QuicFrame) {
        self.inner.frames.get_or_insert_default().push(frame);
    }

    pub(crate) fn unknown_frame(&mut self, frame: &FrameType) {
        let ty = frame.to_u64();
        self.frame(unknown(frame.to_u64(), None))
    }

    pub(super) fn finalize(&mut self, len: usize) {
        self.inner.header.length = Some(len as u16);
    }
}

#[derive(Default)]
pub(crate) struct QlogRecvPacket {
    #[cfg(feature = "qlog")]
    inner: PacketReceived,
}

#[cfg(feature = "qlog")]
impl QlogRecvPacket {
    pub(crate) fn header(&mut self, packet: &Packet, pn: Option<u64>) {
        let header = &packet.header;
        let len = packet.header_data.len() + packet.payload.len();
        let is_0rtt = !packet.header.is_1rtt();
        self.inner.header.scid = header.src_cid().map(encode_cid);
        self.inner.header.dcid = Some(encode_cid(header.dst_cid()));
        self.inner.header.packet_number = pn;
        self.inner.header.packet_type = packet_type(header.space(), is_0rtt);
        self.inner.header.length = Some(len as u16)
    }

    pub(crate) fn frame(&mut self, frame: &crate::Frame) {
        self.inner
            .frames
            .get_or_insert_default()
            .push(frame.to_qlog())
    }
}

fn unknown(ty: u64, len: Option<u64>) -> QuicFrame {
    QuicFrame::Unknown {
        raw_frame_type: ty,
        frame_type_value: Some(ty),
        raw: len.map(|len| qlog::events::RawInfo {
            length: Some(len),
            payload_length: None,
            data: None,
        }),
    }
}

#[cfg(feature = "qlog")]
impl crate::Frame {
    fn to_qlog(&self) -> QuicFrame {
        use qlog::events::quic::AckedRanges;

        use crate::frame::Frame;

        match self {
            Frame::Padding => QuicFrame::Padding {
                length: None,
                // TODO: report correct length
                payload_length: 0,
            },
            Frame::Ping => QuicFrame::Ping {
                length: None,
                payload_length: None,
            },
            Frame::Ack(ack) => QuicFrame::Ack {
                ack_delay: Some(ack.delay as f32),
                acked_ranges: Some(AckedRanges::Double(
                    ack.iter()
                        .map(|range| (*range.start(), *range.end()))
                        .collect(),
                )),
                ect1: ack.ecn.as_ref().map(|e| e.ect1),
                ect0: ack.ecn.as_ref().map(|e| e.ect0),
                ce: ack.ecn.as_ref().map(|e| e.ce),
                length: None,
                payload_length: None,
            },
            Frame::ResetStream(f) => QuicFrame::ResetStream {
                stream_id: f.id.into(),
                error_code: f.error_code.into(),
                final_size: f.final_offset.into(),
                length: None,
                payload_length: None,
            },
            Frame::StopSending(f) => QuicFrame::StopSending {
                stream_id: f.id.into(),
                error_code: f.error_code.into(),
                length: None,
                payload_length: None,
            },
            Frame::Crypto(c) => QuicFrame::Crypto {
                offset: c.offset,
                length: c.data.len() as u64,
            },
            Frame::NewToken(t) => {
                use ::qlog;
                QuicFrame::NewToken {
                    token: qlog::Token {
                        ty: Some(::qlog::TokenType::Retry),
                        raw: Some(qlog::events::RawInfo {
                            data: qlog::HexSlice::maybe_string(Some(&t.token)),
                            length: Some(t.token.len() as u64),
                            payload_length: None,
                        }),
                        details: None,
                    },
                }
            }
            Frame::Stream(s) => QuicFrame::Stream {
                stream_id: s.id.into(),
                offset: s.offset,
                length: s.data.len() as u64,
                fin: Some(s.fin),
                raw: None,
            },
            Frame::MaxData(v) => QuicFrame::MaxData {
                maximum: (*v).into(),
            },
            Frame::MaxStreamData { id, offset } => QuicFrame::MaxStreamData {
                stream_id: (*id).into(),
                maximum: *offset,
            },
            Frame::MaxStreams { dir, count } => QuicFrame::MaxStreams {
                maximum: *count,
                stream_type: (*dir).into(),
            },
            Frame::DataBlocked { offset } => QuicFrame::DataBlocked { limit: *offset },
            Frame::StreamDataBlocked { id, offset } => QuicFrame::StreamDataBlocked {
                stream_id: (*id).into(),
                limit: *offset,
            },
            Frame::StreamsBlocked { dir, limit } => QuicFrame::StreamsBlocked {
                stream_type: (*dir).into(),
                limit: *limit,
            },
            Frame::NewConnectionId(f) => QuicFrame::NewConnectionId {
                sequence_number: f.sequence as u32,
                retire_prior_to: f.retire_prior_to as u32,
                connection_id_length: Some(f.id.len() as u8),
                connection_id: format!("{}", f.id),
                stateless_reset_token: Some(format!("{}", f.reset_token)),
            },
            Frame::RetireConnectionId(f) => QuicFrame::RetireConnectionId {
                sequence_number: f.sequence as u32,
            },
            Frame::PathChallenge(_) => QuicFrame::PathChallenge { data: None },
            Frame::PathResponse(_) => QuicFrame::PathResponse { data: None },
            Frame::Close(close) => QuicFrame::ConnectionClose {
                error_space: None,
                error_code: Some(close.error_code()),
                error_code_value: None,
                reason: None,
                trigger_frame_type: None,
            },
            Frame::Datagram(d) => QuicFrame::Datagram {
                length: d.data.len() as u64,
                raw: None,
            },
            Frame::HandshakeDone => QuicFrame::HandshakeDone,
            // Extensions and unsupported frames → Unknown
            Frame::AckFrequency(_)
            | Frame::ImmediateAck
            | Frame::ObservedAddr(_)
            | Frame::PathAck(_)
            | Frame::PathAbandon(_)
            | Frame::PathAvailable(_)
            | Frame::PathBackup(_)
            | Frame::MaxPathId(_)
            | Frame::PathsBlocked(_)
            | Frame::PathCidsBlocked(_)
            | Frame::AddAddress(_)
            | Frame::PunchMeNow(_)
            | Frame::RemoveAddress(_) => unknown(self.ty().to_u64(), None),
        }
    }
}

#[cfg(feature = "qlog")]
impl From<crate::Dir> for StreamType {
    fn from(value: crate::Dir) -> Self {
        match value {
            crate::Dir::Bi => StreamType::Bidirectional,
            crate::Dir::Uni => StreamType::Unidirectional,
        }
    }
}

#[cfg(feature = "qlog")]
impl From<Option<QlogStream>> for QlogSink {
    fn from(stream: Option<QlogStream>) -> Self {
        Self { stream }
    }
}

#[cfg(feature = "qlog")]
fn packet_type(space: SpaceId, is_0rtt: bool) -> PacketType {
    match space {
        SpaceId::Initial => PacketType::Initial,
        SpaceId::Handshake => PacketType::Handshake,
        SpaceId::Data if is_0rtt => PacketType::ZeroRtt,
        SpaceId::Data => PacketType::OneRtt,
    }
}

#[cfg(feature = "qlog")]
fn encode_cid(cid: ConnectionId) -> String {
    format!("{cid}")
}
