//! WebRTC signaling server module
//!
//! This module implements a simple HTTP-based signaling server for WebRTC connections.
//! It handles SDP offer/answer exchange and manages multiple WebRTC clients, relaying
//! UDP packets between them and broadcasting periodic messages.

use std::{
    io::ErrorKind,
    net::{SocketAddr, UdpSocket},
    sync::mpsc::{self, Receiver, SyncSender, TryRecvError},
    thread,
    time::{Duration, Instant},
};

use rouille::{Request, Response, Server};
use str0m::{
    change::SdpOffer,
    net::{Protocol, Receive},
    Candidate, Input, Rtc,
};
use tracing::{debug, info};

use crate::util::{init_log, select_host_address};

use crate::model::client::Client;

/// Main entry point for the WebRTC signaling server.
///
/// This function:
/// 1. Initializes logging
/// 2. Selects a host address for the UDP socket
/// 3. Binds a random UDP port for WebRTC traffic
/// 4. Spawns a background thread to handle WebRTC client connections
/// 5. Starts an HTTP server on port 3000 for signaling
///
/// # Panics
///
/// Panics if:
/// - Unable to bind a UDP socket
/// - Unable to start the HTTP server
pub fn main() {
    init_log();

    let host_addr = select_host_address();

    let (tx, rx) = mpsc::sync_channel(1);

    let socket = UdpSocket::bind(format!("{host_addr}:0")).expect("binding a random UDP port");
    let addr = socket.local_addr().expect("a local socket address");
    info!("Bound UDP port: {}", addr);

    thread::spawn(move || run(socket, rx));

    let server = Server::new("0.0.0.0:3000", move |request| {
        web_request(request, addr, tx.clone())
    })
    .expect("starting the web server");

    let port = server.server_addr().port();
    info!("Connect a browser to http://{:?}:{:?}", addr.ip(), port);

    server.run();
}

/// Main event loop for managing WebRTC clients.
///
/// This function:
/// - Maintains a list of active clients
/// - Polls each client for output and handles timeouts
/// - Routes incoming UDP packets to the appropriate client
/// - Broadcasts messages to all clients every 5 seconds
/// - Removes disconnected clients
///
/// # Arguments
///
/// * `socket` - The UDP socket for receiving/sending WebRTC traffic
/// * `rx` - Channel receiver for new RTC instances from the web server thread
fn run(socket: UdpSocket, rx: Receiver<Rtc>) {
    let mut clients: Vec<Client> = vec![];
    let mut buf = vec![0; 2000];

    loop {
        // Removedisconnected clients
        clients.retain(|c| c.rtc.is_alive());

        // Spawn new clients from the web server thread
        if let Some(client) = spawn_new_client(&rx) {
            info!("New client connected: {:#?}", client);
            clients.push(client);
        }

        // Poll all clients and get the earliest timeout
        let mut timeout = Instant::now() + Duration::from_millis(100);
        for client in clients.iter_mut() {
            let t = poll_client(client, &socket);
            timeout = timeout.min(t);
        }

        if let Some(input) = read_socket_input(&socket, &mut buf) {
            // The rtc.accepts() call is how we demultiplex the incoming packet to know which
            // Rtc instance the traffic belongs to.
            if let Some(client) = clients.iter_mut().find(|c| c.accepts(&input)) {
                // We found the client that accepts the input.
                client.handle_input(input);
            } else {
                // This is quite common because we don't get the Rtc instance via the mpsc channel
                // quickly enough before the browser send the first STUN.
                debug!("No client accepts UDP input: {:?}", input);
            }
        }

        // Drive time forward in all clients.
        let now = Instant::now();
        for client in &mut clients {
            client.handle_input(Input::Timeout(now));
        }
    }
}

