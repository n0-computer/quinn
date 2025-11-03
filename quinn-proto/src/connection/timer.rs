use rustc_hash::FxHashMap;

use crate::Instant;

use super::PathId;

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub(crate) enum Timer {
    Generic(GenericTimer),
    PerPath(PathId, PerPathTimer),
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub(crate) enum GenericTimer {
    /// When to close the connection after no activity
    Idle = 0,
    /// When the close timer expires, the connection has been gracefully terminated.
    Close = 1,
    /// When keys are discarded because they should not be needed anymore
    KeyDiscard = 2,
    /// When to send a `PING` frame to keep the connection alive
    KeepAlive = 3,
    /// When to invalidate old CID and proactively push new one via NEW_CONNECTION_ID frame
    PushNewCid = 4,
}

impl GenericTimer {
    const VALUES: [Self; 5] = [
        Self::Idle,
        Self::Close,
        Self::KeyDiscard,
        Self::KeepAlive,
        Self::PushNewCid,
    ];
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub(crate) enum PerPathTimer {
    /// When to send an ack-eliciting probe packet or declare unacked packets lost
    LossDetection = 0,
    /// When to abandon a path after no activity
    PathIdle = 1,
    /// When to give up on validating a new path from RFC9000 migration
    PathValidation = 2,
    /// When to give up on validating a new (multi)path
    PathOpen = 3,
    /// When to send a `PING` frame to keep the path alive
    PathKeepAlive = 4,
    /// When pacing will allow us to send a packet
    Pacing = 5,
    /// When to send an immediate ACK if there are unacked ack-eliciting packets of the peer
    MaxAckDelay = 6,
    /// When to clean up state for an abandoned path
    PathAbandoned = 7,
    /// When the peer fails to confirm abandoning the path
    PathNotAbandoned = 8,
}

impl PerPathTimer {
    const VALUES: [Self; 9] = [
        Self::LossDetection,
        Self::PathIdle,
        Self::PathValidation,
        Self::PathOpen,
        Self::PathKeepAlive,
        Self::Pacing,
        Self::MaxAckDelay,
        Self::PathAbandoned,
        Self::PathNotAbandoned,
    ];
}

/// Keeps track of the nearest timeout for each `Timer`
///
/// The [`TimerTable`] is advanced with [`TimerTable::expire_before`].
#[derive(Debug, Clone, Default)]
pub(crate) struct TimerTable {
    generic: [Option<Instant>; GenericTimer::VALUES.len()],
    path_0: PathTimerTable,
    path_1: PathTimerTable,
    path_n: FxHashMap<PathId, PathTimerTable>,
}

#[derive(Debug, Clone, Copy, Default)]
struct PathTimerTable {
    timers: [Option<Instant>; PerPathTimer::VALUES.len()],
}

impl PathTimerTable {
    fn set(&mut self, timer: PerPathTimer, time: Instant) {
        self.timers[timer as usize] = Some(time);
    }

    fn stop(&mut self, timer: PerPathTimer) {
        self.timers[timer as usize] = None;
    }

    fn reset(&mut self) {
        for timer in PerPathTimer::VALUES {
            self.timers[timer as usize] = None;
        }
    }

    /// Remove the next timer up until `now`, including it
    fn expire_before(&mut self, now: Instant) -> Option<(PerPathTimer, Instant)> {
        for timer in PerPathTimer::VALUES {
            if self.timers[timer as usize].is_some()
                && self.timers[timer as usize].expect("checked") <= now
            {
                return self.timers[timer as usize].take().map(|time| (timer, time));
            }
        }

        None
    }
}

impl TimerTable {
    /// Sets the timer unconditionally
    pub(super) fn set(&mut self, timer: Timer, time: Instant) {
        match timer {
            Timer::Generic(timer) => {
                self.generic[timer as usize] = Some(time);
            }
            Timer::PerPath(path_id, timer) => match path_id.0 {
                0 => {
                    self.path_0.set(timer, time);
                }
                1 => {
                    self.path_1.set(timer, time);
                }
                _ => {
                    self.path_n.entry(path_id).or_default().set(timer, time);
                }
            },
        }
    }

    pub(super) fn stop(&mut self, timer: Timer) {
        match timer {
            Timer::Generic(timer) => {
                self.generic[timer as usize] = None;
            }
            Timer::PerPath(path_id, timer) => match path_id.0 {
                0 => {
                    self.path_0.stop(timer);
                }
                1 => {
                    self.path_1.stop(timer);
                }
                _ => {
                    if let Some(e) = self.path_n.get_mut(&path_id) {
                        e.stop(timer);
                    }
                }
            },
        }
    }

    /// Get the next queued timeout
    pub(super) fn peek(&mut self) -> Option<Instant> {
        // TODO: this is currently linear in the number of paths

        let min_generic = self.generic.iter().filter_map(|&x| x).min();
        let min_path_0 = self.path_0.timers.iter().filter_map(|&x| x).min();
        let min_path_1 = self.path_1.timers.iter().filter_map(|&x| x).min();
        let min_path_n = self
            .path_n
            .values()
            .flat_map(|p| p.timers.iter().filter_map(|&x| x))
            .min();

        // TODO: can this be written better? using iterators makes this slower as this is very hot
        match (min_generic, min_path_0, min_path_1, min_path_n) {
            (None, None, None, None) => None,
            (Some(val), None, None, None) => Some(val),
            (None, Some(val), None, None) => Some(val),
            (None, None, Some(val), None) => Some(val),
            (None, None, None, Some(val)) => Some(val),

            (Some(min_generic), Some(min_path), None, None) => Some(min_generic.min(min_path)),
            (None, Some(min_path_0), Some(min_path_1), None) => Some(min_path_0.min(min_path_1)),
            (None, None, Some(min_path_1), Some(min_path_n)) => Some(min_path_1.min(min_path_n)),
            (Some(min_generic), None, Some(min_path_1), None) => Some(min_generic.min(min_path_1)),
            (Some(min_generic), None, None, Some(min_path_n)) => Some(min_generic.min(min_path_n)),
            (None, Some(min_path_0), None, Some(min_path_n)) => Some(min_path_0.min(min_path_n)),

            (Some(min_generic), Some(min_path_0), Some(min_path_1), None) => {
                Some(min_generic.min(min_path_0).min(min_path_1))
            }
            (Some(min_generic), Some(min_path_0), None, Some(min_path_n)) => {
                Some(min_generic.min(min_path_0).min(min_path_n))
            }
            (Some(min_generic), None, Some(min_path_1), Some(min_path_n)) => {
                Some(min_generic.min(min_path_1).min(min_path_n))
            }
            (None, Some(min_path_0), Some(min_path_1), Some(min_path_n)) => {
                Some(min_path_0.min(min_path_1).min(min_path_n))
            }

            (Some(min_generic), Some(min_path_0), Some(min_path_1), Some(min_path_n)) => {
                Some(min_generic.min(min_path_0).min(min_path_1).min(min_path_n))
            }
        }
    }

    /// Remove the next timer up until `now`, including it
    pub(super) fn expire_before(&mut self, now: Instant) -> Option<(Timer, Instant)> {
        // TODO: this is currently linear in the number of paths

        for timer in GenericTimer::VALUES {
            if self.generic[timer as usize].is_some()
                && self.generic[timer as usize].expect("checked") <= now
            {
                return self.generic[timer as usize]
                    .take()
                    .map(|time| (Timer::Generic(timer), time));
            }
        }

        if let Some((timer, time)) = self.path_0.expire_before(now) {
            return Some((Timer::PerPath(PathId(0), timer), time));
        }
        if let Some((timer, time)) = self.path_1.expire_before(now) {
            return Some((Timer::PerPath(PathId(1), timer), time));
        }

        let mut res = None;
        for (path_id, timers) in &mut self.path_n {
            if let Some((timer, time)) = timers.expire_before(now) {
                res = Some((Timer::PerPath(*path_id, timer), time));
                break;
            }
        }

        // clear out old timers
        self.path_n
            .retain(|_path_id, timers| timers.timers.iter().any(|t| t.is_some()));
        res
    }

    pub(super) fn reset(&mut self) {
        self.path_0.reset();
        self.path_1.reset();
        self.path_n.clear();
    }

    #[cfg(test)]
    pub(super) fn values(&self) -> Vec<(Timer, Instant)> {
        let mut values = Vec::new();

        for timer in GenericTimer::VALUES {
            if let Some(time) = self.generic[timer as usize] {
                values.push((Timer::Generic(timer), time));
            }
        }

        for timer in PerPathTimer::VALUES {
            if let Some(time) = self.path_0.timers[timer as usize] {
                values.push((Timer::PerPath(PathId(0), timer), time));
            }
        }

        for timer in PerPathTimer::VALUES {
            if let Some(time) = self.path_1.timers[timer as usize] {
                values.push((Timer::PerPath(PathId(1), timer), time));
            }
        }

        for timer in PerPathTimer::VALUES {
            for (path_id, timers) in &self.path_n {
                if let Some(time) = timers.timers[timer as usize] {
                    values.push((Timer::PerPath(*path_id, timer), time));
                }
            }
        }

        values
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn timer_table() {
        let mut timers = TimerTable::default();
        let sec = Duration::from_secs(1);
        let now = Instant::now() + Duration::from_secs(10);
        timers.set(Timer::Generic(GenericTimer::Idle), now - 3 * sec);
        timers.set(Timer::Generic(GenericTimer::Close), now - 2 * sec);

        assert_eq!(timers.peek(), Some(now - 3 * sec));
        assert_eq!(
            timers.expire_before(now),
            Some((Timer::Generic(GenericTimer::Idle), now - 3 * sec))
        );
        assert_eq!(
            timers.expire_before(now),
            Some((Timer::Generic(GenericTimer::Close), now - 2 * sec))
        );
        assert_eq!(timers.expire_before(now), None);
    }
}
