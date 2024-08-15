//! Address discovery types from
//! <https://datatracker.ietf.org/doc/draft-seemann-quic-address-discovery/>

use crate::VarInt;

pub(crate) const TRANSPORT_PARAMETER_CODE: u64 = 0x9f81a174;

/// The role of each participant.
///
/// When enabled, this is reported as a transport parameter.
#[derive(PartialEq, Eq, Clone, Copy, Debug, Default)]
pub(crate) enum Role {
    /// Is able to report observer addresses to other peers, but it's not interested in receiving
    /// reports about its own address.
    ProvideReportsOnly,
    /// Is interested on reports about its own observed address, but will not report back to other
    /// peers.
    ReceiveReportsOnly,
    /// Will both report and receive reports of observed addresses.
    Both,
    /// Address discovery is disabled.
    #[default]
    Disabled,
}

impl From<Role> for Option<VarInt> {
    fn from(role: Role) -> Self {
        match role {
            Role::ProvideReportsOnly => Some(VarInt(0)),
            Role::ReceiveReportsOnly => Some(VarInt(1)),
            Role::Both => Some(VarInt(2)),
            Role::Disabled => None,
        }
    }
}

impl TryFrom<VarInt> for Role {
    type Error = crate::transport_parameters::Error;

    fn try_from(value: VarInt) -> Result<Self, Self::Error> {
        match value.0 {
            0 => Ok(Self::ProvideReportsOnly),
            1 => Ok(Self::ReceiveReportsOnly),
            2 => Ok(Self::Both),
            _ => Err(crate::transport_parameters::Error::IllegalValue),
        }
    }
}

impl Role {
    /// Whether address discovery is disabled.
    pub(crate) fn is_disabled(&self) -> bool {
        matches!(self, Self::Disabled)
    }

    /// Whether this peer's role allows for address reporting to other peers.
    fn is_reporter(&self) -> bool {
        matches!(self, Self::ProvideReportsOnly | Self::Both)
    }

    /// Whether this peer's role allows to receive observed address reports.
    fn receives_reports(&self) -> bool {
        matches!(self, Self::ReceiveReportsOnly | Self::Both)
    }

    /// Whether this peer should report observed addresses to other peers.
    pub(crate) fn should_report(&self, other: &Self) -> bool {
        self.is_reporter() && other.receives_reports()
    }

    /// Sets whether this peer should provide observed addresses to other peers.
    pub(crate) fn provide_reports_to_peers(&mut self, provide: bool) {
        if provide {
            self.enable_reports_to_peers()
        } else {
            self.disable_reports_to_peers()
        }
    }

    /// Enables reporting of observed addresses to other peers.
    fn enable_reports_to_peers(&mut self) {
        match self {
            Self::ProvideReportsOnly => {} // already enabled
            Self::ReceiveReportsOnly => *self = Self::Both,
            Self::Both => {} // already enabled
            Self::Disabled => *self = Self::ProvideReportsOnly,
        }
    }

    /// Disables reporting of observed addresses to other peers.
    fn disable_reports_to_peers(&mut self) {
        match self {
            Self::ProvideReportsOnly => *self = Self::Disabled,
            Self::ReceiveReportsOnly => {} // already disabled
            Self::Both => *self = Self::ReceiveReportsOnly,
            Self::Disabled => {} // already disabled
        }
    }

    /// Sets whether this peer should accept received reports of observed addresses from other
    /// peers.
    pub(crate) fn receive_reports_from_peers(&mut self, receive: bool) {
        if receive {
            self.enable_receiving_reports_from_peers()
        } else {
            self.disable_receiving_reports_from_peers()
        }
    }

    /// Enables accepting reports of observed addresses from other peers.
    fn enable_receiving_reports_from_peers(&mut self) {
        match self {
            Self::ProvideReportsOnly => *self = Self::Both,
            Self::ReceiveReportsOnly => {} // already enabled
            Self::Both => {}               // already enabled
            Self::Disabled => *self = Self::ReceiveReportsOnly,
        }
    }

    /// Disables accepting reports of observed addresses from other peers.
    fn disable_receiving_reports_from_peers(&mut self) {
        match self {
            Self::ProvideReportsOnly => {} // already disabled
            Self::ReceiveReportsOnly => *self = Self::Disabled,
            Self::Both => *self = Self::ProvideReportsOnly,
            Self::Disabled => {} // already disabled
        }
    }
}
