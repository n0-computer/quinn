use std::{collections::VecDeque, ops::Range};

use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::{VarInt, connection::streams::BytesOrSlice, range_set::ArrayRangeSet};

/// Buffer of outgoing retransmittable stream data
#[derive(Default, Debug)]
pub(super) struct SendBuffer {
    /// Data queued by the application that has to be retained for resends.
    ///
    /// Only data up to the highest contiguous acknowledged offset can be discarded.
    /// We could discard acknowledged in this buffer, but it would require a more
    /// complex data structure. Instead, we track acknowledged ranges in `acks`.
    ///
    /// Data keeps track of the base offset of the buffered data.
    data: SendBufferData,
    /// The first offset that hasn't been sent even once
    ///
    /// Always lies in `data.range()`
    unsent: u64,
    /// Acknowledged ranges which couldn't be discarded yet as they don't include the earliest
    /// offset in `unacked`
    ///
    /// All ranges must be within `data.range().start..(data.range().end - unsent)`, since data
    /// that has never been sent can't be acknowledged.
    // TODO: Recover storage from these by compacting (#700)
    acks: ArrayRangeSet,
    /// Previously transmitted ranges deemed lost and marked for retransmission
    ///
    /// All ranges must be within `data.range().start..(data.range().end - unsent)`, since data
    /// that has never been sent can't be retransmitted.
    ///
    /// This should usually not overlap with `acks`, but this is not strictly enforced.
    retransmits: ArrayRangeSet,
}

/// Maximum number of bytes to combine into a single segment
///
/// Any segment larger than this will be stored as-is, possibly triggering a flush of the buffer.
const MAX_COMBINE: usize = 1452;

/// This is where the data of the send buffer lives. It supports appending at the end,
/// removing from the front, and retrieving data by range.
#[derive(Default, Debug)]
struct SendBufferData {
    /// Start offset of the buffered data
    offset: u64,
    /// Total size of [`Self::segments`] and [`Self::last_segment`]
    len: usize,
    /// Buffered data segments
    segments: VecDeque<Bytes>,
    /// Last segment, possibly empty
    last_segment: BytesMut,
}

impl SendBufferData {
    /// Total size of buffered data
    fn len(&self) -> usize {
        self.len
    }

    /// Range of buffered data
    #[inline(always)]
    fn range(&self) -> Range<u64> {
        self.offset..self.offset + self.len as u64
    }

