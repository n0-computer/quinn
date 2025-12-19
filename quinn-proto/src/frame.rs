use std::{
    fmt::{self, Write},
    mem,
    net::{IpAddr, SocketAddr},
    ops::{Range, RangeInclusive},
};

use bytes::{Buf, BufMut, Bytes};
use tinyvec::TinyVec;

use crate::{
    Dir, MAX_CID_SIZE, RESET_TOKEN_SIZE, ResetToken, StreamId, TransportError, TransportErrorCode,
    VarInt,
    coding::{self, BufExt, BufMutExt, UnexpectedEnd},
    connection::PathId,
    range_set::ArrayRangeSet,
    shared::{ConnectionId, EcnCodepoint},
};

#[cfg(feature = "arbitrary")]
use arbitrary::Arbitrary;

/// Generates the [`FrameType`] enum, and its associated conversions from and to u64
macro_rules! frame_types {
    // Process the unit variants.
    (enum_defs: [$($enum_defs: tt)*] try_from_arms: [$($try_from_arms: tt)*] to_u64_arms:[$($to_u64_arms: tt)*] // accumulators
     $variant: ident = $value: literal, // VariantName = value,
     $($token: tt)*) => {
        frame_types!{
            enum_defs: [$($enum_defs)* $variant,]
            try_from_arms: [$($try_from_arms)* $value => Self::$variant,]
            to_u64_arms: [$($to_u64_arms)* FrameType::$variant => $value,]
            $($token)*
        }
    };

    // Process the tuple variants.
    (enum_defs: [$($enum_defs: tt)*] try_from_arms: [$($try_from_arms: tt)*] to_u64_arms:[$($to_u64_arms: tt)*] // accumulators
     $variant: ident($inner: ident), // VariantName(InnerType),
     $($token: tt)*) => {
        frame_types!{
            enum_defs: [$($enum_defs)* $variant($inner),]
            try_from_arms: [$($try_from_arms)* value if <$inner>::VALUES.contains(&value) => Self::$variant($inner(value as u8)),]
            to_u64_arms: [$($to_u64_arms)* FrameType::$variant($inner(value)) => value as u64,]
            $($token)*
        }
    };

    // Final generation step.
    (enum_defs: [$($enum_defs: tt)+] try_from_arms: [$($try_from_arms: tt)+] to_u64_arms: [$($to_u64_arms: tt)+]) => {
        /// A QUIC frame type
        #[derive(Copy, Clone, Eq, PartialEq, derive_more::Debug, derive_more::Display)]
        #[display(rename_all = "SCREAMING_SNAKE_CASE")]
        #[allow(missing_docs)]
        pub enum FrameType {
            $($enum_defs)*
        }

        pub struct InvalidFrameId(u64);

        impl TryFrom<u64> for FrameType {
            type Error = InvalidFrameId;
            fn try_from(value: u64) -> Result<Self, Self::Error> {
                Ok(match value {
                    $($try_from_arms)*
                    other => return Err(InvalidFrameId(other))
                })
            }
        }

        impl FrameType {
            pub(crate) const fn to_u64(self) -> u64 {
                match self {
                    $($to_u64_arms)*
                }
            }
        }
    };
}

