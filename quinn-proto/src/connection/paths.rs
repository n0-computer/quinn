use std::{cmp, net::SocketAddr};

use identity_hash::IntMap;
use thiserror::Error;
use tracing::{debug, trace};

use super::{
    PathError, PathStats,
    mtud::MtuDiscovery,
    pacing::Pacer,
    spaces::{PacketNumberSpace, SentPacket},
};
use crate::{
    ConnectionId, Duration, FourTuple, Instant, TIMER_GRANULARITY, TransportConfig, VarInt,
    coding::{self, Decodable, Encodable},
    congestion,
    frame::ObservedAddr,
    packet::SpaceId,
};

#[cfg(feature = "qlog")]
use qlog::events::quic::RecoveryMetricsUpdated;

/// Id representing different paths when using multipath extension
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default)]
pub struct PathId(pub(crate) u32);

impl std::hash::Hash for PathId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u32(self.0);
    }
}

impl identity_hash::IdentityHashable for PathId {}

impl Decodable for PathId {
    fn decode<B: bytes::Buf>(r: &mut B) -> coding::Result<Self> {
        let v = VarInt::decode(r)?;
        let v = u32::try_from(v.0).map_err(|_| coding::UnexpectedEnd)?;
        Ok(Self(v))
    }
}

impl Encodable for PathId {
    fn encode<B: bytes::BufMut>(&self, w: &mut B) {
        VarInt(self.0.into()).encode(w)
    }
}

impl PathId {
    /// The maximum path ID allowed.
    pub const MAX: Self = Self(u32::MAX);

    /// The 0 path id.
    pub const ZERO: Self = Self(0);

    /// The number of bytes this [`PathId`] uses when encoded as a [`VarInt`]
    pub(crate) const fn size(&self) -> usize {
        VarInt(self.0 as u64).size()
    }

    /// Saturating integer addition. Computes self + rhs, saturating at the numeric bounds instead
    /// of overflowing.
    pub fn saturating_add(self, rhs: impl Into<Self>) -> Self {
        let rhs = rhs.into();
        let inner = self.0.saturating_add(rhs.0);
        Self(inner)
    }

    /// Saturating integer subtraction. Computes self - rhs, saturating at the numeric bounds
    /// instead of overflowing.
    pub fn saturating_sub(self, rhs: impl Into<Self>) -> Self {
        let rhs = rhs.into();
        let inner = self.0.saturating_sub(rhs.0);
        Self(inner)
    }

    /// Get the next [`PathId`]
    pub(crate) fn next(&self) -> Self {
        self.saturating_add(Self(1))
    }

