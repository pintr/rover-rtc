use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Instant;
use str0m::{
    channel::{ChannelConfig, ChannelId},
    ice::IceConnectionState,
    net::{Protocol, UdpSocket},
    Event, Rtc,
};
use tokio::net::UdpSocket as TokioUdpSocket;
use tracing::{error, info, warn, Level};

const SIGNALING_SERVER_URL: &str = "ws://127.0.0.1:8080";

// --- 1. Define Signaling Message Structure ---
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum SignalMessage {
    Sdp { sdp: String },
    Ice { candidate: String },
}

// --- 2. Create a UdpSocket Wrapper for str0m ---
// This adapter makes tokio's UdpSocket compatible with str0m's trait.
struct TokioUdpSocketAdapter(TokioUdpSocket);

impl UdpSocket for TokioUdpSocketAdapter {
    fn send_to(&self, buf: &[u8], target: SocketAddr) -> std::io::Result<usize> {
        self.0.try_send_to(buf, target)
    }

    fn recv_from(&self, buf: &mut [u8]) -> std::io::Result<(usize, SocketAddr)> {
        self.0.try_recv_from(buf)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let is_offerer = std::env::args().any(|arg| arg == "offerer");
    let peer_id = if is_offerer { "offerer" } else { "answerer" };

    // --- 3. Connect to Signaling Server ---
    let (ws_stream, _) = tokio_tungstenite::connect_async(SIGNALING_SERVER_URL).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    info!("Connected to signaling server");

    // Register with the server by sending our ID.
    ws_sender.send(Message::Text(peer_id.to_string())).await?;

    // --- 4. Initialize WebRTC Stack ---
    let socket = TokioUdpSocket::bind("0.0.0.0:0").await?;
    info!("UDP socket bound to: {}", socket.local_addr()?);
    let socket_adapter = TokioUdpSocketAdapter(socket);

    let mut rtc = Rtc::new();
    rtc.add_ice_server("stun:stun.l.google.com:19302")?;

    let mut main_channel: Option<ChannelId> = None;
    if is_offerer {
        info!("Running in OFFERER mode");
        let config = ChannelConfig {
            label: "rover-data".to_string(),
            ordered: true,
            reliability: Default::default(),
        };
        main_channel = Some(rtc.create_channel(config)?);
        let offer = rtc.create_offer()?;
        let msg = SignalMessage::Sdp { sdp: offer.to_sdp() };
        ws_sender.send(Message::Text(serde_json::to_string(&msg)?)).await?;
        info!("Sent offer");
    } else {
        info!("Running in ANSWERER mode, waiting for offer...");
    }

    // --- 5. Main Event Loop ---
    // 
    // This loop drives the entire application by reacting to network I/O,
    // signaling messages, and internal timers from the Rtc instance.
    let mut buf = vec![0; 2000];
    loop {
        let timeout = rtc.poll_timeout().map(|t| t.saturating_duration_since(Instant::now()));

        tokio::select! {
            // A. Handle incoming UDP packets from the other peer.
            Ok((n, addr)) = socket_adapter.0.recv_from(&mut buf) => {
                if let Err(e) = rtc.handle_input(&buf[..n], addr, Protocol::Udp) {
                    warn!("Error handling UDP input: {}", e);
                }
            },

            // B. Handle incoming messages from the WebSocket signaling server.
            Some(Ok(msg)) = ws_receiver.next() => {
                if let Ok(text) = msg.to_text() {
                    match serde_json::from_str::<SignalMessage>(text) {
                        Ok(SignalMessage::Sdp{sdp}) => {
                             if is_offerer {
                                info!("Received answer");
                                rtc.handle_remote_sdp(&sdp)?;
                            } else {
                                info!("Received offer");
                                rtc.handle_remote_sdp(&sdp)?;
                                let answer = rtc.create_answer()?;
                                let msg = SignalMessage::Sdp { sdp: answer.to_sdp() };
                                ws_sender.send(Message::Text(serde_json::to_string(&msg)?)).await?;
                                info!("Sent answer");
                            }
                        },
                        Ok(SignalMessage::Ice{candidate}) => {
                            info!("Received ICE candidate");
                            rtc.handle_remote_candidate(&candidate)?;
                        },
                        Err(e) => warn!("Failed to parse signaling message: {}", e)
                    }
                }
            },

            // C. Handle the str0m timeout.
            _ = async {
                if let Some(d) = timeout { tokio::time::sleep(d).await; }
                else { std::future::pending::<()>().await; }
            } => {
                rtc.handle_timeout(Instant::now());
            }
        }

        // --- 6. Process RTC Outputs and Events ---
        loop {
            // D. Poll for outgoing network packets to send.
            match rtc.poll_output() {
                Ok(output) => { let _ = socket_adapter.send_to(output.contents, output.target); },
                Err(e) if e == str0m::Error::NoPackets => break,
                Err(e) => { error!("Poll output error: {}", e); break; }
            }
        }

        loop {
            // E. Poll for events from the Rtc state machine.
            match rtc.poll_event() {
                Some(event) => {
                    match event {
                        Event::IceCandidate(c) => {
                            info!("Discovered new ICE candidate");
                            let msg = SignalMessage::Ice { candidate: c.to_sdp() };
                            ws_sender.send(Message::Text(serde_json::to_string(&msg)?)).await?;
                        }
                        Event::ChannelOpen(id, label) => {
                            info!("Data channel '{}' ({:?}) is open!", label, id);
                            if let Some(main_id) = main_channel {
                                if main_id == id {
                                    let channel = rtc.channel(main_id).unwrap();
                                    let message = format!("Hello from Offerer Rover! ({})", chrono::Local::now());
                                    channel.write(true, message.as_bytes())?;
                                }
                            }
                        }
                        Event::ChannelData(d) => {
                            let data_str = String::from_utf8_lossy(&d.data);
                            info!("Received message: {}", data_str);
                            if !is_offerer {
                                let channel = rtc.channel(d.id).unwrap();
                                let response = format!("Echo from Answerer: {}", data_str);
                                channel.write(true, response.as_bytes())?;
                            }
                        }
                        Event::IceConnectionStateChange(s) => {
                            info!("ICE connection state changed: {:?}", s);
                            if s == IceConnectionState::Connected {
                                info!("Connection established! The connection monitor would start now.");
                            }
                        }
                        _ => {}
                    }
                },
                None => break,
            }
        }
    }
}

