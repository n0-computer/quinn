use std::{fmt::Debug, ops::Range};

use crate::{ConnectionId, ResetToken, frame::NewConnectionId};

/// DataType stored in CidQueue buffer
#[derive(Debug, Clone, Copy)]
struct CidData(ConnectionId, Option<ResetToken>);

/// Sliding window of active Connection IDs.
///
/// This represents a circular buffer that can contain gaps due to packet loss or reordering.
/// The buffer has three regions:
/// - Exactly one active CID at `self.buffer[self.cursor]`.
/// - Zero to `Self::LEN - 1` reserved CIDs from `self.cursor` up to `self.cursor_reserved`.
/// - More "available"/"ready" CIDs after `self.cursor_reserved`.
///
/// The range of reserved CIDs is grown by calling `CidQueue::next_reserved`, which takes one of
/// the available ones and returns the CID that was reserved.
///
/// New available/ready CIDs are added by calling [`CidQueue::insert`].
///
/// May contain gaps due to packet loss or reordering.
#[derive(Debug)]
pub(crate) struct CidQueue {
    /// Ring buffer indexed by `self.cursor`
    buffer: [Option<CidData>; Self::LEN],
    /// Index at which circular buffer addressing is based
    cursor: usize,
    /// Sequence number of `self.buffer[cursor]`
    ///
    /// The sequence number of the active CID; must be the smallest among CIDs in `buffer`.
    offset: u64,
    /// Circular index for the last reserved CID, i.e. a CID that is not the active CID, but was
    /// used for probing packets on a different remote address.
    ///
    /// When [`Self::cursor_reserved`] and [`Self::cursor`] are equal, no CID is considered
    /// reserved.
    cursor_reserved: usize,
}

impl CidQueue {
    pub(crate) fn new(cid: ConnectionId) -> Self {
        let mut buffer = [None; Self::LEN];
        buffer[0] = Some(CidData(cid, None));
        Self {
            buffer,
            cursor: 0,
            offset: 0,
            cursor_reserved: 0,
        }
    }

    /// Handle a `NEW_CONNECTION_ID` frame
    ///
    /// Returns a non-empty range of retired sequence numbers and the reset token of the new active
    /// CID iff any CIDs were retired.
    pub(crate) fn insert(
        &mut self,
        cid: NewConnectionId,
    ) -> Result<Option<(Range<u64>, ResetToken)>, InsertError> {
        // Position of new CID wrt. the current active CID
        let Some(index) = cid.sequence.checked_sub(self.offset) else {
            return Err(InsertError::Retired);
        };

        let retired_count = cid.retire_prior_to.saturating_sub(self.offset);
        if index >= Self::LEN as u64 + retired_count {
            return Err(InsertError::ExceedsLimit);
        }

        // Discard retired CIDs, if any
        for i in 0..(retired_count.min(Self::LEN as u64) as usize) {
            self.buffer[(self.cursor + i) % Self::LEN] = None;
        }

        // Record the new CID
        let index = ((self.cursor as u64 + index) % Self::LEN as u64) as usize;
        self.buffer[index] = Some(CidData(cid.id, Some(cid.reset_token)));

        if retired_count == 0 {
            return Ok(None);
        }

        // The active CID was retired. Find the first known CID with sequence number of at least
        // retire_prior_to, and inform the caller that all prior CIDs have been retired, and of
        // the new CID's reset token.
        self.cursor = ((self.cursor as u64 + retired_count) % Self::LEN as u64) as usize;
        let (i, CidData(_, token)) = self
            .iter_from_active()
            .next()
            .expect("it is impossible to retire a CID without supplying a new one");
        self.cursor = (self.cursor + i) % Self::LEN;
        self.cursor_reserved = self.cursor;
        let orig_offset = self.offset;
        self.offset = cid.retire_prior_to + i as u64;
        // We don't immediately retire CIDs in the range (orig_offset +
        // Self::LEN)..self.offset. These are CIDs that we haven't yet received from a
        // NEW_CONNECTION_ID frame, since having previously received them would violate the
        // connection ID limit we specified based on Self::LEN. If we do receive a such a frame
        // in the future, e.g. due to reordering, we'll retire it then. This ensures we can't be
        // made to buffer an arbitrarily large number of RETIRE_CONNECTION_ID frames.
        Ok(Some((
            orig_offset..self.offset.min(orig_offset + Self::LEN as u64),
            token.expect("non-initial CID missing reset token"),
        )))
    }