    /// Get the underlying u32
    pub(crate) fn as_u32(&self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for PathId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: Into<u32>> From<T> for PathId {
    fn from(source: T) -> Self {
        Self(source.into())
    }
}

/// State needed for a single path ID.
///
/// A single path ID can migrate according to the rules in RFC9000 §9, either voluntary or
/// involuntary. We need to keep the [`PathData`] of the previously used such path available
/// in order to defend against migration attacks (see RFC9000 §9.3.1, §9.3.2 and §9.3.3) as
/// well as to support path probing (RFC9000 §9.1).
#[derive(Debug)]
pub(super) struct PathState {
    pub(super) data: PathData,
    pub(super) prev: Option<(ConnectionId, PathData)>,
}

impl PathState {
    /// Update counters to account for a packet becoming acknowledged, lost, or abandoned
    pub(super) fn remove_in_flight(&mut self, packet: &SentPacket) {
        // Visit known paths from newest to oldest to find the one `pn` was sent on
        for path_data in [&mut self.data]
            .into_iter()
            .chain(self.prev.as_mut().map(|(_, data)| data))
        {
            if path_data.remove_in_flight(packet) {
                return;
            }
        }
    }
}

#[derive(Debug)]
pub(super) struct SentChallengeInfo {
    /// When was the challenge sent on the wire.
    pub(super) sent_instant: Instant,
    /// The 4-tuple on which this path challenge was sent.
    pub(super) network_path: FourTuple,
}

/// Description of a particular network path
#[derive(Debug)]
pub(super) struct PathData {
    pub(super) network_path: FourTuple,
    pub(super) rtt: RttEstimator,
    /// Whether we're enabling ECN on outgoing packets
    pub(super) sending_ecn: bool,
    /// Congestion controller state
    pub(super) congestion: Box<dyn congestion::Controller>,
    /// Pacing state
    pub(super) pacing: Pacer,
    /// Actually sent challenges (on the wire).
    pub(super) challenges_sent: IntMap<u64, SentChallengeInfo>,
    /// Whether to *immediately* trigger another PATH_CHALLENGE.
    ///
    /// This is picked up by [`super::Connection::space_can_send`].
    pub(super) send_new_challenge: bool,
    /// Pending responses to PATH_CHALLENGE frames
    pub(super) path_responses: PathResponses,
    /// Whether we're certain the peer can both send and receive on this address
    ///
    /// Initially equal to `use_stateless_retry` for servers, and becomes false again on every
    /// migration. Always true for clients.
    pub(super) validated: bool,
    /// Total size of all UDP datagrams sent on this path
    pub(super) total_sent: u64,
    /// Total size of all UDP datagrams received on this path
    pub(super) total_recvd: u64,
    /// The state of the MTU discovery process
    pub(super) mtud: MtuDiscovery,
    /// Packet number of the first packet sent after an RTT sample was collected on this path
    ///
    /// Used in persistent congestion determination.
    pub(super) first_packet_after_rtt_sample: Option<(SpaceId, u64)>,
    /// The in-flight packets and bytes
    ///
    /// Note that this is across all spaces on this path
    pub(super) in_flight: InFlight,
    /// Whether this path has had it's remote address reported back to the peer. This only happens
    /// if both peers agree to so based on their transport parameters.
    pub(super) observed_addr_sent: bool,
    /// Observed address frame with the largest sequence number received from the peer on this path.
    pub(super) last_observed_addr_report: Option<ObservedAddr>,
    /// The QUIC-MULTIPATH path status
    pub(super) status: PathStatusState,
    /// Number of the first packet sent on this path
    ///
    /// With RFC9000 §9 style migration (i.e. not multipath) the PathId does not change and
    /// hence packet numbers continue. This is used to determine whether a packet was sent
    /// on such an earlier path. Insufficient to determine if a packet was sent on a later
    /// path.
    first_packet: Option<u64>,
    /// The number of times a PTO has been sent without receiving an ack.
    pub(super) pto_count: u32,

    //
    // Per-path idle & keep alive
    //
    /// Idle timeout for the path
    ///
    /// If expired, the path will be abandoned.  This is different from the connection-wide
    /// idle timeout which closes the connection if expired.
    pub(super) idle_timeout: Option<Duration>,
    /// Keep alives to send on this path
    ///
    /// There is also a connection-level keep alive configured in the
    /// [`TransportParameters`].  This triggers activity on any path which can keep the
    /// connection alive.
    ///
    /// [`TransportParameters`]: crate::transport_parameters::TransportParameters
    pub(super) keep_alive: Option<Duration>,

    /// Whether the path has already been considered opened from an application perspective
    ///
    /// This means, for paths other than the original [`PathId::ZERO`], a first path challenge has
    /// been responded to, regardless of the initial validation status of the path. This state is
    /// irreversible, since it's not affected by the path being closed.
    pub(super) open: bool,

    /// State relevant to sending and receiving PATH_ABANDON frames.
    pub(super) abandon_state: AbandonState,

    /// Snapshot of the qlog recovery metrics
    #[cfg(feature = "qlog")]
    recovery_metrics: RecoveryMetrics,

    /// Tag uniquely identifying a path in a connection
    generation: u64,
}

/// The abandon-relevant state a path can be in.
#[derive(Debug)]
pub(super) enum AbandonState {
    /// This path wasn't abandoned yet.
    NotAbandoned,
    /// A PATH_ABANDON frame was sent for this path, and we're expecting a response
    /// by the given deadline.
    ///
    /// If we don't receive PATH_ABANDON by that time *and* we receive a packet on this
    /// path, then we assume our peer has ignored our PATH_ABANDON and error out.
    ///
    /// This is checked in [`crate::Connection::on_packet_authenticated`].
    ExpectingPathAbandon { deadline: Instant },
    /// We received a PATH_ABANDON and are just waiting for the path's packets to drain
    /// and eventually will discard the path.
    ReceivedPathAbandon,
}

impl PathData {
    pub(super) fn new(
        network_path: FourTuple,
        allow_mtud: bool,
        peer_max_udp_payload_size: Option<u16>,
        generation: u64,
        now: Instant,
        config: &TransportConfig,
    ) -> Self {
        let congestion = config
            .congestion_controller_factory
            .clone()
            .build(now, config.get_initial_mtu());
        Self {
            network_path,
            rtt: RttEstimator::new(config.initial_rtt),
            sending_ecn: true,
            pacing: Pacer::new(
                config.initial_rtt,
                congestion.initial_window(),
                config.get_initial_mtu(),
                now,
            ),
            congestion,
            challenges_sent: Default::default(),
            send_new_challenge: false,
            path_responses: PathResponses::default(),
            validated: false,
            total_sent: 0,
            total_recvd: 0,
            mtud: config
                .mtu_discovery_config
                .as_ref()
                .filter(|_| allow_mtud)
                .map_or_else(
                    || MtuDiscovery::disabled(config.get_initial_mtu(), config.min_mtu),
                    |mtud_config| {
                        MtuDiscovery::new(
                            config.get_initial_mtu(),
                            config.min_mtu,
                            peer_max_udp_payload_size,
                            mtud_config.clone(),
                        )
                    },
                ),
            first_packet_after_rtt_sample: None,
            in_flight: InFlight::new(),
            observed_addr_sent: false,
            last_observed_addr_report: None,
            status: Default::default(),
            first_packet: None,
            pto_count: 0,
            idle_timeout: config.default_path_max_idle_timeout,
            keep_alive: config.default_path_keep_alive_interval,
            open: false,
            abandon_state: AbandonState::NotAbandoned,
            #[cfg(feature = "qlog")]
            recovery_metrics: RecoveryMetrics::default(),
            generation,
        }
    }

    /// Create a new path from a previous one.
    ///
    /// This should only be called when migrating paths.
    pub(super) fn from_previous(
        network_path: FourTuple,
        prev: &Self,
        generation: u64,
        now: Instant,
    ) -> Self {
        let congestion = prev.congestion.clone_box();
        let smoothed_rtt = prev.rtt.get();
        Self {
            network_path,
            rtt: prev.rtt,
            pacing: Pacer::new(smoothed_rtt, congestion.window(), prev.current_mtu(), now),
            sending_ecn: true,
            congestion,
            challenges_sent: Default::default(),
            send_new_challenge: false,
            path_responses: PathResponses::default(),
            validated: false,
            total_sent: 0,
            total_recvd: 0,
            mtud: prev.mtud.clone(),
            first_packet_after_rtt_sample: prev.first_packet_after_rtt_sample,
            in_flight: InFlight::new(),
            observed_addr_sent: false,
            last_observed_addr_report: None,
            status: prev.status.clone(),
            first_packet: None,
            pto_count: 0,
            idle_timeout: prev.idle_timeout,
            keep_alive: prev.keep_alive,
            open: false,
            abandon_state: AbandonState::NotAbandoned,
            #[cfg(feature = "qlog")]
            recovery_metrics: prev.recovery_metrics.clone(),
            generation,
        }
    }

    /// Whether we're in the process of validating this path with PATH_CHALLENGEs
    pub(super) fn is_validating_path(&self) -> bool {
        !self.challenges_sent.is_empty() || self.send_new_challenge
    }

    /// Indicates whether we're a server that hasn't validated the peer's address and hasn't
    /// received enough data from the peer to permit sending `bytes_to_send` additional bytes
    pub(super) fn anti_amplification_blocked(&self, bytes_to_send: u64) -> bool {
        !self.validated && self.total_recvd * 3 < self.total_sent + bytes_to_send
    }

    /// Returns the path's current MTU
    pub(super) fn current_mtu(&self) -> u16 {
        self.mtud.current_mtu()
    }

    /// Account for transmission of `packet` with number `pn` in `space`
    pub(super) fn sent(&mut self, pn: u64, packet: SentPacket, space: &mut PacketNumberSpace) {
        self.in_flight.insert(&packet);
        if self.first_packet.is_none() {
            self.first_packet = Some(pn);
        }
        if let Some(forgotten) = space.sent(pn, packet) {
            self.remove_in_flight(&forgotten);
        }
    }

    /// Remove `packet` with number `pn` from this path's congestion control counters, or return
    /// `false` if `pn` was sent before this path was established.
    pub(super) fn remove_in_flight(&mut self, packet: &SentPacket) -> bool {
        if packet.path_generation != self.generation {
            return false;
        }
        self.in_flight.remove(packet);
        true
    }

    /// Increment the total size of sent UDP datagrams
    pub(super) fn inc_total_sent(&mut self, inc: u64) {
        self.total_sent = self.total_sent.saturating_add(inc);
        if !self.validated {
            trace!(
                network_path = %self.network_path,
                anti_amplification_budget = %(self.total_recvd * 3).saturating_sub(self.total_sent),
                "anti amplification budget decreased"
            );
        }
    }

    /// Increment the total size of received UDP datagrams
    pub(super) fn inc_total_recvd(&mut self, inc: u64) {
        self.total_recvd = self.total_recvd.saturating_add(inc);
        if !self.validated {
            trace!(
                network_path = %self.network_path,
                anti_amplification_budget = %(self.total_recvd * 3).saturating_sub(self.total_sent),
                "anti amplification budget increased"
            );
        }
    }

    /// The earliest time at which a sent challenge is considered lost.
    pub(super) fn earliest_expiring_challenge(&self) -> Option<Instant> {
        if self.challenges_sent.is_empty() {
            return None;
        }
        let pto = self.rtt.pto_base();
        self.challenges_sent
            .values()
            .map(|info| info.sent_instant)
            .min()
            .map(|sent_instant| sent_instant + pto)
    }

    /// Handle receiving a PATH_RESPONSE.
    pub(super) fn on_path_response_received(
        &mut self,
        now: Instant,
        token: u64,
        network_path: FourTuple,
    ) -> OnPathResponseReceived {
        match self.challenges_sent.get(&token) {
            // Response to an on-path PathChallenge
            Some(info)
                if info.network_path.is_probably_same_path(&network_path)
                    && self.network_path.is_probably_same_path(&network_path) =>
            {
                self.network_path.update_local_if_same_remote(&network_path);
                let sent_instant = info.sent_instant;
                if !std::mem::replace(&mut self.validated, true) {
                    trace!("new path validated");
                }
                // Clear any other on-path sent challenge.
                self.challenges_sent
                    .retain(|_token, info| !info.network_path.is_probably_same_path(&network_path));

                self.send_new_challenge = false;

                // This RTT can only be used for the initial RTT, not as a normal
                // sample: https://www.rfc-editor.org/rfc/rfc9002#section-6.2.2-2.
                let rtt = now.saturating_duration_since(sent_instant);
                self.rtt.reset_initial_rtt(rtt);

                let was_open = std::mem::replace(&mut self.open, true);
                OnPathResponseReceived::OnPath { was_open }
            }
            // Response to an off-path PathChallenge
            Some(info) if info.network_path.is_probably_same_path(&network_path) => {
                self.challenges_sent
                    .retain(|_token, info| !info.network_path.is_probably_same_path(&network_path));
                OnPathResponseReceived::OffPath
            }
            // Response to a PathChallenge we recognize, but from an invalid remote
            Some(info) => OnPathResponseReceived::Invalid {
                expected: info.network_path,
            },
            // Response to an unknown PathChallenge
            None => OnPathResponseReceived::Unknown,
        }
    }

    #[cfg(feature = "qlog")]
    pub(super) fn qlog_recovery_metrics(
        &mut self,
        path_id: PathId,
    ) -> Option<RecoveryMetricsUpdated> {
        let controller_metrics = self.congestion.metrics();

        let metrics = RecoveryMetrics {
            min_rtt: Some(self.rtt.min),
            smoothed_rtt: Some(self.rtt.get()),
            latest_rtt: Some(self.rtt.latest),
            rtt_variance: Some(self.rtt.var),
            pto_count: Some(self.pto_count),
            bytes_in_flight: Some(self.in_flight.bytes),
            packets_in_flight: Some(self.in_flight.ack_eliciting),

            congestion_window: Some(controller_metrics.congestion_window),
            ssthresh: controller_metrics.ssthresh,
            pacing_rate: controller_metrics.pacing_rate,
        };

        let event = metrics.to_qlog_event(path_id, &self.recovery_metrics);
        self.recovery_metrics = metrics;
        event
    }

    /// Return how long we need to wait before sending `bytes_to_send`
    ///
    /// See [`Pacer::delay`].
    pub(super) fn pacing_delay(&mut self, bytes_to_send: u64, now: Instant) -> Option<Instant> {
        let smoothed_rtt = self.rtt.get();
        self.pacing.delay(
            smoothed_rtt,
            bytes_to_send,
            self.current_mtu(),
            self.congestion.window(),
            now,
        )
    }

    /// Updates the last observed address report received on this path.
    ///
    /// If the address was updated, it's returned to be informed to the application.
    #[must_use = "updated observed address must be reported to the application"]
    pub(super) fn update_observed_addr_report(
        &mut self,
        observed: ObservedAddr,
    ) -> Option<SocketAddr> {
        match self.last_observed_addr_report.as_mut() {
            Some(prev) => {
                if prev.seq_no >= observed.seq_no {
                    // frames that do not increase the sequence number on this path are ignored
                    None
                } else if prev.ip == observed.ip && prev.port == observed.port {
                    // keep track of the last seq_no but do not report the address as updated
                    prev.seq_no = observed.seq_no;
                    None
                } else {
                    let addr = observed.socket_addr();
                    self.last_observed_addr_report = Some(observed);
                    Some(addr)
                }
            }
            None => {
                let addr = observed.socket_addr();
                self.last_observed_addr_report = Some(observed);
                Some(addr)
            }
        }
    }

    pub(crate) fn remote_status(&self) -> Option<PathStatus> {
        self.status.remote_status.map(|(_seq, status)| status)
    }

    pub(crate) fn local_status(&self) -> PathStatus {
        self.status.local_status
    }

    pub(super) fn generation(&self) -> u64 {
        self.generation
    }
}

pub(super) enum OnPathResponseReceived {
    /// This response validates the path on its current remote address.
    OnPath { was_open: bool },
    /// This response is valid, but it's for a remote other than the path's current remote address.
    OffPath,
    /// The received token is unknown.
    Unknown,
    /// The response is invalid.
    Invalid {
        /// The 4-tuple that was expected for this token.
        expected: FourTuple,
    },
}

/// Congestion metrics as described in [`recovery_metrics_updated`].
///
/// [`recovery_metrics_updated`]: https://datatracker.ietf.org/doc/html/draft-ietf-quic-qlog-quic-events.html#name-recovery_metrics_updated
#[cfg(feature = "qlog")]
#[derive(Default, Clone, PartialEq, Debug)]
#[non_exhaustive]
struct RecoveryMetrics {
    pub min_rtt: Option<Duration>,
    pub smoothed_rtt: Option<Duration>,
    pub latest_rtt: Option<Duration>,
    pub rtt_variance: Option<Duration>,
    pub pto_count: Option<u32>,
    pub bytes_in_flight: Option<u64>,
    pub packets_in_flight: Option<u64>,
    pub congestion_window: Option<u64>,
    pub ssthresh: Option<u64>,
    pub pacing_rate: Option<u64>,
}

#[cfg(feature = "qlog")]
impl RecoveryMetrics {
    /// Retain only values that have been updated since the last snapshot.
    fn retain_updated(&self, previous: &Self) -> Self {
        macro_rules! keep_if_changed {
            ($name:ident) => {
                if previous.$name == self.$name {
                    None
                } else {
                    self.$name
                }
            };
        }

        Self {
            min_rtt: keep_if_changed!(min_rtt),
            smoothed_rtt: keep_if_changed!(smoothed_rtt),
            latest_rtt: keep_if_changed!(latest_rtt),
            rtt_variance: keep_if_changed!(rtt_variance),
            pto_count: keep_if_changed!(pto_count),
            bytes_in_flight: keep_if_changed!(bytes_in_flight),
            packets_in_flight: keep_if_changed!(packets_in_flight),
            congestion_window: keep_if_changed!(congestion_window),
            ssthresh: keep_if_changed!(ssthresh),
            pacing_rate: keep_if_changed!(pacing_rate),
        }
    }

    /// Emit a `MetricsUpdated` event containing only updated values
    fn to_qlog_event(&self, path_id: PathId, previous: &Self) -> Option<RecoveryMetricsUpdated> {
        let updated = self.retain_updated(previous);

        if updated == Self::default() {
            return None;
        }

        Some(RecoveryMetricsUpdated {
            min_rtt: updated.min_rtt.map(|rtt| rtt.as_secs_f32()),
            smoothed_rtt: updated.smoothed_rtt.map(|rtt| rtt.as_secs_f32()),
            latest_rtt: updated.latest_rtt.map(|rtt| rtt.as_secs_f32()),
            rtt_variance: updated.rtt_variance.map(|rtt| rtt.as_secs_f32()),
            pto_count: updated
                .pto_count
                .map(|count| count.try_into().unwrap_or(u16::MAX)),
            bytes_in_flight: updated.bytes_in_flight,
            packets_in_flight: updated.packets_in_flight,
            congestion_window: updated.congestion_window,
            ssthresh: updated.ssthresh,
            pacing_rate: updated.pacing_rate,
            path_id: Some(path_id.as_u32() as u64),
        })
    }
}

/// RTT estimation for a particular network path
#[derive(Copy, Clone, Debug)]
pub struct RttEstimator {
    /// The most recent RTT measurement made when receiving an ack for a previously unacked packet
    latest: Duration,
    /// The smoothed RTT of the connection, computed as described in RFC6298
    smoothed: Option<Duration>,
    /// The RTT variance, computed as described in RFC6298
    var: Duration,
    /// The minimum RTT seen in the connection, ignoring ack delay.
    min: Duration,
}

impl RttEstimator {
    pub(super) fn new(initial_rtt: Duration) -> Self {
        Self {
            latest: initial_rtt,
            smoothed: None,
            var: initial_rtt / 2,
            min: initial_rtt,
        }
    }

    /// Resets the estimator using a new initial_rtt value.
    ///
    /// This only resets the initial_rtt **if** no samples have been recorded yet. If there
    /// are any recorded samples the initial estimate can not be adjusted after the fact.
    ///
    /// This is useful when you receive a PATH_RESPONSE in the first packet received on a
    /// new path. In this case you can use the delay of the PATH_CHALLENGE-PATH_RESPONSE as
    /// the initial RTT to get a better expected estimation.
    ///
    /// A PATH_CHALLENGE-PATH_RESPONSE pair later in the connection should not be used
    /// explicitly as an estimation since PATH_CHALLENGE is an ACK-eliciting packet itself
    /// already.
    pub(crate) fn reset_initial_rtt(&mut self, initial_rtt: Duration) {
        if self.smoothed.is_none() {
            self.latest = initial_rtt;
            self.var = initial_rtt / 2;
            self.min = initial_rtt;
        }
    }

    /// The current best RTT estimation.
    pub fn get(&self) -> Duration {
        self.smoothed.unwrap_or(self.latest)
    }

    /// Conservative estimate of RTT
    ///
    /// Takes the maximum of smoothed and latest RTT, as recommended
    /// in 6.1.2 of the recovery spec (draft 29).
    pub fn conservative(&self) -> Duration {
        self.get().max(self.latest)
    }

    /// Minimum RTT registered so far for this estimator.
    pub fn min(&self) -> Duration {
        self.min
    }

    /// PTO computed as described in RFC9002#6.2.1.
    pub(crate) fn pto_base(&self) -> Duration {
        self.get() + cmp::max(4 * self.var, TIMER_GRANULARITY)
    }

    /// Records an RTT sample.
    pub(crate) fn update(&mut self, ack_delay: Duration, rtt: Duration) {
        self.latest = rtt;
        // https://www.rfc-editor.org/rfc/rfc9002.html#section-5.2-3:
        // min_rtt does not adjust for ack_delay to avoid underestimating.
        self.min = cmp::min(self.min, self.latest);
        // Based on RFC6298.
        if let Some(smoothed) = self.smoothed {
            let adjusted_rtt = if self.min + ack_delay <= self.latest {
                self.latest - ack_delay
            } else {
                self.latest
            };
            let var_sample = smoothed.abs_diff(adjusted_rtt);
            self.var = (3 * self.var + var_sample) / 4;
            self.smoothed = Some((7 * smoothed + adjusted_rtt) / 8);
        } else {
            self.smoothed = Some(self.latest);
            self.var = self.latest / 2;
            self.min = self.latest;
        }
    }
}

#[derive(Default, Debug)]
pub(crate) struct PathResponses {
    pending: Vec<PathResponse>,
}

impl PathResponses {
    pub(crate) fn push(&mut self, packet: u64, token: u64, network_path: FourTuple) {
        /// Arbitrary permissive limit to prevent abuse
        const MAX_PATH_RESPONSES: usize = 16;
        let response = PathResponse {
            packet,
            token,
            network_path,
        };
        let existing = self
            .pending
            .iter_mut()
            .find(|x| x.network_path.remote == network_path.remote);
        if let Some(existing) = existing {
            // Update a queued response
            if existing.packet <= packet {
                *existing = response;
            }
            return;
        }
        if self.pending.len() < MAX_PATH_RESPONSES {
            self.pending.push(response);
        } else {
            // We don't expect to ever hit this with well-behaved peers, so we don't bother dropping
            // older challenges.
            trace!("ignoring excessive PATH_CHALLENGE");
        }
    }

    pub(crate) fn pop_off_path(&mut self, network_path: FourTuple) -> Option<(u64, FourTuple)> {
        let response = *self.pending.last()?;
        // We use an exact comparison here, because once we've received for the first time,
        // we really should either already have a local_ip, or we will never get one
        // (because our OS doesn't support it).
        if response.network_path == network_path {
            // We don't bother searching further because we expect that the on-path response will
            // get drained in the immediate future by a call to `pop_on_path`
            return None;
        }
        self.pending.pop();
        Some((response.token, response.network_path))
    }

    pub(crate) fn pop_on_path(&mut self, network_path: FourTuple) -> Option<u64> {
        let response = *self.pending.last()?;
        // Using an exact comparison. See explanation in `pop_off_path`.
        if response.network_path != network_path {
            // We don't bother searching further because we expect that the off-path response will
            // get drained in the immediate future by a call to `pop_off_path`
            return None;
        }
        self.pending.pop();
        Some(response.token)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

#[derive(Copy, Clone, Debug)]
struct PathResponse {
    /// The packet number the corresponding PATH_CHALLENGE was received in
    packet: u64,
    /// The token of the PATH_CHALLENGE
    token: u64,
    /// The path the corresponding PATH_CHALLENGE was received from
    network_path: FourTuple,
}

/// Summary statistics of packets that have been sent on a particular path, but which have not yet
/// been acked or deemed lost
#[derive(Debug)]
pub(super) struct InFlight {
    /// Sum of the sizes of all sent packets considered "in flight" by congestion control
    ///
    /// The size does not include IP or UDP overhead. Packets only containing ACK frames do not
    /// count towards this to ensure congestion control does not impede congestion feedback.
    pub(super) bytes: u64,
    /// Number of packets in flight containing frames other than ACK and PADDING
    ///
    /// This can be 0 even when bytes is not 0 because PADDING frames cause a packet to be
    /// considered "in flight" by congestion control. However, if this is nonzero, bytes will always
    /// also be nonzero.
    pub(super) ack_eliciting: u64,
}

impl InFlight {
    fn new() -> Self {
        Self {
            bytes: 0,
            ack_eliciting: 0,
        }
    }

    fn insert(&mut self, packet: &SentPacket) {
        self.bytes += u64::from(packet.size);
        self.ack_eliciting += u64::from(packet.ack_eliciting);
    }

    /// Update counters to account for a packet becoming acknowledged, lost, or abandoned
    fn remove(&mut self, packet: &SentPacket) {
        self.bytes -= u64::from(packet.size);
        self.ack_eliciting -= u64::from(packet.ack_eliciting);
    }
}

/// State for QUIC-MULTIPATH PATH_STATUS_AVAILABLE and PATH_STATUS_BACKUP frames
#[derive(Debug, Clone, Default)]
pub(super) struct PathStatusState {
    /// The local status
    local_status: PathStatus,
    /// Local sequence number, for both PATH_STATUS_AVAILABLE and PATH_STATUS_BACKUP
    ///
    /// This is the number of the *next* path status frame to be sent.
    local_seq: VarInt,
    /// The status set by the remote
    remote_status: Option<(VarInt, PathStatus)>,
}

impl PathStatusState {
    /// To be called on received PATH_STATUS_AVAILABLE/PATH_STATUS_BACKUP frames
    pub(super) fn remote_update(&mut self, status: PathStatus, seq: VarInt) {
        if self.remote_status.is_some_and(|(curr, _)| curr >= seq) {
            return trace!(%seq, "ignoring path status update");
        }

        let prev = self.remote_status.replace((seq, status)).map(|(_, s)| s);
        if prev != Some(status) {
            debug!(?status, ?seq, "remote changed path status");
        }
    }

    /// Updates the local status
    ///
    /// If the local status changed, the previous value is returned
    pub(super) fn local_update(&mut self, status: PathStatus) -> Option<PathStatus> {
        if self.local_status == status {
            return None;
        }

        self.local_seq = self.local_seq.saturating_add(1u8);
        Some(std::mem::replace(&mut self.local_status, status))
    }

    pub(crate) fn seq(&self) -> VarInt {
        self.local_seq
    }
}

/// The QUIC-MULTIPATH path status
///
/// See section "3.3 Path Status Management":
/// <https://quicwg.org/multipath/draft-ietf-quic-multipath.html#name-path-status-management>
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub enum PathStatus {
    /// Paths marked with as available will be used when scheduling packets
    ///
    /// If multiple paths are available, packets will be scheduled on whichever has
    /// capacity.
    #[default]
    Available,
    /// Paths marked as backup will only be used if there are no available paths
    ///
    /// If the max_idle_timeout is specified the path will be kept alive so that it does not
    /// expire.
    Backup,
}

/// Application events about paths
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathEvent {
    /// A new path has been opened
    Opened {
        /// Which path is now open
        id: PathId,
    },
    /// A path has been closed
    Closed {
        /// Which path has been closed
        id: PathId,
        /// Error code supplied by the peer
        /// See <https://www.ietf.org/archive/id/draft-ietf-quic-multipath-14.html#name-error-codes>
        /// for a list of known errors.
        error_code: VarInt,
    },
    /// All remaining state for a path has been removed
    ///
    /// The [`PathEvent::Closed`] would have been emitted for this path earlier.
    Abandoned {
        /// Which path had its state dropped
        id: PathId,
        /// The final path stats, they are no longer available via [`Connection::stats`]
        ///
        /// [`Connection::stats`]: super::Connection::stats
        path_stats: PathStats,
    },
    /// Path was closed locally
    LocallyClosed {
        /// Path for which the error occurred
        id: PathId,
        /// The error that occurred
        error: PathError,
    },
    /// The remote changed the status of the path
    ///
    /// The local status is not changed because of this event. It is up to the application
    /// to update the local status, which is used for packet scheduling, when the remote
    /// changes the status.
    RemoteStatus {
        /// Path which has changed status
        id: PathId,
        /// The new status set by the remote
        status: PathStatus,
    },
    /// Received an observation of our external address from the peer.
    ObservedAddr {
        /// Path over which the observed address was reported, [`PathId::ZERO`] when multipath is
        /// not negotiated
        id: PathId,
        /// The address observed by the remote over this path
        addr: SocketAddr,
    },
}

/// Error from setting path status
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SetPathStatusError {
    /// Error indicating that a path has not been opened or has already been abandoned
    #[error("closed path")]
    ClosedPath,
    /// Error indicating that this operation requires multipath to be negotiated whereas it hasn't been
    #[error("multipath not negotiated")]
    MultipathNotNegotiated,
}

/// Error indicating that a path has not been opened or has already been abandoned
#[derive(Debug, Default, Error, Clone, PartialEq, Eq)]
#[error("closed path")]
pub struct ClosedPath {
    pub(super) _private: (),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_id_saturating_add() {
        // add within range behaves normally
        let large: PathId = u16::MAX.into();
        let next = u32::from(u16::MAX) + 1;
        assert_eq!(large.saturating_add(1u8), PathId::from(next));

        // outside range saturates
        assert_eq!(PathId::MAX.saturating_add(1u8), PathId::MAX)
    }
}
