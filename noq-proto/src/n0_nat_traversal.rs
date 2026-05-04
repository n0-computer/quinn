//! n0's (<https://n0.computer>) NAT Traversal protocol implementation.

use std::{
    collections::hash_map::Entry,
    fmt::Display,
    net::{IpAddr, SocketAddr},
};

use rustc_hash::{FxHashMap, FxHashSet};
use tracing::{debug, trace};

use crate::{
    FourTuple, Side, VarInt,
    connection::spaces::PendingReachOutFrames,
    frame::{AddAddress, ReachOut, RemoveAddress},
};

/// An IP & port.
///
/// Invariant: This value should always be in the ip family that the local
/// socket operates in.
/// E.g. if the local socket is ipv4, then all `IpPort`s should only have
/// IPv4 addresses, and if the socket supports ipv6, then all `IpPort`s
/// should be IPv6 addresses or IPv6-mapped IPv4 addresses.
///
/// See also [`map_to_local_socket_family`], which powers this conversion.
type IpPort = (IpAddr, u16);

/// An IP & port in canonical form.
///
/// Avoids using ipv6-mapped ipv4 addresses.
/// This is the primary type used to send ip addresses around remotely
/// and the primary type used to canonicalize received addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct CanonicalIpPort {
    canonical_ip: IpAddr,
    port: u16,
}

impl CanonicalIpPort {
    pub(crate) fn ip(&self) -> IpAddr {
        self.canonical_ip
    }

    pub(crate) fn port(&self) -> u16 {
        self.port
    }

    /// Converts this into a local-socket-family-mapped IP & port.
    ///
    /// Instead of using ipv4 and ipv6 addresses, this tries to match `ipv6`, which
    /// should indicate whether the local socket supports ipv6 or not.
    ///
    /// If ipv6 is supported, all ipv4 addresses are mapped using ipv6-mapped ipv4
    /// addresses.
    /// If ipv6 is not supported, then this returns `None` for ipv6 addresses.
    ///
    /// See also [`map_to_local_socket_family`].
    pub(crate) fn as_local_socket_family(&self, ipv6: bool) -> Option<IpPort> {
        Some((
            map_to_local_socket_family(self.canonical_ip, ipv6)?,
            self.port,
        ))
    }

    /// Returns this address as-is with the canonical IP used in a `SocketAddr`.
    pub(crate) fn as_canonical_addr(&self) -> SocketAddr {
        SocketAddr::new(self.canonical_ip, self.port)
    }
}

impl Display for CanonicalIpPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_canonical_addr().fmt(f)
    }
}

impl From<SocketAddr> for CanonicalIpPort {
    fn from(addr: SocketAddr) -> Self {
        Self {
            canonical_ip: addr.ip().to_canonical(),
            port: addr.port(),
        }
    }
}

