//! Connection statistics

use crate::Duration;
use crate::FrameType;

/// Statistics about UDP datagrams transmitted or received on a connection
///
/// All QUIC packets are carried by UDP datagrams. Hence, these statistics cover all traffic on a connection.
#[derive(Default, Debug, Copy, Clone)]
#[non_exhaustive]
pub struct UdpStats {
    /// The amount of UDP datagrams observed
    pub datagrams: u64,
    /// The total amount of bytes which have been transferred inside UDP datagrams
    pub bytes: u64,
    /// The amount of I/O operations executed
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

/// Number of frames transmitted or received of each frame type
#[derive(Default, Copy, Clone)]
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
        match frame_type {
            FrameType::Padding => {}
            FrameType::Ping => self.ping = self.ping.saturating_add(1),
            FrameType::Ack | FrameType::AckEcn => self.acks = self.acks.saturating_add(1),
            FrameType::PathAck | FrameType::PathAckEcn => {
                self.path_acks = self.path_acks.saturating_add(1)
            }
            FrameType::ResetStream => self.reset_stream = self.reset_stream.saturating_add(1),
            FrameType::StopSending => self.stop_sending = self.stop_sending.saturating_add(1),
            FrameType::Crypto => self.crypto = self.crypto.saturating_add(1),
            FrameType::Datagram(_) => self.datagram = self.datagram.saturating_add(1),
            FrameType::NewToken => self.new_token = self.new_token.saturating_add(1),
            FrameType::MaxData => self.max_data = self.max_data.saturating_add(1),
            FrameType::MaxStreamData => {
                self.max_stream_data = self.max_stream_data.saturating_add(1)
            }
            FrameType::MaxStreamsBidi => {
                self.max_streams_bidi = self.max_streams_bidi.saturating_add(1)
            }
            FrameType::MaxStreamsUni => {
                self.max_streams_uni = self.max_streams_uni.saturating_add(1)
            }
            FrameType::DataBlocked => self.data_blocked = self.data_blocked.saturating_add(1),
            FrameType::Stream(_) => self.stream = self.stream.saturating_add(1),
            FrameType::StreamDataBlocked => {
                self.stream_data_blocked = self.stream_data_blocked.saturating_add(1)
            }
            FrameType::StreamsBlockedUni => {
                self.streams_blocked_uni = self.streams_blocked_uni.saturating_add(1)
            }
            FrameType::StreamsBlockedBidi => {
                self.streams_blocked_bidi = self.streams_blocked_bidi.saturating_add(1)
            }
            FrameType::NewConnectionId => {
                self.new_connection_id = self.new_connection_id.saturating_add(1)
            }
            FrameType::PathNewConnectionId => {
                self.path_new_connection_id = self.path_new_connection_id.saturating_add(1)
            }
            FrameType::RetireConnectionId => {
                self.retire_connection_id = self.retire_connection_id.saturating_add(1)
            }
            FrameType::PathRetireConnectionId => {
                self.path_retire_connection_id = self.path_retire_connection_id.saturating_add(1)
            }
            FrameType::PathChallenge => self.path_challenge = self.path_challenge.saturating_add(1),
            FrameType::PathResponse => self.path_response = self.path_response.saturating_add(1),
            FrameType::ConnectionClose | FrameType::ApplicationClose => {
                self.connection_close = self.connection_close.saturating_add(1)
            }
            FrameType::AckFrequency => self.ack_frequency = self.ack_frequency.saturating_add(1),
            FrameType::ImmediateAck => self.immediate_ack = self.immediate_ack.saturating_add(1),
            FrameType::HandshakeDone => {
                self.handshake_done = self.handshake_done.saturating_add(1);
            }
            FrameType::ObservedIpv4Addr | FrameType::ObservedIpv6Addr => {
                self.observed_addr = self.observed_addr.saturating_add(1)
            }
            FrameType::PathAbandon => self.path_abandon = self.path_abandon.saturating_add(1),
            FrameType::PathStatusAvailable => {
                self.path_status_available = self.path_status_available.saturating_add(1)
            }
            FrameType::PathStatusBackup => {
                self.path_status_backup = self.path_status_backup.saturating_add(1)
            }
            FrameType::MaxPathId => self.max_path_id = self.max_path_id.saturating_add(1),
            FrameType::PathsBlocked => self.paths_blocked = self.paths_blocked.saturating_add(1),
            FrameType::PathCidsBlocked => {
                self.path_cids_blocked = self.path_cids_blocked.saturating_add(1)
            }
            FrameType::AddIpv4Address | FrameType::AddIpv6Address => {
                self.add_address = self.add_address.saturating_add(1)
            }
            FrameType::ReachOutAtIpv4 | FrameType::ReachOutAtIpv6 => {
                self.reach_out = self.reach_out.saturating_add(1)
            }
            FrameType::RemoveAddress => self.remove_address = self.remove_address.saturating_add(1),
        };
    }
}

