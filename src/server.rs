use std::{
    collections::VecDeque,
    io::ErrorKind,
    net::{SocketAddr, UdpSocket},
    sync::mpsc::{self, Receiver, SyncSender, TryRecvError},
    thread,
    time::Instant,
};

use rouille::{Request, Response, Server};
use str0m::{
    net::{Protocol, Receive},
    Input, Rtc,
};
use tracing::info;

use crate::util::select_host_address;

use crate::model::{client::Client, propagated::Propagated};

fn init_log() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
}

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

fn run(socket: UdpSocket, rx: Receiver<Rtc>) {}

fn web_request(request: &Request, addr: SocketAddr, tx: SyncSender<Rtc>) -> Response {
    // request.
    info!("{:#?}", request);

    let mut rtc: Rtc = Rtc::builder().build();

    return Response::empty_204();
}

/// Receive new clients from the receiver and create new Client instances.
fn spawn_new_client(rx: &Receiver<Rtc>) -> Option<Client> {
    // try_recv here won't lock up the thread.
    match rx.try_recv() {
        Ok(rtc) => Some(Client::new(rtc)),
        Err(TryRecvError::Empty) => None,
        _ => panic!("Receiver<Rtc> disconnected"),
    }
}

/// Poll all the output from the client until it returns a timeout.
/// Collect any output in the queue, transmit data on the socket, return the timeout
fn poll_until_timeout(
    client: &mut Client,
    queue: &mut VecDeque<Propagated>,
    socket: &UdpSocket,
) -> Instant {
    loop {
        if !client.rtc.is_alive() {
            // This client will be cleaned up in the next run of the main loop.
            return Instant::now();
        }

        let propagated = client.poll_output(socket);

        if let Propagated::Timeout(t) = propagated {
            return t;
        }

        queue.push_back(propagated)
    }
}

/// Sends one "propagated" to all clients, if relevant
fn propagate(propagated: &Propagated, clients: &mut [Client]) {
    // Do not propagate to originating client.
    let Some(client_id) = propagated.client_id() else {
        // If the event doesn't have a client id, it can't be propagated,
        // (it's either a noop or a timeout).
        return;
    };

    for client in &mut *clients {
        if client.id == client_id {
            // Do not propagate to originating client.
            continue;
        }

        match &propagated {
            Propagated::TrackOpen(_, track_in) => client.handle_track_open(track_in.clone()),
            Propagated::MediaData(_, data) => client.handle_media_data_out(client_id, data),
            Propagated::KeyframeRequest(_, req, origin, mid_in) => {
                // Only one origin client handles the keyframe request.
                if *origin == client.id {
                    client.handle_keyframe_request(*req, *mid_in)
                }
            }
            Propagated::Noop | Propagated::Timeout(_) => {}
        }
    }
}

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
