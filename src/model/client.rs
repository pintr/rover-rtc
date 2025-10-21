use std::net::UdpSocket;
use std::ops::Deref;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use str0m::channel::ChannelId;
use str0m::{Event, IceConnectionState, Input, Output, Rtc};
use tracing::{info, warn};

#[derive(Debug)]
pub struct Client {
    pub id: ClientId,
    pub rtc: Rtc,
    cid: Option<ChannelId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientId(u64);

impl Deref for ClientId {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Client {
    pub fn new(rtc: Rtc) -> Client {
        static ID_COUNTER: AtomicU64 = AtomicU64::new(0);
        let next_id = ID_COUNTER.fetch_add(1, Ordering::SeqCst);
        Client {
            id: ClientId(next_id),
            rtc,
            cid: None,
        }
    }

    pub fn accepts(&self, input: &Input) -> bool {
        self.rtc.accepts(input)
    }

    pub fn handle_input(&mut self, input: Input) {
        if !self.rtc.is_alive() {
            return;
        }

        if let Err(e) = self.rtc.handle_input(input) {
            warn!("Client ({}) disconnected: {:?}", *self.id, e);
            self.rtc.disconnect();
        }
    }

    pub fn poll_output(&mut self, socket: &UdpSocket) -> Option<Instant> {
        if !self.rtc.is_alive() {
            return Some(Instant::now());
        }

        match self.rtc.poll_output() {
            Ok(output) => self.handle_output(output, socket),
            Err(e) => {
                warn!("Client ({}) poll_output failed: {:?}", *self.id, e);
                self.rtc.disconnect();
                Some(Instant::now())
            }
        }
    }

    fn handle_output(&mut self, output: Output, socket: &UdpSocket) -> Option<Instant> {
        match output {
            Output::Transmit(transmit) => {
                socket
                    .send_to(&transmit.contents, transmit.destination)
                    .expect("sending UDP data");
                None
            }
            Output::Timeout(t) => Some(t),
            Output::Event(e) => {
                // Log important events
                match &e {
                    Event::IceConnectionStateChange(state) => {
                        info!("🔌 Server Client({}): ICE State = {:?}", *self.id, state);
                    }
                    Event::ChannelOpen(_, _) | Event::ChannelData(_) => {
                        // These are logged in detail below
                    }
                    _ => {
                        info!("Server Client({}): Event: {:?}", *self.id, e);
                    }
                }

                // Handle the event
                match e {
                    Event::IceConnectionStateChange(v) => {
                        if v == IceConnectionState::Disconnected {
                            self.rtc.disconnect();
                        }
                    }
                    Event::ChannelOpen(cid, name) => {
                        info!(
                            "🎉 Server: Data channel opened for Client({}) - Name: '{}', ID: {:?}",
                            *self.id, name, cid
                        );
                        self.cid = Some(cid);
                    }
                    Event::ChannelData(data) => {
                        info!(
                            "📥 Server: Client({}) received data on channel {:?}: {}",
                            *self.id,
                            data.id,
                            String::from_utf8_lossy(&data.data)
                        );
                    }
                    _ => {}
                }
                None
            }
        }
    }

    pub fn send_message(&mut self, message: &str) {
        if let Some(cid) = self.cid {
            if let Some(mut channel) = self.rtc.channel(cid) {
                match channel.write(false, message.as_bytes()) {
                    Ok(_) => {
                        info!("📤 Server: Sent to Client({}): {}", *self.id, message);
                    }
                    Err(e) => {
                        warn!("Server: Failed to send to Client({}): {:?}", *self.id, e);
                    }
                }
            }
        }
    }
}
