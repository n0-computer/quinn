//! n0's (<https://n0.computer>) NAT Traversal protocol implementation.

use std::{
    collections::hash_map::Entry,
    net::{IpAddr, SocketAddr},
};

use rustc_hash::{FxHashMap, FxHashSet};
use tracing::{debug, trace};

use crate::{
    FourTuple, PathId, Side, VarInt,
    frame::{AddAddress, ReachOut, RemoveAddress},
};

type IpPort = (IpAddr, u16);

/// Errors that the nat traversal state might encounter.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An endpoint (local or remote) tried to add too many addresses to their advertised set
    #[error("Tried to add too many addresses to their advertised set")]
    TooManyAddresses,
    /// The operation is not allowed for this endpoint's connection side
    #[error("Not allowed for this endpoint's connection side")]
    WrongConnectionSide,
    /// The extension was not negotiated
    #[error("n0's nat traversal was not negotiated")]
    ExtensionNotNegotiated,
    /// Not enough addresses to complete the operation
    #[error("Not enough addresses")]
    NotEnoughAddresses,
    /// Nat traversal attempt failed due to a multipath error
    #[error("Failed to establish paths {0}")]
    Multipath(super::PathError),
    /// Attempted to initiate NAT traversal on a closed, or closing connection.
    #[error("The connection is already closed")]
    Closed,
}

pub(crate) struct NatTraversalRound {
    /// Sequence number to use for the new reach out frames.
    pub(crate) new_round: VarInt,
    /// Addresses to use to send reach out frames.
    pub(crate) reach_out_at: FxHashSet<IpPort>,
    /// Remotes to probe by attempting to open new paths.
    ///
    /// The addresses include their Id, so that it can be used to signal these should be returned
    /// in a nat traversal continuation by calling [`ClientState::report_in_continuation`].
    ///
    /// These are filtered and mapped to the IP family the local socket supports.
    pub(crate) addresses_to_probe: Vec<(VarInt, IpPort)>,
    /// [`PathId`]s of the cancelled round.
    pub(crate) prev_round_path_ids: Vec<PathId>,
}

/// Event emitted when the client receives ADD_ADDRESS or REMOVE_ADDRESS frames.
#[derive(Debug, Clone)]
pub enum Event {
    /// An ADD_ADDRESS frame was received.
    AddressAdded(SocketAddr),
    /// A REMOVE_ADDRESS frame was received.
    AddressRemoved(SocketAddr),
}

/// State kept for n0's nat traversal
#[derive(Debug, Default)]
pub(crate) enum State {
    #[default]
    NotNegotiated,
    ClientSide(ClientState),
    ServerSide(ServerState),
}

impl State {
    pub(crate) fn new(max_remote_addresses: u8, max_local_addresses: u8, side: Side) -> Self {
        match side {
            Side::Client => Self::ClientSide(ClientState::new(
                max_remote_addresses.into(),
                max_local_addresses.into(),
            )),
            Side::Server => Self::ServerSide(ServerState::new(
                max_remote_addresses.into(),
                max_local_addresses.into(),
            )),
        }
    }

    pub(crate) fn is_negotiated(&self) -> bool {
        match self {
            Self::NotNegotiated => false,
            Self::ClientSide(_) | Self::ServerSide(_) => true,
        }
    }

    pub(crate) fn client_side(&self) -> Result<&ClientState, Error> {
        match self {
            Self::NotNegotiated => Err(Error::ExtensionNotNegotiated),
            Self::ClientSide(client_side) => Ok(client_side),
            Self::ServerSide(_) => Err(Error::WrongConnectionSide),
        }
    }

    pub(crate) fn client_side_mut(&mut self) -> Result<&mut ClientState, Error> {
        match self {
            Self::NotNegotiated => Err(Error::ExtensionNotNegotiated),
            Self::ClientSide(client_side) => Ok(client_side),
            Self::ServerSide(_) => Err(Error::WrongConnectionSide),
        }
    }

    pub(crate) fn server_side_mut(&mut self) -> Result<&mut ServerState, Error> {
        match self {
            Self::NotNegotiated => Err(Error::ExtensionNotNegotiated),
            Self::ClientSide(_) => Err(Error::WrongConnectionSide),
            Self::ServerSide(server_side) => Ok(server_side),
        }
    }