    /// Append data to the end of the buffer
    fn append<'a>(&'a mut self, data: impl BytesOrSlice<'a>) {
        self.len += data.len();
        if data.len() > MAX_COMBINE {
            // use in place
            if !self.last_segment.is_empty() {
                self.segments.push_back(self.last_segment.split().freeze());
            }
            self.segments.push_back(data.into_bytes());
        } else {
            // copy
            let rest = if self.last_segment.len() + data.len() > MAX_COMBINE
                && !self.last_segment.is_empty()
            {
                // fill up last_segment up to MAX_COMBINE and flush
                let capacity = MAX_COMBINE.saturating_sub(self.last_segment.len());
                let (curr, rest) = data.as_ref().split_at(capacity);
                self.last_segment.put_slice(curr);
                self.segments.push_back(self.last_segment.split().freeze());
                rest
            } else {
                data.as_ref()
            };
            // copy the rest into the now empty last_segment
            self.last_segment.extend_from_slice(rest);
        }
    }

    /// Discard data from the front of the buffer
    ///
    /// Calling this with n > len() is allowed and will simply clear the buffer.
    fn pop_front(&mut self, n: usize) {
        let mut n = n.min(self.len);
        self.len -= n;
        self.offset += n as u64;
        while n > 0 {
            // segments is empty, which leaves only last_segment
            let Some(front) = self.segments.front_mut() else {
                break;
            };
            if front.len() <= n {
                // Remove the whole front segment
                n -= front.len();
                self.segments.pop_front();
            } else {
                // Advance within the front segment
                front.advance(n);
                n = 0;
            }
        }
        // the rest has to be in the last segment
        self.last_segment.advance(n);
        // shrink segments if we have a lot of unused capacity
        if self.segments.len() * 4 < self.segments.capacity() {
            self.segments.shrink_to_fit();
        }
    }

    /// Iterator over all segments in order
    ///
    /// Concatenates `segments` and `last_segment` so they can be handled uniformly
    fn segments_iter(&self) -> impl Iterator<Item = &[u8]> {
        self.segments
            .iter()
            .map(|x| x.as_ref())
            .chain(std::iter::once(self.last_segment.as_ref()))
    }

    /// Returns data which is associated with a range
    ///
    /// Requesting a range outside of the buffered data will panic.
    #[cfg(any(test, feature = "bench"))]
    fn get(&self, offsets: Range<u64>) -> &[u8] {
        assert!(
            offsets.start >= self.range().start && offsets.end <= self.range().end,
            "Requested range is outside of buffered data"
        );
        // translate to segment-relative offsets and usize
        let offsets = Range {
            start: (offsets.start - self.offset) as usize,
            end: (offsets.end - self.offset) as usize,
        };
        let mut segment_offset = 0;
        for segment in self.segments_iter() {
            if offsets.start >= segment_offset && offsets.start < segment_offset + segment.len() {
                let start = offsets.start - segment_offset;
                let end = offsets.end - segment_offset;

                return &segment[start..end.min(segment.len())];
            }
            segment_offset += segment.len();
        }

        unreachable!("impossible if segments and range are consistent");
    }

    fn get_into(&self, offsets: Range<u64>, buf: &mut impl BufMut) {
        assert!(
            offsets.start >= self.range().start && offsets.end <= self.range().end,
            "Requested range is outside of buffered data"
        );
        // translate to segment-relative offsets and usize
        let offsets = Range {
            start: (offsets.start - self.offset) as usize,
            end: (offsets.end - self.offset) as usize,
        };
        let mut segment_offset = 0;
        for segment in self.segments_iter() {
            // intersect segment range with requested range
            let start = segment_offset.max(offsets.start);
            let end = (segment_offset + segment.len()).min(offsets.end);
            if start < end {
                // slice range intersects with requested range
                buf.put_slice(&segment[start - segment_offset..end - segment_offset]);
            }
            segment_offset += segment.len();
            if segment_offset >= offsets.end {
                // we are beyond the requested range
                break;
            }
        }
    }

    #[cfg(test)]
    fn to_vec(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(self.len);
        for segment in self.segments_iter() {
            result.extend_from_slice(segment);
        }
        result
    }
}