impl std::fmt::Debug for FrameStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameStats")
            .field("ACK", &self.acks)
            .field("ACK_FREQUENCY", &self.ack_frequency)
            .field("CONNECTION_CLOSE", &self.connection_close)
            .field("CRYPTO", &self.crypto)
            .field("DATA_BLOCKED", &self.data_blocked)
            .field("DATAGRAM", &self.datagram)
            .field("HANDSHAKE_DONE", &self.handshake_done)
            .field("IMMEDIATE_ACK", &self.immediate_ack)
            .field("MAX_DATA", &self.max_data)
            .field("MAX_PATH_ID", &self.max_path_id)
            .field("MAX_STREAM_DATA", &self.max_stream_data)
            .field("MAX_STREAMS_BIDI", &self.max_streams_bidi)
            .field("MAX_STREAMS_UNI", &self.max_streams_uni)
            .field("NEW_CONNECTION_ID", &self.new_connection_id)
            .field("NEW_TOKEN", &self.new_token)
            .field("PATHS_BLOCKED", &self.paths_blocked)
            .field("PATH_ABANDON", &self.path_abandon)
            .field("PATH_ACK", &self.path_acks)
            .field("PATH_STATUS_AVAILABLE", &self.path_status_available)
            .field("PATH_STATUS_BACKUP", &self.path_status_backup)
            .field("PATH_CHALLENGE", &self.path_challenge)
            .field("PATH_CIDS_BLOCKED", &self.path_cids_blocked)
            .field("PATH_NEW_CONNECTION_ID", &self.path_new_connection_id)
            .field("PATH_RESPONSE", &self.path_response)
            .field("PATH_RETIRE_CONNECTION_ID", &self.path_retire_connection_id)
            .field("PING", &self.ping)
            .field("RESET_STREAM", &self.reset_stream)
            .field("RETIRE_CONNECTION_ID", &self.retire_connection_id)
            .field("STREAM_DATA_BLOCKED", &self.stream_data_blocked)
            .field("STREAMS_BLOCKED_BIDI", &self.streams_blocked_bidi)
            .field("STREAMS_BLOCKED_UNI", &self.streams_blocked_uni)
            .field("STOP_SENDING", &self.stop_sending)
            .field("STREAM", &self.stream)
            .finish()
    }
}

/// Statistics related to a transmission path
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PathStats {
    /// Current best estimate of this connection's latency (round-trip-time)
    pub rtt: Duration,
    /// Current congestion window of the connection
    pub cwnd: u64,
    /// Congestion events on the connection
    pub congestion_events: u64,
    /// The amount of packets lost on this path
    pub lost_packets: u64,
    /// The amount of bytes lost on this path
    pub lost_bytes: u64,
    /// The amount of packets sent on this path
    pub sent_packets: u64,
    /// The amount of PLPMTUD probe packets sent on this path (also counted by `sent_packets`)
    pub sent_plpmtud_probes: u64,
    /// The amount of PLPMTUD probe packets lost on this path (ignored by `lost_packets` and
    /// `lost_bytes`)
    pub lost_plpmtud_probes: u64,
    /// The number of times a black hole was detected in the path
    pub black_holes_detected: u64,
    /// Largest UDP payload size the path currently supports
    pub current_mtu: u16,
}

/// Connection statistics
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct ConnectionStats {
    /// Statistics about UDP datagrams transmitted on a connection
    pub udp_tx: UdpStats,
    /// Statistics about UDP datagrams received on a connection
    pub udp_rx: UdpStats,
    /// Statistics about frames transmitted on a connection
    pub frame_tx: FrameStats,
    /// Statistics about frames received on a connection
    pub frame_rx: FrameStats,
}
