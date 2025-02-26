use std::{
    fmt::{self, Write},
    io, mem,
    net::{IpAddr, SocketAddr},
    ops::{Range, RangeInclusive},
};

use bytes::{Buf, BufMut, Bytes};
use tinyvec::TinyVec;

use crate::{
    coding::{self, BufExt, BufMutExt, UnexpectedEnd},
    connection::PathId,
    range_set::ArrayRangeSet,
    shared::{ConnectionId, EcnCodepoint},
    Dir, ResetToken, StreamId, TransportError, TransportErrorCode, VarInt, MAX_CID_SIZE,
    RESET_TOKEN_SIZE,
};

#[cfg(feature = "arbitrary")]
use arbitrary::Arbitrary;

/// A QUIC frame type
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct FrameType(u64);

impl FrameType {
    fn stream(self) -> Option<StreamInfo> {
        if STREAM_TYS.contains(&self.0) {
            Some(StreamInfo(self.0 as u8))
        } else {
            None
        }
    }
    fn datagram(self) -> Option<DatagramInfo> {
        if DATAGRAM_TYS.contains(&self.0) {
            Some(DatagramInfo(self.0 as u8))
        } else {
            None
        }
    }
}

impl coding::Codec for FrameType {
    fn decode<B: Buf>(buf: &mut B) -> coding::Result<Self> {
        Ok(Self(buf.get_var()?))
    }
    fn encode<B: BufMut>(&self, buf: &mut B) {
        buf.write_var(self.0);
    }
}

pub(crate) trait FrameStruct {
    /// Smallest number of bytes this type of frame is guaranteed to fit within.
    const SIZE_BOUND: usize;
}

