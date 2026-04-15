//! Pacing of packet transmissions.

use crate::{Duration, Instant};

use tracing::warn;

/// A simple token-bucket pacer
///
/// The pacer's capacity is derived on a fraction of the congestion window
/// which can be sent in regular intervals
/// Once the bucket is empty, further transmission is blocked.
/// The bucket refills at a rate slightly faster
/// than one congestion window per RTT, as recommended in
/// <https://tools.ietf.org/html/draft-ietf-quic-recovery-34#section-7.7>
#[derive(Debug, PartialEq)]
pub(super) struct Pacer {
    capacity: u64,
    last_window: u64,
    last_mtu: u16,
    tokens: u64,
    pub(super) prev: Instant,
}

impl Pacer {
    /// Obtains a new [`Pacer`].
    pub(super) fn new(smoothed_rtt: Duration, window: u64, mtu: u16, now: Instant) -> Self {
        let capacity = optimal_capacity(smoothed_rtt, window, mtu);
        Self {
            capacity,
            last_window: window,
            last_mtu: mtu,
            tokens: capacity,
            prev: now,
        }
    }

    /// Record that a packet has been transmitted.
    pub(super) fn on_transmit(&mut self, packet_length: u16) {
        self.tokens = self.tokens.saturating_sub(packet_length.into())
    }

    pub(super) fn delay(
        &self,
        smoothed_rtt: Duration,
        bytes_to_send: u64,
        window: u64,
    ) -> Option<Duration> {
        // if we can already send a packet, there is no need for delay
        if self.tokens >= bytes_to_send {
            return None;
        }

        if smoothed_rtt.as_nanos() == 0 {
            return None;
        }

        // we disable pacing for extremely large windows
        let window = u32::try_from(window).ok()?;

        let unscaled_delay = smoothed_rtt
            .checked_mul((bytes_to_send.max(self.capacity) - self.tokens) as _)
            .unwrap_or(Duration::MAX)
            / window;

        // divisions come before multiplications to prevent overflow
        // this is the time at which the pacing window becomes empty
        Some((unscaled_delay / 5) * 4)
    }

    pub(super) fn update(&mut self, smoothed_rtt: Duration, mtu: u16, window: u64, now: Instant) {
        debug_assert_ne!(
            window, 0,
            "zero-sized congestion control window is nonsense"
        );

        if window != self.last_window || mtu != self.last_mtu {
            self.capacity = optimal_capacity(smoothed_rtt, window, mtu);

            // Clamp the tokens
            self.tokens = self.capacity.min(self.tokens);
            self.last_window = window;
            self.last_mtu = mtu;
        }

        // we disable pacing for extremely large windows
        let Ok(window) = u32::try_from(window) else {
            return;
        };

        let time_elapsed = now.checked_duration_since(self.prev).unwrap_or_else(|| {
            warn!("received a timestamp early than a previous recorded time, ignoring");
            Default::default()
        });

        if smoothed_rtt.as_nanos() == 0 {
            return;
        }

        let elapsed_rtts = time_elapsed.as_secs_f64() / smoothed_rtt.as_secs_f64();
        let new_tokens = (window as f64 * 1.25 * elapsed_rtts).round() as u64;
        self.tokens = self.tokens.saturating_add(new_tokens).min(self.capacity);

        // In the unlikely event that we're getting polled faster than tokens are generated, ensure
        // that `elapsed_rtts` can grow until we make progress.
        if new_tokens > 0 {
            self.prev = now;
        }
    }

