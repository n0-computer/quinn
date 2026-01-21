use std::num::NonZeroUsize;

use bytes::BufMut;
use tracing::trace;

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
    datagram_start_offset: usize,
    /// The maximum offset allowed to be used for the current datagram in the buffer
    ///
    /// The first and last datagram in a batch are allowed to be smaller then the maximum
    /// size. All datagrams in between need to be exactly this size.
    datagram_max_offset: usize,
    /// The maximum number of datagrams allowed to write into [`TransmitBuf::buf`]
    max_datagrams: NonZeroUsize,
    /// The number of datagrams already (partially) written into the buffer
    ///
    /// Incremented by a call to [`TransmitBuf::start_new_datagram`].
    pub(super) num_datagrams: usize,
    /// The segment size of this GSO batch, set once the second datagram is started.
    ///
    /// The segment size is the size of each datagram in the GSO batch, only the last
    /// datagram in the batch may be smaller.
    ///
    /// Only set once there is more than one datagram.
    segment_size: Option<usize>,
}

impl<'a> TransmitBuf<'a> {
    pub(super) fn new(buf: &'a mut Vec<u8>, max_datagrams: NonZeroUsize) -> Self {
        buf.clear();
        Self {
            buf,
            datagram_start_offset: 0,
            datagram_max_offset: 0,
            max_datagrams,
            num_datagrams: 0,
            segment_size: None,
        }
    }

    /// Starts the first datagram in the GSO batch.
    ///
    /// The size of the first datagram sets the segment size of the GSO batch.
    pub(super) fn start_first_datagram(&mut self, max_size: usize) {
        debug_assert_eq!(self.num_datagrams, 0, "No datagram can be stared yet");
        debug_assert!(
            self.buf.is_empty(),
            "Buffer must be empty for first datagram"
        );
        self.datagram_max_offset = max_size;
        self.num_datagrams = 1;
        if self.datagram_max_offset > self.buf.capacity() {
            // Reserve all remaining capacity right away.
            let max_batch_capacity = max_size * self.max_datagrams.get();
            self.buf
                .reserve_exact(max_batch_capacity.saturating_sub(self.buf.capacity()));
        }
    }

    /// Starts a subsequent datagram in the GSO batch.
    pub(super) fn start_datagram(&mut self) {
        // Could be enforced with typestate, but that's probably also meh.
        debug_assert!(
            self.num_datagrams >= 1,
            "Use start_first_datagram for first datagram"
        );
        debug_assert!(
            self.buf.len() <= self.datagram_max_offset,
            "Datagram exceeded max offset"
        );
        let segment_size = self
            .segment_size
            .get_or_insert_with(|| self.buf.len() - self.datagram_start_offset);
        let segment_size = *segment_size;
        if self.num_datagrams > 1 {
            debug_assert_eq!(
                self.buf.len(),
                self.datagram_max_offset,
                "Subsequent datagrams must be exactly the segment size"
            );
        }

        self.num_datagrams += 1;
        self.datagram_start_offset = self.buf.len();
        self.datagram_max_offset = self.buf.len() + segment_size;
        if self.datagram_max_offset > self.buf.capacity() {
            // Reserve all remaining capacity right away.
            let max_batch_capacity = segment_size * self.max_datagrams.get();
            self.buf
                .reserve_exact(max_batch_capacity.saturating_sub(self.buf.capacity()));
        }
    }

    /// Mark the first datagram as completely written, setting its size.
    ///
    /// Only valid for the first datagram, when the datagram might be smaller than the
    /// maximum size it was allowed to be. Needed before estimating the available space in
    /// the next datagram based on [`TransmitBuf::segment_size`].
    ///
    /// Use [`TransmitBuf::start_new_datagram_with_size`] if you need to reduce the size of
    /// the last datagram in a batch.
    pub(super) fn end_first_datagram(&mut self) {
        debug_assert_eq!(self.num_datagrams, 1);
        if self.buf.len() < self.datagram_max_offset {
            trace!(
                size = self.buf.len(),
                max_size = self.datagram_max_offset,
                "clipped datagram size"
            );
        }
        self.datagram_max_offset = self.buf.len();
    }

    /// Returns the GSO segment size.
    pub(super) fn segment_size(&self) -> Option<usize> {
        self.segment_size
    }

    /// Returns the maximum size this datagram is allowed to be.
    ///
    /// Once a second datagram is started this is equivalent to the segment size.
    pub(super) fn max_datagram_size(&self) -> usize {
        self.datagram_max_offset - self.datagram_start_offset
    }

    /// Returns the number of datagrams written into the buffer
    ///
    /// The last datagram is not necessarily finished yet.
    pub(super) fn num_datagrams(&self) -> usize {
        self.num_datagrams
    }

    /// Returns the maximum number of datagrams allowed to be written into the buffer
    pub(super) fn max_datagrams(&self) -> NonZeroUsize {
        self.max_datagrams
    }

    /// Returns the start offset of the current datagram in the buffer
    ///
    /// In other words, this offset contains the first byte of the current datagram.
    pub(super) fn datagram_start_offset(&self) -> usize {
        self.datagram_start_offset
    }

    /// Returns the maximum offset in the buffer allowed for the current datagram
    ///
    /// The first and last datagram in a batch are allowed to be smaller then the maximum
    /// size. All datagrams in between need to be exactly this size.
    pub(super) fn datagram_max_offset(&self) -> usize {
        self.datagram_max_offset
    }

    /// Returns the number of bytes that may still be written into this datagram
    pub(super) fn datagram_remaining_mut(&self) -> usize {
        self.datagram_max_offset.saturating_sub(self.buf.len())
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