frame_types! {
    enum_defs: []
    try_from_arms: []
    to_u64_arms: []
    Padding = 0x00,
    Ping = 0x01,
    Ack = 0x02,
    AckEcn = 0x03,
    ResetStream = 0x04,
    StopSending = 0x05,
    Crypto = 0x06,
    NewToken = 0x07,
    // STREAM
    Stream(StreamInfo),
    MaxData = 0x10,
    MaxStreamData = 0x11,
    MaxStreamsBidi = 0x12,
    MaxStreamsUni = 0x13,
    DataBlocked = 0x14,
    StreamDataBlocked = 0x15,
    StreamsBlockedBidi = 0x16,
    StreamsBlockedUni = 0x17,
    NewConnectionId = 0x18,
    RetireConnectionId = 0x19,
    PathChallenge = 0x1a,
    PathResponse = 0x1b,
    ConnectionClose = 0x1c,
    ApplicationClose = 0x1d,
    HandshakeDone = 0x1e,
    // ACK Frequency
    AckFrequency = 0xaf,
    ImmediateAck = 0x1f,
    // DATAGRAM
    Datagram(DatagramInfo),
    // ADDRESS DISCOVERY REPORT
    ObservedIpv4Addr = 0x9f81a6,
    ObservedIpv6Addr = 0x9f81a7,
    // Multipath
    PathAck = 0x15228c00,
    PathAckEcn = 0x15228c01,
    PathAbandon = 0x15228c05,
    PathStatusBackup = 0x15228c07,
    PathStatusAvailable = 0x15228c08,
    PathNewConnectionId = 0x15228c09,
    PathRetireConnectionId = 0x15228c0a,
    MaxPathId = 0x15228c0c,
    PathsBlocked = 0x15228c0d,
    PathCidsBlocked = 0x15228c0e,
    // IROH'S NAT TRAVERSAL
    AddIpv4Address = 0x3d7f90,
    AddIpv6Address = 0x3d7f91,
    ReachOutAtIpv4 = 0x3d7f92,
    ReachOutAtIpv6 = 0x3d7f93,
    RemoveAddress = 0x3d7f94,
}

impl FrameType {
    /// The encoded size of this [`FrameType`].
    const fn size(&self) -> usize {
        VarInt(self.to_u64()).size()
    }
}

impl coding::Codec for FrameType {
    fn decode<B: Buf>(buf: &mut B) -> coding::Result<Self> {
        let id: u64 = buf.get_var()?;
        Self::try_from(id).map_err(|_| coding::UnexpectedEnd)
    }

    fn encode<B: BufMut>(&self, buf: &mut B) {
        buf.write_var(self.to_u64());
    }
}

pub(crate) trait FrameStruct {
    /// Smallest number of bytes this type of frame is guaranteed to fit within.
    const SIZE_BOUND: usize;
}

/// The type used to refer to [`FrameType`]s in closing and transport errors.
#[derive(Copy, Clone, Eq, PartialEq, Debug, derive_more::Display)]
pub enum MaybeFrame {
    /// Not attributed to any particular [`FrameType`].
    None,
    /// Attributed to some frame type this implementation does not recognize.
    #[display("TYPE{:02x}", _0)]
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

impl coding::Codec for MaybeFrame {
    fn decode<B: Buf>(buf: &mut B) -> coding::Result<Self> {
        match FrameType::try_from(buf.get_var()?) {
            Ok(FrameType::Padding) => Ok(Self::None),
            Ok(other_frame) => Ok(Self::Known(other_frame)),
            Err(InvalidFrameId(other)) => Ok(Self::Unknown(other)),
        }
    }

    fn encode<B: BufMut>(&self, buf: &mut B) {
        match self {
            Self::None => buf.write(0u64),
            Self::Unknown(frame_id) => buf.write(*frame_id),
            Self::Known(frame_type) => buf.write(*frame_type),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, derive_more::Display)]
#[display("STREAM")]
pub struct StreamInfo(u8);

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
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, derive_more::Display)]
#[display("DATAGRAM")]
pub struct DatagramInfo(u8);

impl DatagramInfo {
    const VALUES: RangeInclusive<u64> = RangeInclusive::new(0x30, 0x31);

    fn len(self) -> bool {
        self.0 & 0x01 != 0
    }
}

