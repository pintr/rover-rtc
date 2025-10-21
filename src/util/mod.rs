use local_ip_address::list_afinet_netifas;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use str0m::Candidate;
use systemstat::{Platform, System};
use tracing::info;

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

pub fn get_candidates(socket: &UdpSocket) -> Vec<Candidate> {
    let mut candidates: Vec<Candidate> = vec![];
    if let Ok(network_interfaces) = list_afinet_netifas() {
        for (name, ip) in network_interfaces {
            info!("iface: {} / {:?}", name, ip);
            match ip {
                IpAddr::V4(ip4) => {
                    if !ip4.is_loopback() && !ip4.is_link_local() {
                        let socket_addr = SocketAddr::new(ip, socket.local_addr().unwrap().port());
                        candidates.push(
                            Candidate::host(socket_addr, str0m::net::Protocol::Udp)
                                .expect("Failed to create local candidate"),
                        );
                    }
                }
                IpAddr::V6(_ip6) => {}
            }
        }
    }

    candidates
}
