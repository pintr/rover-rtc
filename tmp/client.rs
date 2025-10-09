use anyhow::Result;
use std::io::{self, Read, Write};
use std::net::UdpSocket;
use std::time::{Duration, Instant};
use str0m::change::SdpApi;
use str0m::net::Receive;
use str0m::{Candidate, Event, Input, Rtc};
use tracing::info;

fn init_log() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
}

pub fn main() -> Result<()> {
    init_log();

    let mut rtc = Rtc::new();
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    let addr = socket.local_addr()?;
    info!("Bound UDP to: {}", addr);

    let cand = Candidate::host(addr, "udp")?;
    rtc.add_local_candidate(cand);

    let channel = rtc.create_data_channel("chat", Default::default());

    let (offer, pending) = rtc.create_offer(Default::default())?;

    println!("=== SDP OFFER BEGIN ===");
    println!("{}", offer.to_string());
    println!("=== SDP OFFER END ===\n");
    println!("Paste the SDP answer, then press Enter:");

    let mut ans_text = String::new();
    io::stdin().read_line(&mut ans_text)?;
    let answer = ans_text.trim().parse()?;
    rtc.accept_answer(pending, answer)?;

    println!("✅ Answer accepted, starting connection...");

    socket.set_nonblocking(true)?;
    let mut buf = vec![0; 2000];
    let mut last_poll = Instant::now();

    loop {
        if let Ok(timeout) = rtc.poll_timeout() {
            if let Some(timeout) = timeout {
                let duration = timeout.saturating_duration_since(Instant::now());
                if duration.is_zero() {
                    rtc.handle_timeout(Instant::now())?;
                }
            }
        }

        while let Ok((n, source)) = socket.recv_from(&mut buf) {
            let now = Instant::now();
            let receive = Receive {
                source,
                contents: buf[..n].into(),
                timestamp: now,
            };
            rtc.handle_input(Input::Receive(receive))?;
        }

        while let Some(output) = rtc.poll_output()? {
            match output {
                str0m::Output::Transmit(transmit) => {
                    socket.send_to(&transmit.contents, transmit.destination)?;
                }
                str0m::Output::Event(event) => match event {
                    Event::Connected => {
                        println!("Connected!");
                        channel.write(true, "Hello from client!".as_bytes())?;
                    }
                    Event::DataChannelOpen(id, label) => {
                        println!("Data channel '{}' ({}) opened", label, id);
                    }
                    Event::DataChannelData(data) => {
                        let s = String::from_utf8_lossy(&data.data);
                        println!("Received data: {}", s);
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        if last_poll.elapsed() > Duration::from_millis(100) {
            // For example, send a message every 100ms
            // channel.write(true, "ping".as_bytes())?;
            last_poll = Instant::now();
        }

        std::thread::sleep(Duration::from_millis(1));
    }
}