#[derive(Debug)]
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
    MaxData(VarInt),
    MaxStreamData { id: StreamId, offset: u64 },
    MaxStreams { dir: Dir, count: u64 },
    DataBlocked { offset: u64 },
    StreamDataBlocked { id: StreamId, offset: u64 },
    StreamsBlocked { dir: Dir, limit: u64 },
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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
        match *self {
            Padding => FrameType::Padding,
            ResetStream(_) => FrameType::ResetStream,
            Close(self::Close::Connection(_)) => FrameType::ConnectionClose,
            Close(self::Close::Application(_)) => FrameType::ConnectionClose,
            MaxData(_) => FrameType::MaxData,
            MaxStreamData { .. } => FrameType::MaxStreamData,
            MaxStreams { dir: Dir::Bi, .. } => FrameType::MaxStreamsBidi,
            MaxStreams { dir: Dir::Uni, .. } => FrameType::MaxStreamsUni,
            Ping => FrameType::Ping,
            DataBlocked { .. } => FrameType::DataBlocked,
            StreamDataBlocked { .. } => FrameType::StreamDataBlocked,
            StreamsBlocked { dir: Dir::Bi, .. } => FrameType::StreamsBlockedBidi,
            StreamsBlocked { dir: Dir::Uni, .. } => FrameType::StreamsBlockedUni,
            StopSending { .. } => FrameType::StopSending,
            RetireConnectionId { .. } => FrameType::RetireConnectionId,
            Ack(_) => FrameType::Ack,
            PathAck(_) => FrameType::PathAck,
            Stream(ref x) => {
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
            ObservedAddr(ref observed) => observed.get_type(),
            PathAbandon(_) => FrameType::PathAbandon,
            PathStatusAvailable(_) => FrameType::PathStatusAvailable,
            PathStatusBackup(_) => FrameType::PathStatusBackup,
            MaxPathId(_) => FrameType::MaxPathId,
            PathsBlocked(_) => FrameType::PathsBlocked,
            PathCidsBlocked(_) => FrameType::PathCidsBlocked,
            AddAddress(ref frame) => frame.get_type(),
            ReachOut(ref frame) => frame.get_type(),
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
#[display("PATH_CHALLENGE({_0:08x})")]
pub(crate) struct PathChallenge(pub(crate) u64);

impl PathChallenge {
    pub(crate) const SIZE_BOUND: usize = 9;
}
impl coding::Codec for PathChallenge {
    /// Decode [`Self`] from the buffer, provided that the frame type has been verified
    fn decode<B: Buf>(buf: &mut B) -> coding::Result<Self> {
        Ok(Self(buf.get()?))
    }

    /// Encode [`Self`] into the given buffer
    fn encode<B: BufMut>(&self, buf: &mut B) {
        buf.write(FrameType::PathChallenge);
        buf.write(self.0);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Display)]
#[display("PATH_RESPONSE({_0:08x})")]
pub(crate) struct PathResponse(pub(crate) u64);

impl PathResponse {
    pub(crate) const SIZE_BOUND: usize = 9;
}

impl coding::Codec for PathResponse {
    /// Decode [`Self`] from the buffer, provided that the frame type has been verified
    fn decode<B: Buf>(buf: &mut B) -> coding::Result<Self> {
        Ok(Self(buf.get()?))
    }

    /// Encode [`Self`] into the given buffer
    fn encode<B: BufMut>(&self, buf: &mut B) {
        buf.write(FrameType::PathResponse);
        buf.write(self.0);
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RetireConnectionId {
    pub(crate) path_id: Option<PathId>,
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

    /// Encode [`Self`] into the given buffer
    pub(crate) fn encode<W: BufMut>(&self, buf: &mut W) {
        buf.write(self.get_type());
        if let Some(id) = self.path_id {
            buf.write(id);
        }
        buf.write_var(self.sequence);
    }

    /// Decode [`Self`] from the buffer, provided that the frame type has been verified (either
    /// [`FrameType::PathRetireConnectionId`], or [`FrameType::RetireConnectionId`])
    pub(crate) fn decode<R: Buf>(bytes: &mut R, read_path: bool) -> coding::Result<Self> {
        Ok(Self {
            path_id: if read_path { Some(bytes.get()?) } else { None },
            sequence: bytes.get_var()?,
        })
    }

    /// Get the [`FrameType`] for this [`RetireConnectionId`]
    pub(crate) fn get_type(&self) -> FrameType {
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

#[derive(Clone, Debug)]
pub enum Close {
    Connection(ConnectionClose),
    Application(ApplicationClose),
}

impl Close {
    pub(crate) fn encode<W: BufMut>(&self, out: &mut W, max_len: usize) {
        match *self {
            Self::Connection(ref x) => x.encode(out, max_len),
            Self::Application(ref x) => x.encode(out, max_len),
        }
    }

    pub(crate) fn is_transport_layer(&self) -> bool {
        matches!(*self, Self::Connection(_))
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
pub struct ConnectionClose {
    /// Class of error as encoded in the specification
    pub error_code: TransportErrorCode,
    /// Type of frame that caused the close
    pub frame_type: MaybeFrame,
    /// Human-readable reason for the close
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
pub struct ApplicationClose {
    /// Application-specific reason code
    pub error_code: VarInt,
    /// Human-readable reason for the close
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

#[derive(Clone, Eq, PartialEq)]
pub struct PathAck {
    pub path_id: PathId,
    pub largest: u64,
    pub delay: u64,
    pub additional: Bytes,
    pub ecn: Option<EcnCounts>,
}

impl fmt::Debug for PathAck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ranges = "[".to_string();
        let mut first = true;
        for range in self.into_iter() {
            if !first {
                ranges.push(',');
            }
            write!(ranges, "{range:?}")?;
            first = false;
        }
        ranges.push(']');

        f.debug_struct("PathAck")
            .field("path_id", &self.path_id)
            .field("largest", &self.largest)
            .field("delay", &self.delay)
            .field("ecn", &self.ecn)
            .field("ranges", &ranges)
            .finish()
    }
}

impl<'a> IntoIterator for &'a PathAck {
    type Item = RangeInclusive<u64>;
    type IntoIter = AckIter<'a>;

    fn into_iter(self) -> AckIter<'a> {
        AckIter::new(self.largest, &self.additional[..])
    }
}

impl PathAck {
    /// Encode [`Self`] into the given buffer
    ///
    /// The [`FrameType`] will be either [`FrameType::PathAckEcn`] or [`FrameType::PathAck`]
    /// depending on whether [`EcnCounts`] are provided.
    ///
    /// PANICS: if `ranges` is empty.
    pub fn encode<W: BufMut>(
        path_id: PathId,
        delay: u64,
        ranges: &ArrayRangeSet,
        ecn: Option<&EcnCounts>,
        buf: &mut W,
    ) {
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
        buf.write(path_id);
        buf.write_var(largest);
        buf.write_var(delay);
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

    pub fn into_ack(self) -> (Ack, PathId) {
        let ack = Ack {
            largest: self.largest,
            delay: self.delay,
            additional: self.additional,
            ecn: self.ecn,
        };

        (ack, self.path_id)
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct Ack {
    pub largest: u64,
    pub delay: u64,
    pub additional: Bytes,
    pub ecn: Option<EcnCounts>,
}

impl fmt::Debug for Ack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ranges = "[".to_string();
        let mut first = true;
        for range in self.iter() {
            if !first {
                ranges.push(',');
            }
            write!(ranges, "{range:?}").unwrap();
            first = false;
        }
        ranges.push(']');

        f.debug_struct("Ack")
            .field("largest", &self.largest)
            .field("delay", &self.delay)
            .field("ecn", &self.ecn)
            .field("ranges", &ranges)
            .finish()
    }
}

impl<'a> IntoIterator for &'a Ack {
    type Item = RangeInclusive<u64>;
    type IntoIter = AckIter<'a>;

    fn into_iter(self) -> AckIter<'a> {
        AckIter::new(self.largest, &self.additional[..])
    }
}

impl Ack {
    pub fn encode<W: BufMut>(
        delay: u64,
        ranges: &ArrayRangeSet,
        ecn: Option<&EcnCounts>,
        buf: &mut W,
    ) {
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
        buf.write_var(delay);
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

    pub fn iter(&self) -> AckIter<'_> {
        self.into_iter()
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct EcnCounts {
    pub ect0: u64,
    pub ect1: u64,
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
    pub const ZERO: Self = Self {
        ect0: 0,
        ect1: 0,
        ce: 0,
    };

    pub fn encode<W: BufMut>(&self, out: &mut W) {
        out.write_var(self.ect0);
        out.write_var(self.ect1);
        out.write_var(self.ce);
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Stream {
    pub(crate) id: StreamId,
    pub(crate) offset: u64,
    pub(crate) fin: bool,
    pub(crate) data: Bytes,
}

impl FrameStruct for Stream {
    const SIZE_BOUND: usize = 1 + 8 + 8 + 8;
}

/// Metadata from a stream frame
#[derive(Debug, Clone)]
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
    pub(crate) fn encode<W: BufMut>(&self, length: bool, out: &mut W) {
        let mut ty = *StreamInfo::VALUES.start();
        if self.offsets.start != 0 {
            ty |= 0x04;
        }
        if length {
            ty |= 0x02;
        }
        if self.fin {
            ty |= 0x01;
        }
        out.write_var(ty); // 1 byte
        out.write(self.id); // <=8 bytes
        if self.offsets.start != 0 {
            out.write_var(self.offsets.start); // <=8 bytes
        }
        if length {
            out.write_var(self.offsets.end - self.offsets.start); // <=8 bytes
        }
    }
}

/// A vector of [`StreamMeta`] with optimization for the single element case
pub(crate) type StreamMetaVec = TinyVec<[StreamMeta; 1]>;

#[derive(Debug, Clone)]
pub(crate) struct Crypto {
    pub(crate) offset: u64,
    pub(crate) data: Bytes,
}

impl Crypto {
    pub(crate) const SIZE_BOUND: usize = 17;

    pub(crate) fn encode<W: BufMut>(&self, out: &mut W) {
        out.write(FrameType::Crypto);
        out.write_var(self.offset);
        out.write_var(self.data.len() as u64);
        out.put_slice(&self.data);
    }
}

#[derive(Debug, Clone)]
pub(crate) struct NewToken {
    pub(crate) token: Bytes,
}

impl NewToken {
    pub(crate) fn encode<W: BufMut>(&self, out: &mut W) {
        out.write(FrameType::NewToken);
        out.write_var(self.token.len() as u64);
        out.put_slice(&self.token);
    }

    pub(crate) fn size(&self) -> usize {
        1 + VarInt::from_u64(self.token.len() as u64).unwrap().size() + self.token.len()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct MaxPathId(pub(crate) PathId);

impl MaxPathId {
    pub(crate) const SIZE_BOUND: usize =
        FrameType::MaxPathId.size() + VarInt(u32::MAX as u64).size();

    /// Decode [`Self`] from the buffer, provided that the frame type has been verified
    pub(crate) fn decode<B: Buf>(buf: &mut B) -> coding::Result<Self> {
        Ok(Self(buf.get()?))
    }

    /// Encode [`Self`] into the given buffer
    pub(crate) fn encode<B: BufMut>(&self, buf: &mut B) {
        buf.write(FrameType::MaxPathId);
        buf.write(self.0);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PathsBlocked(pub(crate) PathId);

impl PathsBlocked {
    pub(crate) const SIZE_BOUND: usize =
        FrameType::PathsBlocked.size() + VarInt(u32::MAX as u64).size();

    /// Encode [`Self`] into the given buffer
    pub(crate) fn encode<B: BufMut>(&self, buf: &mut B) {
        buf.write(FrameType::PathsBlocked);
        buf.write(self.0);
    }

    /// Decode [`Self`] from the buffer, provided that the frame type has been verified
    pub(crate) fn decode<B: Buf>(buf: &mut B) -> coding::Result<Self> {
        Ok(Self(buf.get()?))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PathCidsBlocked {
    pub(crate) path_id: PathId,
    pub(crate) next_seq: VarInt,
}

impl PathCidsBlocked {
    pub(crate) const SIZE_BOUND: usize =
        FrameType::PathCidsBlocked.size() + VarInt(u32::MAX as u64).size() + VarInt::MAX.size();

    /// Decode [`Self`] from the buffer, provided that the frame type has been verified
    pub(crate) fn decode<R: Buf>(buf: &mut R) -> coding::Result<Self> {
        Ok(Self {
            path_id: buf.get()?,
            next_seq: buf.get()?,
        })
    }

    // Encode [`Self`] into the given buffer
    pub(crate) fn encode<W: BufMut>(&self, buf: &mut W) {
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
            FrameType::MaxStreamData => Frame::MaxStreamData {
                id: self.bytes.get()?,
                offset: self.bytes.get_var()?,
            },
            FrameType::MaxStreamsBidi => Frame::MaxStreams {
                dir: Dir::Bi,
                count: self.bytes.get_var()?,
            },
            FrameType::MaxStreamsUni => Frame::MaxStreams {
                dir: Dir::Uni,
                count: self.bytes.get_var()?,
            },
            FrameType::Ping => Frame::Ping,
            FrameType::DataBlocked => Frame::DataBlocked {
                offset: self.bytes.get_var()?,
            },
            FrameType::StreamDataBlocked => Frame::StreamDataBlocked {
                id: self.bytes.get()?,
                offset: self.bytes.get_var()?,
            },
            FrameType::StreamsBlockedBidi => Frame::StreamsBlocked {
                dir: Dir::Bi,
                limit: self.bytes.get_var()?,
            },
            FrameType::StreamsBlockedUni => Frame::StreamsBlocked {
                dir: Dir::Uni,
                limit: self.bytes.get_var()?,
            },
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
            FrameType::Ack | FrameType::AckEcn => {
                let largest = self.bytes.get_var()?;
                let delay = self.bytes.get_var()?;
                let extra_blocks = self.bytes.get_var()? as usize;
                let n = scan_ack_blocks(&self.bytes, largest, extra_blocks)?;
                Frame::Ack(Ack {
                    delay,
                    largest,
                    additional: self.bytes.split_to(n),
                    ecn: if ty != FrameType::AckEcn && ty != FrameType::PathAckEcn {
                        None
                    } else {
                        Some(EcnCounts {
                            ect0: self.bytes.get_var()?,
                            ect1: self.bytes.get_var()?,
                            ce: self.bytes.get_var()?,
                        })
                    },
                })
            }
            FrameType::PathAck | FrameType::PathAckEcn => {
                let path_id = self.bytes.get()?;
                let largest = self.bytes.get_var()?;
                let delay = self.bytes.get_var()?;
                let extra_blocks = self.bytes.get_var()? as usize;
                let n = scan_ack_blocks(&self.bytes, largest, extra_blocks)?;
                Frame::PathAck(PathAck {
                    path_id,
                    delay,
                    largest,
                    additional: self.bytes.split_to(n),
                    ecn: if ty != FrameType::AckEcn && ty != FrameType::PathAckEcn {
                        None
                    } else {
                        Some(EcnCounts {
                            ect0: self.bytes.get_var()?,
                            ect1: self.bytes.get_var()?,
                            ce: self.bytes.get_var()?,
                        })
                    },
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

    fn take_remaining(&mut self) -> Bytes {
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

/// Validate exactly `n` ACK ranges in `buf` and return the number of bytes they cover
fn scan_ack_blocks(mut buf: &[u8], largest: u64, n: usize) -> Result<usize, IterErr> {
    let total_len = buf.remaining();
    let first_block = buf.get_var()?;
    let mut smallest = largest.checked_sub(first_block).ok_or(IterErr::Malformed)?;
    for _ in 0..n {
        let gap = buf.get_var()?;
        smallest = smallest.checked_sub(gap + 2).ok_or(IterErr::Malformed)?;
        let block = buf.get_var()?;
        smallest = smallest.checked_sub(block).ok_or(IterErr::Malformed)?;
    }
    Ok(total_len - buf.remaining())
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

#[derive(Debug, Clone)]
pub struct AckIter<'a> {
    largest: u64,
    data: &'a [u8],
}

impl<'a> AckIter<'a> {
    fn new(largest: u64, data: &'a [u8]) -> Self {
        Self { largest, data }
    }
}

impl Iterator for AckIter<'_> {
    type Item = RangeInclusive<u64>;
    fn next(&mut self) -> Option<RangeInclusive<u64>> {
        if !self.data.has_remaining() {
            return None;
        }
        let block = self.data.get_var().unwrap();
        let largest = self.largest;
        if let Ok(gap) = self.data.get_var() {
            self.largest -= block + gap + 2;
        }
        Some(largest - block..=largest)
    }
}

#[allow(unreachable_pub)] // fuzzing only
#[cfg_attr(feature = "arbitrary", derive(Arbitrary))]
#[derive(Debug, Copy, Clone)]
pub struct ResetStream {
    pub(crate) id: StreamId,
    pub(crate) error_code: VarInt,
    pub(crate) final_offset: VarInt,
}

impl FrameStruct for ResetStream {
    const SIZE_BOUND: usize = 1 + 8 + 8 + 8;
}

impl ResetStream {
    pub(crate) fn encode<W: BufMut>(&self, out: &mut W) {
        out.write(FrameType::ResetStream); // 1 byte
        out.write(self.id); // <= 8 bytes
        out.write(self.error_code); // <= 8 bytes
        out.write(self.final_offset); // <= 8 bytes
    }
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct StopSending {
    pub(crate) id: StreamId,
    pub(crate) error_code: VarInt,
}

impl FrameStruct for StopSending {
    const SIZE_BOUND: usize = 1 + 8 + 8;
}

impl StopSending {
    pub(crate) fn encode<W: BufMut>(&self, out: &mut W) {
        out.write(FrameType::StopSending); // 1 byte
        out.write(self.id); // <= 8 bytes
        out.write(self.error_code) // <= 8 bytes
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) struct NewConnectionId {
    pub(crate) path_id: Option<PathId>,
    pub(crate) sequence: u64,
    pub(crate) retire_prior_to: u64,
    pub(crate) id: ConnectionId,
    pub(crate) reset_token: ResetToken,
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

    pub(crate) fn encode<W: BufMut>(&self, out: &mut W) {
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

    pub(crate) fn get_type(&self) -> FrameType {
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
}

impl FrameStruct for NewConnectionId {
    const SIZE_BOUND: usize = 1 + 8 + 8 + 1 + MAX_CID_SIZE + RESET_TOKEN_SIZE;
}

/// An unreliable datagram
#[derive(Debug, Clone)]
pub struct Datagram {
    /// Payload
    pub data: Bytes,
}

impl FrameStruct for Datagram {
    const SIZE_BOUND: usize = 1 + 8;
}

impl Datagram {
    pub(crate) fn encode(&self, length: bool, out: &mut impl BufMut) {
        out.write(FrameType::Datagram(DatagramInfo(
            *DatagramInfo::VALUES.start() as u8 | u8::from(length),
        ))); // 1 byte
        if length {
            // Safe to unwrap because we check length sanity before queueing datagrams
            out.write(VarInt::from_u64(self.data.len() as u64).unwrap()); // <= 8 bytes
        }
        out.put_slice(&self.data);
    }

    pub(crate) fn size(&self, length: bool) -> usize {
        1 + if length {
            VarInt::from_u64(self.data.len() as u64).unwrap().size()
        } else {
            0
        } + self.data.len()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) struct AckFrequency {
    pub(crate) sequence: VarInt,
    pub(crate) ack_eliciting_threshold: VarInt,
    pub(crate) request_max_ack_delay: VarInt,
    pub(crate) reordering_threshold: VarInt,
}

impl AckFrequency {
    pub(crate) fn encode<W: BufMut>(&self, buf: &mut W) {
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
#[derive(Debug, PartialEq, Eq, Clone)]
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
    pub(crate) fn get_type(&self) -> FrameType {
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

    /// Unconditionally write this frame to `buf`.
    pub(crate) fn write<W: BufMut>(&self, buf: &mut W) {
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

/* Multipath <https://datatracker.ietf.org/doc/draft-ietf-quic-multipath/> */

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct PathAbandon {
    pub(crate) path_id: PathId,
    pub(crate) error_code: TransportErrorCode,
}

impl PathAbandon {
    pub(crate) const SIZE_BOUND: usize = FrameType::PathAbandon.size() + 8 + 8;

    /// Encode [`Self`] into the given buffer
    pub(crate) fn encode<W: BufMut>(&self, buf: &mut W) {
        buf.write(FrameType::PathAbandon);
        buf.write(self.path_id);
        buf.write(self.error_code);
    }

    /// Decode [`Self`] from the buffer, provided that the frame type has been verified
    pub(crate) fn decode<R: Buf>(bytes: &mut R) -> coding::Result<Self> {
        Ok(Self {
            path_id: bytes.get()?,
            error_code: bytes.get()?,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct PathStatusAvailable {
    pub(crate) path_id: PathId,
    pub(crate) status_seq_no: VarInt,
}

impl PathStatusAvailable {
    const TYPE: FrameType = FrameType::PathStatusAvailable;
    pub(crate) const SIZE_BOUND: usize = FrameType::PathStatusAvailable.size() + 8 + 8;

    /// Encode [`Self`] into the given buffer
    pub(crate) fn encode<W: BufMut>(&self, buf: &mut W) {
        buf.write(Self::TYPE);
        buf.write(self.path_id);
        buf.write(self.status_seq_no);
    }

    /// Decode [`Self`] from the buffer, provided that the frame type has been verified
    pub(crate) fn decode<R: Buf>(bytes: &mut R) -> coding::Result<Self> {
        Ok(Self {
            path_id: bytes.get()?,
            status_seq_no: bytes.get()?,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct PathStatusBackup {
    pub(crate) path_id: PathId,
    pub(crate) status_seq_no: VarInt,
}

impl PathStatusBackup {
    const TYPE: FrameType = FrameType::PathStatusBackup;

    /// Encode [`Self`] into the given buffer
    pub(crate) fn encode<W: BufMut>(&self, buf: &mut W) {
        buf.write(Self::TYPE);
        buf.write(self.path_id);
        buf.write(self.status_seq_no);
    }

    /// Decode [`Self`] from the buffer, provided that the frame type has been verified
    pub(crate) fn decode<R: Buf>(bytes: &mut R) -> coding::Result<Self> {
        Ok(Self {
            path_id: bytes.get()?,
            status_seq_no: bytes.get()?,
        })
    }
}

/* Nat traversal frames */

/// Conjunction of the information contained in the add address frames
/// ([`FrameType::AddIpv4Address`], [`FrameType::AddIpv6Address`]).
#[derive(Debug, PartialEq, Eq, Copy, Clone, PartialOrd, Ord)]
// TODO(@divma): remove
#[allow(dead_code)]
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
    pub(crate) const fn get_type(&self) -> FrameType {
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

    /// Unconditionally write this frame to `buf`
    pub(crate) fn write<W: BufMut>(&self, buf: &mut W) {
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

/// Conjunction of the information contained in the reach out frames
/// ([`FrameType::ReachOutAtIpv4`], [`FrameType::ReachOutAtIpv6`])
#[derive(Debug, PartialEq, Eq, Clone)]
// TODO(@divma): remove
#[allow(dead_code)]
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

    /// Unconditionally write this frame to `buf`
    pub(crate) fn write<W: BufMut>(&self, buf: &mut W) {
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

/// Frame signaling an address is no longer being advertised
#[derive(Debug, PartialEq, Eq, Copy, Clone, PartialOrd, Ord)]
// TODO(@divma): remove
#[allow(dead_code)]
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

    /// Unconditionally write this frame to `buf`
    pub(crate) fn write<W: BufMut>(&self, buf: &mut W) {
        buf.write(Self::TYPE);
        buf.write(self.seq_no);
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
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::coding::Codec;
    use assert_matches::assert_matches;

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
        Ack::encode(42, &ranges, Some(&ECN), &mut buf);
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
    fn path_ack_coding() {
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
        PathAck::encode(PATH_ID, 42, &ranges, Some(&ECN), &mut buf);
        let frames = frames(buf);
        assert_eq!(frames.len(), 1);
        match frames[0] {
            Frame::PathAck(ref ack) => {
                assert_eq!(ack.path_id, PATH_ID);
                let mut packets = ack.into_iter().flatten().collect::<Vec<_>>();
                packets.sort_unstable();
                assert_eq!(&packets[..], PACKETS);
                assert_eq!(ack.ecn, Some(ECN));
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
        observed_addr.write(&mut buf);

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
        add_address.write(&mut buf);

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
        reach_out.write(&mut buf);

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
        remove_addr.write(&mut buf);

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
