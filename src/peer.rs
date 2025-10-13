use std::time::{Duration, Instant};
use str0m::channel::ChannelConfig;
use str0m::format::PayloadParams;
use str0m::media::{Direction, MediaKind};
use str0m::{Rtc, RtcOutput};

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting modern str0m peer...");

    let mut rtc = Rtc::new();

    // 1. ✅ DECLARE INTENT: Request a new data channel.
    // This registers your desire for a channel; it doesn't create it yet.
    let channel_config = ChannelConfig {
        label: "my-channel".to_string(),
        ordered: true,
        ..Default::default()
    };
    let cid = rtc.data_channels().create(channel_config);
    println!("Requested data channel with future ID: {:?}", cid);

    // 2. ➡️ DRIVE THE STATE MACHINE: The `poll_output` loop.
    // This replaces the direct call to `create_offer`.
    loop {
        // Poll for the next output event from the state machine.
        let output = match rtc.poll_output() {
            Ok(output) => output,
            Err(e) => {
                eprintln!("Polling failed: {}", e);
                break;
            }
        };

        // You must also handle network input and timers.
        // For this example, we'll just handle the output.
        // A real app would have `rtc.handle_input(...)` and `rtc.handle_timeout(...)`.

        match output {
            // 3. ✨ HERE IS YOUR OFFER!
            // str0m generated this because it saw your channel request.
            RtcOutput::Offer(offer) => {
                println!(
                    "\n=> CREATED OFFER (send this to the other peer):\n{}",
                    offer.sdp
                );

                // Break the loop for this example once we have the offer.
                // A real app would continue polling to handle ICE candidates, etc.
                break;
            }
            RtcOutput::IceCandidate(c) => {
                println!("\n=> NEW ICE CANDIDATE (send this to the other peer):");
                println!("{:?}", c.candidate);
            }
            RtcOutput::Event(e) => {
                // Other events like connection state changes will appear here.
                println!("RTC Event: {:?}", e);
            }
            // When there's nothing to do, wait a bit.
            RtcOutput::Timeout(timeout) => {
                let duration = timeout.saturating_duration_since(Instant::now());
                sleep(duration).await;
                rtc.handle_timeout(Instant::now());
            }
            RtcOutput::Transmit(t) => {
                // This is where you'd send network packets for ICE/DTLS
            }
        }
    }

    Ok(())
}
