use std::{
    fmt::{self, Display},
    mem,
    net::{IpAddr, SocketAddr},
    ops::{Range, RangeInclusive},
};

use bytes::{Buf, BufMut, Bytes};
use tinyvec::TinyVec;

use crate::{
    Dir, MAX_CID_SIZE, RESET_TOKEN_SIZE, ResetToken, StreamId, TransportError, TransportErrorCode,
    VarInt,
    coding::{self, BufExt, BufMutExt, Decodable, Encodable, UnexpectedEnd},
    connection::PathId,
    range_set::ArrayRangeSet,
    shared::{ConnectionId, EcnCodepoint},
};

#[cfg(feature = "qlog")]
use super::connection::qlog::ToQlog;

#[cfg(test)]
use crate::varint::varint_u64;
#[cfg(test)]
use proptest::{collection, prelude::any, strategy::Strategy};
#[cfg(test)]
use test_strategy::Arbitrary;

#[derive(
    Copy, Clone, Eq, PartialEq, derive_more::Debug, derive_more::Display, enum_assoc::Assoc,
)]
#[cfg_attr(test, derive(Arbitrary))]
#[display(rename_all = "SCREAMING_SNAKE_CASE")]
#[allow(missing_docs)]
#[func(
    pub(crate) const fn to_u64(self) -> u64,
    const fn from_u64(rev: u64) -> Option<Self>,
)]
pub enum FrameType {
    #[assoc(to_u64 = 0x00)]
    Padding,
    #[assoc(to_u64 = 0x01)]
    Ping,
    #[assoc(to_u64 = 0x02)]
    Ack,
    #[assoc(to_u64 = 0x03)]
    AckEcn,
    #[assoc(to_u64 = 0x04)]
    ResetStream,
    #[assoc(to_u64 = 0x05)]
    StopSending,
    #[assoc(to_u64 = 0x06)]
    Crypto,
    #[assoc(to_u64 = 0x07)]
    NewToken,
    // STREAM
    #[assoc(to_u64 = _0.to_u64())]
    Stream(StreamInfo),
    #[assoc(to_u64 = 0x10)]
    MaxData,
    #[assoc(to_u64 = 0x11)]
    MaxStreamData,
    #[assoc(to_u64 = 0x12)]
    MaxStreamsBidi,
    #[assoc(to_u64 = 0x13)]
    MaxStreamsUni,
    #[assoc(to_u64 = 0x14)]
    DataBlocked,
    #[assoc(to_u64 = 0x15)]
    StreamDataBlocked,
    #[assoc(to_u64 = 0x16)]
    StreamsBlockedBidi,
    #[assoc(to_u64 = 0x17)]
    StreamsBlockedUni,
    #[assoc(to_u64 = 0x18)]
    NewConnectionId,
    #[assoc(to_u64 = 0x19)]
    RetireConnectionId,
    #[assoc(to_u64 = 0x1a)]
    PathChallenge,
    #[assoc(to_u64 = 0x1b)]
    PathResponse,
    #[assoc(to_u64 = 0x1c)]
    ConnectionClose,
    #[assoc(to_u64 = 0x1d)]
    ApplicationClose,
    #[assoc(to_u64 = 0x1e)]
    HandshakeDone,
    // ACK Frequency
    #[assoc(to_u64 = 0xaf)]
    AckFrequency,
    #[assoc(to_u64 = 0x1f)]
    ImmediateAck,
    // DATAGRAM
    #[assoc(to_u64 = _0.to_u64())]
    Datagram(DatagramInfo),
    // ADDRESS DISCOVERY REPORT
    #[assoc(to_u64 = 0x9f81a6)]
    ObservedIpv4Addr,
    #[assoc(to_u64 = 0x9f81a7)]
    ObservedIpv6Addr,
    // Multipath
    #[assoc(to_u64 = 0x3e)]
    PathAck,
    #[assoc(to_u64 = 0x3f)]
    PathAckEcn,
    #[assoc(to_u64 = 0x3e75)]
    PathAbandon,
    #[assoc(to_u64 = 0x3e76)]
    PathStatusBackup,
    #[assoc(to_u64 = 0x3e77)]
    PathStatusAvailable,
    #[assoc(to_u64 = 0x3e78)]
    PathNewConnectionId,
    #[assoc(to_u64 = 0x3e79)]
    PathRetireConnectionId,
    #[assoc(to_u64 = 0x3e7a)]
    MaxPathId,
    #[assoc(to_u64 = 0x3e7b)]
    PathsBlocked,
    #[assoc(to_u64 = 0x3e7c)]
    PathCidsBlocked,
    // IROH'S NAT TRAVERSAL
    #[assoc(to_u64 = 0x3d7f90)]
    AddIpv4Address,
    #[assoc(to_u64 = 0x3d7f91)]
    AddIpv6Address,
    #[assoc(to_u64 = 0x3d7f92)]
    ReachOutAtIpv4,
    #[assoc(to_u64 = 0x3d7f93)]
    ReachOutAtIpv6,
    #[assoc(to_u64 = 0x3d7f94)]
    RemoveAddress,
}

/// Encounter a frame ID that was not valid.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("Invalid frame identifier {_0:02x}")]
pub struct InvalidFrameId(u64);

impl TryFrom<u64> for FrameType {
    type Error = InvalidFrameId;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match Self::from_u64(value) {
            Some(t) => Ok(t),
            None => {
                if DatagramInfo::VALUES.contains(&value) {
                    return Ok(Self::Datagram(DatagramInfo(value as u8)));
                }
                if StreamInfo::VALUES.contains(&value) {
                    return Ok(Self::Stream(StreamInfo(value as u8)));
                }
                Err(InvalidFrameId(value))
            }
        }
    }
}

impl FrameType {
    /// The encoded size of this [`FrameType`].
    const fn size(&self) -> usize {
        VarInt(self.to_u64()).size()
    }
}

impl Decodable for FrameType {
    fn decode<B: Buf>(buf: &mut B) -> coding::Result<Self> {
        Self::try_from(buf.get_var()?).map_err(|_| coding::UnexpectedEnd)
    }
}

impl Encodable for FrameType {
    fn encode<B: BufMut>(&self, buf: &mut B) {
        buf.write_var(self.to_u64());
    }
}

