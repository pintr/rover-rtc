use std::{
    net::{SocketAddr, UdpSocket},
    thread,
    time::{Duration, Instant},
};

use anyhow::Result;
use str0m::{
    change::SdpAnswer,
    media::{Direction, MediaKind},
    net::{Protocol, Receive},
    Candidate, Event, Input, Rtc,
};
use tracing::{info, warn};

pub fn main(server_addr: SocketAddr) -> Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    let local_addr = socket.local_addr()?;
    info!("Bound UDP to {}", local_addr);

    let mut rtc = Rtc::builder().build();

    let candidate = Candidate::host(local_addr, "udp")?;
    rtc.add_local_candidate(candidate);

    let offer = rtc.sdp_api().create_offer(Default::default())?;

    let client = reqwest::blocking::Client::new();
    let answer: SdpAnswer = client
        .post(format!("http://{}:3000", server_addr.ip()))
        .body(serde_json::to_string(&offer)?)
        .send()?
        .json()?;

    rtc.sdp_api().accept_answer(answer)?;

    let mut buf = vec![0; 2000];

    let mut video = rtc.media(MediaKind::Video).unwrap();
    video.set_direction(Direction::SendRecv);
    let mid = video.mid();
    info!("Video mid: {:?}", mid);

    loop {
        let timeout = match rtc.poll_output()? {
            str0m::Output::Timeout(t) => t,
            str0m::Output::Transmit(t) => {
                socket.send_to(&t.contents, t.destination)?;
                Instant::now()
            }
            str0m::Output::Event(e) => {
                match e {
                    Event::Connected => {
                        info!("Connected!");
                    }
                    Event::MediaData(d) => {
                        info!("Media data ({}) from {}: {}", d.mid, d.pt, d.len());
                    }
                    Event::RtpPacket(rtp) => {
                        info!("RTP packet received: {:?}", rtp);
                    }
                    _ => {
                        info!("Event: {:?}", e);
                    }
                }
                Instant::now()
            }
        };

        let duration = timeout.saturating_duration_since(Instant::now());
        socket.set_read_timeout(Some(duration))?;

        match socket.recv_from(&mut buf) {
            Ok((n, source)) => {
                let input = Input::Receive(
                    Instant::now(),
                    Receive {
                        proto: Protocol::Udp,
                        source,
                        destination: local_addr,
                        contents: buf[..n].try_into()?,
                    },
                );
                rtc.handle_input(input)?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // timeout
            }
            Err(e) => {
                warn!("Socket read error: {}", e);
            }
        }

        rtc.handle_input(Input::Timeout(Instant::now()))?;
    }
}
