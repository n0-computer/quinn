
use bytes::{BufMut, Bytes, BytesMut};
use proptest::{prelude::*};
use ring::hmac;
use crate::{
    ApplicationClose, ConnectionId, Datagram, FrameType, MAX_CID_SIZE, PathId, StreamId,
    TransportErrorCode, VarInt,
    coding::Encodable,
    frame::{
        Close, MaybeFrame, NewConnectionId, ResetStream, StopSending, Stream, PathChallenge,
        PathResponse, MaxPathId, PathsBlocked, PathCidsBlocked, PathAbandon, PathStatusAvailable,
        PathStatusBackup, MaxData, MaxStreamData,
    },
    token::ResetToken,
};

use super::frame::{Frame, ConnectionClose};

fn valid_varint_u64() -> impl Strategy<Value = u64> {
    any::<VarInt>().prop_map(|v| v.0)
}

fn connection_close() -> impl Strategy<Value = ConnectionClose> {
    (
        any::<u8>().prop_map(|x| TransportErrorCode::crypto(x)),
        any::<MaybeFrame>(),
        ".*".prop_map(|s| Bytes::from(s)),
    )
        .prop_map(|(error_code, frame_type, reason)| ConnectionClose {
            error_code,
            frame_type,
            reason,
        })
}

fn application_close() -> impl Strategy<Value = ApplicationClose> {
    (any::<VarInt>(), ".*".prop_map(|s| Bytes::from(s)))
        .prop_map(|(error_code, reason)| ApplicationClose { error_code, reason })
}

fn stream() -> impl Strategy<Value = Stream> {
    (
        valid_varint_u64(),
        valid_varint_u64(),
        any::<bool>(),
        prop::collection::vec(any::<u8>(), 0..100),
    )
        .prop_map(|(stream_id, offset, fin, data)| Stream {
            id: StreamId(stream_id),
            offset,
            fin,
            data: Bytes::from(data),
        })
}

fn connection_id() -> impl Strategy<Value = ConnectionId> {
    prop::collection::vec(any::<u8>(), 1..=MAX_CID_SIZE).prop_map(|bytes| ConnectionId::new(&bytes))
}

fn connection_id_and_reset_token() -> impl Strategy<Value = (ConnectionId, ResetToken)> {
    (connection_id(), any::<[u8; 64]>()).prop_map(|(id, reset_key)| {
        let key = hmac::Key::new(hmac::HMAC_SHA256, &reset_key);
        (id, ResetToken::new(&key, id))
    })
}

fn new_connection_id() -> impl Strategy<Value = NewConnectionId> {
    (
        any::<Option<PathId>>(),
        valid_varint_u64(),
        valid_varint_u64(),
        connection_id_and_reset_token(),
    )
        .prop_map(|(path_id, a, b, (id, reset_token))| {
            let sequence = std::cmp::max(a, b);
            let retire_prior_to = std::cmp::min(a, b);
            NewConnectionId {
                path_id,
                sequence,
                retire_prior_to,
                id,
                reset_token,
            }
        })
}

fn datagram() -> impl Strategy<Value = Datagram> {
    prop::collection::vec(any::<u8>(), 0..100).prop_map(|data| Datagram {
        data: Bytes::from(data),
    })
}