/// Wrapper type for the encodable frames.
///
/// This includes some "encoder" types instead of the actual read frame, when writing directly to
/// a buffer is more efficient than building the Frame itself.
#[derive(derive_more::From, enum_assoc::Assoc, derive_more::Display)]
#[func(fn encode_inner<B: BufMut>(&self, buf: &mut B) {_0.encode(buf)})]
#[func(pub(crate) const fn get_type(&self) -> FrameType {_0.get_type()})]
#[cfg_attr(feature = "qlog", func(pub(crate) fn to_qlog(&self) -> qlog::events::quic::QuicFrame {_0.to_qlog()}))]
pub(super) enum EncodableFrame<'a> {
    PathAck(PathAckEncoder<'a>),
    Ack(AckEncoder<'a>),
    Close(CloseEncoder<'a>),
    PathResponse(PathResponse),
    HandshakeDone(HandshakeDone),
    ReachOut(ReachOut),
    ObservedAddr(ObservedAddr),
    Ping(Ping),
    ImmediateAck(ImmediateAck),
    AckFrequency(AckFrequency),
    PathChallenge(PathChallenge),
    Crypto(Crypto),
    PathAbandon(PathAbandon),
    PathStatusAvailable(PathStatusAvailable),
    PathStatusBackup(PathStatusBackup),
    MaxPathId(MaxPathId),
    PathsBlocked(PathsBlocked),
    PathCidsBlocked(PathCidsBlocked),
    ResetStream(ResetStream),
    StopSending(StopSending),
    NewConnectionId(NewConnectionId),
    RetireConnectionId(RetireConnectionId),
    Datagram(Datagram),
    NewToken(NewToken),
    AddAddress(AddAddress),
    RemoveAddress(RemoveAddress),
    StreamMeta(StreamMetaEncoder),
    MaxData(MaxData),
    MaxStreamData(MaxStreamData),
    MaxStreams(MaxStreams),
}

impl<'a> Encodable for EncodableFrame<'a> {
    fn encode<B: BufMut>(&self, buf: &mut B) {
        self.encode_inner(buf)
    }
}

pub(crate) trait FrameStruct {
    /// Smallest number of bytes this type of frame is guaranteed to fit within.
    const SIZE_BOUND: usize;
}

/// The type used to refer to [`FrameType`]s in closing and transport errors.
#[derive(Copy, Clone, Eq, PartialEq, derive_more::Debug, derive_more::Display)]
pub enum MaybeFrame {
    /// Not attributed to any particular [`FrameType`].
    None,
    /// Attributed to some frame type this implementation does not recognize.
    #[display("UNKNOWN({:02x})", _0)]
    #[debug("Unknown({:02x})", _0)]
    Unknown(u64),
    /// Attributed to a specific [`FrameType`], never [`FrameType::Padding`].
    Known(FrameType),
}

impl MaybeFrame {
    /// Encoded size of this [`MaybeFrame`].
    const fn size(&self) -> usize {
        match self {
            Self::None => VarInt(0).size(),
            Self::Unknown(other) => VarInt(*other).size(),
            Self::Known(frame_type) => frame_type.size(),
        }
    }
}

impl Decodable for MaybeFrame {
    fn decode<B: Buf>(buf: &mut B) -> coding::Result<Self> {
        match FrameType::try_from(buf.get_var()?) {
            Ok(FrameType::Padding) => Ok(Self::None),
            Ok(other_frame) => Ok(Self::Known(other_frame)),
            Err(InvalidFrameId(other)) => Ok(Self::Unknown(other)),
        }
    }
}

impl Encodable for MaybeFrame {
    fn encode<B: BufMut>(&self, buf: &mut B) {
        match self {
            Self::None => buf.write_var(0u64),
            Self::Unknown(frame_id) => buf.write_var(*frame_id),
            Self::Known(frame_type) => buf.write(*frame_type),
        }
    }
}

#[cfg(test)]
impl proptest::arbitrary::Arbitrary for MaybeFrame {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        prop_oneof![
            Just(Self::None),
            any::<VarInt>().prop_map(|v| Self::Unknown(v.0)),
            // do not generate padding frames here, since they are not allowed in MaybeFrame::Known
            any::<FrameType>()
                .prop_filter("not Padding", |ft| *ft != FrameType::Padding)
                .prop_map(MaybeFrame::Known),
        ]
        .boxed()
    }
}

#[derive(derive_more::Display)]
#[cfg_attr(test, derive(Arbitrary, Debug, Clone, PartialEq, Eq))]
#[display("HANDSHAKE_DONE")]
pub(crate) struct HandshakeDone;

impl HandshakeDone {
    const fn get_type(&self) -> FrameType {
        FrameType::HandshakeDone
    }
}

impl Encodable for HandshakeDone {
    fn encode<B: BufMut>(&self, buf: &mut B) {
        FrameType::HandshakeDone.encode(buf);
    }
}

#[derive(derive_more::Display)]
#[cfg_attr(test, derive(Arbitrary, Debug, Clone, PartialEq, Eq))]
#[display("PING")]
pub(crate) struct Ping;

impl Ping {
    const fn get_type(&self) -> FrameType {
        FrameType::Ping
    }
}

impl Encodable for Ping {
    fn encode<B: BufMut>(&self, buf: &mut B) {
        FrameType::Ping.encode(buf);
    }
}

#[derive(derive_more::Display)]
#[cfg_attr(test, derive(Arbitrary, Debug, Clone, PartialEq, Eq))]
#[display("IMMEDIATE_ACK")]
pub(crate) struct ImmediateAck;

impl ImmediateAck {
    const fn get_type(&self) -> FrameType {
        FrameType::ImmediateAck
    }
}

impl Encodable for ImmediateAck {
    fn encode<B: BufMut>(&self, buf: &mut B) {
        FrameType::ImmediateAck.encode(buf);
    }
}

#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, derive_more::Display)]
#[display("STREAM")]
#[cfg_attr(test, derive(Arbitrary))]
pub struct StreamInfo(#[cfg_attr(test, strategy(0x08u8..=0x0f))] u8);

impl StreamInfo {
    const VALUES: RangeInclusive<u64> = RangeInclusive::new(0x08, 0x0f);
    fn fin(self) -> bool {
        self.0 & 0x01 != 0
    }
    fn len(self) -> bool {
        self.0 & 0x02 != 0
    }
    fn off(self) -> bool {
        self.0 & 0x04 != 0
    }

    const fn to_u64(self) -> u64 {
        self.0 as u64
    }
}

#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, derive_more::Display)]
#[display("DATAGRAM")]
#[cfg_attr(test, derive(Arbitrary))]
pub struct DatagramInfo(#[cfg_attr(test, strategy(0x30u8..=0x31))] u8);

impl DatagramInfo {
    const VALUES: RangeInclusive<u64> = RangeInclusive::new(0x30, 0x31);

    fn len(self) -> bool {
        self.0 & 0x01 != 0
    }

    const fn to_u64(self) -> u64 {
        self.0 as u64
    }
}

#[derive(Debug)]
#[cfg_attr(test, derive(Arbitrary))]
pub(crate) enum Frame {
    Padding,
    Ping,
    Ack(Ack),
    PathAck(PathAck),
    ResetStream(ResetStream),
    StopSending(StopSending),
    Crypto(Crypto),
    NewToken(NewToken),
    Stream(Stream),
    MaxData(MaxData),
    MaxStreamData(MaxStreamData),
    MaxStreams(MaxStreams),
    DataBlocked(DataBlocked),
    StreamDataBlocked(StreamDataBlocked),
    StreamsBlocked(StreamsBlocked),
    NewConnectionId(NewConnectionId),
    RetireConnectionId(RetireConnectionId),
    PathChallenge(PathChallenge),
    PathResponse(PathResponse),
    Close(Close),
    Datagram(Datagram),
    AckFrequency(AckFrequency),
    ImmediateAck,
    HandshakeDone,
    ObservedAddr(ObservedAddr),
    PathAbandon(PathAbandon),
    PathStatusAvailable(PathStatusAvailable),
    PathStatusBackup(PathStatusBackup),
    MaxPathId(MaxPathId),
    PathsBlocked(PathsBlocked),
    PathCidsBlocked(PathCidsBlocked),
    AddAddress(AddAddress),
    ReachOut(ReachOut),
    RemoveAddress(RemoveAddress),
}

impl fmt::Display for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Eventually all our frames will support fmt::Display and be able to be used to log
        // consistently. For now we fall back to fmt::Debug.
        match self {
            Self::Padding => write!(f, "PADDING"),
            Self::Ping => write!(f, "PING"),
            Self::PathChallenge(frame) => write!(f, "{frame}"),
            Self::PathResponse(frame) => write!(f, "{frame}"),
            Self::ImmediateAck => write!(f, "IMMEDIATE_ACK"),
            Self::HandshakeDone => write!(f, "HANDSHAKE_DONE"),
            _ => write!(f, "{self:?}"),
        }
    }
}

impl Frame {
    pub(crate) fn ty(&self) -> FrameType {
        use Frame::*;
        match &self {
            Padding => FrameType::Padding,
            ResetStream(_) => FrameType::ResetStream,
            Close(self::Close::Connection(_)) => FrameType::ConnectionClose,
            Close(self::Close::Application(_)) => FrameType::ConnectionClose,
            MaxData(_) => FrameType::MaxData,
            MaxStreamData(_) => FrameType::MaxStreamData,
            MaxStreams(max_streams) => max_streams.get_type(),
            Ping => FrameType::Ping,
            DataBlocked(_) => FrameType::DataBlocked,
            StreamDataBlocked(sdb) => sdb.get_type(),
            StreamsBlocked(sb) => sb.get_type(),
            StopSending { .. } => FrameType::StopSending,
            RetireConnectionId(retire_frame) => retire_frame.get_type(),
            Ack(ack) => ack.get_type(),
            PathAck(path_ack) => path_ack.get_type(),
            Stream(x) => {
                let mut ty = *StreamInfo::VALUES.start() as u8;
                if x.fin {
                    ty |= 0x01;
                }
                if x.offset != 0 {
                    ty |= 0x04;
                }
                // TODO(@divma): move all this to getframetype for Stream
                FrameType::Stream(StreamInfo(ty))
            }
            PathChallenge(_) => FrameType::PathChallenge,
            PathResponse(_) => FrameType::PathResponse,
            NewConnectionId(cid) => cid.get_type(),
            Crypto(_) => FrameType::Crypto,
            NewToken(_) => FrameType::NewToken,
            Datagram(_) => FrameType::Datagram(DatagramInfo(*DatagramInfo::VALUES.start() as u8)),
            AckFrequency(_) => FrameType::AckFrequency,
            ImmediateAck => FrameType::ImmediateAck,
            HandshakeDone => FrameType::HandshakeDone,
            ObservedAddr(observed) => observed.get_type(),
            PathAbandon(_) => FrameType::PathAbandon,
            PathStatusAvailable(_) => FrameType::PathStatusAvailable,
            PathStatusBackup(_) => FrameType::PathStatusBackup,
            MaxPathId(_) => FrameType::MaxPathId,
            PathsBlocked(_) => FrameType::PathsBlocked,
            PathCidsBlocked(_) => FrameType::PathCidsBlocked,
            AddAddress(frame) => frame.get_type(),
            ReachOut(frame) => frame.get_type(),
            RemoveAddress(_) => self::RemoveAddress::TYPE,
        }
    }

    pub(crate) fn is_ack_eliciting(&self) -> bool {
        !matches!(
            *self,
            Self::Ack(_) | Self::PathAck(_) | Self::Padding | Self::Close(_)
        )
    }

    /// Returns `true` if this frame MUST be sent in 1-RTT space
    pub(crate) fn is_1rtt(&self) -> bool {
        // See also https://www.ietf.org/archive/id/draft-ietf-quic-multipath-17.html#section-4-1:
        // > All frames defined in this document MUST only be sent in 1-RTT packets.
        // > If an endpoint receives a multipath-specific frame in a different packet type, it MUST close the
        // > connection with an error of type PROTOCOL_VIOLATION.

        self.is_multipath_frame() || self.is_qad_frame()
    }

    fn is_qad_frame(&self) -> bool {
        matches!(*self, Self::ObservedAddr(_))
    }

    fn is_multipath_frame(&self) -> bool {
        matches!(
            *self,
            Self::PathAck(_)
                | Self::PathAbandon(_)
                | Self::PathStatusBackup(_)
                | Self::PathStatusAvailable(_)
                | Self::MaxPathId(_)
                | Self::PathsBlocked(_)
                | Self::PathCidsBlocked(_)
                | Self::NewConnectionId(NewConnectionId {
                    path_id: Some(_),
                    ..
                })
                | Self::RetireConnectionId(RetireConnectionId {
                    path_id: Some(_),
                    ..
                })
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, derive_more::Display)]
#[cfg_attr(test, derive(Arbitrary))]
#[display("PATH_CHALLENGE({_0:08x})")]
pub(crate) struct PathChallenge(pub(crate) u64);

impl PathChallenge {
    pub(crate) const SIZE_BOUND: usize = 9;

    const fn get_type(&self) -> FrameType {
        FrameType::PathChallenge
    }
}

impl Decodable for PathChallenge {
    fn decode<B: Buf>(buf: &mut B) -> coding::Result<Self> {
        Ok(Self(buf.get()?))
    }
}

impl Encodable for PathChallenge {
    fn encode<B: BufMut>(&self, buf: &mut B) {
        buf.write(FrameType::PathChallenge);
        buf.write(self.0);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Display)]
#[cfg_attr(test, derive(Arbitrary))]
#[display("PATH_RESPONSE({_0:08x})")]
pub(crate) struct PathResponse(pub(crate) u64);

impl PathResponse {
    pub(crate) const SIZE_BOUND: usize = 9;

    const fn get_type(&self) -> FrameType {
        FrameType::PathResponse
    }
}

impl Decodable for PathResponse {
    fn decode<B: Buf>(buf: &mut B) -> coding::Result<Self> {
        Ok(Self(buf.get()?))
    }
}

impl Encodable for PathResponse {
    fn encode<B: BufMut>(&self, buf: &mut B) {
        buf.write(FrameType::PathResponse);
        buf.write(self.0);
    }
}

#[derive(Debug, Clone, Copy, derive_more::Display)]
#[cfg_attr(test, derive(Arbitrary, PartialEq, Eq))]
#[display("DATA_BLOCKED offset: {_0}")]
pub(crate) struct DataBlocked(#[cfg_attr(test, strategy(varint_u64()))] pub(crate) u64);

impl Encodable for DataBlocked {
    fn encode<B: BufMut>(&self, buf: &mut B) {
        buf.write(FrameType::DataBlocked);
        buf.write_var(self.0);
    }
}

#[derive(Debug, Clone, Copy, derive_more::Display)]
#[cfg_attr(test, derive(Arbitrary, PartialEq, Eq))]
#[display("STREAM_DATA_BLOCKED id: {id} offset: {offset}")]
pub(crate) struct StreamDataBlocked {
    pub(crate) id: StreamId,
    #[cfg_attr(test, strategy(varint_u64()))]
    pub(crate) offset: u64,
}

impl StreamDataBlocked {
    const fn get_type(&self) -> FrameType {
        FrameType::StreamDataBlocked
    }
}

impl Encodable for StreamDataBlocked {
    fn encode<B: BufMut>(&self, buf: &mut B) {
        buf.write(FrameType::StreamDataBlocked);
        buf.write(self.id);
        buf.write_var(self.offset);
    }
}

#[derive(Debug, Clone, Copy, derive_more::Display)]
#[cfg_attr(test, derive(Arbitrary, PartialEq, Eq))]
#[display("STREAMS_BLOCKED dir: {:?} limit: {limit}", dir)]
pub(crate) struct StreamsBlocked {
    pub(crate) dir: Dir,
    #[cfg_attr(test, strategy(varint_u64()))]
    pub(crate) limit: u64,
}

impl StreamsBlocked {
    const fn get_type(&self) -> FrameType {
        match self.dir {
            Dir::Bi => FrameType::StreamsBlockedBidi,
            Dir::Uni => FrameType::StreamsBlockedUni,
        }
    }
}

impl Encodable for StreamsBlocked {
    fn encode<B: BufMut>(&self, buf: &mut B) {
        buf.write(self.get_type());
        buf.write_var(self.limit);
    }
}

#[derive(Debug, Clone, Copy, derive_more::Display)]
#[cfg_attr(test, derive(Arbitrary, PartialEq, Eq))]
#[display("MAX_DATA({_0})")]
pub(crate) struct MaxData(pub(crate) VarInt);

impl MaxData {
    const fn get_type(&self) -> FrameType {
        FrameType::MaxData
    }
}

impl Decodable for MaxData {
    fn decode<B: Buf>(buf: &mut B) -> coding::Result<Self> {
        Ok(Self(buf.get()?))
    }
}

impl Encodable for MaxData {
    fn encode<B: BufMut>(&self, buf: &mut B) {
        buf.write(FrameType::MaxData);
        buf.write(self.0);
    }
}

#[derive(Debug, Clone, Copy, derive_more::Display)]
#[cfg_attr(test, derive(Arbitrary, PartialEq, Eq))]
#[display("MAX_STREAM_DATA id: {id} max: {offset}")]
pub(crate) struct MaxStreamData {
    pub(crate) id: StreamId,
    #[cfg_attr(test, strategy(varint_u64()))]
    pub(crate) offset: u64,
}

impl MaxStreamData {
    const fn get_type(&self) -> FrameType {
        FrameType::MaxStreamData
    }
}

impl Decodable for MaxStreamData {
    fn decode<B: Buf>(buf: &mut B) -> coding::Result<Self> {
        Ok(Self {
            id: buf.get()?,
            offset: buf.get_var()?,
        })
    }
}

impl Encodable for MaxStreamData {
    fn encode<B: BufMut>(&self, buf: &mut B) {
        buf.write(FrameType::MaxStreamData);
        buf.write(self.id);
        buf.write_var(self.offset);
    }
}

#[derive(Debug, Clone, Copy, derive_more::Display)]
#[cfg_attr(test, derive(Arbitrary, PartialEq, Eq))]
#[display("{} count: {count}", self.get_type())]
pub(crate) struct MaxStreams {
    pub(crate) dir: Dir,
    #[cfg_attr(test, strategy(varint_u64()))]
    pub(crate) count: u64,
}

impl MaxStreams {
    const fn get_type(&self) -> FrameType {
        match self.dir {
            Dir::Bi => FrameType::MaxStreamsBidi,
            Dir::Uni => FrameType::MaxStreamsUni,
        }
    }
}

impl Encodable for MaxStreams {
    fn encode<B: BufMut>(&self, buf: &mut B) {
        buf.write(self.get_type());
        buf.write_var(self.count);
    }
}

#[derive(Debug, PartialEq, Eq, derive_more::Display)]
#[cfg_attr(test, derive(Arbitrary))]
#[display("{} {} seq: {sequence}", self.get_type(), DisplayOption::new("path_id", path_id.as_ref()))]
pub(crate) struct RetireConnectionId {
    pub(crate) path_id: Option<PathId>,
    #[cfg_attr(test, strategy(varint_u64()))]
    pub(crate) sequence: u64,
}

impl RetireConnectionId {
    /// Maximum size of this frame when the frame type is [`FrameType::RetireConnectionId`]
    pub(crate) const SIZE_BOUND: usize = {
        let type_len = FrameType::RetireConnectionId.size();
        let seq_max_len = 8usize;
        type_len + seq_max_len
    };

    /// Maximum size of this frame when the frame type is [`FrameType::PathRetireConnectionId`]
    pub(crate) const SIZE_BOUND_MULTIPATH: usize = {
        let type_len = FrameType::PathRetireConnectionId.size();
        let path_id_len = VarInt::from_u32(u32::MAX).size();
        let seq_max_len = 8usize;
        type_len + path_id_len + seq_max_len
    };

    /// Decode [`Self`] from the buffer, provided that the frame type has been verified (either
    /// [`FrameType::PathRetireConnectionId`], or [`FrameType::RetireConnectionId`])
    pub(crate) fn decode<R: Buf>(bytes: &mut R, read_path: bool) -> coding::Result<Self> {
        Ok(Self {
            path_id: if read_path { Some(bytes.get()?) } else { None },
            sequence: bytes.get_var()?,
        })
    }

    /// Get the [`FrameType`] for this [`RetireConnectionId`]
    const fn get_type(&self) -> FrameType {
        if self.path_id.is_some() {
            FrameType::PathRetireConnectionId
        } else {
            FrameType::RetireConnectionId
        }
    }

    /// Returns the maximum encoded size on the wire
    ///
    /// `path_retire_cid` determines whether this frame is a multipath frame. This is a rough upper
    /// estimate, does not squeeze every last byte out.
    pub(crate) const fn size_bound(path_retire_cid: bool) -> usize {
        match path_retire_cid {
            true => Self::SIZE_BOUND_MULTIPATH,
            false => Self::SIZE_BOUND,
        }
    }
}

impl Encodable for RetireConnectionId {
    fn encode<W: BufMut>(&self, buf: &mut W) {
        buf.write(self.get_type());
        if let Some(id) = self.path_id {
            buf.write(id);
        }
        buf.write_var(self.sequence);
    }
}

#[cfg_attr(test, derive(Arbitrary))]
#[derive(Clone, Debug, derive_more::Display)]
pub(crate) enum Close {
    Connection(ConnectionClose),
    Application(ApplicationClose),
}

impl Close {
    pub(crate) fn encoder(&self, max_len: usize) -> CloseEncoder<'_> {
        CloseEncoder {
            close: self,
            max_len,
        }
    }

    pub(crate) fn is_transport_layer(&self) -> bool {
        matches!(*self, Self::Connection(_))
    }
}

#[derive(derive_more::Display)]
#[display("{close}")]
pub(crate) struct CloseEncoder<'a> {
    pub(crate) close: &'a Close,
    max_len: usize,
}

impl<'a> CloseEncoder<'a> {
    const fn get_type(&self) -> FrameType {
        match self.close {
            Close::Connection(_) => FrameType::ConnectionClose,
            Close::Application(_) => FrameType::ApplicationClose,
        }
    }
}

impl<'a> Encodable for CloseEncoder<'a> {
    fn encode<W: BufMut>(&self, out: &mut W) {
        match self.close {
            Close::Connection(x) => x.encode(out, self.max_len),
            Close::Application(x) => x.encode(out, self.max_len),
        }
    }
}

impl From<TransportError> for Close {
    fn from(x: TransportError) -> Self {
        Self::Connection(x.into())
    }
}
impl From<ConnectionClose> for Close {
    fn from(x: ConnectionClose) -> Self {
        Self::Connection(x)
    }
}
impl From<ApplicationClose> for Close {
    fn from(x: ApplicationClose) -> Self {
        Self::Application(x)
    }
}

/// Reason given by the transport for closing the connection
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct ConnectionClose {
    /// Class of error as encoded in the specification
    pub error_code: TransportErrorCode,
    /// Type of frame that caused the close
    pub frame_type: MaybeFrame,
    /// Human-readable reason for the close
    #[cfg_attr(test, strategy(proptest::collection::vec(any::<u8>(), 0..64).prop_map(Bytes::from)))]
    pub reason: Bytes,
}

impl fmt::Display for ConnectionClose {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error_code.fmt(f)?;
        if !self.reason.as_ref().is_empty() {
            f.write_str(": ")?;
            f.write_str(&String::from_utf8_lossy(&self.reason))?;
        }
        Ok(())
    }
}

