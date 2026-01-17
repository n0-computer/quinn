use std::num::NonZeroUsize;

use bytes::BufMut;

use crate::packet::BufLen;

/// The buffer in which to write datagrams for [`Connection::poll_transmit`]
///
/// The `poll_transmit` function writes zero or more datagrams to a buffer. Multiple
/// datagrams are possible in case GSO (Generic Segmentation Offload) is supported.
///
/// This buffer tracks datagrams being written to it. There is always a "current" datagram,
/// which is started by calling [`TransmitBuf::start_new_datagram`]. Writing to the buffer
/// is done through the [`BufMut`] interface.
///
/// Usually a datagram contains one QUIC packet, though QUIC-TRANSPORT 12.2 Coalescing
/// Packets allows for placing multiple packets into a single datagram provided all but the
/// last packet uses long headers. This is normally used during connection setup where often
/// the initial, handshake and sometimes even a 1-RTT packet can be coalesced into a single
/// datagram.
///
/// Inside a single packet multiple QUIC frames are written.
///
/// The buffer managed here is passed straight to the OS' `sendmsg` call (or variant) once
/// `poll_transmit` returns.  So needs to contain the datagrams as they are sent on the
/// wire.
///
/// [`Connection::poll_transmit`]: super::Connection::poll_transmit
#[derive(Debug)]
pub(super) struct TransmitBuf<'a> {
    /// The buffer itself, packets are written to this buffer
    buf: &'a mut Vec<u8>,
    /// Offset into the buffer at which the current datagram starts
    ///
    /// Note that when coalescing packets this might be before the start of the current
    /// packet.
    datagram_start: usize,
    /// The maximum number of datagrams allowed to write into [`TransmitBuf::buf`]
    max_datagrams: NonZeroUsize,
    state: State,
}

#[derive(Debug)]
enum State {
    FirstSegment {
        pmtu: usize,
    },
    Batch {
        segment_size: NonZeroUsize,
        /// Max size for the current segment.
        ///
        /// Allways less than the segment_size.
        /// Should only be set by callers for the last segment.
        max_size: Option<NonZeroUsize>,
    },
}

impl State {
    fn batch(segment_size: NonZeroUsize, max_segment_size: Option<usize>) -> Self {
        let mut max_size = None;
        if let Some(max) = max_segment_size.and_then(NonZeroUsize::new)
            && max < segment_size
        {
            // Only apply the max segment size if it's positive and less than the segment_size
            max_size = Some(max)
        };

        Self::Batch {
            segment_size,
            max_size,
        }
    }
}

impl<'a> TransmitBuf<'a> {
    pub(super) fn new(buf: &'a mut Vec<u8>, max_datagrams: NonZeroUsize, pmtu: usize) -> Self {
        buf.clear();
        // We reserve the maximum space for sending `max_datagrams` upfront to avoid any
        // reallocations if more datagrams have to be appended later on.  Benchmarks have
        // shown a 5-10% throughput improvement compared to continuously resizing the
        // datagram buffer. While this will lead to over-allocation for small transmits
        // (e.g. purely containing ACKs), modern memory allocators (e.g. mimalloc and
        // jemalloc) will pool certain allocation sizes and therefore this is still rather
        // efficient.
        buf.reserve_exact(max_datagrams.get() * pmtu);
        Self {
            buf,
            datagram_start: 0,
            max_datagrams,
            state: State::FirstSegment { pmtu },
        }
    }

    /// Same as [`Self::new`], but reusing the existing reference.
    pub(super) fn reset(&mut self, max_datagrams: NonZeroUsize, pmtu: usize) {
        self.buf.clear();
        self.buf.reserve_exact(max_datagrams.get() * pmtu);
        self.datagram_start = 0;
        self.max_datagrams = max_datagrams;
        self.state = State::FirstSegment { pmtu };
    }

    /// Returns the number of datagrams written into the buffer
    ///
    /// The last datagram is not necessarily finished yet.
    pub(super) fn num_datagrams(&self) -> usize {
        match self.state {
            State::FirstSegment { .. } => {
                if self.buf.is_empty() {
                    0
                } else {
                    1
                }
            }
            State::Batch { segment_size, .. } => {
                let finalized_segments = self.buf.len() / segment_size.get();
                if self.buf.len() % segment_size.get() > 0 {
                    finalized_segments + 1
                } else {
                    finalized_segments
                }
            }
        }
    }

    /// Starts a new datagram in the transmit buffer
    ///
    /// If this starts the second datagram the segment size will be set to the size of the
    /// first datagram.
    ///
    /// If the underlying buffer does not have enough capacity yet this will allocate enough
    /// capacity for all the datagrams allowed in a single batch. Use
    /// [`TransmitBuf::start_new_datagram_with_size`] if you know you will need less.
    pub(super) fn start_new_datagram(&mut self) {
        self.start_new_datagram_inner(None);
    }