macro_rules! frame_types {
    {$($name:ident = $val:expr,)*} => {
        impl FrameType {
            $(pub(crate) const $name: FrameType = FrameType($val);)*
        }

        impl fmt::Debug for FrameType {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self.0 {
                    $($val => f.write_str(stringify!($name)),)*
                    _ => write!(f, "Type({:02x})", self.0)
                }
            }
        }

        impl fmt::Display for FrameType {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self.0 {
                    $($val => f.write_str(stringify!($name)),)*
                    x if STREAM_TYS.contains(&x) => f.write_str("STREAM"),
                    x if DATAGRAM_TYS.contains(&x) => f.write_str("DATAGRAM"),
                    _ => write!(f, "<unknown {:02x}>", self.0),
                }
            }
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct StreamInfo(u8);

impl StreamInfo {
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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct DatagramInfo(u8);

impl DatagramInfo {
    fn len(self) -> bool {
        self.0 & 0x01 != 0
    }
}

frame_types! {
    PADDING = 0x00,
    PING = 0x01,
    ACK = 0x02,
    ACK_ECN = 0x03,
    RESET_STREAM = 0x04,
    STOP_SENDING = 0x05,
    CRYPTO = 0x06,
    NEW_TOKEN = 0x07,
    // STREAM
    MAX_DATA = 0x10,
    MAX_STREAM_DATA = 0x11,
    MAX_STREAMS_BIDI = 0x12,
    MAX_STREAMS_UNI = 0x13,
    DATA_BLOCKED = 0x14,
    STREAM_DATA_BLOCKED = 0x15,
    STREAMS_BLOCKED_BIDI = 0x16,
    STREAMS_BLOCKED_UNI = 0x17,
    NEW_CONNECTION_ID = 0x18,
    RETIRE_CONNECTION_ID = 0x19,
    PATH_CHALLENGE = 0x1a,
    PATH_RESPONSE = 0x1b,
    CONNECTION_CLOSE = 0x1c,
    APPLICATION_CLOSE = 0x1d,
    HANDSHAKE_DONE = 0x1e,
    // ACK Frequency
    ACK_FREQUENCY = 0xaf,
    IMMEDIATE_ACK = 0x1f,
    // DATAGRAM
    // ADDRESS DISCOVERY REPORT
    OBSERVED_IPV4_ADDR = 0x9f81a6,
    OBSERVED_IPV6_ADDR = 0x9f81a7,
    // Multipath
    PATH_ACK = 0x15228c00,
    PATH_ACK_ECN = 0x15228c01,
    PATH_ABANDON = 0x15228c05,
    PATH_BACKUP = 0x15228c07,
    PATH_AVAILABLE = 0x15228c08,
    PATH_NEW_CONNECTION_ID = 0x15228c09,
    PATH_RETIRE_CONNECTION_ID = 0x15228c0a,
    MAX_PATH_ID = 0x15228c0c,
    PATHS_BLOCKED = 0x15228c0d,
}

const STREAM_TYS: RangeInclusive<u64> = RangeInclusive::new(0x08, 0x0f);
const DATAGRAM_TYS: RangeInclusive<u64> = RangeInclusive::new(0x30, 0x31);

#[derive(Debug)]
pub(crate) enum Frame {
    Padding,
    Ping,
    Ack(Ack),
    ResetStream(ResetStream),
    StopSending(StopSending),
    Crypto(Crypto),
    NewToken {
        token: Bytes,
    },
    Stream(Stream),
    MaxData(VarInt),
    MaxStreamData {
        id: StreamId,
        offset: u64,
    },
    MaxStreams {
        dir: Dir,
        count: u64,
    },
    DataBlocked {
        offset: u64,
    },
    StreamDataBlocked {
        id: StreamId,
        offset: u64,
    },
    StreamsBlocked {
        dir: Dir,
        limit: u64,
    },
    NewConnectionId(NewConnectionId),
    RetireConnectionId(RetireConnectionId),
    PathChallenge(u64),
    PathResponse(u64),
    Close(Close),
    Datagram(Datagram),
    AckFrequency(AckFrequency),
    ImmediateAck,
    HandshakeDone,
    ObservedAddr(ObservedAddr),
    #[allow(dead_code)] // TODO(flub)
    PathAbandon(PathAbandon),
    #[allow(dead_code)] // TODO(flub)
    PathAvailable(PathAvailable),
    #[allow(dead_code)] // TODO(flub)
    MaxPathId(PathId),
    #[allow(dead_code)] // TODO(flub)
    PathsBlocked(PathId),
}

impl Frame {
    pub(crate) fn ty(&self) -> FrameType {
        use self::Frame::*;
        match *self {
            Padding => FrameType::PADDING,
            ResetStream(_) => FrameType::RESET_STREAM,
            Close(self::Close::Connection(_)) => FrameType::CONNECTION_CLOSE,
            Close(self::Close::Application(_)) => FrameType::APPLICATION_CLOSE,
            MaxData(_) => FrameType::MAX_DATA,
            MaxStreamData { .. } => FrameType::MAX_STREAM_DATA,
            MaxStreams { dir: Dir::Bi, .. } => FrameType::MAX_STREAMS_BIDI,
            MaxStreams { dir: Dir::Uni, .. } => FrameType::MAX_STREAMS_UNI,
            Ping => FrameType::PING,
            DataBlocked { .. } => FrameType::DATA_BLOCKED,
            StreamDataBlocked { .. } => FrameType::STREAM_DATA_BLOCKED,
            StreamsBlocked { dir: Dir::Bi, .. } => FrameType::STREAMS_BLOCKED_BIDI,
            StreamsBlocked { dir: Dir::Uni, .. } => FrameType::STREAMS_BLOCKED_UNI,
            StopSending { .. } => FrameType::STOP_SENDING,
            RetireConnectionId { .. } => FrameType::RETIRE_CONNECTION_ID,
            // TODO(@divma): wth?
            Ack(_) => FrameType::ACK,
            Stream(ref x) => {
                let mut ty = *STREAM_TYS.start();
                if x.fin {
                    ty |= 0x01;
                }
                if x.offset != 0 {
                    ty |= 0x04;
                }
                FrameType(ty)
            }
            PathChallenge(_) => FrameType::PATH_CHALLENGE,
            PathResponse(_) => FrameType::PATH_RESPONSE,
            NewConnectionId(cid) => cid.get_type(),
            Crypto(_) => FrameType::CRYPTO,
            NewToken { .. } => FrameType::NEW_TOKEN,
            Datagram(_) => FrameType(*DATAGRAM_TYS.start()),
            AckFrequency(_) => FrameType::ACK_FREQUENCY,
            ImmediateAck => FrameType::IMMEDIATE_ACK,
            HandshakeDone => FrameType::HANDSHAKE_DONE,
            ObservedAddr(ref observed) => observed.get_type(),
            PathAbandon(_) => FrameType::PATH_ABANDON,
            PathAvailable(ref path_avaiable) => path_avaiable.get_type(),
            MaxPathId(_) => FrameType::MAX_PATH_ID,
            PathsBlocked(_) => FrameType::PATHS_BLOCKED,
        }
    }

    pub(crate) fn is_ack_eliciting(&self) -> bool {
        !matches!(*self, Self::Ack(_) | Self::Padding | Self::Close(_))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RetireConnectionId {
    pub(crate) path_id: Option<PathId>,
    pub(crate) sequence: u64,
}

impl RetireConnectionId {
    // TODO(@divma): docs
    pub(crate) fn write<W: BufMut>(&self, buf: &mut W) {
        buf.write(self.get_type());
        if let Some(id) = self.path_id {
            buf.write(id);
        }
        buf.write(self.sequence);
    }

    // TODO(@divma): docs
    // should only be called after the frame type has been verified
    pub(crate) fn read<R: Buf>(bytes: &mut R, read_path: bool) -> coding::Result<Self> {
        Ok(Self {
            path_id: if read_path { Some(bytes.get()?) } else { None },
            sequence: bytes.get()?,
        })
    }

    fn get_type(&self) -> FrameType {
        if self.path_id.is_some() {
            FrameType::PATH_RETIRE_CONNECTION_ID
        } else {
            FrameType::RETIRE_CONNECTION_ID
        }
    }

    /// Returns the maximum encoded size on the wire.
    ///
    /// This is a rough upper estimate, does not squeeze every last byte out.
    pub(crate) fn size_bound(path_retire_cid: bool) -> usize {
        let type_id = match path_retire_cid {
            true => FrameType::PATH_RETIRE_CONNECTION_ID.0,
            false => FrameType::RETIRE_CONNECTION_ID.0,
        };
        let type_len = VarInt::try_from(type_id).unwrap().size();
        let path_id_len = match path_retire_cid {
            true => VarInt::from(u32::MAX).size(),
            false => 0,
        };
        let seq_max_len = 8usize;
        type_len + path_id_len + seq_max_len
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
    pub frame_type: Option<FrameType>,
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
        out.write(FrameType::CONNECTION_CLOSE); // 1 byte
        out.write(self.error_code); // <= 8 bytes
        let ty = self.frame_type.map_or(0, |x| x.0);
        out.write_var(ty); // <= 8 bytes
        let max_len = max_len
            - 3
            - VarInt::from_u64(ty).unwrap().size()
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
        out.write(FrameType::APPLICATION_CLOSE); // 1 byte
        out.write(self.error_code); // <= 8 bytes
        let max_len = max_len - 3 - VarInt::from_u64(self.reason.len() as u64).unwrap().size();
        let actual_len = self.reason.len().min(max_len);
        out.write_var(actual_len as u64); // <= 8 bytes
        out.put_slice(&self.reason[0..actual_len]); // whatever's left
    }
}

// TODO(@divma): for now reusing the struct, good or bad idea?
#[derive(Clone, Eq, PartialEq)]
pub struct Ack {
    pub path_id: Option<PathId>,
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
            .field("path_id", &self.path_id)
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
        path_id: Option<PathId>,
        delay: u64,
        ranges: &ArrayRangeSet,
        ecn: Option<&EcnCounts>,
        buf: &mut W,
    ) {
        let mut rest = ranges.iter().rev();
        let first = rest.next().unwrap();
        let largest = first.end - 1;
        let first_size = first.end - first.start;
        let kind = match (path_id.is_some(), ecn.is_some()) {
            (true, true) => FrameType::PATH_ACK_ECN,
            (true, false) => FrameType::PATH_ACK,
            (false, true) => FrameType::ACK_ECN,
            (false, false) => FrameType::ACK,
        };
        buf.write(kind);
        if let Some(id) = path_id {
            buf.write(id);
        }

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
        let mut ty = *STREAM_TYS.start();
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
        out.write(FrameType::CRYPTO);
        out.write_var(self.offset);
        out.write_var(self.data.len() as u64);
        out.put_slice(&self.data);
    }
}

pub(crate) struct Iter {
    // TODO: ditch io::Cursor after bytes 0.5
    bytes: io::Cursor<Bytes>,
    last_ty: Option<FrameType>,
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
            bytes: io::Cursor::new(payload),
            last_ty: None,
        })
    }

    fn take_len(&mut self) -> Result<Bytes, UnexpectedEnd> {
        let len = self.bytes.get_var()?;
        if len > self.bytes.remaining() as u64 {
            return Err(UnexpectedEnd);
        }
        let start = self.bytes.position() as usize;
        self.bytes.advance(len as usize);
        Ok(self.bytes.get_ref().slice(start..(start + len as usize)))
    }

    #[track_caller]
    fn try_next(&mut self) -> Result<Frame, IterErr> {
        let ty = self.bytes.get::<FrameType>()?;
        self.last_ty = Some(ty);
        Ok(match ty {
            FrameType::PADDING => Frame::Padding,
            FrameType::RESET_STREAM => Frame::ResetStream(ResetStream {
                id: self.bytes.get()?,
                error_code: self.bytes.get()?,
                final_offset: self.bytes.get()?,
            }),
            FrameType::CONNECTION_CLOSE => Frame::Close(Close::Connection(ConnectionClose {
                error_code: self.bytes.get()?,
                frame_type: {
                    let x = self.bytes.get_var()?;
                    if x == 0 {
                        None
                    } else {
                        Some(FrameType(x))
                    }
                },
                reason: self.take_len()?,
            })),
            FrameType::APPLICATION_CLOSE => Frame::Close(Close::Application(ApplicationClose {
                error_code: self.bytes.get()?,
                reason: self.take_len()?,
            })),
            FrameType::MAX_DATA => Frame::MaxData(self.bytes.get()?),
            FrameType::MAX_STREAM_DATA => Frame::MaxStreamData {
                id: self.bytes.get()?,
                offset: self.bytes.get_var()?,
            },
            FrameType::MAX_STREAMS_BIDI => Frame::MaxStreams {
                dir: Dir::Bi,
                count: self.bytes.get_var()?,
            },
            FrameType::MAX_STREAMS_UNI => Frame::MaxStreams {
                dir: Dir::Uni,
                count: self.bytes.get_var()?,
            },
            FrameType::PING => Frame::Ping,
            FrameType::DATA_BLOCKED => Frame::DataBlocked {
                offset: self.bytes.get_var()?,
            },
            FrameType::STREAM_DATA_BLOCKED => Frame::StreamDataBlocked {
                id: self.bytes.get()?,
                offset: self.bytes.get_var()?,
            },
            FrameType::STREAMS_BLOCKED_BIDI => Frame::StreamsBlocked {
                dir: Dir::Bi,
                limit: self.bytes.get_var()?,
            },
            FrameType::STREAMS_BLOCKED_UNI => Frame::StreamsBlocked {
                dir: Dir::Uni,
                limit: self.bytes.get_var()?,
            },
            FrameType::STOP_SENDING => Frame::StopSending(StopSending {
                id: self.bytes.get()?,
                error_code: self.bytes.get()?,
            }),
            FrameType::RETIRE_CONNECTION_ID | FrameType::PATH_RETIRE_CONNECTION_ID => {
                Frame::RetireConnectionId(RetireConnectionId::read(
                    &mut self.bytes,
                    ty == FrameType::PATH_RETIRE_CONNECTION_ID,
                )?)
            }
            FrameType::ACK | FrameType::ACK_ECN | FrameType::PATH_ACK | FrameType::PATH_ACK_ECN => {
                let path_id = if ty == FrameType::PATH_ACK || ty == FrameType::PATH_ACK_ECN {
                    Some(self.bytes.get()?)
                } else {
                    None
                };
                let largest = self.bytes.get_var()?;
                let delay = self.bytes.get_var()?;
                let extra_blocks = self.bytes.get_var()? as usize;
                let start = self.bytes.position() as usize;
                scan_ack_blocks(&mut self.bytes, largest, extra_blocks)?;
                let end = self.bytes.position() as usize;
                Frame::Ack(Ack {
                    path_id,
                    delay,
                    largest,
                    additional: self.bytes.get_ref().slice(start..end),
                    ecn: if ty == FrameType::ACK_ECN || ty == FrameType::PATH_ACK_ECN {
                        Some(EcnCounts {
                            ect0: self.bytes.get_var()?,
                            ect1: self.bytes.get_var()?,
                            ce: self.bytes.get_var()?,
                        })
                    } else {
                        None
                    },
                })
            }
            FrameType::PATH_CHALLENGE => Frame::PathChallenge(self.bytes.get()?),
            FrameType::PATH_RESPONSE => Frame::PathResponse(self.bytes.get()?),
            FrameType::NEW_CONNECTION_ID | FrameType::PATH_NEW_CONNECTION_ID => {
                let read_path = ty == FrameType::PATH_NEW_CONNECTION_ID;
                Frame::NewConnectionId(NewConnectionId::read(&mut self.bytes, read_path)?)
            }
            FrameType::CRYPTO => Frame::Crypto(Crypto {
                offset: self.bytes.get_var()?,
                data: self.take_len()?,
            }),
            FrameType::NEW_TOKEN => Frame::NewToken {
                token: self.take_len()?,
            },
            FrameType::HANDSHAKE_DONE => Frame::HandshakeDone,
            FrameType::ACK_FREQUENCY => Frame::AckFrequency(AckFrequency {
                sequence: self.bytes.get()?,
                ack_eliciting_threshold: self.bytes.get()?,
                request_max_ack_delay: self.bytes.get()?,
                reordering_threshold: self.bytes.get()?,
            }),
            FrameType::IMMEDIATE_ACK => Frame::ImmediateAck,
            FrameType::OBSERVED_IPV4_ADDR | FrameType::OBSERVED_IPV6_ADDR => {
                let is_ipv6 = ty == FrameType::OBSERVED_IPV6_ADDR;
                let observed = ObservedAddr::read(&mut self.bytes, is_ipv6)?;
                Frame::ObservedAddr(observed)
            }
            FrameType::PATH_ABANDON => Frame::PathAbandon(PathAbandon::read(&mut self.bytes)?),
            FrameType::PATH_BACKUP | FrameType::PATH_AVAILABLE => {
                let is_backup = ty == FrameType::PATH_BACKUP;
                Frame::PathAvailable(PathAvailable::read(&mut self.bytes, is_backup)?)
            }
            FrameType::MAX_PATH_ID => Frame::MaxPathId(self.bytes.get()?),
            FrameType::PATHS_BLOCKED => Frame::PathsBlocked(self.bytes.get()?),
            _ => {
                if let Some(s) = ty.stream() {
                    Frame::Stream(Stream {
                        id: self.bytes.get()?,
                        offset: if s.off() { self.bytes.get_var()? } else { 0 },
                        fin: s.fin(),
                        data: if s.len() {
                            self.take_len()?
                        } else {
                            self.take_remaining()
                        },
                    })
                } else if let Some(d) = ty.datagram() {
                    Frame::Datagram(Datagram {
                        data: if d.len() {
                            self.take_len()?
                        } else {
                            self.take_remaining()
                        },
                    })
                } else {
                    return Err(IterErr::InvalidFrameId);
                }
            }
        })
    }

    fn take_remaining(&mut self) -> Bytes {
        let mut x = mem::replace(self.bytes.get_mut(), Bytes::new());
        x.advance(self.bytes.position() as usize);
        self.bytes.set_position(0);
        x
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
                self.bytes = io::Cursor::new(Bytes::new());
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
    pub(crate) ty: Option<FrameType>,
    pub(crate) reason: &'static str,
}

impl From<InvalidFrame> for TransportError {
    fn from(err: InvalidFrame) -> Self {
        let mut te = Self::FRAME_ENCODING_ERROR(err.reason);
        te.frame = err.ty;
        te
    }
}

fn scan_ack_blocks(buf: &mut io::Cursor<Bytes>, largest: u64, n: usize) -> Result<(), IterErr> {
    let first_block = buf.get_var()?;
    let mut smallest = largest.checked_sub(first_block).ok_or(IterErr::Malformed)?;
    for _ in 0..n {
        let gap = buf.get_var()?;
        smallest = smallest.checked_sub(gap + 2).ok_or(IterErr::Malformed)?;
        let block = buf.get_var()?;
        smallest = smallest.checked_sub(block).ok_or(IterErr::Malformed)?;
    }
    Ok(())
}

#[derive(Debug)]
enum IterErr {
    UnexpectedEnd,
    InvalidFrameId,
    Malformed,
}

impl IterErr {
    fn reason(&self) -> &'static str {
        use self::IterErr::*;
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
    data: io::Cursor<&'a [u8]>,
}

impl<'a> AckIter<'a> {
    fn new(largest: u64, payload: &'a [u8]) -> Self {
        let data = io::Cursor::new(payload);
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
        out.write(FrameType::RESET_STREAM); // 1 byte
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
        out.write(FrameType::STOP_SENDING); // 1 byte
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
            FrameType::PATH_NEW_CONNECTION_ID
        } else {
            FrameType::NEW_CONNECTION_ID
        }
    }

    /// Returns the maximum encoded size on the wire.
    ///
    /// This is a rough upper estimate, does not squeeze every last byte out.
    pub(crate) fn size_bound(path_new_cid: bool, cid_len: usize) -> usize {
        let type_id = match path_new_cid {
            true => FrameType::PATH_NEW_CONNECTION_ID.0,
            false => FrameType::NEW_CONNECTION_ID.0,
        };
        let type_len = VarInt::try_from(type_id).unwrap().size();
        let path_id_len = match path_new_cid {
            true => VarInt::from(u32::MAX).size(),
            false => 0,
        };
        let seq_max_len = 8usize;
        let retire_prior_to_max_len = 8usize;
        let cid_len = 1 + cid_len;
        let reset_token_len = 16;
        type_len + path_id_len + seq_max_len + retire_prior_to_max_len + cid_len + reset_token_len
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
    pub(crate) fn encode(&self, length: bool, out: &mut Vec<u8>) {
        out.write(FrameType(*DATAGRAM_TYS.start() | u64::from(length))); // 1 byte
        if length {
            // Safe to unwrap because we check length sanity before queueing datagrams
            out.write(VarInt::from_u64(self.data.len() as u64).unwrap()); // <= 8 bytes
        }
        out.extend_from_slice(&self.data);
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
        buf.write(FrameType::ACK_FREQUENCY);
        buf.write(self.sequence);
        buf.write(self.ack_eliciting_threshold);
        buf.write(self.request_max_ack_delay);
        buf.write(self.reordering_threshold);
    }
}

/* Address Discovery https://datatracker.ietf.org/doc/draft-seemann-quic-address-discovery/ */

/// Conjuction of the information contained in the address discovery frames
/// ([`FrameType::OBSERVED_IPV4_ADDR`], [`FrameType::OBSERVED_IPV6_ADDR`]).
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
            FrameType::OBSERVED_IPV6_ADDR
        } else {
            FrameType::OBSERVED_IPV4_ADDR
        }
    }

    /// Compute the number of bytes needed to encode the frame.
    pub(crate) fn size(&self) -> usize {
        let type_size = VarInt(self.get_type().0).size();
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
    /// Should only be called when the fram type has been identified as
    /// [`FrameType::OBSERVED_IPV4_ADDR`] or [`FrameType::OBSERVED_IPV6_ADDR`].
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

// TODO(@divma): AbandonPath? PathAbandon is the name in the spec....
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct PathAbandon {
    path_id: PathId,
    // TODO(@divma): this is TransportErrorCode plus two new errors
    error_code: TransportErrorCode,
}

#[allow(dead_code)] // TODO(flub)
impl PathAbandon {
    // TODO(@divma): docs
    pub(crate) fn write<W: BufMut>(&self, buf: &mut W) {
        buf.write(FrameType::PATH_ABANDON);
        buf.write(self.path_id);
        buf.write(self.error_code);
    }

    // TODO(@divma): docs
    // should only be called after the frame type has been verified
    pub(crate) fn read<R: Buf>(bytes: &mut R) -> coding::Result<Self> {
        Ok(Self {
            path_id: bytes.get()?,
            error_code: bytes.get()?,
        })
    }
}

// TODO(@divma): split?
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct PathAvailable {
    is_backup: bool,
    path_id: PathId,
    status_seq_no: VarInt,
}

#[allow(dead_code)] // TODO(flub)
impl PathAvailable {
    // TODO(@divma): docs
    pub(crate) fn write<W: BufMut>(&self, buf: &mut W) {
        buf.write(self.get_type());
        buf.write(self.path_id);
        buf.write(self.status_seq_no);
    }

    // TODO(@divma): docs
    // should only be called after the frame type has been verified
    pub(crate) fn read<R: Buf>(bytes: &mut R, is_backup: bool) -> coding::Result<Self> {
        Ok(Self {
            is_backup,
            path_id: bytes.get()?,
            status_seq_no: bytes.get()?,
        })
    }

    fn get_type(&self) -> FrameType {
        if self.is_backup {
            FrameType::PATH_BACKUP
        } else {
            FrameType::PATH_AVAILABLE
        }
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
    #[allow(clippy::range_plus_one)]
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
        Ack::encode(None, 42, &ranges, Some(&ECN), &mut buf);
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
        const PATH_ID: Option<PathId> = Some(PathId(u32::MAX));
        Ack::encode(PATH_ID, 42, &ranges, Some(&ECN), &mut buf);
        let frames = frames(buf);
        assert_eq!(frames.len(), 1);
        match frames[0] {
            Frame::Ack(ref ack) => {
                assert_eq!(ack.path_id, PATH_ID);
                let mut packets = ack.iter().flatten().collect::<Vec<_>>();
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
        FrameType::IMMEDIATE_ACK.encode(&mut buf);
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
        abandon.write(&mut buf);

        let mut decoded = frames(buf);
        assert_eq!(decoded.len(), 1);
        match decoded.pop().expect("non empty") {
            Frame::PathAbandon(decoded) => assert_eq!(decoded, abandon),
            x => panic!("incorrect frame {x:?}"),
        }
    }

    #[test]
    fn test_path_available_roundtrip() {
        let path_avaiable = PathAvailable {
            is_backup: true,
            path_id: PathId(42),
            status_seq_no: VarInt(73),
        };
        let mut buf = Vec::new();
        path_avaiable.write(&mut buf);

        let mut decoded = frames(buf);
        assert_eq!(decoded.len(), 1);
        match decoded.pop().expect("non empty") {
            Frame::PathAvailable(decoded) => assert_eq!(decoded, path_avaiable),
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
        retire_cid.write(&mut buf);

        let mut decoded = frames(buf);
        assert_eq!(decoded.len(), 1);
        match decoded.pop().expect("non empty") {
            Frame::RetireConnectionId(decoded) => assert_eq!(decoded, retire_cid),
            x => panic!("incorrect frame {x:?}"),
        }
    }
}