    /// Return how long we need to wait before sending `bytes_to_send`.
    ///
    /// If we can send a packet right away, this returns `None`. Otherwise, returns
    /// `Some(d)`, where `d` is the duration after which this function should be called
    /// again.
    ///
    /// The 5/4 ratio used here comes from the suggestion that N = 1.25 in the draft IETF
    /// RFC for QUIC.
    pub(super) fn update_and_delay(
        &mut self,
        smoothed_rtt: Duration,
        bytes_to_send: u64,
        mtu: u16,
        window: u64,
        now: Instant,
    ) -> Option<Duration> {
        debug_assert_ne!(
            window, 0,
            "zero-sized congestion control window is nonsense"
        );

        if window != self.last_window || mtu != self.last_mtu {
            self.capacity = optimal_capacity(smoothed_rtt, window, mtu);

            // Clamp the tokens
            self.tokens = self.capacity.min(self.tokens);
            self.last_window = window;
            self.last_mtu = mtu;
        }

        // if we can already send a packet, there is no need for delay
        if self.tokens >= bytes_to_send {
            return None;
        }

        // we disable pacing for extremely large windows
        if window > u64::from(u32::MAX) {
            return None;
        }

        let window = window as u32;

        let time_elapsed = now.checked_duration_since(self.prev).unwrap_or_else(|| {
            warn!("received a timestamp early than a previous recorded time, ignoring");
            Default::default()
        });

        if smoothed_rtt.as_nanos() == 0 {
            return None;
        }

        let elapsed_rtts = time_elapsed.as_secs_f64() / smoothed_rtt.as_secs_f64();
        let new_tokens = (window as f64 * 1.25 * elapsed_rtts).round() as u64;
        self.tokens = self.tokens.saturating_add(new_tokens).min(self.capacity);

        // In the unlikely event that we're getting polled faster than tokens are generated, ensure
        // that `elapsed_rtts` can grow until we make progress.
        if new_tokens > 0 {
            self.prev = now;
        }

        // if we can already send a packet, there is no need for delay
        if self.tokens >= bytes_to_send {
            return None;
        }

        let unscaled_delay = smoothed_rtt
            .checked_mul((bytes_to_send.max(self.capacity) - self.tokens) as _)
            .unwrap_or(Duration::MAX)
            / window;

        // divisions come before multiplications to prevent overflow
        // this is the time at which the pacing window becomes empty
        Some((unscaled_delay / 5) * 4)
    }
}

/// Calculates a pacer capacity for a certain window and RTT
///
/// The goal is to emit a burst (of size `capacity`) in timer intervals
/// which compromise between
/// - ideally distributing datagrams over time
/// - constantly waking up the connection to produce additional datagrams
///
/// Too short burst intervals means we will never meet them since the timer
/// accuracy in user-space is not high enough. If we miss the interval by more
/// than 25%, we will lose that part of the congestion window since no additional
/// tokens for the extra-elapsed time can be stored.
///
/// Too long burst intervals make pacing less effective.
fn optimal_capacity(smoothed_rtt: Duration, window: u64, mtu: u16) -> u64 {
    let rtt = smoothed_rtt.as_nanos().max(1);
    let mtu = u64::from(mtu);

    let target_capacity = ((window as u128 * TARGET_BURST_INTERVAL.as_nanos()) / rtt) as u64;
    // Never restrict capacity below one MTU.
    let max_capacity = Ord::max(
        ((window as u128 * MAX_BURST_INTERVAL.as_nanos()) / rtt) as u64,
        mtu,
    );

    // Batch the greater of `TARGET_BURST_INTERVAL` or `MIN_BURST_SIZE` worth of traffic at a
    // time. To avoid inducing excessive latency, limit that result to at most `MAX_BURST_INTERVAL`
    // worth of traffic.
    Ord::min(
        max_capacity,
        target_capacity.clamp(MIN_BURST_SIZE * mtu, MAX_BURST_SIZE * mtu),
    )
}

/// Period of traffic to batch together on a reasonably fast connection
const TARGET_BURST_INTERVAL: Duration = Duration::from_millis(2);

/// Maximum period of traffic to batch together on a slow connection
///
/// Takes precedence over [`MIN_BURST_SIZE`].
const MAX_BURST_INTERVAL: Duration = Duration::from_millis(10);

/// Minimum number of datagrams to batch together, so long as we won't have to wait for more than
/// [`MAX_BURST_INTERVAL`]
const MIN_BURST_SIZE: u64 = 10;

/// Creating 256 packets took 1ms in a benchmark, so larger bursts don't make sense.
const MAX_BURST_SIZE: u64 = 256;

#[cfg(test)]
mod tests {
    use proptest::{prelude::Strategy, prop_assert_eq};
    use test_strategy::proptest;

