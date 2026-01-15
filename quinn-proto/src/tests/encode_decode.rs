use bytes::{BufMut, Bytes, BytesMut};
use proptest::{prelude::*};
use ring::hmac;
use std::net::IpAddr;
use crate::{
    ApplicationClose, ConnectionId, Datagram, FrameType, MAX_CID_SIZE, PathId, TransportErrorCode,
    VarInt,
    coding::{BufMutExt, Encodable},
    frame::{
        Close, MaybeFrame, NewConnectionId, ResetStream, StopSending, Stream, PathChallenge,
        PathResponse, MaxPathId, PathsBlocked, PathCidsBlocked, PathAbandon, PathStatusAvailable,
        PathStatusBackup, MaxData, MaxStreamData, Ping, ImmediateAck, HandshakeDone, MaxStreams,
        RetireConnectionId, Crypto, NewToken, AckFrequency, ObservedAddr, AddAddress, ReachOut,
        RemoveAddress, DataBlocked, StreamDataBlocked, StreamsBlocked,
    },
    token::ResetToken,
};

use super::frame::{Frame, ConnectionClose};

fn valid_varint_u64() -> impl Strategy<Value = u64> {
    any::<VarInt>().prop_map(|v| v.0)
}

fn connection_close() -> impl Strategy<Value = ConnectionClose> {
    (
        any::<u8>().prop_map(TransportErrorCode::crypto),
        any::<MaybeFrame>(),
        ".*".prop_map(Bytes::from),
    )
        .prop_map(|(error_code, frame_type, reason)| ConnectionClose {
            error_code,
            frame_type,
            reason,
        })
}