    /// Adds a local address to use for nat traversal.
    ///
    /// When this endpoint is the server within the connection, these addresses will be sent to the
    /// client in add address frames. For clients, these addresses will be sent in reach out frames
    /// when nat traversal attempts are initiated.
    ///
    /// If a frame should be sent, it is returned.
    pub(crate) fn add_local_address(
        &mut self,
        address: SocketAddr,
    ) -> Result<Option<AddAddress>, Error> {
        let ip_port = IpPort::from((address.ip(), address.port()));
        match self {
            Self::NotNegotiated => Err(Error::ExtensionNotNegotiated),
            Self::ClientSide(client_state) => {
                client_state.add_local_address(ip_port)?;
                Ok(None)
            }
            Self::ServerSide(server_state) => server_state.add_local_address(ip_port),
        }
    }

    /// Removes a local address from the advertised set for nat traversal.
    ///
    /// When this endpoint is the server, removed addresses must be reported with remove address
    /// frames. Clients will simply stop reporting these addresses in reach out frames.
    ///
    /// If a frame should be sent, it is returned.
    pub(crate) fn remove_local_address(
        &mut self,
        address: SocketAddr,
    ) -> Result<Option<RemoveAddress>, Error> {
        let address = IpPort::from((address.ip(), address.port()));
        match self {
            Self::NotNegotiated => Err(Error::ExtensionNotNegotiated),
            Self::ClientSide(client_state) => {
                client_state.remove_local_address(&address);
                Ok(None)
            }
            Self::ServerSide(server_state) => Ok(server_state.remove_local_address(&address)),
        }
    }

    pub(crate) fn get_local_nat_traversal_addresses(&self) -> Result<Vec<SocketAddr>, Error> {
        match self {
            Self::NotNegotiated => Err(Error::ExtensionNotNegotiated),
            Self::ClientSide(client_state) => Ok(client_state
                .local_addresses
                .iter()
                .copied()
                .map(Into::into)
                .collect()),
            Self::ServerSide(server_state) => Ok(server_state
                .local_addresses
                .keys()
                .copied()
                .map(Into::into)
                .collect()),
        }
    }
}

#[derive(Debug)]
pub(crate) struct ClientState {
    /// Max number of remote addresses we allow
    ///
    /// This is set by the local endpoint.
    max_remote_addresses: usize,
    /// Max number of local addresses allowed
    ///
    /// This is set by the remote endpoint.
    max_local_addresses: usize,
    /// Candidate addresses the remote server reports as potentially reachable, to use for nat
    /// traversal attempts.
    ///
    /// These are indexed by their advertised Id. For each address, whether the address should be
    /// reported in nat traversal continuations is kept.
    remote_addresses: FxHashMap<VarInt, (IpPort, bool)>,
    /// Candidate addresses the local client reports as potentially reachable, to use for nat
    /// traversal attempts.
    local_addresses: FxHashSet<IpPort>,
    /// Current nat traversal round.
    round: VarInt,
    /// [`PathId`]s used to probe remotes assigned to this round.
    round_path_ids: Vec<PathId>,
}

impl ClientState {
    fn new(max_remote_addresses: usize, max_local_addresses: usize) -> Self {
        Self {
            max_remote_addresses,
            max_local_addresses,
            remote_addresses: Default::default(),
            local_addresses: Default::default(),
            round: Default::default(),
            round_path_ids: Default::default(),
        }
    }

    fn add_local_address(&mut self, address: IpPort) -> Result<(), Error> {
        if self.local_addresses.len() < self.max_local_addresses {
            self.local_addresses.insert(address);
            Ok(())
        } else if self.local_addresses.contains(&address) {
            // at capacity, but the address is known, no issues here
            Ok(())
        } else {
            // at capacity and the address is new
            Err(Error::TooManyAddresses)
        }
    }

    fn remove_local_address(&mut self, address: &IpPort) {
        self.local_addresses.remove(address);
    }

