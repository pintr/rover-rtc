//! Utility functions for network configuration
//!
//! This module provides helper functions for discovering network interfaces,
//! selecting appropriate IP addresses, and generating ICE candidates for WebRTC.

use local_ip_address::list_afinet_netifas;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use str0m::Candidate;
use systemstat::{Platform, System};
use tracing::info;

/// Selects an appropriate IPv4 address for WebRTC communication.
///
/// Iterates over all network interfaces provided by `systemstat`, skipping any
/// loopback, link-local, broadcast addresses, Docker networks, and bridge networks.
/// Only returns interfaces that have internet access. The first routable interface
/// with internet connectivity is returned as an [`IpAddr`].
///
/// # Returns
///
/// * `IpAddr` - The first routable IPv4 network interface with internet access
///
/// # Panics
///
/// Panics if the host exposes no usable IPv4 address. This is acceptable for
/// the prototype CLI binaries, but production callers should consider wrapping
/// the logic in a fallible API and handling the error gracefully.
pub fn select_host_address() -> IpAddr {
    let system = System::new();
    let networks = system.networks().expect("Networks should be available.");

    info!("Networks {:#?}", networks);

    for (name, net) in networks {
        // Skip Docker and bridge interfaces by name
        let name_lower = name.to_lowercase();
        if name_lower.contains("docker")
            || name_lower.starts_with("br-")
            || name_lower.starts_with("veth")
            || name_lower.starts_with("virbr")
        {
            info!("Skipping interface {} (Docker/bridge)", name);
            continue;
        }

        for n in &net.addrs {
            if let systemstat::IpAddr::V4(v) = n.addr {
                if !v.is_loopback() && !v.is_link_local() && !v.is_broadcast() {
                    let ip_addr = IpAddr::V4(v);

                    // Verify internet connectivity by trying to bind and connect
                    if has_internet_access(&ip_addr) {
                        info!("Selected interface {} with IP {}", name, ip_addr);
                        return ip_addr;
                    } else {
                        info!("Interface {} has no internet access, skipping", name);
                    }
                }
            }
        }
    }

    panic!("Found no usable network interface with internet access");
}

/// Checks if a given IP address has internet access.
///
/// Attempts to create a UDP socket bound to the given IP and connect to
/// Google's public DNS server (8.8.8.8:53) to verify internet connectivity.
///
/// # Arguments
///
/// * `ip` - The IP address to check for internet access
///
/// # Returns
///
/// * `bool` - `true` if the interface has internet access, `false` otherwise
fn has_internet_access(ip: &IpAddr) -> bool {
    // Try to bind to the specific IP and connect to a public DNS server
    let bind_addr = SocketAddr::new(*ip, 0);

    match UdpSocket::bind(bind_addr) {
        Ok(socket) => {
            // Try to connect to Google's public DNS (8.8.8.8:53)
            // This doesn't send data, just verifies routing is possible
            socket.connect("8.8.8.8:53").is_ok()
        }
        Err(_) => false,
    }
}

/// Generates a list of ICE candidates from available network interfaces.
///
/// Discovers all network interfaces on the system and creates host ICE candidates
/// for each routable IPv4 address. Skips loopback and link-local addresses.
/// IPv6 addresses are currently not supported.
///
/// # Arguments
///
/// * `socket` - The UDP socket whose port will be used for the candidates
///
/// # Returns
///
/// A vector of [`Candidate`] objects representing the available network interfaces
///
/// # Note
///
/// The function logs all discovered interfaces for debugging purposes.
pub fn get_candidates(socket: &UdpSocket) -> Vec<Candidate> {
    let mut candidates: Vec<Candidate> = vec![];
    if let Ok(network_interfaces) = list_afinet_netifas() {
        for (name, ip) in network_interfaces {
            info!("iface: {} / {:?}", name, ip);
            match ip {
                IpAddr::V4(ip4) => {
                    if !ip4.is_loopback() && !ip4.is_link_local() {
                        let socket_addr = SocketAddr::new(
                            ip,
                            socket
                                .local_addr()
                                .expect("Local address should be available.")
                                .port(),
                        );
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

/// Initializes the tracing subscriber with environment-based filtering.
///
/// Defaults to INFO level logging, but can be overridden via the `RUST_LOG`
/// environment variable. Enables debug logging for HTTP and str0m.
pub fn init_log() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,http_post=debug,str0m=debug"));

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(env_filter)
        .init();
}