/// Handles incoming HTTP requests for WebRTC signaling.
///
/// This function processes SDP offers from clients, creates an SDP answer,
/// and sends the new RTC instance to the main event loop via the channel.
///
/// # Arguments
///
/// * `request` - The incoming HTTP request containing the SDP offer
/// * `addr` - The socket address of the UDP port for WebRTC traffic
/// * `tx` - Channel sender for passing new RTC instances to the main loop
///
/// # Returns
///
/// An HTTP response containing the SDP answer in JSON format
fn web_request(request: &Request, addr: SocketAddr, tx: SyncSender<Rtc>) -> Response {
    // request.
    info!("{:#?}", request);

    let mut data = request.data().expect("body to be available");

    let offer: SdpOffer = serde_json::from_reader(&mut data).expect("serialised offer");
    info!(
        "Received offer with {} data channels",
        offer.to_string().matches("m=application").count()
    );
    let mut rtc: Rtc = Rtc::builder().build();

    let candidate = Candidate::host(addr, "udp").expect("a host candidate");
    rtc.add_local_candidate(candidate).unwrap();

    let answer = rtc
        .sdp_api()
        .accept_offer(offer)
        .expect("Offer to be accepted.");

    info!("Created answer, sending to client thread");

    tx.send(rtc).expect("to send the rtc instance.");

    let body = serde_json::to_vec(&answer).expect("answer to serialise.");

    info!("Send answer");
    Response::from_data("application/json", body)
}

/// Attempts to receive new clients from the channel and create Client instances.
///
/// Uses `try_recv` to avoid blocking the main thread.
///
/// # Arguments
///
/// * `rx` - The receiver channel for new RTC instances
///
/// # Returns
///
/// * `Some(Client)` - A new client instance if one was received
/// * `None` - If no client is available in the channel
///
/// # Panics
///
/// Panics if the receiver channel is disconnected
fn spawn_new_client(rx: &Receiver<Rtc>) -> Option<Client> {
    // try_recv here won't lock up the thread.
    match rx.try_recv() {
        Ok(rtc) => Some(Client::new(rtc)),
        Err(TryRecvError::Empty) => None,
        _ => panic!("Receiver<Rtc> disconnected"),
    }
}

/// Polls a client for output events and handles them until a timeout is returned.
///
/// This function processes all available output from the client (transmit events)
/// and returns when the next timeout should occur.
///
/// # Arguments
///
/// * `client` - The client to poll
/// * `socket` - The UDP socket for sending outgoing traffic
///
/// # Returns
///
/// The instant at which the next timeout should occur
fn poll_client(client: &mut Client, socket: &UdpSocket) -> Instant {
    loop {
        if !client.rtc.is_alive() {
            // This client will be cleaned up in the next run of the main loop.
            return Instant::now();
        }

        match client.poll_output(socket) {
            Some(timeout) => return timeout,
            None => continue,
        }
    }
}

/// Attempts to read incoming data from the UDP socket.
///
/// Handles socket read timeouts gracefully and converts received data into
/// str0m `Input` events for processing by RTC instances.
///
/// # Arguments
///
/// * `socket` - The UDP socket to read from
/// * `buf` - A buffer for storing received data
///
/// # Returns
///
/// * `Some(Input)` - An input event containing the received data and source address
/// * `None` - If the read timed out or the socket would block
///
/// # Panics
///
/// Panics on unexpected socket errors (other than timeout/would block)
fn read_socket_input<'a>(socket: &UdpSocket, buf: &'a mut Vec<u8>) -> Option<Input<'a>> {
    buf.resize(2000, 0);

    match socket.recv_from(buf) {
        Ok((n, source)) => {
            buf.truncate(n);

            // Parse data to a DatagramRecv, which help preparse network data to
            // figure out the multiplexing of all protocols on one UDP port.
            let Ok(contents) = buf.as_slice().try_into() else {
                return None;
            };

            Some(Input::Receive(
                Instant::now(),
                Receive {
                    proto: Protocol::Udp,
                    source,
                    destination: socket.local_addr().unwrap(),
                    contents,
                },
            ))
        }

        Err(e) => match e.kind() {
            // Expected error for set_read_timeout(). One for windows, one for the rest.
            ErrorKind::WouldBlock | ErrorKind::TimedOut => None,
            _ => panic!("UdpSocket read failed: {e:?}"),
        },
    }
}