    /// Initiates a new nat traversal round.
    ///
    /// A nat traversal round involves advertising the client's local addresses in `REACH_OUT`
    /// frames, and initiating probing of the known remote addresses. When a new round is
    /// initiated, the previous one is cancelled, and paths that have not been opened should be
    /// closed.
    ///
    /// `ipv6` indicates if the connection runs on a socket that supports IPv6. If so, then all
    /// addresses returned in [`NatTraversalRound`] will be IPv6 addresses (and IPv4-mapped IPv6
    /// addresses if necessary). Otherwise they're all IPv4 addresses.
    /// See also [`map_to_local_socket_family`].
    pub(crate) fn initiate_nat_traversal_round(
        &mut self,
        ipv6: bool,
    ) -> Result<NatTraversalRound, Error> {
        if self.local_addresses.is_empty() {
            return Err(Error::NotEnoughAddresses);
        }

        let prev_round_path_ids = std::mem::take(&mut self.round_path_ids);
        self.round = self.round.saturating_add(1u8);
        let mut addresses_to_probe = Vec::with_capacity(self.remote_addresses.len());
        for (id, ((ip, port), report_in_continuation)) in self.remote_addresses.iter_mut() {
            *report_in_continuation = false;

            if let Some(ip) = map_to_local_socket_family(*ip, ipv6) {
                addresses_to_probe.push((*id, (ip, *port)));
            } else {
                trace!(?ip, "not using IPv6 nat candidate for IPv4 socket");
            }
        }

        Ok(NatTraversalRound {
            new_round: self.round,
            reach_out_at: self.local_addresses.iter().copied().collect(),
            addresses_to_probe,
            prev_round_path_ids,
        })
    }

    /// Mark a remote address to be reported back in a nat traversal continuation if the error is
    /// considered spurious from a nat traversal point of view.
    ///
    /// Ids not present are silently ignored.
    pub(crate) fn report_in_continuation(&mut self, id: VarInt, e: crate::PathError) {
        match e {
            crate::PathError::MaxPathIdReached | crate::PathError::RemoteCidsExhausted => {
                if let Some((_address, report_in_continuation)) = self.remote_addresses.get_mut(&id)
                {
                    *report_in_continuation = true;
                }
            }
            _ => {}
        }
    }

    /// Returns an address that needs to be probed, if any.
    ///
    /// The address will not be returned twice unless marked as such again with
    /// [`Self::report_in_continuation`].
    ///
    /// `ipv6` indicates if the connection runs on a socket that supports IPv6. If so, then all
    /// addresses returned in [`NatTraversalRound`] will be IPv6 addresses (and IPv4-mapped IPv6
    /// addresses if necessary). Otherwise they're all IPv4 addresses.
    /// See also [`map_to_local_socket_family`].
    pub(crate) fn continue_nat_traversal_round(&mut self, ipv6: bool) -> Option<(VarInt, IpPort)> {
        // this being random depends on iteration not returning always on the same order
        let (id, (address, report_in_continuation)) = self
            .remote_addresses
            .iter_mut()
            .filter(|(_id, (_addr, report))| *report)
            .filter_map(|(id, ((ip, port), report))| {
                // only continue with addresses we can send on our local socket
                let Some(ip) = map_to_local_socket_family(*ip, ipv6) else {
                    trace!(?ip, "not using IPv6 nat candidate for IPv4 socket");
                    return None;
                };
                Some((*id, ((ip, *port), report)))
            })
            .next()?;
        *report_in_continuation = false;
        Some((id, address))
    }

    /// Add a [`PathId`] as part of the current attempts to create paths based on the server's
    /// advertised addresses.
    pub(crate) fn set_round_path_ids(&mut self, path_ids: Vec<PathId>) {
        self.round_path_ids = path_ids;
    }

    /// Add a [`PathId`] as part of the current attempts to create paths based on the server's
    /// advertised addresses.
    pub(crate) fn add_round_path_id(&mut self, path_id: PathId) {
        self.round_path_ids.push(path_id);
    }