impl SendBuffer {
    /// Construct an empty buffer at the initial offset
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// Append application data to the end of the stream
    pub(super) fn write<'a>(&'a mut self, data: impl BytesOrSlice<'a>) {
        self.data.append(data);
    }

    /// Discard a range of acknowledged stream data
    pub(super) fn ack(&mut self, mut range: Range<u64>) {
        // Clamp the range to data which is still tracked
        let base_offset = self.fully_acked_offset();
        range.start = base_offset.max(range.start);
        range.end = base_offset.max(range.end);

        self.acks.insert(range);

        while self.acks.min() == Some(self.fully_acked_offset()) {
            let prefix = self.acks.pop_min().unwrap();
            let to_advance = (prefix.end - prefix.start) as usize;
            self.data.pop_front(to_advance);
        }

        // Remove retransmit ranges which have been acknowledged
        //
        // We have to do this since we have just dropped the data, and asking
        // for non-present data would be an error.
        self.retransmits.remove(0..self.fully_acked_offset());
    }

    /// Compute the next range to transmit on this stream and update state to account for that
    /// transmission.
    ///
    /// `max_len` here includes the space which is available to transmit the
    /// offset and length of the data to send. The caller has to guarantee that
    /// there is at least enough space available to write maximum-sized metadata
    /// (8 byte offset + 8 byte length).
    ///
    /// The method returns a tuple:
    /// - The first return value indicates the range of data to send
    /// - The second return value indicates whether the length needs to be encoded
    ///   in the STREAM frames metadata (`true`), or whether it can be omitted
    ///   since the selected range will fill the whole packet.
    pub(super) fn poll_transmit(&mut self, mut max_len: usize) -> (Range<u64>, bool) {
        debug_assert!(max_len >= 8 + 8);
        let mut encode_length = false;

        if let Some(range) = self.retransmits.pop_min() {
            // Retransmit sent data

            // When the offset is known, we know how many bytes are required to encode it.
            // Offset 0 requires no space
            if range.start != 0 {
                max_len -= VarInt::size(unsafe { VarInt::from_u64_unchecked(range.start) });
            }
            if range.end - range.start < max_len as u64 {
                encode_length = true;
                max_len -= 8;
            }

            let end = range.end.min((max_len as u64).saturating_add(range.start));
            if end != range.end {
                self.retransmits.insert(end..range.end);
            }
            return (range.start..end, encode_length);
        }

        // Transmit new data

        // When the offset is known, we know how many bytes are required to encode it.
        // Offset 0 requires no space
        if self.unsent != 0 {
            max_len -= VarInt::size(unsafe { VarInt::from_u64_unchecked(self.unsent) });
        }
        if self.offset() - self.unsent < max_len as u64 {
            encode_length = true;
            max_len -= 8;
        }

        let end = self
            .offset()
            .min((max_len as u64).saturating_add(self.unsent));
        let result = self.unsent..end;
        self.unsent = end;
        (result, encode_length)
    }

    /// Returns data which is associated with a range
    ///
    /// This function can return a subset of the range, if the data is stored
    /// in noncontiguous fashion in the send buffer. In this case callers
    /// should call the function again with an incremented start offset to
    /// retrieve more data.
    #[cfg(any(test, feature = "bench"))]
    pub(super) fn get(&self, offsets: Range<u64>) -> &[u8] {
        self.data.get(offsets)
    }

    pub(super) fn get_into(&self, offsets: Range<u64>, buf: &mut impl BufMut) {
        self.data.get_into(offsets, buf)
    }

    /// Queue a range of sent but unacknowledged data to be retransmitted
    pub(super) fn retransmit(&mut self, mut range: Range<u64>) {
        debug_assert!(range.end <= self.unsent, "unsent data can't be lost");
        // don't allow retransmitting data that has already been fully acknowledged,
        // since we don't have it anymore.
        //
        // Note that we do allow retransmitting data that has been acknowledged
        // for simplicity. Not doing so would require clipping the range against
        // all acknowledged ranges.
        range.start = range.start.max(self.fully_acked_offset());
        self.retransmits.insert(range);
    }

    pub(super) fn retransmit_all_for_0rtt(&mut self) {
        // check that we still got all data - we didn't get any acks.
        debug_assert_eq!(self.fully_acked_offset(), 0);
        self.unsent = 0;
    }

    /// Offset up to which all data has been acknowledged
    fn fully_acked_offset(&self) -> u64 {
        self.data.range().start
    }

    /// First stream offset unwritten by the application, i.e. the offset that the next write will
    /// begin at
    pub(super) fn offset(&self) -> u64 {
        self.data.range().end
    }

    /// Whether all sent data has been acknowledged
    pub(super) fn is_fully_acked(&self) -> bool {
        self.data.len() == 0
    }

    /// Whether there's data to send
    ///
    /// There may be sent unacknowledged data even when this is false.
    pub(super) fn has_unsent_data(&self) -> bool {
        self.unsent != self.offset() || !self.retransmits.is_empty()
    }

    /// Compute the amount of data that hasn't been acknowledged
    pub(super) fn unacked(&self) -> u64 {
        self.data.len() as u64 - self.acks.iter().map(|x| x.end - x.start).sum::<u64>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fragment_with_length() {
        let mut buf = SendBuffer::new();
        const MSG: &[u8] = b"Hello, world!";
        buf.write(MSG);
        // 0 byte offset => 19 bytes left => 13 byte data isn't enough
        // with 8 bytes reserved for length 11 payload bytes will fit
        assert_eq!(buf.poll_transmit(19), (0..11, true));
        assert_eq!(
            buf.poll_transmit(MSG.len() + 16 - 11),
            (11..MSG.len() as u64, true)
        );
        assert_eq!(
            buf.poll_transmit(58),
            (MSG.len() as u64..MSG.len() as u64, true)
        );
    }

    #[test]
    fn fragment_without_length() {
        let mut buf = SendBuffer::new();
        const MSG: &[u8] = b"Hello, world with some extra data!";
        buf.write(MSG);
        // 0 byte offset => 19 bytes left => can be filled by 34 bytes payload
        assert_eq!(buf.poll_transmit(19), (0..19, false));
        assert_eq!(
            buf.poll_transmit(MSG.len() - 19 + 1),
            (19..MSG.len() as u64, false)
        );
        assert_eq!(
            buf.poll_transmit(58),
            (MSG.len() as u64..MSG.len() as u64, true)
        );
    }

    #[test]
    fn reserves_encoded_offset() {
        let mut buf = SendBuffer::new();

        // Pretend we have more than 1 GB of data in the buffer
        let chunk: Bytes = Bytes::from_static(&[0; 1024 * 1024]);
        for _ in 0..1025 {
            buf.write(chunk.clone());
        }

        const SIZE1: u64 = 64;
        const SIZE2: u64 = 16 * 1024;
        const SIZE3: u64 = 1024 * 1024 * 1024;

        // Offset 0 requires no space
        assert_eq!(buf.poll_transmit(16), (0..16, false));
        buf.retransmit(0..16);
        assert_eq!(buf.poll_transmit(16), (0..16, false));
        let mut transmitted = 16u64;

        // Offset 16 requires 1 byte
        assert_eq!(
            buf.poll_transmit((SIZE1 - transmitted + 1) as usize),
            (transmitted..SIZE1, false)
        );
        buf.retransmit(transmitted..SIZE1);
        assert_eq!(
            buf.poll_transmit((SIZE1 - transmitted + 1) as usize),
            (transmitted..SIZE1, false)
        );
        transmitted = SIZE1;

        // Offset 64 requires 2 bytes
        assert_eq!(
            buf.poll_transmit((SIZE2 - transmitted + 2) as usize),
            (transmitted..SIZE2, false)
        );
        buf.retransmit(transmitted..SIZE2);
        assert_eq!(
            buf.poll_transmit((SIZE2 - transmitted + 2) as usize),
            (transmitted..SIZE2, false)
        );
        transmitted = SIZE2;

        // Offset 16384 requires requires 4 bytes
        assert_eq!(
            buf.poll_transmit((SIZE3 - transmitted + 4) as usize),
            (transmitted..SIZE3, false)
        );
        buf.retransmit(transmitted..SIZE3);
        assert_eq!(
            buf.poll_transmit((SIZE3 - transmitted + 4) as usize),
            (transmitted..SIZE3, false)
        );
        transmitted = SIZE3;

        // Offset 1GB requires 8 bytes
        assert_eq!(
            buf.poll_transmit(chunk.len() + 8),
            (transmitted..transmitted + chunk.len() as u64, false)
        );
        buf.retransmit(transmitted..transmitted + chunk.len() as u64);
        assert_eq!(
            buf.poll_transmit(chunk.len() + 8),
            (transmitted..transmitted + chunk.len() as u64, false)
        );
    }

    /// tests that large segments are copied as-is in the SendBuffer
    #[test]
    fn multiple_large_segments() {
        // this must be bigger than MAX_COMBINE so we don't get writes coalesced.
        const N: usize = 2000;
        const K: u64 = N as u64;
        fn dup(data: &[u8]) -> Bytes {
            let mut buf = BytesMut::with_capacity(data.len() * N);
            for c in data {
                for _ in 0..N {
                    buf.put_u8(*c);
                }
            }
            buf.freeze()
        }

        fn same(a: &[u8], b: &[u8]) -> bool {
            // surprisingly, eq also checks the fat pointer metadata aka length
            std::ptr::eq(a.as_ptr(), b.as_ptr())
        }

        let mut buf = SendBuffer::new();
        let msg: Bytes = dup(b"Hello, world!");
        let msg_len: u64 = msg.len() as u64;

        let seg1: Bytes = dup(b"He");
        buf.write(seg1.clone());
        let seg2: Bytes = dup(b"llo,");
        buf.write(seg2.clone());
        let seg3: Bytes = dup(b" w");
        buf.write(seg3.clone());
        let seg4: Bytes = dup(b"o");
        buf.write(seg4.clone());
        let seg5: Bytes = dup(b"rld!");
        buf.write(seg5.clone());
        assert_eq!(aggregate_unacked(&buf), msg);
        // Check that the segments were stored as-is
        assert!(same(buf.get(0..5 * K), &seg1));
        assert!(same(buf.get(2 * K..8 * K), &seg2));
        assert!(same(buf.get(6 * K..8 * K), &seg3));
        assert!(same(buf.get(8 * 2000..msg_len), &seg4));
        assert!(same(buf.get(9 * 2000..msg_len), &seg5));
        // Now drain the segments
        buf.ack(0..K);
        assert_eq!(aggregate_unacked(&buf), &msg[N..]);
        buf.ack(0..3 * K);
        assert_eq!(aggregate_unacked(&buf), &msg[3 * N..]);
        buf.ack(3 * K..5 * K);
        assert_eq!(aggregate_unacked(&buf), &msg[5 * N..]);
        // ack with gap, doesn't free anything
        buf.ack(7 * K..9 * K);
        assert_eq!(aggregate_unacked(&buf), &msg[5 * N..]);
        // fill the gap, free up to 9 K
        buf.ack(4 * K..7 * K);
        assert_eq!(aggregate_unacked(&buf), &msg[9 * N..]);
        // ack all
        buf.ack(0..msg_len);
        assert_eq!(aggregate_unacked(&buf), &[] as &[u8]);
    }

    #[test]
    fn retransmit() {
        let mut buf = SendBuffer::new();
        const MSG: &[u8] = b"Hello, world with extra data!";
        buf.write(MSG);
        // Transmit two frames
        assert_eq!(buf.poll_transmit(16), (0..16, false));
        assert_eq!(buf.poll_transmit(16), (16..23, true));
        // Lose the first, but not the second
        buf.retransmit(0..16);
        // Ensure we only retransmit the lost frame, then continue sending fresh data
        assert_eq!(buf.poll_transmit(16), (0..16, false));
        assert_eq!(buf.poll_transmit(16), (23..MSG.len() as u64, true));
        // Lose the second frame
        buf.retransmit(16..23);
        assert_eq!(buf.poll_transmit(16), (16..23, true));
    }

    #[test]
    fn ack() {
        let mut buf = SendBuffer::new();
        const MSG: &[u8] = b"Hello, world!";
        buf.write(MSG);
        assert_eq!(buf.poll_transmit(16), (0..8, true));
        buf.ack(0..8);
        assert_eq!(aggregate_unacked(&buf), &MSG[8..]);
    }

    #[test]
    fn reordered_ack() {
        let mut buf = SendBuffer::new();
        const MSG: &[u8] = b"Hello, world with extra data!";
        buf.write(MSG);
        assert_eq!(buf.poll_transmit(16), (0..16, false));
        assert_eq!(buf.poll_transmit(16), (16..23, true));
        buf.ack(16..23);
        assert_eq!(aggregate_unacked(&buf), MSG);
        buf.ack(0..16);
        assert_eq!(aggregate_unacked(&buf), &MSG[23..]);
        assert!(buf.acks.is_empty());
    }

    fn aggregate_unacked(buf: &SendBuffer) -> Vec<u8> {
        buf.data.to_vec()
    }

    #[test]
    #[should_panic(expected = "Requested range is outside of buffered data")]
    fn send_buffer_get_out_of_range() {
        let data = SendBufferData::default();
        data.get(0..1);
    }

    #[test]
    #[should_panic(expected = "Requested range is outside of buffered data")]
    fn send_buffer_get_into_out_of_range() {
        let data = SendBufferData::default();
        let mut buf = Vec::new();
        data.get_into(0..1, &mut buf);
    }
}

