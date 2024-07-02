//! Address discovery types from
//! <https://datatracker.ietf.org/doc/draft-seemann-quic-address-discovery/>

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
    Oservee,
    /// Will both report and receive reports of observed addresses.
    Both,
}