    /// Adds an address to the remote set
    ///
    /// On success returns the address if it was new to the set. It will error when the set has no
    /// capacity for the address.
    pub(crate) fn add_remote_address(
        &mut self,
        add_addr: AddAddress,
    ) -> Result<Option<SocketAddr>, Error> {
        let AddAddress { seq_no, ip, port } = add_addr;
        let address = (ip, port);
        let allow_new = self.remote_addresses.len() < self.max_remote_addresses;
        match self.remote_addresses.entry(seq_no) {
            Entry::Occupied(mut occupied_entry) => {
                let is_update = occupied_entry.get().0 != address;
                if is_update {
                    occupied_entry.insert((address, false));
                }
                // The value might be different. This should not happen, but we assume that the new
                // address is more recent than the previous, and thus worth updating
                Ok(is_update.then_some(address.into()))
            }
            Entry::Vacant(vacant_entry) if allow_new => {
                vacant_entry.insert((address, false));
                Ok(Some(address.into()))
            }
            _ => Err(Error::TooManyAddresses),
        }
    }

    /// Removes an address from the remote set
    ///
    /// Returns whether the address was present.
    pub(crate) fn remove_remote_address(
        &mut self,
        remove_addr: RemoveAddress,
    ) -> Option<SocketAddr> {
        self.remote_addresses
            .remove(&remove_addr.seq_no)
            .map(|(address, _report_in_continuation)| address.into())
    }

    /// Checks that a received remote address is valid
    ///
    /// An address is valid as long as it does not change the value of a known address id.
    pub(crate) fn check_remote_address(&self, add_addr: &AddAddress) -> bool {
        match self.remote_addresses.get(&add_addr.seq_no) {
            None => true,
            Some((existing, _)) => existing == &add_addr.ip_port(),
        }
    }

    pub(crate) fn get_remote_nat_traversal_addresses(&self) -> Vec<SocketAddr> {
        self.remote_addresses
            .values()
            .map(|(address, _report_in_continuation)| (*address).into())
            .collect()
    }
}

/// Maximum number of times we send a NAT probe to the same remote address in a round.
///
/// This is a trade-off between several factors:
/// - Probe packets could be lost. This allows recovery.
/// - We may need two probes to reach the NAT firewall to get through.
/// - We may be sending probes to innocent bystanders on the internet.
/// - A round never "finishes": probing of remotes only stops when:
///   1. A new round is started.
///   2. A probe was successful.
///   3. This number of attempts is exhausted.
///
/// Currently probes are retried after 2/3rd of the configured initial RTT. At a
/// fixed interval, so without exponential backoff. For the default initial RTT of 333ms
/// this is 222ms. So 10 attempts covers 2220ms.
// TODO(flub): I would like to improve this sometime so that we cover about 2s but with only
//    about 5-6 probes. The three initial probes should be faster, later probes should start
//    to slow down. Unfortunately we only have one timer for the entire round currently, we
//    would need to have a timer per remote. Because REACH_OUT frames can appear in the
//    middle of a round.
pub(crate) const MAX_NAT_PROBE_ATTEMPTS: u8 = 10;

/// State of an off-path NAT traversal probe to a remote address.
#[derive(Debug)]
enum ProbeState {
    /// The remote still needs to be probed in this round.
    ///
    /// The remaining number of retries are stored in the `u8`.
    Active(u8),
    /// We received a probe response for this remote.
    Succeeded,
}

#[derive(Debug)]
pub(crate) struct ServerState {
    /// Max number of remote addresses we allow.
    ///
    /// This is set by the local endpoint.
    max_remote_addresses: usize,
    /// Max number of local addresses allowed.
    ///
    /// This is set by the remote endpoint.
    max_local_addresses: usize,
    /// Candidate addresses the server reports as potentially reachable, to use for nat
    /// traversal attempts.
    local_addresses: FxHashMap<IpPort, VarInt>,
    /// The next id to use for local addresses sent to the client.
    next_local_addr_id: VarInt,
    /// Current nat traversal round
    ///
    /// Servers keep track of the client's most recent round and cancel probing related to previous
    /// rounds.
    round: VarInt,
    /// The remote addresses participating in this round.
    ///
    /// The set is cleared when a new round starts.
    remotes: FxHashMap<IpPort, ProbeState>,
    /// The data of PATH_CHALLENGE frames sent in probes.
    ///
    /// These are cleared when a new round starts, so any late-arriving PATH_RESPONSEs will
    /// have no effect.
    sent_challenges: FxHashMap<u64, IpPort>,
    /// Queued probes to be sent in the next [`poll_transmit`] call.
    ///
    /// At the beginning of a round this is populated from REACH_OUT frames and at every
    /// retry this is populated from [`Self::remotes`].
    ///
    /// [`poll_transmit`]: crate::connection::Connection::poll_transmit
    pending_probes: FxHashSet<IpPort>,
}

