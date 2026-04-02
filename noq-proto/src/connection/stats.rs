//! Connection statistics

use rustc_hash::FxHashMap;

use crate::Duration;
use crate::FrameType;

use super::PathId;

/// Statistics about UDP datagrams transmitted or received on a connection.
///
/// All QUIC packets are carried by UDP datagrams. Hence, these statistics cover all traffic
/// on a connection.
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, derive_more::Add, derive_more::AddAssign)]
#[non_exhaustive]
pub struct UdpStats {
    /// The number of UDP datagrams observed.
    pub datagrams: u64,
    /// The total amount of bytes which have been transferred inside UDP datagrams.
    pub bytes: u64,
    /// The number of I/O operations executed.
    ///
    /// Can be less than `datagrams` when GSO, GRO, and/or batched system calls are in use.
    pub ios: u64,
}

impl UdpStats {
    pub(crate) fn on_sent(&mut self, datagrams: u64, bytes: usize) {
        self.datagrams += datagrams;
        self.bytes += bytes as u64;
        self.ios += 1;
    }
}

/// Number of frames transmitted or received of each frame type.
#[derive(Default, Copy, Clone, PartialEq, Eq, derive_more::Add, derive_more::AddAssign)]
#[non_exhaustive]
#[allow(missing_docs)]
pub struct FrameStats {
    pub acks: u64,
    pub path_acks: u64,
    pub ack_frequency: u64,
    pub crypto: u64,
    pub connection_close: u64,
    pub data_blocked: u64,
    pub datagram: u64,
    pub handshake_done: u8,
    pub immediate_ack: u64,
    pub max_data: u64,
    pub max_stream_data: u64,
    pub max_streams_bidi: u64,
    pub max_streams_uni: u64,
    pub new_connection_id: u64,
    pub path_new_connection_id: u64,
    pub new_token: u64,
    pub path_challenge: u64,
    pub path_response: u64,
    pub ping: u64,
    pub reset_stream: u64,
    pub retire_connection_id: u64,
    pub path_retire_connection_id: u64,
    pub stream_data_blocked: u64,
    pub streams_blocked_bidi: u64,
    pub streams_blocked_uni: u64,
    pub stop_sending: u64,
    pub stream: u64,
    pub observed_addr: u64,
    pub path_abandon: u64,
    pub path_status_available: u64,
    pub path_status_backup: u64,
    pub max_path_id: u64,
    pub paths_blocked: u64,
    pub path_cids_blocked: u64,
    pub add_address: u64,
    pub reach_out: u64,
    pub remove_address: u64,
}

impl FrameStats {
    pub(crate) fn record(&mut self, frame_type: FrameType) {
        use FrameType::*;
        // Increments the field. Added for readability
        macro_rules! inc {
            ($field_name: ident) => {{ self.$field_name = self.$field_name.saturating_add(1) }};
        }
        match frame_type {
            Padding => {}
            Ping => inc!(ping),
            Ack | AckEcn => inc!(acks),
            PathAck | PathAckEcn => inc!(path_acks),
            ResetStream => inc!(reset_stream),
            StopSending => inc!(stop_sending),
            Crypto => inc!(crypto),
            Datagram(_) => inc!(datagram),
            NewToken => inc!(new_token),
            MaxData => inc!(max_data),
            MaxStreamData => inc!(max_stream_data),
            MaxStreamsBidi => inc!(max_streams_bidi),
            MaxStreamsUni => inc!(max_streams_uni),
            DataBlocked => inc!(data_blocked),
            Stream(_) => inc!(stream),
            StreamDataBlocked => inc!(stream_data_blocked),
            StreamsBlockedUni => inc!(streams_blocked_uni),
            StreamsBlockedBidi => inc!(streams_blocked_bidi),
            NewConnectionId => inc!(new_connection_id),
            PathNewConnectionId => inc!(path_new_connection_id),
            RetireConnectionId => inc!(retire_connection_id),
            PathRetireConnectionId => inc!(path_retire_connection_id),
            PathChallenge => inc!(path_challenge),
            PathResponse => inc!(path_response),
            ConnectionClose | ApplicationClose => inc!(connection_close),
            AckFrequency => inc!(ack_frequency),
            ImmediateAck => inc!(immediate_ack),
            HandshakeDone => inc!(handshake_done),
            ObservedIpv4Addr | ObservedIpv6Addr => inc!(observed_addr),
            PathAbandon => inc!(path_abandon),
            PathStatusAvailable => inc!(path_status_available),
            PathStatusBackup => inc!(path_status_backup),
            MaxPathId => inc!(max_path_id),
            PathsBlocked => inc!(paths_blocked),
            PathCidsBlocked => inc!(path_cids_blocked),
            AddIpv4Address | AddIpv6Address => inc!(add_address),
            ReachOutAtIpv4 | ReachOutAtIpv6 => inc!(reach_out),
            RemoveAddress => inc!(remove_address),
        };
    }
}