impl From<TransportError> for ConnectionClose {
    fn from(x: TransportError) -> Self {
        Self {
            error_code: x.code,
            frame_type: x.frame,
            reason: x.reason.into(),
        }
    }
}

impl FrameStruct for ConnectionClose {
    const SIZE_BOUND: usize = 1 + 8 + 8 + 8;
}

impl ConnectionClose {
    pub(crate) fn encode<W: BufMut>(&self, out: &mut W, max_len: usize) {
        out.write(FrameType::ConnectionClose); // 1 byte
        out.write(self.error_code); // <= 8 bytes
        out.write(self.frame_type); // <= 8 bytes
        let max_len = max_len
            - 3
            - self.frame_type.size()
            - VarInt::from_u64(self.reason.len() as u64).unwrap().size();
        let actual_len = self.reason.len().min(max_len);
        out.write_var(actual_len as u64); // <= 8 bytes
        out.put_slice(&self.reason[0..actual_len]); // whatever's left
    }
}

/// Reason given by an application for closing the connection
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct ApplicationClose {
    /// Application-specific reason code
    pub error_code: VarInt,
    /// Human-readable reason for the close
    #[cfg_attr(test, strategy(proptest::collection::vec(any::<u8>(), 0..64).prop_map(Bytes::from)))]
    pub reason: Bytes,
}

impl fmt::Display for ApplicationClose {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.reason.as_ref().is_empty() {
            f.write_str(&String::from_utf8_lossy(&self.reason))?;
            f.write_str(" (code ")?;
            self.error_code.fmt(f)?;
            f.write_str(")")?;
        } else {
            self.error_code.fmt(f)?;
        }
        Ok(())
    }
}

impl FrameStruct for ApplicationClose {
    const SIZE_BOUND: usize = 1 + 8 + 8;
}

impl ApplicationClose {
    pub(crate) fn encode<W: BufMut>(&self, out: &mut W, max_len: usize) {
        out.write(FrameType::ApplicationClose); // 1 byte
        out.write(self.error_code); // <= 8 bytes
        let max_len = max_len - 3 - VarInt::from_u64(self.reason.len() as u64).unwrap().size();
        let actual_len = self.reason.len().min(max_len);
        out.write_var(actual_len as u64); // <= 8 bytes
        out.put_slice(&self.reason[0..actual_len]); // whatever's left
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct PathAck {
    pub path_id: PathId,
    pub largest: u64,
    pub delay: u64,
    pub ranges: ArrayRangeSet,
    pub ecn: Option<EcnCounts>,
}

#[cfg(test)]
impl proptest::arbitrary::Arbitrary for PathAck {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;

    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        (
            any::<PathId>(),
            varint_u64(),
            any::<ArrayRangeSet>()
                .prop_filter("ranges must be non empty", |ranges| !ranges.is_empty()),
            any::<Option<EcnCounts>>(),
        )
            .prop_map(|(path_id, delay, ranges, ecn)| Self {
                path_id,
                largest: ranges.max().expect("ranges must be non empty"),
                delay,
                ranges,
                ecn,
            })
            .boxed()
    }
}

impl PathAck {
    pub(crate) fn into_ack(self) -> (Ack, PathId) {
        let ack = Ack {
            largest: self.largest,
            delay: self.delay,
            ranges: self.ranges,
            ecn: self.ecn,
        };

        (ack, self.path_id)
    }

    fn get_type(&self) -> FrameType {
        if self.ecn.is_some() {
            FrameType::PathAckEcn
        } else {
            FrameType::PathAck
        }
    }

    pub(crate) fn encoder<'a>(
        path_id: PathId,
        delay: u64,
        ranges: &'a ArrayRangeSet,
        ecn: Option<&'a EcnCounts>,
    ) -> PathAckEncoder<'a> {
        PathAckEncoder {
            path_id,
            delay,
            ranges,
            ecn,
        }
    }
}

#[derive(derive_more::Display)]
#[display("{} path_id: {path_id} ranges: {ranges:?} delay: {delay}µs", self.get_type())]
pub(crate) struct PathAckEncoder<'a> {
    pub(super) path_id: PathId,
    pub(super) delay: u64,
    pub(super) ranges: &'a ArrayRangeSet,
    pub(super) ecn: Option<&'a EcnCounts>,
}

impl<'a> PathAckEncoder<'a> {
    const fn get_type(&self) -> FrameType {
        match self.ecn.is_some() {
            true => FrameType::PathAckEcn,
            false => FrameType::PathAck,
        }
    }
}