#[derive(Debug, test_strategy::Arbitrary)]
enum TestFrame {
    ConnectionClose(#[strategy(connection_close())] ConnectionClose),
    ApplicationClose(#[strategy(application_close())] ApplicationClose),
    ResetStream(ResetStream),
    StopSending(StopSending),
    NewConnectionId(#[strategy(new_connection_id())] NewConnectionId),
    Datagram(#[strategy(datagram())] Datagram),
    MaxData(MaxData),
    MaxStreamData(MaxStreamData),
    PathChallenge(PathChallenge),
    PathResponse(PathResponse),
    MaxPathId(MaxPathId),
    PathsBlocked(PathsBlocked),
    PathCidsBlocked(PathCidsBlocked),
    PathAbandon(PathAbandon),
    PathStatusAvailable(PathStatusAvailable),
    PathStatusBackup(PathStatusBackup),
}

impl TryFrom<Frame> for TestFrame {
    type Error = &'static str;
    fn try_from(frame: Frame) -> Result<Self, Self::Error> {
        match frame {
            Frame::Close(Close::Connection(cc)) => Ok(Self::ConnectionClose(cc)),
            Frame::Close(Close::Application(ac)) => Ok(Self::ApplicationClose(ac)),
            Frame::ResetStream(rs) => Ok(Self::ResetStream(rs)),
            Frame::StopSending(ss) => Ok(Self::StopSending(ss)),
            Frame::NewConnectionId(nc) => Ok(Self::NewConnectionId(nc)),
            Frame::Datagram(dg) => Ok(Self::Datagram(dg)),
            Frame::MaxData(md) => Ok(Self::MaxData(md)),
            Frame::MaxStreamData(msd) => Ok(Self::MaxStreamData(msd)),
            Frame::PathChallenge(pc) => Ok(Self::PathChallenge(pc)),
            Frame::PathResponse(pr) => Ok(Self::PathResponse(pr)),
            Frame::MaxPathId(mpi) => Ok(Self::MaxPathId(mpi)),
            Frame::PathsBlocked(pb) => Ok(Self::PathsBlocked(pb)),
            Frame::PathCidsBlocked(pcb) => Ok(Self::PathCidsBlocked(pcb)),
            Frame::PathAbandon(pa) => Ok(Self::PathAbandon(pa)),
            Frame::PathStatusAvailable(psa) => Ok(Self::PathStatusAvailable(psa)),
            Frame::PathStatusBackup(psb) => Ok(Self::PathStatusBackup(psb)),
            _ => Err("unsupported frame type"),
        }
    }
}

impl PartialEq for TestFrame {
    fn eq(&self, other: &Self) -> bool {
        let mut a = Vec::new();
        let mut b = Vec::new();
        self.encode(&mut a);
        other.encode(&mut b);
        a == b
    }
}

impl Encodable for TestFrame {
    fn encode<B: BufMut>(&self, buf: &mut B) {
        match self {
            TestFrame::ConnectionClose(cc) => cc.encode(buf, usize::MAX),
            TestFrame::ApplicationClose(ac) => ac.encode(buf, usize::MAX),
            TestFrame::ResetStream(rs) => rs.encode(buf),
            TestFrame::StopSending(ss) => ss.encode(buf),
            TestFrame::NewConnectionId(nc) => nc.encode(buf),
            TestFrame::Datagram(dg) => dg.encode(buf),
            TestFrame::MaxData(md) => md.encode(buf),
            TestFrame::MaxStreamData(msd) => msd.encode(buf),
            TestFrame::PathChallenge(pc) => pc.encode(buf),
            TestFrame::PathResponse(pr) => pr.encode(buf),
            TestFrame::MaxPathId(mpi) => mpi.encode(buf),
            TestFrame::PathsBlocked(pb) => pb.encode(buf),
            TestFrame::PathCidsBlocked(pcb) => pcb.encode(buf),
            TestFrame::PathAbandon(pa) => pa.encode(buf),
            TestFrame::PathStatusAvailable(psa) => psa.encode(buf),
            TestFrame::PathStatusBackup(psb) => psb.encode(buf),
        }
    }
}

fn frame() -> impl Strategy<Value = TestFrame> {
    prop_oneof![connection_close().prop_map(|cc| TestFrame::ConnectionClose(cc)),]
}

#[test_strategy::proptest]
fn encode_decode_roundtrip(frame: TestFrame) {
    let mut encoded = BytesMut::new();
    frame.encode(&mut encoded);
    let mut iter = crate::frame::Iter::new(encoded.freeze()).unwrap();
    let decoded = iter.next().unwrap().unwrap();
    assert_eq!(TestFrame::try_from(decoded), Ok(frame));
}

#[test_strategy::proptest]
fn maybe_frame_known_never_padding(frame: MaybeFrame) {
    // MaybeFrame::Known should never contain FrameType::Padding
    if let MaybeFrame::Known(ft) = frame {
        prop_assert_ne!(ft, FrameType::Padding);
    }
}
