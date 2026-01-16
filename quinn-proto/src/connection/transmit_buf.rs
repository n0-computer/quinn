use std::num::NonZeroUsize;

use bytes::BufMut;

use crate::{Transmit, TransmitInfo, packet::BufLen};

/// The buffer in which to write datagrams for [`Connection::poll_transmit`]
///
/// The `poll_transmit` function writes zero or more datagrams to a buffer. Multiple
/// datagrams are possible in case GSO (Generic Segmentation Offload) is supported.
///
/// This buffer tracks datagrams being written to it. There is always a "current" datagram,
/// which is started by calling [`TransmitBuf::start_new_datagram`]. Writing to the buffer
/// is done through the [`BufMut`] interface.
///
/// A GSO Batch, when more than one segment is written to it, has a GSO segment size. Segments in a
/// batch MUST be all of size equal to the GSO segment size, except for the last one.
///
/// Usually a datagram contains one QUIC packet, though QUIC-TRANSPORT 12.2 Coalescing
/// Packets allows for placing multiple packets into a single d<F3><F3>atagram provided all but the
/// last packet uses long headers. This is normally used during connection setup where often
/// the initial, handshake and sometimes even a 1-RTT packet can be coalesced into a single
/// datagram.<F3><F3>
///
/// Inside a single packet multiple QUIC frames are written.
///
/// The buffer managed here is passed straight to the OS' `sendmsg` call (or variant) once
/// `poll_transmit` returns.  So needs to contain the datagrams as they are sent on the
/// wire.
///
/// [`Connection::poll_transmit`]: super::Connection::poll_transmit
#[derive(Debug, derive_more::From)]
pub(crate) enum TransmitBuf<'a> {
    FirstSegment(FirstSegment<'a>),
    Batch(GsoBatch<'a>),
}

#[derive(Debug)]
pub(crate) struct FirstSegment<'a> {
    #[allow(unused)]
    // TODO(@divma): ideally this should be used to pad the datagram if another one is created
    pmtu: usize,
    max_segments: NonZeroUsize,
    buf: &'a mut Vec<u8>,
}

impl<'a> FirstSegment<'a> {
    fn new(buf: &'a mut Vec<u8>, max_segments: NonZeroUsize, pmtu: usize) -> Self {
        buf.clear();
        buf.reserve_exact(max_segments.get() * pmtu);
        Self {
            pmtu,
            max_segments,
            buf,
        }
    }

    pub(crate) fn new_datagram(self) -> GsoBatch<'a> {
        debug_assert!(self.buf.len() <= self.pmtu, "first segment exceeded pmtu");
        GsoBatch::new(self)
    }

    pub(crate) fn finish(self, info: TransmitInfo) -> Transmit {
        let size = self.buf.len();
        debug_assert!(size <= self.pmtu, "segment exceeded pmtu");
        Transmit::new(info, size, None)
    }
}

#[derive(Debug)]
pub(crate) struct GsoBatch<'a> {
    segment_size: usize,
    max_segments: NonZeroUsize,
    finished_segments: NonZeroUsize,
    segment_start: usize,
    buf: &'a mut Vec<u8>,
}

impl<'a> GsoBatch<'a> {
    fn new(first: FirstSegment<'a>) -> Self {
        let FirstSegment {
            pmtu: _,
            max_segments,
            buf,
        } = first;

        let segment_size = buf.len();
        // check that the first segment is not empty
        debug_assert!(segment_size > 0);
        let finished_segments = NonZeroUsize::MIN;
        // we are starting the second segment, this should not exceed max_segments
        debug_assert!(finished_segments < max_segments);
        let total_desired_capacity = max_segments.get() * segment_size;
        let needed = total_desired_capacity.saturating_sub(buf.capacity());
        buf.reserve_exact(needed);

        Self {
            segment_size,
            max_segments: first.max_segments,
            finished_segments,
            segment_start: segment_size,
            buf,
        }
    }

    pub(crate) fn new_segment(&mut self) {
        // check that segments are aligned
        debug_assert_eq!(self.buf.len() % self.segment_size, 0);
        self.finished_segments = self.finished_segments.saturating_add(1);
        self.segment_start = self.buf.len();
        // starting a new segment should not exceed max_segments
        debug_assert!(self.finished_segments < self.max_segments);
        let total_desired_capacity = self.max_segments.get() * self.segment_size;
        let needed = total_desired_capacity.saturating_sub(self.buf.capacity());
        self.buf.reserve_exact(needed);
    }