impl From<IpPort> for CanonicalIpPort {
    fn from((ip, port): IpPort) -> Self {
        Self {
            canonical_ip: ip.to_canonical(),
            port,
        }
    }
}

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
        match self {
            Self::NotNegotiated => Err(Error::ExtensionNotNegotiated),
            Self::ClientSide(client_state) => {
                client_state.add_local_address(address)?;
                Ok(None)
            }
            Self::ServerSide(server_state) => server_state.add_local_address(address),
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
                .map(CanonicalIpPort::as_canonical_addr)
                .collect()),
            Self::ServerSide(server_state) => Ok(server_state
                .local_addresses
                .keys()
                .map(CanonicalIpPort::as_canonical_addr)
                .collect()),
        }
    }

    /// Returns the next ready probe's address.
    ///
    /// If this is actually sent you must call [`Self::mark_probe_sent`].
    pub(crate) fn next_probe_addr(&self) -> Option<IpPort> {
        match self {
            Self::NotNegotiated => None,
            Self::ClientSide(state) => state.next_probe_addr(),
            Self::ServerSide(state) => state.next_probe_addr(),
        }
    }

    /// Marks a probe as sent to the address with the challenge.
    pub(crate) fn mark_probe_sent(&mut self, remote: IpPort, challenge: u64) {
        match self {
            Self::NotNegotiated => (),
            Self::ClientSide(state) => state.mark_probe_sent(remote, challenge),
            Self::ServerSide(state) => state.mark_probe_sent(remote, challenge),
        }
    }

    /// Re-queues probes that have not yet succeeded or reached 0 remaining retries.
    ///
    /// Returns whether any probes are now queued to send. In this case the
    /// `NatTraversalProbeRetry` timer needs to be reset.
    pub(crate) fn queue_retries(&mut self, ipv6: bool) -> bool {
        match self {
            Self::NotNegotiated => false,
            Self::ClientSide(state) => state.queue_retries(ipv6),
            Self::ServerSide(state) => state.queue_retries(),
        }
    }

    /// Marks a remote as successful if the response matches a sent probe.
    ///
    /// Returns the open network path if it was a response to one of the NAT traversal
    /// probes. Note that the NAT probes are not padded to 1200 bytes so only the address is
    /// validated, but not the entire path.
    pub(crate) fn handle_path_response(&mut self, src: FourTuple, challenge: u64) -> bool {
        match self {
            Self::NotNegotiated => false,
            Self::ClientSide(state) => state.handle_path_response(src, challenge),
            Self::ServerSide(state) => state.handle_path_response(src, challenge),
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
    /// Candidate addresses the remote endpoint advertises.
    ///
    /// These are addresses on which the server is potentially reachable, to use for NAT
    /// traversal attempts.
    ///
    /// They are indexed by their ADD_ADDRESS sequence id and stored in **canonical
    /// form**. Not in the socket-native form as usual. This because we need to store them
    /// so we have the correct sequence IDs.
    remote_addresses: FxHashMap<VarInt, (CanonicalIpPort, ProbeState)>,
    /// Candidate addresses for the local endpoint.
    ///
    /// These are addresses on which we are potentially reachable, to use for NAT traversal
    /// attempts.
    ///
    /// They are stored in **canonical form**, not in socket-native form as usual. We may
    /// nave a reflexive address that is IPv6 even if our local socket can only handle IPv4.
    local_addresses: FxHashSet<CanonicalIpPort>,
    /// Current nat traversal round.
    round: VarInt,
    /// The data of PATH_CHALLENGE frames sent in probes.
    ///
    /// These are cleared when a new round starts, so any late-arriving PATH_RESPONSEs will
    /// have no effect.
    ///
    /// They are stored in the usual socket-native form.
    sent_challenges: FxHashMap<u64, IpPort>,
    /// Queued probes to be sent in the next [`poll_transmit`] call.
    ///
    /// [`poll_transmit`]: crate::connection::Connection::poll_transmit
    ///
    /// They are stored in the usual socket-native form. Probes to address families not
    /// addressable by the family are never inserted.
    pending_probes: FxHashSet<IpPort>,
    /// Network paths that were successfully probed but not yet opened.
    ///
    /// When we do not have enough CIDs or free path IDs we might not have been able to open
    /// a new path. This allows us to try re-open the path when we get new CIDs or a new
    /// MAX_PATH_ID.
    // TODO(flub): perhaps there should be a time-limit on these?
    paths_to_be_opened: Vec<FourTuple>,
}

impl ClientState {
    fn new(max_remote_addresses: usize, max_local_addresses: usize) -> Self {
        Self {
            max_remote_addresses,
            max_local_addresses,
            remote_addresses: Default::default(),
            local_addresses: Default::default(),
            round: Default::default(),
            sent_challenges: Default::default(),
            pending_probes: Default::default(),
            paths_to_be_opened: Default::default(),
        }
    }

    fn add_local_address(&mut self, address: SocketAddr) -> Result<(), Error> {
        let address = CanonicalIpPort::from(address);
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
        let address = CanonicalIpPort::from(*address);
        self.local_addresses.remove(&address);
    }

    /// Initiates a new nat traversal round.
    ///
    /// A nat traversal round involves advertising the client's local addresses in
    /// `REACH_OUT` frames, and initiating probing of the known remote addresses. When a new
    /// round is initiated, the previous one is cancelled.
    ///
    /// `ipv6` indicates if the connection runs on a socket that supports IPv6. If so, then
    /// all addresses returned [`PendingReachOutFrames`] will be IPv6 addresses (and
    /// IPv4-mapped IPv6 addresses if necessary). Otherwise they're all IPv4 addresses.  See
    /// also [`map_to_local_socket_family`].
    ///
    /// # Returns
    ///
    /// The REACH_OUT frames that need to be sent to the peer and the probed addresses. The
    /// probed addresses are only informational, the pending probes are stored in
    /// [`Self::pending_probes`].
    ///
    /// If the probed addresses are non-empty the `NatTraversalProbeRetry` timer needs to be
    /// set.
    pub(crate) fn initiate_nat_traversal_round(
        &mut self,
        ipv6: bool,
    ) -> Result<(PendingReachOutFrames, Vec<SocketAddr>), Error> {
        if self.local_addresses.is_empty() {
            return Err(Error::NotEnoughAddresses);
        }

        self.round = self.round.saturating_add(1u8);
        self.sent_challenges.clear();
        self.pending_probes.clear();

        // Enqueue the NAT probes to known remote addresses.
        self.remote_addresses
            .values_mut()
            .for_each(|(ip_port, state)| {
                if let Some(ip_port) = ip_port.as_local_socket_family(ipv6) {
                    self.pending_probes.insert(ip_port);
                    *state = ProbeState::Active(MAX_NAT_PROBE_ATTEMPTS - 1);
                } else {
                    trace!(%ip_port, "not using IPv6 NAT candidate for IPv4 socket");
                    *state = ProbeState::Active(0);
                }
            });
        let probed_addrs: Vec<SocketAddr> = self
            .pending_probes
            .iter()
            .copied()
            .map(Into::into)
            .collect();

        // Build the REACH_OUT frames.
        let reach_out_frames: PendingReachOutFrames = self
            .local_addresses
            .iter()
            .map(|ip_port| ReachOut {
                round: self.round,
                ip: ip_port.ip(),
                port: ip_port.port(),
            })
            .collect();

        trace!(
            round = %self.round,
            reach_out = %reach_out_frames.len(),
            to_probe = %self.pending_probes.len(),
            "initiating NAT traversal round",
        );
        Ok((reach_out_frames, probed_addrs))
    }

    /// Re-queues probes that have not yet succeeded or reached 0 remaining retries.
    ///
    /// Returns whether any probes are now queued to send. In this case the
    /// `NatTraversalProbeRetry` timer needs to be reset.
    ///
    /// `ipv6` as for [`Self::initiate_nat_traversal_round`].
    pub(crate) fn queue_retries(&mut self, ipv6: bool) -> bool {
        self.remote_addresses
            .values_mut()
            .for_each(|(ip_port, state)| match state {
                ProbeState::Active(remaining) if *remaining > 0 => {
                    *remaining -= 1;
                    if let Some(ip_port) = ip_port.as_local_socket_family(ipv6) {
                        self.pending_probes.insert(ip_port);
                    } else {
                        trace!(%ip_port, "skipping IPv6 NAT candidate for IPv4 socket");
                        *remaining = 0;
                    }
                }
                ProbeState::Active(_) | ProbeState::Succeeded => {}
            });
        !self.pending_probes.is_empty()
    }

    /// Returns the next ready probe's address.
    ///
    /// If this is actually sent you must call [`Self::mark_probe_sent`].
    fn next_probe_addr(&self) -> Option<IpPort> {
        self.pending_probes.iter().next().copied()
    }

    /// Marks a probe as sent to the address with the challenge.
    fn mark_probe_sent(&mut self, remote: IpPort, challenge: u64) {
        self.pending_probes.remove(&remote);
        self.sent_challenges.insert(challenge, remote);
    }

    /// Adds an address to the remote set.
    ///
    /// On success returns the address if it was new to the set. It will error when the set
    /// has no capacity for the address.
    ///
    /// If this is called while a round is in progress this will effectively add the address
    /// to the current round. There is no guarantee however that the current round is still
    /// in progress however, if the last [`Self::queue_retries`] call returned `false` the
    /// round has stopped.
    // TODO(flub): probably should add an event to signal that the round is finished, so
    //    that the application knows to start a new round.
    pub(crate) fn add_remote_address(
        &mut self,
        add_addr: AddAddress,
    ) -> Result<Option<SocketAddr>, Error> {
        let AddAddress { seq_no, ip, port } = add_addr;
        let address = CanonicalIpPort::from((ip, port));
        let allow_new = self.remote_addresses.len() < self.max_remote_addresses;
        match self.remote_addresses.entry(seq_no) {
            Entry::Occupied(mut occupied_entry) => {
                let is_update = occupied_entry.get().0 != address;
                if is_update {
                    occupied_entry.insert((address, ProbeState::Active(MAX_NAT_PROBE_ATTEMPTS)));
                }
                // The value might be different. This should not happen, but we assume that the new
                // address is more recent than the previous, and thus worth updating
                Ok(is_update.then_some(address.as_canonical_addr()))
            }
            Entry::Vacant(vacant_entry) if allow_new => {
                vacant_entry.insert((address, ProbeState::Active(MAX_NAT_PROBE_ATTEMPTS)));
                Ok(Some(address.as_canonical_addr()))
            }
            _ => Err(Error::TooManyAddresses),
        }
    }

    /// Removes an address from the remote set.
    ///
    /// Returns whether the address was present.
    pub(crate) fn remove_remote_address(
        &mut self,
        remove_addr: RemoveAddress,
    ) -> Option<SocketAddr> {
        self.remote_addresses
            .remove(&remove_addr.seq_no)
            .map(|(address, _)| address.as_canonical_addr())
    }

    /// Checks that a received remote address is valid.
    ///
    /// An address is valid as long as it does not change the value of a known address id.
    pub(crate) fn check_remote_address(&self, add_addr: &AddAddress) -> bool {
        match self.remote_addresses.get(&add_addr.seq_no) {
            None => true,
            Some((existing, _)) => *existing == CanonicalIpPort::from(add_addr.ip_port()),
        }
    }

    pub(crate) fn get_remote_nat_traversal_addresses(&self) -> Vec<SocketAddr> {
        self.remote_addresses
            .values()
            .map(|(address, _)| (*address).as_canonical_addr())
            .collect()
    }

    /// Marks a remote as successful if the response matches a sent probe.
    ///
    /// Returns `true` if it was a response to one of the NAT traversal probes. In that case
    /// [`Self::pop_pending_path_open`] should be called to open the next path.
    fn handle_path_response(&mut self, network_path: FourTuple, challenge: u64) -> bool {
        if let Entry::Occupied(entry) = self.sent_challenges.entry(challenge) {
            let remote = (network_path.remote().ip(), network_path.remote().port());
            if *entry.get() == remote {
                entry.remove();

                // self.remote_addresses is stored in canonical form.
                let remote = CanonicalIpPort::from(remote);
                // TODO: linear search is sad.
                if let Some(seq) = self
                    .remote_addresses
                    .iter()
                    .filter_map(
                        |(seq, (addr, _))| {
                            if *addr == remote { Some(*seq) } else { None }
                        },
                    )
                    .next()
                {
                    trace!(
                        ?network_path,
                        challenge = %display(format_args!("0x{challenge:x}")),
                        "Received valid NAT traversal probe response",
                    );
                    self.remote_addresses
                        .insert(seq, (remote, ProbeState::Succeeded));
                    self.paths_to_be_opened.push(network_path);
                    return true;
                } else {
                    debug!("inconsistent remote addrs and seq");
                }
            } else {
                debug!(
                    ?network_path.remote,
                    expected_remote = ?entry.get(),
                    challenge = %display(format_args!("0x{challenge:x}")),
                    "PATH_RESPONSE matched a NAT traversal probe but mismatching addr",
                )
            }
        }
        false
    }

    /// Returns a path that was NAT traversed and needs to be opened.
    pub(crate) fn pop_pending_path_open(&mut self) -> Option<FourTuple> {
        self.paths_to_be_opened.pop()
    }

    /// Put back a path that needs to be opened, e.g. for a temporary failure.
    pub(crate) fn push_pending_path_open(&mut self, network_path: FourTuple) {
        self.paths_to_be_opened.push(network_path)
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
    ///
    /// They are stored in **canonical form**, not in socket-native form as usual. We may
    /// nave a reflexive address that is IPv6 even if our local socket can only handle IPv4.
    local_addresses: FxHashMap<CanonicalIpPort, VarInt>,
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
    ///
    /// These are stored in the usual local-socket native form.
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

    fn add_local_address(&mut self, address: SocketAddr) -> Result<Option<AddAddress>, Error> {
        let address = CanonicalIpPort::from(address);
        let allow_new = self.local_addresses.len() < self.max_local_addresses;
        match self.local_addresses.entry(address) {
            Entry::Occupied(_) => Ok(None),
            Entry::Vacant(vacant_entry) if allow_new => {
                let id = self.next_local_addr_id;
                self.next_local_addr_id = self.next_local_addr_id.saturating_add(1u8);
                vacant_entry.insert(id);
                Ok(Some(AddAddress::new((address.ip(), address.port()), id)))
            }
            _ => Err(Error::TooManyAddresses),
        }
    }

    fn remove_local_address(&mut self, address: &IpPort) -> Option<RemoveAddress> {
        let address = CanonicalIpPort::from(*address);
        self.local_addresses
            .remove(&address)
            .map(RemoveAddress::new)
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
    /// Returns whether any probes are now queued to send. In this case the
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
    fn next_probe_addr(&self) -> Option<IpPort> {
        self.pending_probes.iter().next().cloned()
    }

    /// Marks a probe as sent to the address with the challenge.
    fn mark_probe_sent(&mut self, remote: IpPort, challenge: u64) {
        self.pending_probes.remove(&remote);
        self.sent_challenges.insert(challenge, remote);
    }

    /// Marks a remote as successful if the response matches a sent probe.
    ///
    /// Returns `true` if it was a response to one of the NAT traversal probes.
    fn handle_path_response(&mut self, src: FourTuple, challenge: u64) -> bool {
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
            state.mark_probe_sent(remote, challenge);
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