impl<'a> Encodable for PathAckEncoder<'a> {
    /// Encode [`Self`] into the given buffer
    ///
    /// The [`FrameType`] will be either [`FrameType::PathAckEcn`] or [`FrameType::PathAck`]
    /// depending on whether [`EcnCounts`] are provided.
    ///
    /// PANICS: if `ranges` is empty.
    fn encode<W: BufMut>(&self, buf: &mut W) {
        let PathAckEncoder {
            path_id,
            delay,
            ranges,
            ecn,
        } = self;
        let mut rest = ranges.iter().rev();
        let first = rest
            .next()
            .expect("Caller has verified ranges is non empty");
        let largest = first.end - 1;
        let first_size = first.end - first.start;
        let kind = match ecn.is_some() {
            true => FrameType::PathAckEcn,
            false => FrameType::PathAck,
        };
        buf.write(kind);
        buf.write(*path_id);
        buf.write_var(largest);
        buf.write_var(*delay);
        buf.write_var(ranges.len() as u64 - 1);
        buf.write_var(first_size - 1);
        let mut prev = first.start;
        for block in rest {
            let size = block.end - block.start;
            buf.write_var(prev - block.end - 1);
            buf.write_var(size - 1);
            prev = block.start;
        }
        if let Some(x) = ecn {
            x.encode(buf)
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct Ack {
    pub largest: u64,
    pub delay: u64,
    pub ranges: ArrayRangeSet,
    pub ecn: Option<EcnCounts>,
}

#[cfg(test)]
impl proptest::arbitrary::Arbitrary for Ack {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;

    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        (
            varint_u64(),
            any::<ArrayRangeSet>()
                .prop_filter("ranges must be non empty", |ranges| !ranges.is_empty()),
            any::<Option<EcnCounts>>(),
        )
            .prop_map(|(delay, ranges, ecn)| Self {
                largest: ranges.max().expect("ranges must be non empty"),
                delay,
                ranges,
                ecn,
            })
            .boxed()
    }
}

impl Ack {
    pub(crate) fn encoder<'a>(
        delay: u64,
        ranges: &'a ArrayRangeSet,
        ecn: Option<&'a EcnCounts>,
    ) -> AckEncoder<'a> {
        AckEncoder { delay, ranges, ecn }
    }

    pub(crate) fn iter(&self) -> impl DoubleEndedIterator<Item = Range<u64>> + '_ {
        self.ranges.iter()
    }

    pub(crate) const fn get_type(&self) -> FrameType {
        if self.ecn.is_some() {
            FrameType::AckEcn
        } else {
            FrameType::Ack
        }
    }
}

#[derive(derive_more::Display)]
#[display("{} ranges: {ranges:?} delay: {delay}µs", self.get_type())]
pub(crate) struct AckEncoder<'a> {
    pub(crate) delay: u64,
    pub(crate) ranges: &'a ArrayRangeSet,
    pub(crate) ecn: Option<&'a EcnCounts>,
}

impl<'a> AckEncoder<'a> {
    const fn get_type(&self) -> FrameType {
        match self.ecn.is_some() {
            true => FrameType::AckEcn,
            false => FrameType::Ack,
        }
    }
}

impl<'a> Encodable for AckEncoder<'a> {
    fn encode<W: BufMut>(&self, buf: &mut W) {
        let AckEncoder { delay, ranges, ecn } = self;
        let mut rest = ranges.iter().rev();
        let first = rest.next().unwrap();
        let largest = first.end - 1;
        let first_size = first.end - first.start;
        let kind = match ecn.is_some() {
            true => FrameType::AckEcn,
            false => FrameType::Ack,
        };
        buf.write(kind);
        buf.write_var(largest);
        buf.write_var(*delay);
        buf.write_var(ranges.len() as u64 - 1);
        buf.write_var(first_size - 1);
        let mut prev = first.start;
        for block in rest {
            let size = block.end - block.start;
            buf.write_var(prev - block.end - 1);
            buf.write_var(size - 1);
            prev = block.start;
        }
        if let Some(x) = ecn {
            x.encode(buf)
        }
    }
}

#[cfg_attr(test, derive(Arbitrary))]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) struct EcnCounts {
    #[cfg_attr(test, strategy(varint_u64()))]
    pub ect0: u64,
    #[cfg_attr(test, strategy(varint_u64()))]
    pub ect1: u64,
    #[cfg_attr(test, strategy(varint_u64()))]
    pub ce: u64,
}

impl std::ops::AddAssign<EcnCodepoint> for EcnCounts {
    fn add_assign(&mut self, rhs: EcnCodepoint) {
        match rhs {
            EcnCodepoint::Ect0 => {
                self.ect0 += 1;
            }
            EcnCodepoint::Ect1 => {
                self.ect1 += 1;
            }
            EcnCodepoint::Ce => {
                self.ce += 1;
            }
        }
    }
}

impl EcnCounts {
    pub(crate) const ZERO: Self = Self {
        ect0: 0,
        ect1: 0,
        ce: 0,
    };
}

impl Encodable for EcnCounts {
    fn encode<W: BufMut>(&self, out: &mut W) {
        out.write_var(self.ect0);
        out.write_var(self.ect1);
        out.write_var(self.ce);
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(Arbitrary, PartialEq, Eq))]
pub(crate) struct Stream {
    pub(crate) id: StreamId,
    #[cfg_attr(test, strategy(varint_u64()))]
    pub(crate) offset: u64,
    pub(crate) fin: bool,
    #[cfg_attr(test, strategy(Strategy::prop_map(collection::vec(any::<u8>(), 0..100), Bytes::from)))]
    pub(crate) data: Bytes,
}

impl FrameStruct for Stream {
    const SIZE_BOUND: usize = 1 + 8 + 8 + 8;
}

/// Metadata from a stream frame
#[derive(Debug, Clone, derive_more::Display)]
#[display("STREAM id: {id} off: {} len: {} fin: {fin}", offsets.start, offsets.end - offsets.start)]
pub(crate) struct StreamMeta {
    pub(crate) id: StreamId,
    pub(crate) offsets: Range<u64>,
    pub(crate) fin: bool,
}

// This manual implementation exists because `Default` is not implemented for `StreamId`
impl Default for StreamMeta {
    fn default() -> Self {
        Self {
            id: StreamId(0),
            offsets: 0..0,
            fin: false,
        }
    }
}

impl StreamMeta {
    pub(crate) fn encoder(self, encode_length: bool) -> StreamMetaEncoder {
        StreamMetaEncoder {
            meta: self,
            encode_length,
        }
    }

    const fn get_type(&self, encode_length: bool) -> StreamInfo {
        let mut ty = *StreamInfo::VALUES.start();
        if self.offsets.start != 0 {
            ty |= 0x04;
        }
        if encode_length {
            ty |= 0x02;
        }
        if self.fin {
            ty |= 0x01;
        }
        StreamInfo(ty as u8)
    }
}

#[derive(derive_more::Display)]
#[display("{meta}")]
pub(crate) struct StreamMetaEncoder {
    pub(crate) meta: StreamMeta,
    encode_length: bool,
}

impl StreamMetaEncoder {
    const fn get_type(&self) -> FrameType {
        FrameType::Stream(self.meta.get_type(self.encode_length))
    }
}

impl Encodable for StreamMetaEncoder {
    fn encode<W: BufMut>(&self, out: &mut W) {
        let Self {
            meta,
            encode_length,
        } = self;
        out.write_var(meta.get_type(*encode_length).0 as u64); // 1 byte
        out.write(meta.id); // <=8 bytes
        if meta.offsets.start != 0 {
            out.write_var(meta.offsets.start); // <=8 bytes
        }
        if *encode_length {
            out.write_var(meta.offsets.end - meta.offsets.start); // <=8 bytes
        }
    }
}

/// A vector of [`StreamMeta`] with optimization for the single element case
pub(crate) type StreamMetaVec = TinyVec<[StreamMeta; 1]>;

#[derive(Debug, Clone, derive_more::Display)]
#[cfg_attr(test, derive(Arbitrary, PartialEq, Eq))]
#[display("CRYPTO off: {offset} len = {}", data.len())]
pub(crate) struct Crypto {
    #[cfg_attr(test, strategy(varint_u64()))]
    pub(crate) offset: u64,
    #[cfg_attr(test, strategy(Strategy::prop_map(collection::vec(any::<u8>(), 0..1024), Bytes::from)))]
    pub(crate) data: Bytes,
}

impl Crypto {
    pub(crate) const SIZE_BOUND: usize = 17;

    const fn get_type(&self) -> FrameType {
        FrameType::Crypto
    }
}

impl Encodable for Crypto {
    fn encode<W: BufMut>(&self, out: &mut W) {
        out.write(FrameType::Crypto);
        out.write_var(self.offset);
        out.write_var(self.data.len() as u64);
        out.put_slice(&self.data);
    }
}

#[derive(Debug, Clone, derive_more::Display)]
#[cfg_attr(test, derive(Arbitrary, PartialEq, Eq))]
#[display("NEW_TOKEN")]
pub(crate) struct NewToken {
    #[cfg_attr(test, strategy(Strategy::prop_map(collection::vec(any::<u8>(), 0..1024), Bytes::from)))]
    pub(crate) token: Bytes,
}

impl Encodable for NewToken {
    fn encode<W: BufMut>(&self, out: &mut W) {
        out.write(FrameType::NewToken);
        out.write_var(self.token.len() as u64);
        out.put_slice(&self.token);
    }
}

impl NewToken {
    pub(crate) fn size(&self) -> usize {
        1 + VarInt::from_u64(self.token.len() as u64).unwrap().size() + self.token.len()
    }

    const fn get_type(&self) -> FrameType {
        FrameType::NewToken
    }
}

#[derive(Debug, Clone, derive_more::Display)]
#[cfg_attr(test, derive(Arbitrary, PartialEq, Eq))]
#[display("MAX_PATH_ID path_id: {_0}")]
pub(crate) struct MaxPathId(pub(crate) PathId);

impl MaxPathId {
    pub(crate) const SIZE_BOUND: usize =
        FrameType::MaxPathId.size() + VarInt(u32::MAX as u64).size();

    const fn get_type(&self) -> FrameType {
        FrameType::MaxPathId
    }
}

impl Decodable for MaxPathId {
    fn decode<B: Buf>(buf: &mut B) -> coding::Result<Self> {
        Ok(Self(buf.get()?))
    }
}

