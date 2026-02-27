use bytes::{BufMut, BytesMut};
use test_strategy::proptest;
use proptest::{prop_assert_ne, prelude::*};

use crate::{
    FrameType,
    coding::Encodable,
    frame::{
        Ack, Frame, HandshakeDone, ImmediateAck, MaybeFrame, PathAck, Ping, Stream, StreamMeta,
    },
};

impl PartialEq for Frame {
    fn eq(&self, other: &Self) -> bool {
        let mut a = Vec::new();
        let mut b = Vec::new();
        encode_frame(self, &mut a);
        encode_frame(other, &mut b);
        a == b
    }
}

fn encode_frame<B: BufMut>(frame: &Frame, buf: &mut B) {
    match frame {
        Frame::Padding => buf.put_u8(0),
        Frame::Ping => Ping.encode(buf),
        Frame::Ack(a) => Ack::encoder(a.delay, &a.ranges, a.ecn.as_ref()).encode(buf),
        Frame::PathAck(pa) => {
            PathAck::encoder(pa.path_id, pa.delay, &pa.ranges, pa.ecn.as_ref()).encode(buf)
        }
        Frame::ResetStream(rs) => rs.encode(buf),
        Frame::StopSending(ss) => ss.encode(buf),
        Frame::Crypto(c) => c.encode(buf),
        Frame::NewToken(nt) => nt.encode(buf),
        Frame::Stream(s) => encode_stream(s, buf),
        Frame::MaxData(md) => md.encode(buf),
        Frame::MaxStreamData(msd) => msd.encode(buf),
        Frame::MaxStreams(ms) => ms.encode(buf),
        Frame::DataBlocked(db) => db.encode(buf),
        Frame::StreamDataBlocked(sdb) => sdb.encode(buf),
        Frame::StreamsBlocked(sb) => sb.encode(buf),
        Frame::NewConnectionId(nc) => nc.encode(buf),
        Frame::RetireConnectionId(rci) => rci.encode(buf),
        Frame::PathChallenge(pc) => pc.encode(buf),
        Frame::PathResponse(pr) => pr.encode(buf),
        Frame::Close(c) => c.encoder(usize::MAX).encode(buf),
        Frame::Datagram(dg) => dg.encode(buf),
        Frame::AckFrequency(af) => af.encode(buf),
        Frame::ImmediateAck => ImmediateAck.encode(buf),
        Frame::HandshakeDone => HandshakeDone.encode(buf),
        Frame::ObservedAddr(oa) => oa.encode(buf),
        Frame::PathAbandon(pa) => pa.encode(buf),
        Frame::PathStatusAvailable(psa) => psa.encode(buf),
        Frame::PathStatusBackup(psb) => psb.encode(buf),
        Frame::MaxPathId(mpi) => mpi.encode(buf),
        Frame::PathsBlocked(pb) => pb.encode(buf),
        Frame::PathCidsBlocked(pcb) => pcb.encode(buf),
        Frame::AddAddress(aa) => aa.encode(buf),
        Frame::ReachOut(ro) => ro.encode(buf),
        Frame::RemoveAddress(ra) => ra.encode(buf),
    }
}

fn encode_stream<B: BufMut>(s: &Stream, buf: &mut B) {
    let meta = StreamMeta {
        id: s.id,
        offsets: s.offset..s.offset + s.data.len() as u64,
        fin: s.fin,
    };
    meta.encoder(true).encode(buf);
    buf.put_slice(&s.data);
}

#[proptest]
fn encode_decode_roundtrip(
    #[strategy(any::<Frame>().prop_filter("no padding", |frame| frame.ty() != FrameType::Padding))]
    frame: Frame,
) {
    let mut encoded = BytesMut::new();
    encode_frame(&frame, &mut encoded);
    let mut iter = crate::frame::Iter::new(encoded.freeze()).unwrap();
    let decoded = iter.next().unwrap().unwrap();
    assert_eq!(decoded, frame);
    assert!(iter.take_remaining().is_empty());
}

#[proptest]
fn maybe_frame_known_never_padding(frame: MaybeFrame) {
    // MaybeFrame::Known should never contain FrameType::Padding
    if let MaybeFrame::Known(ft) = frame {
        prop_assert_ne!(ft, FrameType::Padding);
    }
}
