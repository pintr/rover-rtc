use std::{
    net::IpAddr,
    sync::{mpsc, Arc},
};
use systemstat::{Platform, System};
use webrtc::{
    api::APIBuilder,
    ice_transport::{ice_gatherer_state::RTCIceGathererState, ice_server::RTCIceServer},
    peer_connection::configuration::RTCConfiguration,
    Error,
};

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

pub async fn create_data_channel_offer(label: &str) -> Result<String, Error> {
    // WebRTC setup
    let api = APIBuilder::new().build();
    let config = RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        }],
        ..Default::default()
    };
    let peer_connection = Arc::new(
        api.new_peer_connection(config)
            .await
            .expect("peer connection."),
    );

    // Data channel to be included in the offer
    let _data_channel = peer_connection
        .create_data_channel(label, None)
        .await
        .expect("data channel.");

    // Completed ICE gathering
    let (done_tx, done_rx) = mpsc::channel();
    peer_connection.on_ice_gathering_state_change(Box::new(move |state| {
        if state == RTCIceGathererState::Complete {
            done_tx.send(()).unwrap();
        }
        Box::pin(async {})
    }));

    // Offer creation
    let offer = peer_connection
        .create_offer(None)
        .await
        .expect("create session desription");
    peer_connection
        .set_local_description(offer)
        .await
        .expect("local description set.");
    let _ = done_rx.recv();

    let sdp_string = peer_connection
        .local_description()
        .await
        .map(|ld| ld.sdp)
        .ok_or_else(|| Error::new("No local description found".to_string()))
        .expect("SDP string.");

    Ok(sdp_string)
}