impl Encodable for MaxPathId {
    fn encode<B: BufMut>(&self, buf: &mut B) {
        buf.write(FrameType::MaxPathId);
        buf.write(self.0);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
#[cfg_attr(test, derive(Arbitrary))]
#[display("PATHS_BLOCKED remote_max_path_id: {_0}")]
pub(crate) struct PathsBlocked(pub(crate) PathId);

impl PathsBlocked {
    pub(crate) const SIZE_BOUND: usize =
        FrameType::PathsBlocked.size() + VarInt(u32::MAX as u64).size();

    const fn get_type(&self) -> FrameType {
        FrameType::PathsBlocked
    }
}

impl Encodable for PathsBlocked {
    fn encode<B: BufMut>(&self, buf: &mut B) {
        buf.write(FrameType::PathsBlocked);
        buf.write(self.0);
    }
}

impl Decodable for PathsBlocked {
    /// Decode [`Self`] from the buffer, provided that the frame type has been verified
    fn decode<B: Buf>(buf: &mut B) -> coding::Result<Self> {
        Ok(Self(buf.get()?))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
#[cfg_attr(test, derive(Arbitrary))]
#[display("PATH_CIDS_BLOCKED path_id: {path_id} next_seq: {next_seq}")]
pub(crate) struct PathCidsBlocked {
    pub(crate) path_id: PathId,
    pub(crate) next_seq: VarInt,
}

impl PathCidsBlocked {
    pub(crate) const SIZE_BOUND: usize =
        FrameType::PathCidsBlocked.size() + VarInt(u32::MAX as u64).size() + VarInt::MAX.size();

    const fn get_type(&self) -> FrameType {
        FrameType::PathCidsBlocked
    }
}

impl Decodable for PathCidsBlocked {
    fn decode<R: Buf>(buf: &mut R) -> coding::Result<Self> {
        Ok(Self {
            path_id: buf.get()?,
            next_seq: buf.get()?,
        })
    }
}

impl Encodable for PathCidsBlocked {
    fn encode<W: BufMut>(&self, buf: &mut W) {
        buf.write(FrameType::PathCidsBlocked);
        buf.write(self.path_id);
        buf.write(self.next_seq);
    }
}

pub(crate) struct Iter {
    bytes: Bytes,
    last_ty: MaybeFrame,
}

impl Iter {
    pub(crate) fn new(payload: Bytes) -> Result<Self, TransportError> {
        if payload.is_empty() {
            // "An endpoint MUST treat receipt of a packet containing no frames as a
            // connection error of type PROTOCOL_VIOLATION."
            // https://www.rfc-editor.org/rfc/rfc9000.html#name-frames-and-frame-types
            return Err(TransportError::PROTOCOL_VIOLATION(
                "packet payload is empty",
            ));
        }

        Ok(Self {
            bytes: payload,
            last_ty: MaybeFrame::None,
        })
    }

    fn take_len(&mut self) -> Result<Bytes, UnexpectedEnd> {
        let len = self.bytes.get_var()?;
        if len > self.bytes.remaining() as u64 {
            return Err(UnexpectedEnd);
        }
        Ok(self.bytes.split_to(len as usize))
    }

    #[track_caller]
    fn try_next(&mut self) -> Result<Frame, IterErr> {
        self.last_ty = self.bytes.get()?;

        let ty = match self.last_ty {
            MaybeFrame::None => FrameType::Padding,
            MaybeFrame::Unknown(_other) => return Err(IterErr::InvalidFrameId),
            MaybeFrame::Known(frame_type) => frame_type,
        };
        Ok(match ty {
            FrameType::Padding => Frame::Padding,
            FrameType::ResetStream => Frame::ResetStream(ResetStream {
                id: self.bytes.get()?,
                error_code: self.bytes.get()?,
                final_offset: self.bytes.get()?,
            }),
            FrameType::ConnectionClose => Frame::Close(Close::Connection(ConnectionClose {
                error_code: self.bytes.get()?,
                frame_type: self.bytes.get()?,
                reason: self.take_len()?,
            })),
            FrameType::ApplicationClose => Frame::Close(Close::Application(ApplicationClose {
                error_code: self.bytes.get()?,
                reason: self.take_len()?,
            })),
            FrameType::MaxData => Frame::MaxData(self.bytes.get()?),
            FrameType::MaxStreamData => Frame::MaxStreamData(self.bytes.get()?),
            FrameType::MaxStreamsBidi => Frame::MaxStreams(MaxStreams {
                dir: Dir::Bi,
                count: self.bytes.get_var()?,
            }),
            FrameType::MaxStreamsUni => Frame::MaxStreams(MaxStreams {
                dir: Dir::Uni,
                count: self.bytes.get_var()?,
            }),
            FrameType::Ping => Frame::Ping,
            FrameType::DataBlocked => Frame::DataBlocked(DataBlocked(self.bytes.get_var()?)),
            FrameType::StreamDataBlocked => Frame::StreamDataBlocked(StreamDataBlocked {
                id: self.bytes.get()?,
                offset: self.bytes.get_var()?,
            }),
            FrameType::StreamsBlockedBidi => Frame::StreamsBlocked(StreamsBlocked {
                dir: Dir::Bi,
                limit: self.bytes.get_var()?,
            }),
            FrameType::StreamsBlockedUni => Frame::StreamsBlocked(StreamsBlocked {
                dir: Dir::Uni,
                limit: self.bytes.get_var()?,
            }),
            FrameType::StopSending => Frame::StopSending(StopSending {
                id: self.bytes.get()?,
                error_code: self.bytes.get()?,
            }),
            FrameType::RetireConnectionId | FrameType::PathRetireConnectionId => {
                Frame::RetireConnectionId(RetireConnectionId::decode(
                    &mut self.bytes,
                    ty == FrameType::PathRetireConnectionId,
                )?)
            }
            FrameType::Ack => {
                let largest = self.bytes.get_var()?;
                let delay = self.bytes.get_var()?;
                let ranges = read_ack_blocks(&mut self.bytes, largest)?;
                Frame::Ack(Ack {
                    delay,
                    largest,
                    ranges,
                    ecn: None,
                })
            }
            FrameType::AckEcn => {
                let largest = self.bytes.get_var()?;
                let delay = self.bytes.get_var()?;
                let ranges = read_ack_blocks(&mut self.bytes, largest)?;
                let ecn = Some(EcnCounts {
                    ect0: self.bytes.get_var()?,
                    ect1: self.bytes.get_var()?,
                    ce: self.bytes.get_var()?,
                });

                Frame::Ack(Ack {
                    delay,
                    largest,
                    ranges,
                    ecn,
                })
            }
            FrameType::PathAck => {
                let path_id = self.bytes.get()?;
                let largest = self.bytes.get_var()?;
                let delay = self.bytes.get_var()?;
                let ranges = read_ack_blocks(&mut self.bytes, largest)?;
                Frame::PathAck(PathAck {
                    path_id,
                    delay,
                    largest,
                    ranges,
                    ecn: None,
                })
            }
            FrameType::PathAckEcn => {
                let path_id = self.bytes.get()?;
                let largest = self.bytes.get_var()?;
                let delay = self.bytes.get_var()?;
                let ranges = read_ack_blocks(&mut self.bytes, largest)?;
                let ecn = Some(EcnCounts {
                    ect0: self.bytes.get_var()?,
                    ect1: self.bytes.get_var()?,
                    ce: self.bytes.get_var()?,
                });
                Frame::PathAck(PathAck {
                    path_id,
                    delay,
                    largest,
                    ranges,
                    ecn,
                })
            }
            FrameType::PathChallenge => Frame::PathChallenge(self.bytes.get()?),
            FrameType::PathResponse => Frame::PathResponse(self.bytes.get()?),
            FrameType::NewConnectionId | FrameType::PathNewConnectionId => {
                let read_path = ty == FrameType::PathNewConnectionId;
                Frame::NewConnectionId(NewConnectionId::read(&mut self.bytes, read_path)?)
            }
            FrameType::Crypto => Frame::Crypto(Crypto {
                offset: self.bytes.get_var()?,
                data: self.take_len()?,
            }),
            FrameType::NewToken => Frame::NewToken(NewToken {
                token: self.take_len()?,
            }),
            FrameType::HandshakeDone => Frame::HandshakeDone,
            FrameType::AckFrequency => Frame::AckFrequency(AckFrequency {
                sequence: self.bytes.get()?,
                ack_eliciting_threshold: self.bytes.get()?,
                request_max_ack_delay: self.bytes.get()?,
                reordering_threshold: self.bytes.get()?,
            }),
            FrameType::ImmediateAck => Frame::ImmediateAck,
            FrameType::ObservedIpv4Addr | FrameType::ObservedIpv6Addr => {
                let is_ipv6 = ty == FrameType::ObservedIpv6Addr;
                let observed = ObservedAddr::read(&mut self.bytes, is_ipv6)?;
                Frame::ObservedAddr(observed)
            }
            FrameType::PathAbandon => Frame::PathAbandon(PathAbandon::decode(&mut self.bytes)?),
            FrameType::PathStatusAvailable => {
                Frame::PathStatusAvailable(PathStatusAvailable::decode(&mut self.bytes)?)
            }
            FrameType::PathStatusBackup => {
                Frame::PathStatusBackup(PathStatusBackup::decode(&mut self.bytes)?)
            }
            FrameType::MaxPathId => Frame::MaxPathId(MaxPathId::decode(&mut self.bytes)?),
            FrameType::PathsBlocked => Frame::PathsBlocked(PathsBlocked::decode(&mut self.bytes)?),
            FrameType::PathCidsBlocked => {
                Frame::PathCidsBlocked(PathCidsBlocked::decode(&mut self.bytes)?)
            }
            FrameType::AddIpv4Address | FrameType::AddIpv6Address => {
                let is_ipv6 = ty == FrameType::AddIpv6Address;
                let add_address = AddAddress::read(&mut self.bytes, is_ipv6)?;
                Frame::AddAddress(add_address)
            }
            FrameType::ReachOutAtIpv4 | FrameType::ReachOutAtIpv6 => {
                let is_ipv6 = ty == FrameType::ReachOutAtIpv6;
                let reach_out = ReachOut::read(&mut self.bytes, is_ipv6)?;
                Frame::ReachOut(reach_out)
            }
            FrameType::RemoveAddress => Frame::RemoveAddress(RemoveAddress::read(&mut self.bytes)?),
            FrameType::Stream(s) => Frame::Stream(Stream {
                id: self.bytes.get()?,
                offset: if s.off() { self.bytes.get_var()? } else { 0 },
                fin: s.fin(),
                data: if s.len() {
                    self.take_len()?
                } else {
                    self.take_remaining()
                },
            }),
            FrameType::Datagram(d) => Frame::Datagram(Datagram {
                data: if d.len() {
                    self.take_len()?
                } else {
                    self.take_remaining()
                },
            }),
        })
    }

    pub(crate) fn take_remaining(&mut self) -> Bytes {
        mem::take(&mut self.bytes)
    }
}

impl Iterator for Iter {
    type Item = Result<Frame, InvalidFrame>;
    fn next(&mut self) -> Option<Self::Item> {
        if !self.bytes.has_remaining() {
            return None;
        }
        match self.try_next() {
            Ok(x) => Some(Ok(x)),
            Err(e) => {
                // Corrupt frame, skip it and everything that follows
                self.bytes.clear();
                Some(Err(InvalidFrame {
                    ty: self.last_ty,
                    reason: e.reason(),
                }))
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct InvalidFrame {
    pub(crate) ty: MaybeFrame,
    pub(crate) reason: &'static str,
}

impl From<InvalidFrame> for TransportError {
    fn from(err: InvalidFrame) -> Self {
        let mut te = Self::FRAME_ENCODING_ERROR(err.reason);
        te.frame = err.ty;
        te
    }
}

/// Decodes the ACK Ranges from the given buffer.
///  This means, reading these three values
///
/// - ACK Range Count
/// - First ACK Range
/// - ACK Ranges
///
/// Ref <https://www.rfc-editor.org/rfc/rfc9000.html#name-ack-ranges>
fn read_ack_blocks(buf: &mut Bytes, mut largest: u64) -> Result<ArrayRangeSet, IterErr> {
    // Ack Range Count
    let num_blocks = buf.get_var()?;

    let mut out = ArrayRangeSet::new();
    let mut block_to_block;
    let mut range;

    for num_block in 0..num_blocks + 1 {
        range = buf.get_var()?;
        range += 1;

        let start = (largest + 1).checked_sub(range).ok_or(IterErr::Malformed)?;

        if range > 0 {
            out.insert(start..largest + 1);
        }

        // no gap on the last "block"
        if num_block < num_blocks {
            // skip the gap
            block_to_block = buf.get_var()?;
            block_to_block += 1;
            block_to_block += range;

            largest = largest
                .checked_sub(block_to_block)
                .ok_or(IterErr::Malformed)?;
        }
    }

    Ok(out)
}

#[derive(Debug)]
enum IterErr {
    UnexpectedEnd,
    InvalidFrameId,
    Malformed,
}

impl IterErr {
    fn reason(&self) -> &'static str {
        use IterErr::*;
        match *self {
            UnexpectedEnd => "unexpected end",
            InvalidFrameId => "invalid frame ID",
            Malformed => "malformed",
        }
    }
}

impl From<UnexpectedEnd> for IterErr {
    fn from(_: UnexpectedEnd) -> Self {
        Self::UnexpectedEnd
    }
}

#[allow(unreachable_pub)] // fuzzing only
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(test, derive(Arbitrary))]
#[derive(Debug, Copy, Clone, derive_more::Display)]
#[display("RESET_STREAM id: {id}")]
pub struct ResetStream {
    pub(crate) id: StreamId,
    pub(crate) error_code: VarInt,
    pub(crate) final_offset: VarInt,
}

impl ResetStream {
    const fn get_type(&self) -> FrameType {
        FrameType::ResetStream
    }
}

impl FrameStruct for ResetStream {
    const SIZE_BOUND: usize = 1 + 8 + 8 + 8;
}

impl Encodable for ResetStream {
    fn encode<W: BufMut>(&self, out: &mut W) {
        out.write(FrameType::ResetStream); // 1 byte
        out.write(self.id); // <= 8 bytes
        out.write(self.error_code); // <= 8 bytes
        out.write(self.final_offset); // <= 8 bytes
    }
}

#[cfg_attr(test, derive(Arbitrary))]
#[derive(Debug, Copy, Clone, derive_more::Display)]
#[display("STOP_SENDING id: {id}")]
pub(crate) struct StopSending {
    pub(crate) id: StreamId,
    pub(crate) error_code: VarInt,
}

impl FrameStruct for StopSending {
    const SIZE_BOUND: usize = 1 + 8 + 8;
}

impl StopSending {
    const fn get_type(&self) -> FrameType {
        FrameType::StopSending
    }
}

impl Encodable for StopSending {
    fn encode<W: BufMut>(&self, out: &mut W) {
        out.write(FrameType::StopSending); // 1 byte
        out.write(self.id); // <= 8 bytes
        out.write(self.error_code) // <= 8 bytes
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, derive_more::Display)]
#[display("{} {} seq: {sequence} id: {id}", self.get_type(), DisplayOption::new("path_id", path_id.as_ref()))]
pub(crate) struct NewConnectionId {
    pub(crate) path_id: Option<PathId>,
    pub(crate) sequence: u64,
    pub(crate) retire_prior_to: u64,
    pub(crate) id: ConnectionId,
    pub(crate) reset_token: ResetToken,
}

#[cfg(test)]
fn connection_id_and_reset_token() -> impl Strategy<Value = (crate::ConnectionId, ResetToken)> {
    (any::<ConnectionId>(), any::<[u8; 64]>()).prop_map(|(id, reset_key)| {
        #[cfg(all(feature = "aws-lc-rs", not(feature = "ring")))]
        use aws_lc_rs::hmac;
        #[cfg(feature = "ring")]
        use ring::hmac;
        let key = hmac::Key::new(hmac::HMAC_SHA256, &reset_key);
        (id, ResetToken::new(&key, id))
    })
}

#[cfg(test)]
impl proptest::arbitrary::Arbitrary for NewConnectionId {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;

    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        (
            any::<Option<PathId>>(),
            varint_u64(),
            varint_u64(),
            connection_id_and_reset_token(),
        )
            .prop_map(|(path_id, a, b, (id, reset_token))| {
                let sequence = std::cmp::max(a, b);
                let retire_prior_to = std::cmp::min(a, b);
                Self {
                    path_id,
                    sequence,
                    retire_prior_to,
                    id,
                    reset_token,
                }
            })
            .boxed()
    }
}

impl NewConnectionId {
    /// Maximum size of this frame when the frame type is [`FrameType::NewConnectionId`],
    pub(crate) const SIZE_BOUND: usize = {
        let type_len = FrameType::NewConnectionId.size();
        let seq_max_len = 8usize;
        let retire_prior_to_max_len = 8usize;
        let cid_len_len = 1;
        let cid_len = 160;
        let reset_token_len = 16;
        type_len + seq_max_len + retire_prior_to_max_len + cid_len_len + cid_len + reset_token_len
    };

    /// Maximum size of this frame when the frame type is [`FrameType::PathNewConnectionId`],
    pub(crate) const SIZE_BOUND_MULTIPATH: usize = {
        let type_len = FrameType::PathNewConnectionId.size();
        let path_id_len = VarInt::from_u32(u32::MAX).size();
        let seq_max_len = 8usize;
        let retire_prior_to_max_len = 8usize;
        let cid_len_len = 1;
        let cid_len = 160;
        let reset_token_len = 16;
        type_len
            + path_id_len
            + seq_max_len
            + retire_prior_to_max_len
            + cid_len_len
            + cid_len
            + reset_token_len
    };

    const fn get_type(&self) -> FrameType {
        if self.path_id.is_some() {
            FrameType::PathNewConnectionId
        } else {
            FrameType::NewConnectionId
        }
    }

    /// Returns the maximum encoded size on the wire.
    ///
    /// This is a rough upper estimate, does not squeeze every last byte out.
    pub(crate) const fn size_bound(path_new_cid: bool, cid_len: usize) -> usize {
        let upper_bound = match path_new_cid {
            true => Self::SIZE_BOUND_MULTIPATH,
            false => Self::SIZE_BOUND,
        };
        // instead of using the maximum cid len, use the provided one
        upper_bound - 160 + cid_len
    }

    fn read<R: Buf>(bytes: &mut R, read_path: bool) -> Result<Self, IterErr> {
        let path_id = if read_path { Some(bytes.get()?) } else { None };
        let sequence = bytes.get_var()?;
        let retire_prior_to = bytes.get_var()?;
        if retire_prior_to > sequence {
            return Err(IterErr::Malformed);
        }
        let length = bytes.get::<u8>()? as usize;
        if length > MAX_CID_SIZE || length == 0 {
            return Err(IterErr::Malformed);
        }
        if length > bytes.remaining() {
            return Err(IterErr::UnexpectedEnd);
        }
        let mut stage = [0; MAX_CID_SIZE];
        bytes.copy_to_slice(&mut stage[0..length]);
        let id = ConnectionId::new(&stage[..length]);
        if bytes.remaining() < 16 {
            return Err(IterErr::UnexpectedEnd);
        }
        let mut reset_token = [0; RESET_TOKEN_SIZE];
        bytes.copy_to_slice(&mut reset_token);
        Ok(Self {
            path_id,
            sequence,
            retire_prior_to,
            id,
            reset_token: reset_token.into(),
        })
    }

    pub(crate) fn issued(&self) -> crate::shared::IssuedCid {
        crate::shared::IssuedCid {
            path_id: self.path_id.unwrap_or_default(),
            sequence: self.sequence,
            id: self.id,
            reset_token: self.reset_token,
        }
    }
}

impl Encodable for NewConnectionId {
    fn encode<W: BufMut>(&self, out: &mut W) {
        out.write(self.get_type());
        if let Some(id) = self.path_id {
            out.write(id);
        }
        out.write_var(self.sequence);
        out.write_var(self.retire_prior_to);
        out.write(self.id.len() as u8);
        out.put_slice(&self.id);
        out.put_slice(&self.reset_token);
    }
}

impl FrameStruct for NewConnectionId {
    const SIZE_BOUND: usize = 1 + 8 + 8 + 1 + MAX_CID_SIZE + RESET_TOKEN_SIZE;
}

/// An unreliable datagram
#[derive(Debug, Clone, derive_more::Display)]
#[cfg_attr(test, derive(Arbitrary))]
#[display("DATAGRAM len: {}", data.len())]
pub struct Datagram {
    /// Payload
    #[cfg_attr(test, strategy(Strategy::prop_map(collection::vec(any::<u8>(), 0..100), Bytes::from)))]
    pub data: Bytes,
}

impl FrameStruct for Datagram {
    const SIZE_BOUND: usize = 1 + 8;
}

impl Datagram {
    pub(crate) fn size(&self, length: bool) -> usize {
        1 + if length {
            VarInt::from_u64(self.data.len() as u64).unwrap().size()
        } else {
            0
        } + self.data.len()
    }

    const fn get_type(&self) -> FrameType {
        FrameType::Datagram(DatagramInfo(*DatagramInfo::VALUES.start() as u8))
    }
}

impl Encodable for Datagram {
    fn encode<B: BufMut>(&self, out: &mut B) {
        // A datagram is encoded only after this is verified.
        const ENCODE_LEN: bool = true;
        out.write(FrameType::Datagram(DatagramInfo(
            *DatagramInfo::VALUES.start() as u8 | u8::from(ENCODE_LEN),
        ))); // 1 byte
        // Safe to unwrap because we check length sanity before queueing datagrams
        out.write(VarInt::from_u64(self.data.len() as u64).unwrap()); // <= 8 bytes
        out.put_slice(&self.data);
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, derive_more::Display)]
#[cfg_attr(test, derive(Arbitrary))]
#[display("ACK_FREQUENCY max_ack_delay: {}µs", request_max_ack_delay.0)]
pub(crate) struct AckFrequency {
    pub(crate) sequence: VarInt,
    pub(crate) ack_eliciting_threshold: VarInt,
    pub(crate) request_max_ack_delay: VarInt,
    pub(crate) reordering_threshold: VarInt,
}

impl AckFrequency {
    const fn get_type(&self) -> FrameType {
        FrameType::AckFrequency
    }
}

impl Encodable for AckFrequency {
    fn encode<W: BufMut>(&self, buf: &mut W) {
        buf.write(FrameType::AckFrequency);
        buf.write(self.sequence);
        buf.write(self.ack_eliciting_threshold);
        buf.write(self.request_max_ack_delay);
        buf.write(self.reordering_threshold);
    }
}

/* Address Discovery https://datatracker.ietf.org/doc/draft-seemann-quic-address-discovery/ */

/// Conjunction of the information contained in the address discovery frames
/// ([`FrameType::ObservedIpv4Addr`], [`FrameType::ObservedIpv6Addr`]).
#[derive(Debug, PartialEq, Eq, Clone, derive_more::Display)]
#[display("{} seq_no: {seq_no} addr: {}", self.get_type(), self.socket_addr())]
#[cfg_attr(test, derive(Arbitrary))]
pub(crate) struct ObservedAddr {
    /// Monotonically increasing integer within the same connection.
    pub(crate) seq_no: VarInt,
    /// Reported observed address.
    pub(crate) ip: IpAddr,
    /// Reported observed port.
    pub(crate) port: u16,
}

impl ObservedAddr {
    pub(crate) fn new<N: Into<VarInt>>(remote: std::net::SocketAddr, seq_no: N) -> Self {
        Self {
            ip: remote.ip(),
            port: remote.port(),
            seq_no: seq_no.into(),
        }
    }

    /// Get the [`FrameType`] for this frame.
    const fn get_type(&self) -> FrameType {
        if self.ip.is_ipv6() {
            FrameType::ObservedIpv6Addr
        } else {
            FrameType::ObservedIpv4Addr
        }
    }

    /// Compute the number of bytes needed to encode the frame.
    pub(crate) fn size(&self) -> usize {
        let type_size = self.get_type().size();
        let req_id_bytes = self.seq_no.size();
        let ip_bytes = if self.ip.is_ipv6() { 16 } else { 4 };
        let port_bytes = 2;
        type_size + req_id_bytes + ip_bytes + port_bytes
    }

    /// Reads the frame contents from the buffer.
    ///
    /// Should only be called when the frame type has been identified as
    /// [`FrameType::ObservedIpv4Addr`] or [`FrameType::ObservedIpv6Addr`].
    pub(crate) fn read<R: Buf>(bytes: &mut R, is_ipv6: bool) -> coding::Result<Self> {
        let seq_no = bytes.get()?;
        let ip = if is_ipv6 {
            IpAddr::V6(bytes.get()?)
        } else {
            IpAddr::V4(bytes.get()?)
        };
        let port = bytes.get()?;
        Ok(Self { seq_no, ip, port })
    }

    /// Gives the [`SocketAddr`] reported in the frame.
    pub(crate) fn socket_addr(&self) -> SocketAddr {
        (self.ip, self.port).into()
    }
}

impl Encodable for ObservedAddr {
    fn encode<W: BufMut>(&self, buf: &mut W) {
        buf.write(self.get_type());
        buf.write(self.seq_no);
        match self.ip {
            IpAddr::V4(ipv4_addr) => {
                buf.write(ipv4_addr);
            }
            IpAddr::V6(ipv6_addr) => {
                buf.write(ipv6_addr);
            }
        }
        buf.write::<u16>(self.port);
    }
}

/* Multipath <https://datatracker.ietf.org/doc/draft-ietf-quic-multipath/> */

#[derive(Debug, PartialEq, Eq, derive_more::Display)]
#[cfg_attr(test, derive(Arbitrary))]
#[display("PATH_ABANDON path_id: {path_id}")]
pub(crate) struct PathAbandon {
    pub(crate) path_id: PathId,
    pub(crate) error_code: TransportErrorCode,
}

impl PathAbandon {
    pub(crate) const SIZE_BOUND: usize = FrameType::PathAbandon.size() + 8 + 8;

    const fn get_type(&self) -> FrameType {
        FrameType::PathAbandon
    }
}

impl Encodable for PathAbandon {
    fn encode<W: BufMut>(&self, buf: &mut W) {
        buf.write(FrameType::PathAbandon);
        buf.write(self.path_id);
        buf.write(self.error_code);
    }
}

impl Decodable for PathAbandon {
    fn decode<R: Buf>(bytes: &mut R) -> coding::Result<Self> {
        Ok(Self {
            path_id: bytes.get()?,
            error_code: bytes.get()?,
        })
    }
}

#[derive(Debug, PartialEq, Eq, derive_more::Display)]
#[cfg_attr(test, derive(Arbitrary))]
#[display("PATH_STATUS_AVAILABLE path_id: {path_id} seq_no: {status_seq_no}")]
pub(crate) struct PathStatusAvailable {
    pub(crate) path_id: PathId,
    pub(crate) status_seq_no: VarInt,
}

impl PathStatusAvailable {
    const TYPE: FrameType = FrameType::PathStatusAvailable;
    pub(crate) const SIZE_BOUND: usize = FrameType::PathStatusAvailable.size() + 8 + 8;

    const fn get_type(&self) -> FrameType {
        FrameType::PathStatusAvailable
    }
}

impl Encodable for PathStatusAvailable {
    fn encode<W: BufMut>(&self, buf: &mut W) {
        buf.write(Self::TYPE);
        buf.write(self.path_id);
        buf.write(self.status_seq_no);
    }
}

impl Decodable for PathStatusAvailable {
    fn decode<R: Buf>(bytes: &mut R) -> coding::Result<Self> {
        Ok(Self {
            path_id: bytes.get()?,
            status_seq_no: bytes.get()?,
        })
    }
}

#[derive(Debug, PartialEq, Eq, derive_more::Display)]
#[cfg_attr(test, derive(Arbitrary))]
#[display("PATH_STATUS_BACKUP path_id: {path_id} seq_no: {status_seq_no}")]
pub(crate) struct PathStatusBackup {
    pub(crate) path_id: PathId,
    pub(crate) status_seq_no: VarInt,
}

impl PathStatusBackup {
    const TYPE: FrameType = FrameType::PathStatusBackup;

    const fn get_type(&self) -> FrameType {
        FrameType::PathStatusBackup
    }
}

impl Encodable for PathStatusBackup {
    fn encode<W: BufMut>(&self, buf: &mut W) {
        buf.write(Self::TYPE);
        buf.write(self.path_id);
        buf.write(self.status_seq_no);
    }
}

impl Decodable for PathStatusBackup {
    fn decode<R: Buf>(bytes: &mut R) -> coding::Result<Self> {
        Ok(Self {
            path_id: bytes.get()?,
            status_seq_no: bytes.get()?,
        })
    }
}

/* Nat traversal frames */

/// Conjunction of the information contained in the add address frames
/// ([`FrameType::AddIpv4Address`], [`FrameType::AddIpv6Address`]).
#[derive(Debug, PartialEq, Eq, Copy, Clone, PartialOrd, Ord, derive_more::Display)]
#[display("{} seq_no: {seq_no} addr: {}", self.get_type(), self.socket_addr())]
#[cfg_attr(test, derive(Arbitrary))]
pub(crate) struct AddAddress {
    /// Monotonically increasing integer within the same connection
    // TODO(@divma): both assumed, the draft has no mention of this but it's standard
    pub(crate) seq_no: VarInt,
    /// Address to include in the known set
    pub(crate) ip: IpAddr,
    /// Port to use with this address
    pub(crate) port: u16,
}

// TODO(@divma): remove
#[allow(dead_code)]
impl AddAddress {
    /// Smallest number of bytes this type of frame is guaranteed to fit within.
    pub(crate) const SIZE_BOUND: usize = Self {
        ip: IpAddr::V6(std::net::Ipv6Addr::LOCALHOST),
        port: u16::MAX,
        seq_no: VarInt::MAX,
    }
    .size();

    pub(crate) const fn new((ip, port): (IpAddr, u16), seq_no: VarInt) -> Self {
        Self { ip, port, seq_no }
    }

    /// Get the [`FrameType`] for this frame.
    const fn get_type(&self) -> FrameType {
        if self.ip.is_ipv6() {
            FrameType::AddIpv6Address
        } else {
            FrameType::AddIpv4Address
        }
    }

    /// Compute the number of bytes needed to encode the frame
    pub(crate) const fn size(&self) -> usize {
        let type_size = self.get_type().size();
        let seq_no_bytes = self.seq_no.size();
        let ip_bytes = if self.ip.is_ipv6() { 16 } else { 4 };
        let port_bytes = 2;
        type_size + seq_no_bytes + ip_bytes + port_bytes
    }

    /// Read the frame contents from the buffer
    ///
    /// Should only be called when the frame type has been identified as
    /// [`FrameType::AddIpv4Address`] or [`FrameType::AddIpv6Address`].
    pub(crate) fn read<R: Buf>(bytes: &mut R, is_ipv6: bool) -> coding::Result<Self> {
        let seq_no = bytes.get()?;
        let ip = if is_ipv6 {
            IpAddr::V6(bytes.get()?)
        } else {
            IpAddr::V4(bytes.get()?)
        };
        let port = bytes.get()?;
        Ok(Self { seq_no, ip, port })
    }

    /// Give the [`SocketAddr`] encoded in the frame
    pub(crate) fn socket_addr(&self) -> SocketAddr {
        self.ip_port().into()
    }

    pub(crate) fn ip_port(&self) -> (IpAddr, u16) {
        (self.ip, self.port)
    }
}

impl Encodable for AddAddress {
    fn encode<W: BufMut>(&self, buf: &mut W) {
        buf.write(self.get_type());
        buf.write(self.seq_no);
        match self.ip {
            IpAddr::V4(ipv4_addr) => {
                buf.write(ipv4_addr);
            }
            IpAddr::V6(ipv6_addr) => {
                buf.write(ipv6_addr);
            }
        }
        buf.write::<u16>(self.port);
    }
}

/// Conjunction of the information contained in the reach out frames
/// ([`FrameType::ReachOutAtIpv4`], [`FrameType::ReachOutAtIpv6`])
#[derive(Debug, PartialEq, Eq, Clone, derive_more::Display)]
#[display("REACH_OUT round: {round} local_addr: {}", self.socket_addr())]
#[cfg_attr(test, derive(Arbitrary))]
pub(crate) struct ReachOut {
    /// The sequence number of the NAT Traversal attempts
    pub(crate) round: VarInt,
    /// Address to use
    pub(crate) ip: IpAddr,
    /// Port to use with this address
    pub(crate) port: u16,
}

// TODO(@divma): remove
#[allow(dead_code)]
impl ReachOut {
    /// Smallest number of bytes this type of frame is guaranteed to fit within
    pub(crate) const SIZE_BOUND: usize = Self {
        round: VarInt::MAX,
        ip: IpAddr::V6(std::net::Ipv6Addr::LOCALHOST),
        port: u16::MAX,
    }
    .size();

    pub(crate) const fn new(round: VarInt, (ip, port): (IpAddr, u16)) -> Self {
        Self { round, ip, port }
    }

    /// Get the [`FrameType`] for this frame
    pub(crate) const fn get_type(&self) -> FrameType {
        if self.ip.is_ipv6() {
            FrameType::ReachOutAtIpv6
        } else {
            FrameType::ReachOutAtIpv4
        }
    }

    /// Compute the number of bytes needed to encode the frame
    pub(crate) const fn size(&self) -> usize {
        let type_size = self.get_type().size();
        let round_bytes = self.round.size();
        let ip_bytes = if self.ip.is_ipv6() { 16 } else { 4 };
        let port_bytes = 2;
        type_size + round_bytes + ip_bytes + port_bytes
    }

    /// Read the frame contents from the buffer
    ///
    /// Should only be called when the frame type has been identified as
    /// [`FrameType::ReachOutAtIpv4`] or [`FrameType::ReachOutAtIpv6`].
    pub(crate) fn read<R: Buf>(bytes: &mut R, is_ipv6: bool) -> coding::Result<Self> {
        let round = bytes.get()?;
        let ip = if is_ipv6 {
            IpAddr::V6(bytes.get()?)
        } else {
            IpAddr::V4(bytes.get()?)
        };
        let port = bytes.get()?;
        Ok(Self { round, ip, port })
    }

    /// Give the [`SocketAddr`] encoded in the frame
    pub(crate) fn socket_addr(&self) -> SocketAddr {
        (self.ip, self.port).into()
    }
}

impl Encodable for ReachOut {
    fn encode<W: BufMut>(&self, buf: &mut W) {
        buf.write(self.get_type());
        buf.write(self.round);
        match self.ip {
            IpAddr::V4(ipv4_addr) => {
                buf.write(ipv4_addr);
            }
            IpAddr::V6(ipv6_addr) => {
                buf.write(ipv6_addr);
            }
        }
        buf.write::<u16>(self.port);
    }
}

/// Frame signaling an address is no longer being advertised
#[derive(Debug, PartialEq, Eq, Copy, Clone, PartialOrd, Ord, derive_more::Display)]
#[cfg_attr(test, derive(Arbitrary))]
#[display("REMOVE_ADDRESS seq_no: {seq_no}")]
pub(crate) struct RemoveAddress {
    /// The sequence number of the address advertisement to be removed
    pub(crate) seq_no: VarInt,
}

// TODO(@divma): remove
#[allow(dead_code)]
impl RemoveAddress {
    /// [`FrameType`] of this frame
    pub(crate) const TYPE: FrameType = FrameType::RemoveAddress;

    /// Smallest number of bytes this type of frame is guaranteed to fit within
    pub(crate) const SIZE_BOUND: usize = Self::new(VarInt::MAX).size();

    pub(crate) const fn new(seq_no: VarInt) -> Self {
        Self { seq_no }
    }

    /// Compute the number of bytes needed to encode the frame
    pub(crate) const fn size(&self) -> usize {
        let type_size = Self::TYPE.size();
        let seq_no_bytes = self.seq_no.size();
        type_size + seq_no_bytes
    }

    /// Read the frame contents from the buffer
    ///
    /// Should only be called when the frame type has been identified as
    /// [`FrameType::RemoveAddress`].
    pub(crate) fn read<R: Buf>(bytes: &mut R) -> coding::Result<Self> {
        Ok(Self {
            seq_no: bytes.get()?,
        })
    }

    const fn get_type(&self) -> FrameType {
        FrameType::RemoveAddress
    }
}

impl Encodable for RemoveAddress {
    fn encode<W: BufMut>(&self, buf: &mut W) {
        buf.write(Self::TYPE);
        buf.write(self.seq_no);
    }
}

/// Helper struct for display implementations.
// NOTE: Due to lifetimes in fmt::Arguments it's not possible to make this a simple function that
// avoids allocations.
struct DisplayOption<T: Display> {
    field_name: &'static str,
    op: Option<T>,
}

impl<T: Display> DisplayOption<T> {
    fn new(field_name: &'static str, op: Option<T>) -> Self {
        Self { field_name, op }
    }
}

impl<T: Display> Display for DisplayOption<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(x) = self.op.as_ref() {
            write!(f, "{}: {x}", self.field_name)
        } else {
            fmt::Result::Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::coding::Encodable;
    use assert_matches::assert_matches;

    #[test]
    fn frame_type() {
        assert_eq!(
            FrameType::try_from(FrameType::Padding.to_u64()),
            Ok(FrameType::Padding),
        );

        assert_eq!(
            FrameType::try_from(FrameType::Datagram(DatagramInfo(0x30)).to_u64()),
            Ok(FrameType::Datagram(DatagramInfo(0x30))),
        );

        assert_eq!(
            FrameType::try_from(FrameType::Stream(StreamInfo(0x08)).to_u64()),
            Ok(FrameType::Stream(StreamInfo(0x08))),
        );
    }

    #[track_caller]
    fn frames(buf: Vec<u8>) -> Vec<Frame> {
        Iter::new(Bytes::from(buf))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }

    #[test]
    fn ack_coding() {
        const PACKETS: &[u64] = &[1, 2, 3, 5, 10, 11, 14];
        let mut ranges = ArrayRangeSet::new();
        for &packet in PACKETS {
            ranges.insert(packet..packet + 1);
        }
        let mut buf = Vec::new();
        const ECN: EcnCounts = EcnCounts {
            ect0: 42,
            ect1: 24,
            ce: 12,
        };
        Ack::encoder(42, &ranges, Some(&ECN)).encode(&mut buf);
        let frames = frames(buf);
        assert_eq!(frames.len(), 1);
        match frames[0] {
            Frame::Ack(ref ack) => {
                let mut packets = ack.iter().flatten().collect::<Vec<_>>();
                packets.sort_unstable();
                assert_eq!(&packets[..], PACKETS);
                assert_eq!(ack.ecn, Some(ECN));
            }
            ref x => panic!("incorrect frame {x:?}"),
        }
    }

    #[test]
    #[allow(clippy::range_plus_one)]
    fn path_ack_coding_with_ecn() {
        const PACKETS: &[u64] = &[1, 2, 3, 5, 10, 11, 14];
        let mut ranges = ArrayRangeSet::new();
        for &packet in PACKETS {
            ranges.insert(packet..packet + 1);
        }
        let mut buf = Vec::new();
        const ECN: EcnCounts = EcnCounts {
            ect0: 42,
            ect1: 24,
            ce: 12,
        };
        const PATH_ID: PathId = PathId::MAX;
        PathAck::encoder(PATH_ID, 42, &ranges, Some(&ECN)).encode(&mut buf);
        let frames = frames(buf);
        assert_eq!(frames.len(), 1);
        match frames[0] {
            Frame::PathAck(ref ack) => {
                assert_eq!(ack.path_id, PATH_ID);
                let mut packets = ack.ranges.iter().flatten().collect::<Vec<_>>();
                packets.sort_unstable();
                assert_eq!(&packets[..], PACKETS);
                assert_eq!(ack.ecn, Some(ECN));
            }
            ref x => panic!("incorrect frame {x:?}"),
        }
    }

    #[test]
    #[allow(clippy::range_plus_one)]
    fn path_ack_coding_no_ecn() {
        const PACKETS: &[u64] = &[1, 2, 3, 5, 10, 11, 14];
        let mut ranges = ArrayRangeSet::new();
        for &packet in PACKETS {
            ranges.insert(packet..packet + 1);
        }
        let mut buf = Vec::new();
        const PATH_ID: PathId = PathId::MAX;
        PathAck::encoder(PATH_ID, 42, &ranges, None).encode(&mut buf);
        let frames = frames(buf);
        assert_eq!(frames.len(), 1);
        match frames[0] {
            Frame::PathAck(ref ack) => {
                assert_eq!(ack.path_id, PATH_ID);
                let mut packets = ack.ranges.iter().flatten().collect::<Vec<_>>();
                packets.sort_unstable();
                assert_eq!(&packets[..], PACKETS);
                assert_eq!(ack.ecn, None);
            }
            ref x => panic!("incorrect frame {x:?}"),
        }
    }

    #[test]
    fn ack_frequency_coding() {
        let mut buf = Vec::new();
        let original = AckFrequency {
            sequence: VarInt(42),
            ack_eliciting_threshold: VarInt(20),
            request_max_ack_delay: VarInt(50_000),
            reordering_threshold: VarInt(1),
        };
        original.encode(&mut buf);
        let frames = frames(buf);
        assert_eq!(frames.len(), 1);
        match &frames[0] {
            Frame::AckFrequency(decoded) => assert_eq!(decoded, &original),
            x => panic!("incorrect frame {x:?}"),
        }
    }

    #[test]
    fn immediate_ack_coding() {
        let mut buf = Vec::new();
        FrameType::ImmediateAck.encode(&mut buf);
        let frames = frames(buf);
        assert_eq!(frames.len(), 1);
        assert_matches!(&frames[0], Frame::ImmediateAck);
    }

    /// Test that encoding and decoding [`ObservedAddr`] produces the same result.
    #[test]
    fn test_observed_addr_roundrip() {
        let observed_addr = ObservedAddr {
            seq_no: VarInt(42),
            ip: std::net::Ipv4Addr::LOCALHOST.into(),
            port: 4242,
        };
        let mut buf = Vec::with_capacity(observed_addr.size());
        observed_addr.encode(&mut buf);

        assert_eq!(
            observed_addr.size(),
            buf.len(),
            "expected written bytes and actual size differ"
        );

        let mut decoded = frames(buf);
        assert_eq!(decoded.len(), 1);
        match decoded.pop().expect("non empty") {
            Frame::ObservedAddr(decoded) => assert_eq!(decoded, observed_addr),
            x => panic!("incorrect frame {x:?}"),
        }
    }

    #[test]
    fn test_path_abandon_roundtrip() {
        let abandon = PathAbandon {
            path_id: PathId(42),
            error_code: TransportErrorCode::NO_ERROR,
        };
        let mut buf = Vec::new();
        abandon.encode(&mut buf);

        let mut decoded = frames(buf);
        assert_eq!(decoded.len(), 1);
        match decoded.pop().expect("non empty") {
            Frame::PathAbandon(decoded) => assert_eq!(decoded, abandon),
            x => panic!("incorrect frame {x:?}"),
        }
    }

    #[test]
    fn test_path_status_available_roundtrip() {
        let path_status_available = PathStatusAvailable {
            path_id: PathId(42),
            status_seq_no: VarInt(73),
        };
        let mut buf = Vec::new();
        path_status_available.encode(&mut buf);

        let mut decoded = frames(buf);
        assert_eq!(decoded.len(), 1);
        match decoded.pop().expect("non empty") {
            Frame::PathStatusAvailable(decoded) => assert_eq!(decoded, path_status_available),
            x => panic!("incorrect frame {x:?}"),
        }
    }

    #[test]
    fn test_path_status_backup_roundtrip() {
        let path_status_backup = PathStatusBackup {
            path_id: PathId(42),
            status_seq_no: VarInt(73),
        };
        let mut buf = Vec::new();
        path_status_backup.encode(&mut buf);

        let mut decoded = frames(buf);
        assert_eq!(decoded.len(), 1);
        match decoded.pop().expect("non empty") {
            Frame::PathStatusBackup(decoded) => assert_eq!(decoded, path_status_backup),
            x => panic!("incorrect frame {x:?}"),
        }
    }

    #[test]
    fn test_path_new_connection_id_roundtrip() {
        let cid = NewConnectionId {
            path_id: Some(PathId(22)),
            sequence: 31,
            retire_prior_to: 13,
            id: ConnectionId::new(&[0xAB; 8]),
            reset_token: ResetToken::from([0xCD; crate::RESET_TOKEN_SIZE]),
        };
        let mut buf = Vec::new();
        cid.encode(&mut buf);

        let mut decoded = frames(buf);
        assert_eq!(decoded.len(), 1);
        match decoded.pop().expect("non empty") {
            Frame::NewConnectionId(decoded) => assert_eq!(decoded, cid),
            x => panic!("incorrect frame {x:?}"),
        }
    }

    #[test]
    fn test_path_retire_connection_id_roundtrip() {
        let retire_cid = RetireConnectionId {
            path_id: Some(PathId(22)),
            sequence: 31,
        };
        let mut buf = Vec::new();
        retire_cid.encode(&mut buf);

        let mut decoded = frames(buf);
        assert_eq!(decoded.len(), 1);
        match decoded.pop().expect("non empty") {
            Frame::RetireConnectionId(decoded) => assert_eq!(decoded, retire_cid),
            x => panic!("incorrect frame {x:?}"),
        }
    }

    #[test]
    fn test_paths_blocked_path_cids_blocked_roundtrip() {
        let mut buf = Vec::new();

        let frame0 = PathsBlocked(PathId(22));
        frame0.encode(&mut buf);
        let frame1 = PathCidsBlocked {
            path_id: PathId(23),
            next_seq: VarInt(32),
        };
        frame1.encode(&mut buf);

        let mut decoded = frames(buf);
        assert_eq!(decoded.len(), 2);
        match decoded.pop().expect("non empty") {
            Frame::PathCidsBlocked(decoded) => assert_eq!(decoded, frame1),
            x => panic!("incorrect frame {x:?}"),
        }
        match decoded.pop().expect("non empty") {
            Frame::PathsBlocked(decoded) => assert_eq!(decoded, frame0),
            x => panic!("incorrect frame {x:?}"),
        }
    }

    /// Test that encoding and decoding [`AddAddress`] produces the same result
    #[test]
    fn test_add_address_roundrip() {
        let add_address = AddAddress {
            seq_no: VarInt(42),
            ip: std::net::Ipv4Addr::LOCALHOST.into(),
            port: 4242,
        };
        let mut buf = Vec::with_capacity(add_address.size());
        add_address.encode(&mut buf);

        assert_eq!(
            add_address.size(),
            buf.len(),
            "expected written bytes and actual size differ"
        );

        let mut decoded = frames(buf);
        assert_eq!(decoded.len(), 1);
        match decoded.pop().expect("non empty") {
            Frame::AddAddress(decoded) => assert_eq!(decoded, add_address),
            x => panic!("incorrect frame {x:?}"),
        }
    }

    /// Test that encoding and decoding [`AddAddress`] produces the same result
    #[test]
    fn test_reach_out_roundrip() {
        let reach_out = ReachOut {
            round: VarInt(42),
            ip: std::net::Ipv6Addr::LOCALHOST.into(),
            port: 4242,
        };
        let mut buf = Vec::with_capacity(reach_out.size());
        reach_out.encode(&mut buf);

        assert_eq!(
            reach_out.size(),
            buf.len(),
            "expected written bytes and actual size differ"
        );

        let mut decoded = frames(buf);
        assert_eq!(decoded.len(), 1);
        match decoded.pop().expect("non empty") {
            Frame::ReachOut(decoded) => assert_eq!(decoded, reach_out),
            x => panic!("incorrect frame {x:?}"),
        }
    }

    /// Test that encoding and decoding [`RemoveAddress`] produces the same result
    #[test]
    fn test_remove_address_roundrip() {
        let remove_addr = RemoveAddress::new(VarInt(10));
        let mut buf = Vec::with_capacity(remove_addr.size());
        remove_addr.encode(&mut buf);

        assert_eq!(
            remove_addr.size(),
            buf.len(),
            "expected written bytes and actual size differ"
        );

        let mut decoded = frames(buf);
        assert_eq!(decoded.len(), 1);
        match decoded.pop().expect("non empty") {
            Frame::RemoveAddress(decoded) => assert_eq!(decoded, remove_addr),
            x => panic!("incorrect frame {x:?}"),
        }
    }
}