#[cfg(all(test, not(target_family = "wasm")))]
mod proptests {
    use super::*;

    use proptest::prelude::*;
    use test_strategy::{Arbitrary, proptest};
    use crate::tests::subscribe;
    use tracing::trace;

    #[derive(Debug, Clone, Arbitrary)]
    enum Op {
        // write the given bytes
        Write(#[strategy(proptest::collection::vec(any::<u8>(), 0..1024))] Vec<u8>),
        // ack a random range
        Ack(Range<u64>),
        // retransmit a random range
        Retransmit(Range<u64>),
        // poll_transmit with the given max len
        PollTransmit(#[strategy(16usize..1024)] usize),
    }

    /// Map a range into a target range
    ///
    /// If the target range is empty, it will be returned as is.
    /// For a non-empty target range, 0 in the input range will be mapped to
    /// the start of the target range, and the input range will wrap around
    /// the target range as needed.
    fn map_range(input: Range<u64>, target: Range<u64>) -> Range<u64> {
        if target.is_empty() {
            return target;
        }
        let size = target.end - target.start;
        let a = target.start + (input.start % size);
        let b = target.start + (input.end % size);
        a.min(b)..a.max(b)
    }

    #[proptest]
    fn send_buffer_matches_reference(
        #[strategy(proptest::collection::vec(any::<Op>(), 1..100))] ops: Vec<Op>,
    ) {
        let _guard = subscribe();
        let mut sb = SendBuffer::new();
        // all data written to the send buffer
        let mut buf = Vec::new();
        // max offset that has been returned by poll_transmit
        let mut max_send_offset = 0u64;
        // max offset up to which data has been fully acked
        let mut max_full_send_offset = 0u64;
        trace!("");
        for op in ops {
            match op {
                Op::Write(data) => {
                    trace!("Op::Write({})", data.len());
                    buf.extend_from_slice(&data);
                    sb.write(Bytes::from(data));
                }
                Op::Ack(range) => {
                    // we can only get acks for data that has been sent
                    let range = map_range(range, 0..max_send_offset);
                    // update fully acked range
                    if range.contains(&max_full_send_offset) {
                        max_full_send_offset = range.end;
                    }
                    trace!("Op::Ack({:?})", range);
                    sb.ack(range);
                }
                Op::Retransmit(range) => {
                    // we can only get retransmits for data that has been sent
                    let range = map_range(range, 0..max_send_offset);
                    trace!("Op::Retransmit({:?})", range);
                    sb.retransmit(range);
                }
                Op::PollTransmit(max_len) => {
                    trace!("Op::PollTransmit({})", max_len);
                    let (range, _partial) = sb.poll_transmit(max_len);
                    max_send_offset = max_send_offset.max(range.end);
                    assert!(
                        range.start >= max_full_send_offset,
                        "poll_transmit returned already fully acked data: range={:?}, max_full_send_offset={}",
                        range,
                        max_full_send_offset
                    );

                    let mut t1 = Vec::new();
                    sb.get_into(range.clone(), &mut t1);

                    let mut t2 = Vec::new();
                    t2.extend_from_slice(&buf[range.start as usize..range.end as usize]);

                    assert_eq!(t1, t2, "Data mismatch for range {:?}", range);
                }
            }
        }
        // Drain all remaining data
        trace!("Op::Retransmit({:?})", 0..max_send_offset);
        sb.retransmit(0..max_send_offset);
        loop {
            trace!("Op::PollTransmit({})", 1024);
            let (range, _partial) = sb.poll_transmit(1024);
            if range.is_empty() {
                break;
            }
            trace!("Op::Ack({:?})", range);
            sb.ack(range);
        }
        assert!(
            sb.is_fully_acked(),
            "SendBuffer not fully acked at end of ops"
        );
    }
}

#[cfg(feature = "bench")]
pub mod send_buffer_benches {
    //! Bench fns for SendBuffer
    //!
    //! These are defined here and re-exported via `bench_exports` in lib.rs,
    //! so we can access the private `SendBuffer` struct.
    use bytes::Bytes;
    use criterion::Criterion;