    /// Switch to next active CID if possible, return
    /// 1) the corresponding ResetToken and 2) a non-empty range preceding it to retire
    pub(crate) fn next(&mut self) -> Option<(ResetToken, Range<u64>)> {
        let (i, cid_data) = self.iter_from_reserved().nth(1)?;
        let reserved = self.reserved_len();
        for j in 0..=reserved {
            self.buffer[self.cursor + j] = None;
        }
        let orig_offset = self.offset;
        self.offset += (i + reserved) as u64;
        self.cursor = (self.cursor_reserved + i) % Self::LEN;
        self.cursor_reserved = self.cursor;

        Some((cid_data.1.unwrap(), orig_offset..self.offset))
    }

    /// Returns a CID from the available ones and marks it as reserved.
    ///
    /// If there's no more CIDs in the ready set, this will return None.
    /// CIDs marked as reserved will be skipped when the active one advances.
    pub(crate) fn next_reserved(&mut self) -> Option<ConnectionId> {
        let (i, cid_data) = self.iter_from_reserved().nth(1)?;

        self.cursor_reserved = (self.cursor_reserved + i) % Self::LEN;
        Some(cid_data.0)
    }

    /// Iterate CIDs in CidQueue that are not `None`, including the active CID
    fn iter_from_active(&self) -> impl Iterator<Item = (usize, CidData)> + '_ {
        (0..Self::LEN).filter_map(move |step| {
            let index = (self.cursor + step) % Self::LEN;
            self.buffer[index].map(|cid_data| (step, cid_data))
        })
    }

    /// Iterate CIDs in CidQueue that are not `None`, including the active CID.
    ///
    /// Along with the CID, it returns the offset counted from [`Self::cursor_reserved`] where the CID is stored.
    fn iter_from_reserved(&self) -> impl Iterator<Item = (usize, CidData)> + '_ {
        (0..(Self::LEN - self.reserved_len())).filter_map(move |step| {
            let index = (self.cursor_reserved + step) % Self::LEN;
            self.buffer[index].map(|cid_data| (step, cid_data))
        })
    }

    /// The length of the internal buffer's section of CIDs that are marked as reserved.
    fn reserved_len(&self) -> usize {
        if self.cursor_reserved >= self.cursor {
            self.cursor_reserved - self.cursor
        } else {
            self.cursor_reserved + Self::LEN - self.cursor
        }
    }

    /// Replace the initial CID
    pub(crate) fn update_initial_cid(&mut self, cid: ConnectionId) {
        debug_assert_eq!(self.offset, 0);
        self.buffer[self.cursor] = Some(CidData(cid, None));
    }

    /// Return active remote CID itself
    pub(crate) fn active(&self) -> ConnectionId {
        self.buffer[self.cursor].unwrap().0
    }

    /// Return the sequence number of active remote CID
    pub(crate) fn active_seq(&self) -> u64 {
        self.offset
    }

    pub(crate) const LEN: usize = 5;
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum InsertError {
    /// CID was already retired
    Retired,
    /// Sequence number violates the leading edge of the window
    ExceedsLimit,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cid(sequence: u64, retire_prior_to: u64) -> NewConnectionId {
        NewConnectionId {
            path_id: None,
            sequence,
            id: ConnectionId::new(&sequence.to_be_bytes()),
            reset_token: ResetToken::from([0xCD; crate::RESET_TOKEN_SIZE]),
            retire_prior_to,
        }
    }

    fn initial_cid() -> ConnectionId {
        ConnectionId::new(&[0xFF; 8])
    }

    #[test]
    fn next_dense() {
        let mut q = CidQueue::new(initial_cid());
        assert!(q.next().is_none());
        assert!(q.next().is_none());

        for i in 1..CidQueue::LEN as u64 {
            q.insert(cid(i, 0)).unwrap();
        }
        for i in 1..CidQueue::LEN as u64 {
            let (_, retire) = q.next().unwrap();
            assert_eq!(q.active_seq(), i);
            assert_eq!(retire.end - retire.start, 1);
        }
        assert!(q.next().is_none());
    }

    #[test]
    fn next_sparse() {
        let mut q = CidQueue::new(initial_cid());
        let seqs = (1..CidQueue::LEN as u64).filter(|x| x % 2 == 0);
        for i in seqs.clone() {
            q.insert(cid(i, 0)).unwrap();
        }
        for i in seqs {
            let (_, retire) = q.next().unwrap();
            dbg!(&retire);
            assert_eq!(q.active_seq(), i);
            assert_eq!(retire, (q.active_seq().saturating_sub(2))..q.active_seq());
        }
        assert!(q.next().is_none());
    }

    #[test]
    fn wrap() {
        let mut q = CidQueue::new(initial_cid());

        for i in 1..CidQueue::LEN as u64 {
            q.insert(cid(i, 0)).unwrap();
        }
        for _ in 1..(CidQueue::LEN as u64 - 1) {
            q.next().unwrap();
        }
        for i in CidQueue::LEN as u64..(CidQueue::LEN as u64 + 3) {
            q.insert(cid(i, 0)).unwrap();
        }
        for i in (CidQueue::LEN as u64 - 1)..(CidQueue::LEN as u64 + 3) {
            q.next().unwrap();
            assert_eq!(q.active_seq(), i);
        }
        assert!(q.next().is_none());
    }

    #[test]
    fn retire_dense() {
        let mut q = CidQueue::new(initial_cid());

        for i in 1..CidQueue::LEN as u64 {
            q.insert(cid(i, 0)).unwrap();
        }
        assert_eq!(q.active_seq(), 0);

        assert_eq!(q.insert(cid(4, 2)).unwrap().unwrap().0, 0..2);
        assert_eq!(q.active_seq(), 2);
        assert_eq!(q.insert(cid(4, 2)), Ok(None));

        for i in 2..(CidQueue::LEN as u64 - 1) {
            let _ = q.next().unwrap();
            assert_eq!(q.active_seq(), i + 1);
            assert_eq!(q.insert(cid(i + 1, i + 1)), Ok(None));
        }

        assert!(q.next().is_none());
    }

    #[test]
    fn retire_sparse() {
        // Retiring CID 0 when CID 1 is not known should retire CID 1 as we move to CID 2
        let mut q = CidQueue::new(initial_cid());
        q.insert(cid(2, 0)).unwrap();
        assert_eq!(q.insert(cid(3, 1)).unwrap().unwrap().0, 0..2,);
        assert_eq!(q.active_seq(), 2);
    }

    #[test]
    fn retire_many() {
        let mut q = CidQueue::new(initial_cid());
        q.insert(cid(2, 0)).unwrap();
        assert_eq!(
            q.insert(cid(1_000_000, 1_000_000)).unwrap().unwrap().0,
            0..CidQueue::LEN as u64,
        );
        assert_eq!(q.active_seq(), 1_000_000);
    }

    #[test]
    fn insert_limit() {
        let mut q = CidQueue::new(initial_cid());
        assert_eq!(q.insert(cid(CidQueue::LEN as u64 - 1, 0)), Ok(None));
        assert_eq!(
            q.insert(cid(CidQueue::LEN as u64, 0)),
            Err(InsertError::ExceedsLimit)
        );
    }

    #[test]
    fn insert_duplicate() {
        let mut q = CidQueue::new(initial_cid());
        q.insert(cid(0, 0)).unwrap();
        q.insert(cid(0, 0)).unwrap();
    }

    #[test]
    fn insert_retired() {
        let mut q = CidQueue::new(initial_cid());
        assert_eq!(
            q.insert(cid(0, 0)),
            Ok(None),
            "reinserting active CID succeeds"
        );
        assert!(q.next().is_none(), "active CID isn't requeued");
        q.insert(cid(1, 0)).unwrap();
        q.next().unwrap();
        assert_eq!(
            q.insert(cid(0, 0)),
            Err(InsertError::Retired),
            "previous active CID is already retired"
        );
    }

    #[test]
    fn retire_then_insert_next() {
        let mut q = CidQueue::new(initial_cid());
        for i in 1..CidQueue::LEN as u64 {
            q.insert(cid(i, 0)).unwrap();
        }
        q.next().unwrap();
        q.insert(cid(CidQueue::LEN as u64, 0)).unwrap();
        assert_eq!(
            q.insert(cid(CidQueue::LEN as u64 + 1, 0)),
            Err(InsertError::ExceedsLimit)
        );
    }

    #[test]
    fn always_valid() {
        let mut q = CidQueue::new(initial_cid());
        assert!(q.next().is_none());
        assert_eq!(q.active(), initial_cid());
        assert_eq!(q.active_seq(), 0);
    }

    #[test]
    fn reserved_smoke() {
        let mut q = CidQueue::new(initial_cid());
        assert_eq!(q.next_reserved(), None);

        let one = cid(1, 0);
        q.insert(one).unwrap();
        assert_eq!(q.next_reserved(), Some(one.id));

        let two = cid(2, 2);
        let (retired_range, reset_token) = q.insert(two).unwrap().unwrap();
        assert_eq!(reset_token, two.reset_token);
        assert_eq!(retired_range, 0..2);

        assert_eq!(q.next_reserved(), None);

        let four = cid(4, 2);
        q.insert(four).unwrap();
        println!("{q:?}");
        assert_eq!(q.next_reserved(), Some(four.id));
        assert_eq!(q.active(), two.id);

        assert_eq!(q.next(), None);
    }

    #[test]
    fn reserve_multiple() {
        let mut q = CidQueue::new(initial_cid());
        let one = cid(1, 0);
        let two = cid(2, 0);
        q.insert(one).unwrap();
        q.insert(two).unwrap();
        assert_eq!(q.next_reserved(), Some(one.id));
        assert_eq!(q.next_reserved(), Some(two.id));
        assert_eq!(q.next_reserved(), None);
    }

    #[test]
    fn reserve_multiple_sparse() {
        let mut q = CidQueue::new(initial_cid());
        let two = cid(2, 0);
        let four = cid(4, 0);
        q.insert(two).unwrap();
        q.insert(four).unwrap();
        assert_eq!(q.next_reserved(), Some(two.id));
        assert_eq!(q.next_reserved(), Some(four.id));
        assert_eq!(q.next_reserved(), None);
    }

    #[test]
    fn reserve_many_next_clears() {
        let mut q = CidQueue::new(initial_cid());
        for i in 1..CidQueue::LEN {
            q.insert(cid(i as u64, 0)).unwrap();
        }

        for _ in 0..CidQueue::LEN - 2 {
            assert!(q.next_reserved().is_some());
        }

        assert!(q.next().is_some());
        assert_eq!(q.next(), None);
    }

    #[test]
    fn reserve_many_next_reserved_none() {
        let mut q = CidQueue::new(initial_cid());
        for i in 1..CidQueue::LEN {
            q.insert(cid(i as u64, 0)).unwrap();
        }

        for _ in 0..CidQueue::LEN - 1 {
            assert!(q.next_reserved().is_some());
        }

        assert_eq!(q.next_reserved(), None);
    }

    #[test]
    fn one_active_all_else_reserved_next_none() {
        let mut q = CidQueue::new(initial_cid());
        for i in 1..CidQueue::LEN {
            q.insert(cid(i as u64, 0)).unwrap();
        }

        for _ in 0..CidQueue::LEN - 1 {
            assert!(q.next_reserved().is_some());
        }

        assert_eq!(q.next(), None);
    }

    #[test]
    fn insert_reserve_advance() {
        let mut q = CidQueue::new(initial_cid());

        let first = cid(1, 0);
        let second = cid(2, 0);
        let third = cid(3, 0);

        q.insert(first).unwrap();
        q.insert(second).unwrap();

        assert_eq!(q.next_reserved(), Some(first.id));
        q.insert(third).unwrap();
        q.next();
        assert_eq!(q.active(), second.id);
    }

    #[test]
    fn sparse_insert_reserve_insert_advance() {
        let mut q = CidQueue::new(initial_cid());

        let one = cid(1, 0);
        let two = cid(2, 0);
        let three = cid(3, 0);

        q.insert(two).unwrap();
        q.insert(three).unwrap();
        assert_eq!(q.next_reserved(), Some(two.id));
        q.insert(one).unwrap();
        q.next();
        assert_eq!(q.active(), three.id);
        assert_eq!(q.next_reserved(), None);
    }
}
