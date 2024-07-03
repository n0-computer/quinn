//! Address discovery types from
//! <https://datatracker.ietf.org/doc/draft-seemann-quic-address-discovery/>

use crate::VarInt;

pub(crate) const TRANSPORT_PARAMETER_CODE: u64 = 0x9f81a174;

/// The role of each participant.
///
/// When enabled, this is reported as a transport parameter.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub(crate) enum Role {
    /// Is able to report observer addresses to other peers, but it's not interested in receiving
    /// reports about its own address.
    Observer,
    /// Is interested on reports about its own observed address, but will not report back to other
    /// peers.
    Observee,
    /// Will both report and receive reports of observed addresses.
    Both,
}

impl From<Role> for VarInt {
    fn from(role: Role) -> Self {
        match role {
            Role::Observer => VarInt(0),
            Role::Observee => VarInt(1),
            Role::Both => VarInt(2),
        }
    }
}

impl TryFrom<VarInt> for Role {
    type Error = crate::transport_parameters::Error;

    fn try_from(value: VarInt) -> Result<Self, Self::Error> {
        match value.0 {
            0 => Ok(Role::Observer),
            1 => Ok(Role::Observee),
            2 => Ok(Role::Both),
            _ => Err(crate::transport_parameters::Error::IllegalValue),
        }
    }
}