    use crate::tests::subscribe;

    use super::*;

    #[test]
    fn does_not_panic_on_bad_instant() {
        let old_instant = Instant::now();
        let new_instant = old_instant + Duration::from_micros(15);
        let rtt = Duration::from_micros(400);

        assert!(
            Pacer::new(rtt, 30000, 1500, new_instant)
                .update_and_delay(Duration::from_micros(0), 0, 1500, 1, old_instant)
                .is_none()
        );
        assert!(
            Pacer::new(rtt, 30000, 1500, new_instant)
                .update_and_delay(Duration::from_micros(0), 1600, 1500, 1, old_instant)
                .is_none()
        );
        assert!(
            Pacer::new(rtt, 30000, 1500, new_instant)
                .update_and_delay(Duration::from_micros(0), 1500, 1500, 3000, old_instant)
                .is_none()
        );
    }

    #[test]
    fn derives_initial_capacity() {
        let window = 2_000_000;
        let mtu = 1500;
        let rtt = Duration::from_millis(50);
        let now = Instant::now();

        let pacer = Pacer::new(rtt, window, mtu, now);
        assert_eq!(
            pacer.capacity,
            (window as u128 * TARGET_BURST_INTERVAL.as_nanos() / rtt.as_nanos()) as u64
        );
        assert_eq!(pacer.tokens, pacer.capacity);

        let pacer = Pacer::new(Duration::from_millis(0), window, mtu, now);
        assert_eq!(pacer.capacity, MAX_BURST_SIZE * mtu as u64);
        assert_eq!(pacer.tokens, pacer.capacity);

        let pacer = Pacer::new(rtt, 1, mtu, now);
        assert_eq!(pacer.capacity, mtu as u64);
        assert_eq!(pacer.tokens, pacer.capacity);
    }

    #[test]
    fn adjusts_capacity() {
        let window = 2_000_000;
        let mtu = 1500;
        let rtt = Duration::from_millis(50);
        let now = Instant::now();

        let mut pacer = Pacer::new(rtt, window, mtu, now);
        assert_eq!(
            pacer.capacity,
            (window as u128 * TARGET_BURST_INTERVAL.as_nanos() / rtt.as_nanos()) as u64
        );
        assert_eq!(pacer.tokens, pacer.capacity);
        let initial_tokens = pacer.tokens;

        pacer.update_and_delay(rtt, mtu as u64, mtu, window * 2, now);
        assert_eq!(
            pacer.capacity,
            (2 * window as u128 * TARGET_BURST_INTERVAL.as_nanos() / rtt.as_nanos()) as u64
        );
        assert_eq!(pacer.tokens, initial_tokens);

        pacer.update_and_delay(rtt, mtu as u64, mtu, window / 2, now);
        assert_eq!(
            pacer.capacity,
            (window as u128 / 2 * TARGET_BURST_INTERVAL.as_nanos() / rtt.as_nanos()) as u64
        );
        assert_eq!(pacer.tokens, initial_tokens / 2);

        pacer.update_and_delay(rtt, mtu as u64, mtu * 2, window, now);
        assert_eq!(
            pacer.capacity,
            (window as u128 * TARGET_BURST_INTERVAL.as_nanos() / rtt.as_nanos()) as u64
        );

        pacer.update_and_delay(rtt, mtu as u64, 20_000, window, now);
        assert_eq!(pacer.capacity, 20_000_u64 * MIN_BURST_SIZE);
    }