impl ServerState {
    fn new(max_remote_addresses: usize, max_local_addresses: usize) -> Self {
        Self {
            max_remote_addresses,
            max_local_addresses,
            local_addresses: Default::default(),
            next_local_addr_id: Default::default(),
            round: Default::default(),
            remotes: Default::default(),
            sent_challenges: Default::default(),
            pending_probes: Default::default(),
        }
    }

    fn add_local_address(&mut self, address: IpPort) -> Result<Option<AddAddress>, Error> {
        let allow_new = self.local_addresses.len() < self.max_local_addresses;
        match self.local_addresses.entry(address) {
            Entry::Occupied(_) => Ok(None),
            Entry::Vacant(vacant_entry) if allow_new => {
                let id = self.next_local_addr_id;
                self.next_local_addr_id = self.next_local_addr_id.saturating_add(1u8);
                vacant_entry.insert(id);
                Ok(Some(AddAddress::new(address, id)))
            }
            _ => Err(Error::TooManyAddresses),
        }
    }

    fn remove_local_address(&mut self, address: &IpPort) -> Option<RemoveAddress> {
        self.local_addresses.remove(address).map(RemoveAddress::new)
    }

    /// Returns the current NAT traversal round number.
    pub(crate) fn current_round(&self) -> VarInt {
        self.round
    }

    /// Handles a received REACH_OUT frame.
    ///
    /// This might ignore the reach out frame if it belongs to an older round or if the
    /// frame contains an IPv6 address while the local socket is IPv4-only.
    ///
    /// If a new round was started, the `NatTraversalProbeRetry` timer needs to be reset.
    pub(crate) fn handle_reach_out(
        &mut self,
        reach_out: ReachOut,
        ipv6: bool,
    ) -> Result<(), Error> {
        let ReachOut { round, ip, port } = reach_out;

        if round < self.round {
            trace!(current_round=%self.round, "ignoring REACH_OUT for previous round");
            return Ok(());
        }
        let Some(ip) = map_to_local_socket_family(ip, ipv6) else {
            trace!("Ignoring IPv6 REACH_OUT frame due to not supporting IPv6 locally");
            return Ok(());
        };

        if round > self.round {
            self.round = round;
            self.remotes.clear();
            self.sent_challenges.clear();
            self.pending_probes.clear();
        } else if self.remotes.contains_key(&(ip, port)) {
            // Retransmitted frame.
            return Ok(());
        } else if self.remotes.len() >= self.max_remote_addresses {
            return Err(Error::TooManyAddresses);
        }
        self.remotes
            .entry((ip, port))
            .or_insert(ProbeState::Active(MAX_NAT_PROBE_ATTEMPTS - 1));
        self.pending_probes.insert((ip, port));
        Ok(())
    }

    /// Re-queues probes that have not yet succeeded or reached [`MAX_NAT_PROBE_ATTEMPTS`].
    ///
    /// Returns whether any probes are now queued to send.  In this case the
    /// `NatTraversalProbeRetry` timer needs to be reset.
    pub(crate) fn queue_retries(&mut self) -> bool {
        self.remotes
            .iter_mut()
            .for_each(|(remote, state)| match state {
                ProbeState::Active(remaining) if *remaining > 0 => {
                    *remaining -= 1;
                    self.pending_probes.insert(*remote);
                }
                ProbeState::Active(_) | ProbeState::Succeeded => (),
            });
        !self.pending_probes.is_empty()
    }

    /// Returns the next ready probe's address.
    ///
    /// If this is actually sent you must call [`Self::mark_probe_sent`].
    pub(crate) fn next_probe_addr(&self) -> Option<SocketAddr> {
        self.pending_probes.iter().next().map(|addr| (*addr).into())
    }

