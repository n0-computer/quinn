// Function bodies in this module are regularly cfg'd out
#![allow(unused_variables)]

#[cfg(feature = "qlog")]
use qlog::{
    events::{
        Event, EventData,
        quic::{
            PacketHeader, PacketLost, PacketLostTrigger, PacketReceived, PacketSent, PacketType,
            QuicFrame,
        },
    },
    streamer::QlogStreamer,
};
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
use crate::{FrameType, packet::Header};

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
        pn: u64,
        space: SpaceId,
        is_0rtt: bool,
        now: Instant,
        orig_rem_cid: ConnectionId,
    ) {
        #[cfg(feature = "qlog")]
        {
            let Some(stream) = self.stream.as_ref() else {
                return;
            };

            let event = PacketReceived {
                header: PacketHeader {
                    packet_number: Some(pn),
                    packet_type: packet_type(space, is_0rtt),
                    ..Default::default()
                },
                ..Default::default()
            };

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
        self.frame(QuicFrame::Unknown {
            raw_frame_type: ty,
            frame_type_value: Some(ty),
            raw: None,
        });
    }

    pub(super) fn finalize(&mut self, len: usize) {
        self.inner.header.length = Some(len as u16);
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