fn application_close() -> impl Strategy<Value = ApplicationClose> {
    (any::<VarInt>(), ".*".prop_map(Bytes::from))
        .prop_map(|(error_code, reason)| ApplicationClose { error_code, reason })
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

fn ip_addr() -> impl Strategy<Value = IpAddr> {
    prop_oneof![
        any::<[u8; 4]>().prop_map(IpAddr::from),
        any::<[u16; 8]>().prop_map(IpAddr::from),
    ]
}

fn observed_addr() -> impl Strategy<Value = ObservedAddr> {
    (any::<VarInt>(), ip_addr(), any::<u16>()).prop_map(|(seq_no, ip, port)| ObservedAddr {
        seq_no,
        ip,
        port,
    })
}

fn add_address() -> impl Strategy<Value = AddAddress> {
    (any::<VarInt>(), ip_addr(), any::<u16>()).prop_map(|(seq_no, ip, port)| AddAddress {
        seq_no,
        ip,
        port,
    })
}

fn reach_out() -> impl Strategy<Value = ReachOut> {
    (any::<VarInt>(), ip_addr(), any::<u16>()).prop_map(|(round, ip, port)| ReachOut {
        round,
        ip,
        port,
    })
}

#[derive(Debug, test_strategy::Arbitrary)]
enum TestFrame {
    ConnectionClose(#[strategy(connection_close())] ConnectionClose),
    ApplicationClose(#[strategy(application_close())] ApplicationClose),
    ResetStream(ResetStream),
    StopSending(StopSending),
    NewConnectionId(#[strategy(new_connection_id())] NewConnectionId),
    Datagram(Datagram),
    MaxData(MaxData),
    MaxStreamData(MaxStreamData),
    MaxStreams(MaxStreams),
    RetireConnectionId(RetireConnectionId),
    PathChallenge(PathChallenge),
    PathResponse(PathResponse),
    MaxPathId(MaxPathId),
    PathsBlocked(PathsBlocked),
    PathCidsBlocked(PathCidsBlocked),
    PathAbandon(PathAbandon),
    PathStatusAvailable(PathStatusAvailable),
    PathStatusBackup(PathStatusBackup),
    Ping(Ping),
    ImmediateAck(ImmediateAck),
    HandshakeDone(HandshakeDone),
    Crypto(Crypto),
    NewToken(NewToken),
    AckFrequency(AckFrequency),
    ObservedAddr(#[strategy(observed_addr())] ObservedAddr),
    AddAddress(#[strategy(add_address())] AddAddress),
    ReachOut(#[strategy(reach_out())] ReachOut),
    RemoveAddress(RemoveAddress),
    DataBlocked(DataBlocked),
    StreamDataBlocked(StreamDataBlocked),
    StreamsBlocked(StreamsBlocked),
    Stream(Stream),
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
            Frame::MaxStreams(ms) => Ok(Self::MaxStreams(ms)),
            Frame::RetireConnectionId(rci) => Ok(Self::RetireConnectionId(rci)),
            Frame::PathChallenge(pc) => Ok(Self::PathChallenge(pc)),
            Frame::PathResponse(pr) => Ok(Self::PathResponse(pr)),
            Frame::MaxPathId(mpi) => Ok(Self::MaxPathId(mpi)),
            Frame::PathsBlocked(pb) => Ok(Self::PathsBlocked(pb)),
            Frame::PathCidsBlocked(pcb) => Ok(Self::PathCidsBlocked(pcb)),
            Frame::PathAbandon(pa) => Ok(Self::PathAbandon(pa)),
            Frame::PathStatusAvailable(psa) => Ok(Self::PathStatusAvailable(psa)),
            Frame::PathStatusBackup(psb) => Ok(Self::PathStatusBackup(psb)),
            Frame::Ping => Ok(Self::Ping(Ping)),
            Frame::ImmediateAck => Ok(Self::ImmediateAck(ImmediateAck)),
            Frame::HandshakeDone => Ok(Self::HandshakeDone(HandshakeDone)),
            Frame::Crypto(c) => Ok(Self::Crypto(c)),
            Frame::NewToken(nt) => Ok(Self::NewToken(nt)),
            Frame::AckFrequency(af) => Ok(Self::AckFrequency(af)),
            Frame::ObservedAddr(oa) => Ok(Self::ObservedAddr(oa)),
            Frame::AddAddress(aa) => Ok(Self::AddAddress(aa)),
            Frame::ReachOut(ro) => Ok(Self::ReachOut(ro)),
            Frame::RemoveAddress(ra) => Ok(Self::RemoveAddress(ra)),
            Frame::DataBlocked(db) => Ok(Self::DataBlocked(db)),
            Frame::StreamDataBlocked(sdb) => Ok(Self::StreamDataBlocked(sdb)),
            Frame::StreamsBlocked(sb) => Ok(Self::StreamsBlocked(sb)),
            Frame::Stream(s) => Ok(Self::Stream(s)),
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
            Self::ConnectionClose(cc) => cc.encode(buf, usize::MAX),
            Self::ApplicationClose(ac) => ac.encode(buf, usize::MAX),
            Self::ResetStream(rs) => rs.encode(buf),
            Self::StopSending(ss) => ss.encode(buf),
            Self::NewConnectionId(nc) => nc.encode(buf),
            Self::Datagram(dg) => dg.encode(buf),
            Self::MaxData(md) => md.encode(buf),
            Self::MaxStreamData(msd) => msd.encode(buf),
            Self::MaxStreams(ms) => ms.encode(buf),
            Self::RetireConnectionId(rci) => rci.encode(buf),
            Self::PathChallenge(pc) => pc.encode(buf),
            Self::PathResponse(pr) => pr.encode(buf),
            Self::MaxPathId(mpi) => mpi.encode(buf),
            Self::PathsBlocked(pb) => pb.encode(buf),
            Self::PathCidsBlocked(pcb) => pcb.encode(buf),
            Self::PathAbandon(pa) => pa.encode(buf),
            Self::PathStatusAvailable(psa) => psa.encode(buf),
            Self::PathStatusBackup(psb) => psb.encode(buf),
            Self::Ping(p) => p.encode(buf),
            Self::ImmediateAck(ia) => ia.encode(buf),
            Self::HandshakeDone(hd) => hd.encode(buf),
            Self::Crypto(c) => c.encode(buf),
            Self::NewToken(nt) => nt.encode(buf),
            Self::AckFrequency(af) => af.encode(buf),
            Self::ObservedAddr(oa) => oa.encode(buf),
            Self::AddAddress(aa) => aa.encode(buf),
            Self::ReachOut(ro) => ro.encode(buf),
            Self::RemoveAddress(ra) => ra.encode(buf),
            Self::DataBlocked(db) => db.encode(buf),
            Self::StreamDataBlocked(sdb) => sdb.encode(buf),
            Self::StreamsBlocked(sb) => sb.encode(buf),
            Self::Stream(s) => {
                // Encode STREAM frame manually
                let mut ty = 0x08u8;
                if s.fin {
                    ty |= 0x01;
                }
                ty |= 0x02; // LEN bit (always set for testing)
                if s.offset != 0 {
                    ty |= 0x04; // OFF bit
                }
                buf.write_var(ty as u64);
                buf.write(s.id);
                if s.offset != 0 {
                    buf.write_var(s.offset);
                }
                buf.write_var(s.data.len() as u64);
                buf.put_slice(&s.data);
            }
        }
    }
}

fn frame() -> impl Strategy<Value = TestFrame> {
    prop_oneof![connection_close().prop_map(TestFrame::ConnectionClose),]
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