    #[test]
    fn computes_pause_correctly() {
        let window = 2_000_000u64;
        let mtu = 1000;
        let rtt = Duration::from_millis(50);
        let old_instant = Instant::now();

        let mut pacer = Pacer::new(rtt, window, mtu, old_instant);
        let packet_capacity = pacer.capacity / mtu as u64;

        for _ in 0..packet_capacity {
            assert_eq!(
                pacer.update_and_delay(rtt, mtu as u64, mtu, window, old_instant),
                None,
                "When capacity is available packets should be sent immediately"
            );

            pacer.on_transmit(mtu);
        }

        let pace_duration = Duration::from_nanos((TARGET_BURST_INTERVAL.as_nanos() * 4 / 5) as u64);

        let actual_delay = pacer
            .update_and_delay(rtt, mtu as u64, mtu, window, old_instant)
            .expect("Send must be delayed");

        let diff = actual_delay.abs_diff(pace_duration);

        // Allow up to 2ns difference due to rounding
        assert!(
            diff < Duration::from_nanos(2),
            "expected ≈ {pace_duration:?}, got {actual_delay:?} (diff {diff:?})"
        );
        // Refill half of the tokens
        assert_eq!(
            pacer.update_and_delay(
                rtt,
                mtu as u64,
                mtu,
                window,
                old_instant + pace_duration / 2
            ),
            None
        );
        assert_eq!(pacer.tokens, pacer.capacity / 2);

        for _ in 0..packet_capacity / 2 {
            assert_eq!(
                pacer.update_and_delay(rtt, mtu as u64, mtu, window, old_instant),
                None,
                "When capacity is available packets should be sent immediately"
            );

            pacer.on_transmit(mtu);
        }

        // Refill all capacity by waiting more than the expected duration
        assert_eq!(
            pacer.update_and_delay(
                rtt,
                mtu as u64,
                mtu,
                window,
                old_instant + pace_duration * 3 / 2
            ),
            None
        );
        assert_eq!(pacer.tokens, pacer.capacity);
    }

    #[derive(Debug, Clone, Copy, test_strategy::Arbitrary)]
    struct PacerParams {
        #[strategy((0u64..1_000).prop_map(Duration::from_millis))]
        smoothed_rtt: Duration,
        #[strategy(1u64..12_000)]
        window: u64,
        #[strategy((12u16..15).prop_map(|x| x * 100))]
        mtu: u16,
    }

    impl PacerParams {
        fn into_pacer(self, now: Instant) -> Pacer {
            Pacer::new(self.smoothed_rtt, self.window, self.mtu, now)
        }
    }

    #[proptest(cases = 256000)]
    fn pacer_separate_and_combined_check_equal(
        params: PacerParams,
        #[strategy(1u64..12_000)] transmitted: u64,
        #[strategy((0u64..1_000).prop_map(Duration::from_millis))] smoothed_rtt: Duration,
        #[strategy(1u64..1_500)] bytes_to_send: u64,
        #[strategy((12u16..15).prop_map(|x| x * 100))] mtu: u16,
        #[strategy(1u64..12_000)] window: u64,
        #[strategy((0u64..1_000).prop_map(Duration::from_millis))] after: Duration,
    ) {
        let _guard = subscribe();

        // let _guard = subscribe();
        let start = Instant::now();
        let mut pacer1 = params.into_pacer(start);
        let mut pacer2 = params.into_pacer(start);

        pacer1.tokens = pacer1.tokens.saturating_sub(transmitted);
        pacer2.tokens = pacer2.tokens.saturating_sub(transmitted);

        // tracing::trace!(?pacer1);

        let now = start + after;
        pacer1.update(smoothed_rtt, mtu, window, now);
        let separate = pacer1.delay(smoothed_rtt, bytes_to_send, window);
        let combined = pacer2.update_and_delay(smoothed_rtt, bytes_to_send, mtu, window, now);
        prop_assert_eq!(separate, combined);
        // prop_assert_eq!(pacer1, pacer2);
    }

    #[test]
    fn regression() {
        let now = Instant::now();
        let mut pacer1 = Pacer {
            capacity: 1200,
            last_window: 12000,
            last_mtu: 1200,
            tokens: 0,
            prev: now,
        };
        let mut pacer2 = Pacer {
            capacity: 1200,
            last_window: 12000,
            last_mtu: 1200,
            tokens: 0,
            prev: now,
        };

        let smoothed_rtt = Duration::ZERO;
        let bytes_to_send = 1200;
        let mtu = 1200;
        let window = 12000;

        let reference = pacer1.update_and_delay(smoothed_rtt, bytes_to_send, mtu, window, now);

        pacer2.update(smoothed_rtt, mtu, window, now);
        let actual = pacer2.delay(smoothed_rtt, bytes_to_send, window);

        assert_eq!(actual, reference);
    }
}
