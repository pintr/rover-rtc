//! WebRTC peer client module
//!
//! This module implements a WebRTC peer client that establishes a direct P2P
//! connection with another peer via a signaling server. It creates a data channel
//! for bidirectional communication and handles the complete ICE negotiation process.

use std::{
    error::Error,
    io::ErrorKind,
    net::{SocketAddrV4, UdpSocket},
    time::{Duration, Instant},
};

use str0m::{
    change::SdpAnswer,
    net::{Protocol, Receive},
    Event, IceConnectionState, Input, Output, Rtc,
};

use tracing::info;

use crate::{
    model::payload::Payload,
    util::{get_candidates, init_log},
};

/// Errors that can occur during WebRTC peer operations.
#[derive(Debug)]
pub enum WebrtcError {
    /// Error communicating with the signaling server
    ServerError(Box<dyn Error + Send + Sync>),
    /// Error related to SDP negotiation
    SdpError,
    /// General WebRTC error
    WebrtcError(Box<dyn Error + Send + Sync>),
    /// Network communication error
    NetworkError(Box<dyn Error + Send + Sync>),
    /// Error sending data on a channel
    SendError(String),
    /// No ICE candidates were found
    NoCandidates,
}

/// Main entry point for the WebRTC peer client.
///
/// This async function performs the complete WebRTC connection sequence:
/// 1. Creates a new RTC instance and binds a UDP socket
/// 2. Discovers and adds local ICE candidates
/// 3. Creates a data channel and generates an SDP offer
/// 4. Sends the offer to the signaling server and receives an answer
/// 5. Accepts the answer and starts the connection process
/// 6. Enters the main event loop to handle ICE state changes, channel events, and data
/// 7. Processes incoming/outgoing UDP packets and drives the WebRTC state machine
///
/// # Returns
///
/// * `Ok(())` - If the peer completes successfully or disconnects gracefully
/// * `Err(Box<dyn Error>)` - If any error occurs during the connection process
///
/// # Example Data Channel
///
/// The peer creates a data channel named "test" which can be used to send and receive
/// arbitrary binary data once the connection is established.
#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting modern str0m peer...");
    init_log();

    const CHANNEL: &str = "test";

    let mut rtc = Rtc::new();

    let socket = UdpSocket::bind("0.0.0.0:0".parse::<SocketAddrV4>().unwrap())
        .expect("Should bind udp socket");
    let candidates = get_candidates(&socket);

    // Store the first candidate's address to use as destination in receives
    // All candidates share the same port, so we can use any of them
    let local_addr = candidates
        .first()
        .map(|c| c.addr())
        .expect("At least one candidate should be available");

    for candidate in candidates {
        rtc.add_local_candidate(candidate);
    }

    let mut change = rtc.sdp_api();
    let cid = change.add_channel(CHANNEL.to_string());

    let (offer, pending) = change.apply().ok_or("Failed to apply sdp change")?;

    info!(" Offer SDP:\n{}", offer);

    // // 1. DECLARE INTENT: Request a new data channel.
    // // This registers your desire for a channel; it doesn't create it yet.

    info!(
        "Peer: Requested data channel '{}' with ID: {:?}",
        CHANNEL, cid
    );

    // // 2. DRIVE THE STATE MACHINE: The `poll_output` loop.
    // // This replaces the direct call to `create_offer`.

    let mut buf = vec![0; 2000];
    let client = reqwest::Client::new();
    let answer: SdpAnswer = client
        .post("http://172.17.0.1:3000")
        .body(serde_json::to_string(&offer)?)
        .send()
        .await?
        .json()
        .await?;

    info!("Answer SDP:\n{}", answer);

    rtc.sdp_api().accept_answer(pending, answer)?;

    info!("Peer: Answer accepted, waiting for ICE connection and channel to open...");

    let mut channel_opened = false;
    let mut last_message_time = Instant::now();

    loop {
        let timeout = match rtc.poll_output().unwrap() {
            Output::Timeout(instant) => {
                // info!("{:?}", instant);
                instant
            }
            Output::Transmit(transmit) => {
                socket.send_to(&transmit.contents, transmit.destination)?;
                continue;
            }
            Output::Event(event) => {
                // Always log events, but filter out too verbose ones
                match &event {
                    Event::IceConnectionStateChange(_)
                    | Event::ChannelOpen(_, _)
                    | Event::ChannelData(_) => {
                        info!("Event: {:?}", event);
                    }
                    _ => {
                        // Still log other events at debug level
                        info!("Event (other): {:?}", event);
                    }
                }

                // Track ICE connection state changes
                if let Event::IceConnectionStateChange(state) = &event {
                    info!("ICE Connection State: {:?}", state);
                    match state {
                        IceConnectionState::New => info!("ICE is starting..."),
                        IceConnectionState::Checking => info!("ICE is checking candidates..."),
                        IceConnectionState::Connected => {
                            info!("ICE Connected! Data channel should open soon.")
                        }
                        IceConnectionState::Completed => info!("ICE Completed!"),
                        IceConnectionState::Disconnected => info!("ICE Disconnected"),
                    }
                }

                // Handle channel opening
                if let Event::ChannelOpen(channel_id, name) = &event {
                    info!(
                        "Peer: Channel opened - Name: '{}', ID: {:?}, Expected ID: {:?}",
                        name, channel_id, cid
                    );
                    if channel_id == &cid {
                        info!("   Channel ID matches expected ID!");
                        channel_opened = true;
                    } else {
                        info!("WARNING: Channel ID does NOT match expected ID!");
                    }
                }

                // Handle incoming data
                if let Event::ChannelData(msg) = &event {
                    info!(
                        "Received data on channel {:?}: {:?}",
                        msg.id,
                        String::from_utf8_lossy(&msg.data)
                    );
                }

                // Abort if we disconnect
                if event == Event::IceConnectionStateChange(IceConnectionState::Disconnected) {
                    info!("Disconnecting due to ICE state change");
                    break;
                }

                continue;
            }
        };

        // Send periodic timestamps to server if channel is open
        if channel_opened && last_message_time.elapsed() > Duration::from_secs(2) {
            if let Some(mut channel) = rtc.channel(cid) {
                let payload: Payload = Payload::new("ciao".as_bytes());
                info!(
                    "Sending message {}\n Timestamp: {}",
                    payload.data(),
                    payload.timestamp()
                );
                match channel.write(false, &Payload::serialize(payload)) {
                    Ok(_) => {
                        info!("Message sent");
                        last_message_time = Instant::now();
                        // Continue immediately to poll_output and flush the written data
                        continue;
                    }
                    Err(e) => {
                        info!("Peer: Failed to send message: {:?}", e);
                    }
                }
            }
        }

        // Duration until timeout.
        // Cap the duration at 100ms to ensure we process incoming packets frequently
        let duration = (timeout - Instant::now())
            .max(Duration::from_millis(1))
            .min(Duration::from_millis(100));

        // socket.set_read_timeout(Some(0)) is not ok
        if duration.is_zero() {
            // Drive time forwards in rtc straight away.
            rtc.handle_input(Input::Timeout(Instant::now())).unwrap();
            continue;
        }

        socket.set_read_timeout(Some(duration)).unwrap();

        // Scale up buffer to receive an entire UDP packet.
        buf.resize(2000, 0);

        // Try to receive. Because we have a timeout on the socket,
        // we will either receive a packet, or timeout.
        let input = match socket.recv_from(&mut buf) {
            Ok((n, source)) => {
                // UDP data received.
                buf.truncate(n);
                Input::Receive(
                    Instant::now(),
                    Receive {
                        proto: Protocol::Udp,
                        source,
                        destination: local_addr,
                        contents: buf.as_slice().try_into().unwrap(),
                    },
                )
            }

            Err(e) => match e.kind() {
                // Expected error for set_read_timeout().
                // One for windows, one for the rest.
                ErrorKind::WouldBlock | ErrorKind::TimedOut => Input::Timeout(Instant::now()),

                // Any other error is unexpected and should be propagated.
                // We can't handle it here, so we pass it up to the caller.
                _ => return Err(e.into()),
            },
        };

        // Input is either a Timeout or Receive of data. Both drive the state forward.
        rtc.handle_input(input).unwrap();
    }

    Ok(())
}