impl std::fmt::Debug for FrameStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            acks,
            path_acks,
            ack_frequency,
            crypto,
            connection_close,
            data_blocked,
            datagram,
            handshake_done,
            immediate_ack,
            max_data,
            max_stream_data,
            max_streams_bidi,
            max_streams_uni,
            new_connection_id,
            path_new_connection_id,
            new_token,
            path_challenge,
            path_response,
            ping,
            reset_stream,
            retire_connection_id,
            path_retire_connection_id,
            stream_data_blocked,
            streams_blocked_bidi,
            streams_blocked_uni,
            stop_sending,
            stream,
            observed_addr,
            path_abandon,
            path_status_available,
            path_status_backup,
            max_path_id,
            paths_blocked,
            path_cids_blocked,
            add_address,
            reach_out,
            remove_address,
        } = self;
        f.debug_struct("FrameStats")
            .field("ACK", acks)
            .field("ACK_FREQUENCY", ack_frequency)
            .field("CONNECTION_CLOSE", connection_close)
            .field("CRYPTO", crypto)
            .field("DATA_BLOCKED", data_blocked)
            .field("DATAGRAM", datagram)
            .field("HANDSHAKE_DONE", handshake_done)
            .field("IMMEDIATE_ACK", immediate_ack)
            .field("MAX_DATA", max_data)
            .field("MAX_PATH_ID", max_path_id)
            .field("MAX_STREAM_DATA", max_stream_data)
            .field("MAX_STREAMS_BIDI", max_streams_bidi)
            .field("MAX_STREAMS_UNI", max_streams_uni)
            .field("NEW_CONNECTION_ID", new_connection_id)
            .field("NEW_TOKEN", new_token)
            .field("PATHS_BLOCKED", paths_blocked)
            .field("PATH_ABANDON", path_abandon)
            .field("PATH_ACK", path_acks)
            .field("PATH_STATUS_AVAILABLE", path_status_available)
            .field("PATH_STATUS_BACKUP", path_status_backup)
            .field("PATH_CHALLENGE", path_challenge)
            .field("PATH_CIDS_BLOCKED", path_cids_blocked)
            .field("PATH_NEW_CONNECTION_ID", path_new_connection_id)
            .field("PATH_RESPONSE", path_response)
            .field("PATH_RETIRE_CONNECTION_ID", path_retire_connection_id)
            .field("PING", ping)
            .field("RESET_STREAM", reset_stream)
            .field("RETIRE_CONNECTION_ID", retire_connection_id)
            .field("STREAM_DATA_BLOCKED", stream_data_blocked)
            .field("STREAMS_BLOCKED_BIDI", streams_blocked_bidi)
            .field("STREAMS_BLOCKED_UNI", streams_blocked_uni)
            .field("STOP_SENDING", stop_sending)
            .field("STREAM", stream)
            .field("OBSERVED_ADDRESS", observed_addr)
            .field("ADD_ADDRESS", add_address)
            .field("REACH_OUT", reach_out)
            .field("REMOVE_ADDRESS", remove_address)
            .finish()
    }
}

/// Statistics related to a transmission path.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PathStats {
    /// Current best estimate of this connection's latency (round-trip-time).
    pub rtt: Duration,
    /// Statistics about datagrams and bytes sent on this path.
    pub udp_tx: UdpStats,
    /// Statistics about datagrams and bytes received on this path.
    pub udp_rx: UdpStats,
    /// Statistics about frames transmitted on this path.
    pub frame_tx: FrameStats,
    /// Statistics about frames received on this path.
    pub frame_rx: FrameStats,
    /// Current congestion window of the connection.
    pub cwnd: u64,
    /// Congestion events on the connection.
    pub congestion_events: u64,
    /// The number of packets lost on this path.
    pub lost_packets: u64,
    /// The number of bytes lost on this path.
    pub lost_bytes: u64,
    /// The number of PLPMTUD probe packets sent on this path.
    ///
    /// These are also counted by [`UdpStats::datagrams`].
    pub sent_plpmtud_probes: u64,
    /// The number of PLPMTUD probe packets lost on this path.
    ///
    /// These are not included in [`Self::lost_packets`] and [`Self::lost_bytes`].
    pub lost_plpmtud_probes: u64,
    /// The number of times a black hole was detected in the path.
    pub black_holes_detected: u64,
    /// Largest UDP payload size the path currently supports.
    pub current_mtu: u16,
}