    /// Marks a probe as sent to the address with the challenge.
    pub(crate) fn mark_probe_sent(&mut self, remote: IpPort, challenge: u64) {
        self.pending_probes.remove(&remote);
        self.sent_challenges.insert(challenge, remote);
    }

    /// Marks a remote as successful if the response matches a sent probe.
    ///
    /// Returns `true` if it was a response to one of the NAT traversal probes.
    pub(crate) fn handle_path_response(&mut self, src: FourTuple, challenge: u64) -> bool {
        if let Entry::Occupied(entry) = self.sent_challenges.entry(challenge) {
            let remote = (src.remote().ip(), src.remote().port());
            if *entry.get() == remote {
                entry.remove();
                self.remotes.insert(remote, ProbeState::Succeeded);
                return true;
            } else {
                debug!(
                    ?challenge,
                    ?src.remote,
                    "PATH_RESPONSE matched a NAT traversal probe but mismatching addr",
                )
            }
        }
        false
    }
}

/// Returns the given address as canonicalized IP address.
///
/// This checks that the address family is supported by our local socket.
/// If it is supported, then the address is mapped to the respective IP address.
/// If the given address is an IPv6 address, but our local socket doesn't support
/// IPv6, then this returns `None`.
pub(crate) fn map_to_local_socket_family(address: IpAddr, ipv6: bool) -> Option<IpAddr> {
    let ip = match address {
        IpAddr::V4(addr) if ipv6 => IpAddr::V6(addr.to_ipv6_mapped()),
        IpAddr::V4(_) => address,
        IpAddr::V6(_) if ipv6 => address,
        IpAddr::V6(addr) => IpAddr::V4(addr.to_ipv4_mapped()?),
    };
    Some(ip)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_server_state() {
        let mut state = ServerState::new(2, 2);

        state
            .handle_reach_out(
                ReachOut {
                    round: 1u32.into(),
                    ip: std::net::Ipv4Addr::LOCALHOST.into(),
                    port: 1,
                },
                true,
            )
            .unwrap();

        state
            .handle_reach_out(
                ReachOut {
                    round: 1u32.into(),
                    ip: "1.1.1.1".parse().unwrap(), //std::net::Ipv4Addr::LOCALHOST.into(),
                    port: 2,
                },
                true,
            )
            .unwrap();

        dbg!(&state);
        assert_eq!(state.pending_probes.len(), 2);

        // Helper: send next ready probe
        let mut challenge = 0;
        let mut send_probe = |state: &mut ServerState| {
            let remote = state.next_probe_addr().unwrap();
            challenge += 1;
            state.mark_probe_sent((remote.ip(), remote.port()), challenge);
        };

        send_probe(&mut state);
        send_probe(&mut state);

        // After sending both probes, no ready probes remain but they're still tracked.
        assert!(state.next_probe_addr().is_none());

        // After queuing retries, probes become available again
        assert!(state.queue_retries());
        send_probe(&mut state);
        send_probe(&mut state);

        // After 2 attempts each, retries still available (max is 10)
        assert!(state.queue_retries());
        send_probe(&mut state);
        send_probe(&mut state);

        // Exhaust remaining attempts
        for _ in 3..MAX_NAT_PROBE_ATTEMPTS {
            assert!(state.queue_retries());
            send_probe(&mut state);
            send_probe(&mut state);
        }

        // After max attempts, probes are removed
        assert!(!state.queue_retries());
        assert!(state.next_probe_addr().is_none());
    }

    #[test]
    fn test_map_to_local_socket() {
        assert_eq!(
            map_to_local_socket_family("1.1.1.1".parse().unwrap(), false),
            Some("1.1.1.1".parse().unwrap())
        );
        assert_eq!(
            map_to_local_socket_family("1.1.1.1".parse().unwrap(), true),
            Some("::ffff:1.1.1.1".parse().unwrap())
        );
        assert_eq!(
            map_to_local_socket_family("::1".parse().unwrap(), true),
            Some("::1".parse().unwrap())
        );
        assert_eq!(
            map_to_local_socket_family("::1".parse().unwrap(), false),
            None
        );
        assert_eq!(
            map_to_local_socket_family("::ffff:1.1.1.1".parse().unwrap(), false),
            Some("1.1.1.1".parse().unwrap())
        )
    }
}