    pub(super) fn start_new_datagram_with_size(&mut self, max_size: usize) {
        self.start_new_datagram_inner(Some(max_size));
    }

    pub(super) fn start_new_datagram_inner(&mut self, max_size: Option<usize>) {
        match self.state {
            State::FirstSegment { pmtu } => {
                // Only start a new datagram is something was writen to the buffer.
                // Otherwise, this is still the first segment.
                let segment_size = self.buf.len();
                debug_assert!(segment_size <= pmtu, "first segment exceeds pmtu");

                if let Some(segment_size) = NonZeroUsize::new(segment_size) {
                    self.datagram_start = segment_size.get();
                    self.state = State::batch(segment_size, max_size);
                } else if let Some(max) = max_size {
                    // buffer is still empty, use the new pmtu when defined
                    self.state = State::FirstSegment { pmtu: max };
                }
            }
            State::Batch {
                segment_size,
                max_size: _, // NOTE: even if Some we ignore it as long as the segments align
            } => {
                let current_size = self.buf.len();
                debug_assert_eq!(current_size % segment_size, 0, "missaligned segments");
                let finalized_segments = current_size / segment_size.get();
                debug_assert!(finalized_segments < self.max_datagrams.get());

                self.datagram_start += segment_size.get();

                self.state = State::batch(segment_size, max_size);
            }
        }
    }

    /// Max allowed datagram size.
    ///
    /// If this is the first datagram, this will be the provided pmtu. If this is a GSO Batch, this
    /// will be the segment size.
    ///
    /// If the last datagram was created using [`TransmitBuf::start_new_datagram_with_size`]
    /// the the segment size will be greater than the current datagram is allowed to be.
    /// Thus [`TransmitBuf::datagram_remaining_mut`] should be used if you need to know the
    /// amount of data that can be written into the datagram.
    pub(super) fn max_datagram_size(&self) -> usize {
        match self.state {
            State::FirstSegment { pmtu } => pmtu,
            State::Batch { segment_size, .. } => segment_size.get(),
        }
    }

    /// Returns the maximum number of datagrams allowed to be written into the buffer
    pub(super) fn max_datagrams(&self) -> NonZeroUsize {
        self.max_datagrams
    }

    /// Max size for the current datagram.
    fn max_current_segment_size(&self) -> usize {
        match self.state {
            State::FirstSegment { pmtu } => pmtu,
            State::Batch {
                segment_size,
                max_size,
                ..
            } => max_size.unwrap_or(segment_size).get(),
        }
    }

    /// Returns the start offset of the current datagram in the buffer
    ///
    /// In other words, this offset contains the first byte of the current datagram.
    pub(super) fn datagram_start_offset(&self) -> usize {
        self.datagram_start
    }

    /// Returns the maximum offset in the buffer allowed for the current datagram
    ///
    /// The first and last datagram in a batch are allowed to be smaller then the maximum
    /// size. All datagrams in between need to be exactly this size.
    pub(super) fn datagram_max_offset(&self) -> usize {
        let max_datagram_size = self.max_current_segment_size();
        self.datagram_start + max_datagram_size
    }

    /// Returns the number of bytes that may still be written into this datagram
    pub(super) fn datagram_remaining_mut(&self) -> usize {
        self.datagram_max_offset().saturating_sub(self.buf.len())
    }

    /// Returns `true` if the buffer did not have anything written into it
    pub(super) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The number of bytes written into the buffer so far
    pub(super) fn len(&self) -> usize {
        self.buf.len()
    }

    /// Returns the already written bytes in the buffer
    pub(super) fn as_mut_slice(&mut self) -> &mut [u8] {
        self.buf.as_mut_slice()
    }

    pub(crate) fn buf_mut(&mut self) -> &mut Vec<u8> {
        self.buf
    }

    /// Returns the buffer length and gso segment size.
    pub(crate) fn finish(self) -> (usize, Option<usize>) {
        let len = self.len();
        let num_datagrams = self.num_datagrams();
        debug_assert!(num_datagrams <= self.max_datagrams.get());
        let gso_segment_size = if num_datagrams < 2 {
            None
        } else {
            Some(self.max_datagram_size())
        };
        (len, gso_segment_size)
    }
}

unsafe impl BufMut for TransmitBuf<'_> {
    fn remaining_mut(&self) -> usize {
        self.buf.remaining_mut()
    }

    unsafe fn advance_mut(&mut self, cnt: usize) {
        unsafe { self.buf.advance_mut(cnt) };
    }

    fn chunk_mut(&mut self) -> &mut bytes::buf::UninitSlice {
        self.buf.chunk_mut()
    }
}

impl BufLen for TransmitBuf<'_> {
    fn len(&self) -> usize {
        self.len()
    }
}
