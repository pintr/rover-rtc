use std::{
    net::{SocketAddr, UdpSocket},
    sync::mpsc::{self, Receiver, SyncSender},
    thread,
};

use rouille::{Request, Response, Server};
use str0m::Rtc;
use tracing::info;

use crate::util::select_host_address;

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
    info!("Connect a browser to https://{:?}:{:?}", addr.ip(), port);

    server.run();
}

fn run(socket: UdpSocket, rx: Receiver<Rtc>) {}

fn web_request(request: &Request, addr: SocketAddr, tx: SyncSender<Rtc>) -> Response {
    // request.
    info!("{:#?}", request);
    return Response::empty_204();
}