    /// Consume the batch returning the buffer length and gso segment_size
    pub(crate) fn finish(self, info: TransmitInfo) -> Transmit {
        let segments = self.finished_segments.get() + 1;
        let len = self.buf.len();
        let segment_size = self.segment_size;
        tracing::trace!(segments, len, segment_size, "finished GSO batch");
        Transmit::new(info, len, Some(segment_size))
    }
}

impl<'a> TransmitBuf<'a> {
    pub(crate) fn new(buf: &'a mut Vec<u8>, max_segments: NonZeroUsize, pmtu: usize) -> Self {
        TransmitBuf::FirstSegment(FirstSegment::new(buf, max_segments, pmtu))
    }

    fn buf(&self) -> &Vec<u8> {
        match self {
            TransmitBuf::FirstSegment(first_segment) => first_segment.buf,
            TransmitBuf::Batch(gso_batch) => gso_batch.buf,
        }
    }

    fn buf_mut(&mut self) -> &mut Vec<u8> {
        match self {
            TransmitBuf::FirstSegment(first_segment) => first_segment.buf,
            TransmitBuf::Batch(gso_batch) => gso_batch.buf,
        }
    }

    /// Returns the maximum number of datagrams allowed to be written into the buffer
    pub(super) fn max_datagrams(&self) -> NonZeroUsize {
        match self {
            TransmitBuf::FirstSegment(first_segment) => first_segment.max_segments,
            TransmitBuf::Batch(gso_batch) => gso_batch.max_segments,
        }
    }

    /// Returns the start offset of the current datagram in the buffer
    ///
    /// In other words, this offset contains the first byte of the current datagram.
    pub(super) fn datagram_start_offset(&self) -> usize {
        match self {
            TransmitBuf::FirstSegment(_) => 0,
            TransmitBuf::Batch(gso_batch) => gso_batch.segment_start,
        }
    }

    /// Returns the maximum offset in the buffer allowed for the current datagram
    ///
    /// The first and last datagram in a batch are allowed to be smaller then the maximum
    /// size. All datagrams in between need to be exactly this size.
    pub(super) fn datagram_max_offset(&self) -> usize {
        match self {
            TransmitBuf::FirstSegment(first_segment) => first_segment.pmtu,
            TransmitBuf::Batch(gso_batch) => gso_batch.segment_start + gso_batch.segment_size,
        }
    }

    /// Returns the number of bytes that may still be written into this datagram
    pub(super) fn datagram_remaining_mut(&self) -> usize {
        match self {
            TransmitBuf::FirstSegment(first_segment) => {
                first_segment.pmtu.saturating_sub(first_segment.buf.len())
            }
            TransmitBuf::Batch(gso_batch) => (gso_batch.segment_start + gso_batch.segment_size)
                .saturating_sub(gso_batch.buf.len()),
        }
    }

    /// Returns `true` if the buffer did not have anything written into it
    pub(super) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The number of bytes written into the buffer so far
    pub(super) fn len(&self) -> usize {
        self.buf().len()
    }

    /// Returns the already written bytes in the buffer
    pub(super) fn as_mut_slice(&mut self) -> &mut [u8] {
        self.buf_mut().as_mut_slice()
    }

    /// Consumes the [`TransmitBuf`] and returns the length of the buffer, number of written
    /// segments and segment_size.
    pub(crate) fn finish(self, info: TransmitInfo) -> Transmit {
        match self {
            TransmitBuf::FirstSegment(first_segment) => first_segment.finish(info),
            TransmitBuf::Batch(gso_batch) => gso_batch.finish(info),
        }
    }
}

unsafe impl BufMut for TransmitBuf<'_> {
    fn remaining_mut(&self) -> usize {
        self.buf().remaining_mut()
    }

    unsafe fn advance_mut(&mut self, cnt: usize) {
        unsafe { self.buf_mut().advance_mut(cnt) };
    }

    fn chunk_mut(&mut self) -> &mut bytes::buf::UninitSlice {
        self.buf_mut().chunk_mut()
    }
}

impl BufLen for TransmitBuf<'_> {
    fn len(&self) -> usize {
        self.len()
    }
}
