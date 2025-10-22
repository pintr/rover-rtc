//! Client management for WebRTC connections
//!
//! This module provides the [`Client`] abstraction for managing individual WebRTC
//! peer connections on the server side. Each client represents a connected peer with
//! its own RTC instance, data channel, and connection state.

use std::net::UdpSocket;
use std::ops::Deref;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use str0m::channel::ChannelId;
use str0m::{Event, IceConnectionState, Input, Output, Rtc};
use tracing::{info, warn};

/// Represents a connected WebRTC client with its own RTC instance.
///
/// Each client has a unique ID and maintains its own WebRTC state, including
/// ICE connection status and data channel information.
#[derive(Debug)]
pub struct Client {
    /// Unique identifier for this client
    pub id: ClientId,
    /// The str0m RTC instance managing the WebRTC connection
    pub rtc: Rtc,
    /// The ID of the data channel, if one has been opened
    cid: Option<ChannelId>,
}

/// Unique identifier for a client connection.
///
/// Client IDs are assigned sequentially using an atomic counter to ensure
/// uniqueness across all clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientId(u64);

impl Deref for ClientId {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Client {
    /// Creates a new client with a unique ID and the given RTC instance.
    ///
    /// # Arguments
    ///
    /// * `rtc` - The str0m RTC instance for this client
    ///
    /// # Returns
    ///
    /// A new `Client` instance with a unique ID
    pub fn new(rtc: Rtc) -> Client {
        static ID_COUNTER: AtomicU64 = AtomicU64::new(0);
        let next_id = ID_COUNTER.fetch_add(1, Ordering::SeqCst);
        Client {
            id: ClientId(next_id),
            rtc,
            cid: None,
        }
    }

    /// Checks if this client accepts the given input.
    ///
    /// This is used for demultiplexing incoming UDP packets to determine which
    /// client they belong to based on the connection's 5-tuple and other state.
    ///
    /// # Arguments
    ///
    /// * `input` - The input event to check
    ///
    /// # Returns
    ///
    /// `true` if this client should handle the input, `false` otherwise
    pub fn accepts(&self, input: &Input) -> bool {
        self.rtc.accepts(input)
    }

    /// Handles an input event for this client.
    ///
    /// Passes the input to the RTC instance for processing. If the client is
    /// not alive or an error occurs, the client is marked for disconnection.
    ///
    /// # Arguments
    ///
    /// * `input` - The input event to handle (timeout or received data)
    pub fn handle_input(&mut self, input: Input) {
        if !self.rtc.is_alive() {
            return;
        }

        if let Err(e) = self.rtc.handle_input(input) {
            warn!("Client ({}) disconnected: {:?}", *self.id, e);
            self.rtc.disconnect();
        }
    }

    /// Polls the client for output events.
    ///
    /// This method drives the WebRTC state machine forward and processes any
    /// output events (transmit, timeout, or application events).
    ///
    /// # Arguments
    ///
    /// * `socket` - The UDP socket for sending outgoing packets
    ///
    /// # Returns
    ///
    /// * `Some(Instant)` - The next timeout instant if a timeout event was received
    /// * `None` - If a transmit or application event was handled
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

    /// Handles output events from the RTC instance.
    ///
    /// Processes three types of output:
    /// - `Transmit`: Sends UDP packets to the peer
    /// - `Timeout`: Returns the next timeout instant
    /// - `Event`: Handles WebRTC events (ICE state changes, channel open/data)
    ///
    /// # Arguments
    ///
    /// * `output` - The output event from the RTC instance
    /// * `socket` - The UDP socket for sending packets
    ///
    /// # Returns
    ///
    /// * `Some(Instant)` - The next timeout instant for timeout events
    /// * `None` - For transmit and application events
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

    /// Sends a message to the client over the data channel.
    ///
    /// If a data channel is open, this method writes the message as bytes.
    /// Logs success or failure of the send operation.
    ///
    /// # Arguments
    ///
    /// * `message` - The string message to send
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