    use super::SendBuffer;

    /// Pathological case: many segments, get from end
    pub fn get_into_many_segments(criterion: &mut Criterion) {
        let mut group = criterion.benchmark_group("get_into_many_segments");
        let mut buf = SendBuffer::new();

        const SEGMENTS: u64 = 10000;
        const SEGMENT_SIZE: u64 = 10;
        const PACKET_SIZE: u64 = 1200;
        const BYTES: u64 = SEGMENTS * SEGMENT_SIZE;

        // 10000 segments of 10 bytes each = 100KB total (same data size)
        for i in 0..SEGMENTS {
            buf.write(Bytes::from(vec![i as u8; SEGMENT_SIZE as usize]));
        }

        let mut tgt = Vec::with_capacity(PACKET_SIZE as usize);
        group.bench_function("get_into", |b| {
            b.iter(|| {
                // Get from end (very slow - scans through all 1000 segments)
                tgt.clear();
                buf.get_into(BYTES - PACKET_SIZE..BYTES, std::hint::black_box(&mut tgt));
            });
        });
    }

    /// Get segments in the old way, using a loop of get calls
    pub fn get_loop_many_segments(criterion: &mut Criterion) {
        let mut group = criterion.benchmark_group("get_loop_many_segments");
        let mut buf = SendBuffer::new();

        const SEGMENTS: u64 = 10000;
        const SEGMENT_SIZE: u64 = 10;
        const PACKET_SIZE: u64 = 1200;
        const BYTES: u64 = SEGMENTS * SEGMENT_SIZE;

        // 10000 segments of 10 bytes each = 100KB total (same data size)
        for i in 0..SEGMENTS {
            buf.write(Bytes::from(vec![i as u8; SEGMENT_SIZE as usize]));
        }

        let mut tgt = Vec::with_capacity(PACKET_SIZE as usize);
        group.bench_function("get_loop", |b| {
            b.iter(|| {
                // Get from end (very slow - scans through all 1000 segments)
                tgt.clear();
                let mut range = BYTES - PACKET_SIZE..BYTES;
                while range.start < range.end {
                    let slice = std::hint::black_box(buf.get(range.clone()));
                    range.start += slice.len() as u64;
                    tgt.extend_from_slice(slice);
                }
            });
        });
    }
}