/// Connection statistics.
///
/// The fields here are a sum of the respective fields in the [`PathStats`] for all the
/// paths that exist as well as all paths that previously existed.
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct ConnectionStats {
    /// Statistics about UDP datagrams transmitted on the connection.
    pub udp_tx: UdpStats,
    /// Statistics about UDP datagrams received on the connection.
    pub udp_rx: UdpStats,
    /// Statistics about frames transmitted on the connection.
    pub frame_tx: FrameStats,
    /// Statistics about frames received on the connection.
    pub frame_rx: FrameStats,
    /// The number of packets lost on the connection.
    pub lost_packets: u64,
    /// The number of bytes lost on the connection.
    pub lost_bytes: u64,
}

impl std::ops::Add<PathStats> for ConnectionStats {
    type Output = Self;

    fn add(self, rhs: PathStats) -> Self::Output {
        // Be aware that Connection::stats() relies on the fact this function ignores the
        // rtt, cwnd and current_mtu fields.
        let PathStats {
            rtt: _,
            udp_tx,
            udp_rx,
            frame_tx,
            frame_rx,
            cwnd: _,
            congestion_events: _,
            lost_packets,
            lost_bytes,
            sent_plpmtud_probes: _,
            lost_plpmtud_probes: _,
            black_holes_detected: _,
            current_mtu: _,
        } = rhs;
        Self {
            udp_tx: self.udp_tx + udp_tx,
            udp_rx: self.udp_rx + udp_rx,
            frame_tx: self.frame_tx + frame_tx,
            frame_rx: self.frame_rx + frame_rx,
            lost_packets: self.lost_packets + lost_packets,
            lost_bytes: self.lost_bytes + lost_bytes,
        }
    }
}

impl std::ops::AddAssign<PathStats> for ConnectionStats {
    fn add_assign(&mut self, rhs: PathStats) {
        // Be aware that Connection::stats() relies on the fact this function ignores the
        // rtt, cwnd and current_mtu fields.
        let PathStats {
            rtt: _,
            udp_tx: path_udp_tx,
            udp_rx: path_udp_rx,
            frame_tx: path_frame_tx,
            frame_rx: path_frame_rx,
            cwnd: _,
            congestion_events: _,
            lost_packets: path_lost_packets,
            lost_bytes: path_lost_bytes,
            sent_plpmtud_probes: _,
            lost_plpmtud_probes: _,
            black_holes_detected: _,
            current_mtu: _,
        } = rhs;
        let Self {
            udp_tx,
            udp_rx,
            frame_tx,
            frame_rx,
            lost_packets,
            lost_bytes,
        } = self;
        *udp_tx += path_udp_tx;
        *udp_rx += path_udp_rx;
        *frame_tx += path_frame_tx;
        *frame_rx += path_frame_rx;
        *lost_packets += path_lost_packets;
        *lost_bytes += path_lost_bytes;
    }
}

/// Helper to make [`PathStats`] infallibly available.
///
/// This helper also helps with borrowing issues compared to having the [`Self::for_path`]
/// function as a helper directly on [`Connection`].
///
/// [`Connection`]: super::Connection
#[derive(Debug, Default)]
pub(super) struct PathStatsMap(FxHashMap<PathId, PathStats>);

impl PathStatsMap {
    /// Returns the [`PathStats`] for the path.
    pub(super) fn for_path(&mut self, path_id: PathId) -> &mut PathStats {
        self.0.entry(path_id).or_default()
    }

    /// An iterator over all contained [`PathStats`].
    pub(super) fn iter_stats(&self) -> impl Iterator<Item = &PathStats> {
        self.0.values()
    }

    /// Removes the stats for a given path.
    ///
    /// Only do this once you are discarding the path.
    pub(super) fn discard(&mut self, path_id: &PathId) -> PathStats {
        self.0.remove(path_id).unwrap_or_default()
    }
}
