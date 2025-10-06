use std::net::IpAddr;
use systemstat::{Platform, System};

/// Pick an IPv4 address that can be shared with a remote ICE peer.
///
/// Iterates over all network interfaces provided by `systemstat`, skipping any
/// loopback, link-local or broadcast addresses. The first routable interface is
/// returned as an [`IpAddr`].
///
/// ## Returns
///
/// * `IpAddr`: The first routable network interface.
///
/// ## Panics
///
/// Panics if the host exposes no usable IPv4 address. This is acceptable for
/// the prototype CLI binaries, but production callers should consider wrapping
/// the logic in a fallible API and handling the error gracefully.
pub fn select_host_address() -> IpAddr {
    let system = System::new();
    let networks = system.networks().unwrap();

    for net in networks.values() {
        for n in &net.addrs {
            if let systemstat::IpAddr::V4(v) = n.addr {
                if !v.is_loopback() && !v.is_link_local() && !v.is_broadcast() {
                    return IpAddr::V4(v);
                }
            }
        }
    }

    panic!("Found no usable network interface");
}
