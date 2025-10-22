//! Rover RTC - A WebRTC-based P2P communication system for rovers
//!
//! This crate provides a WebRTC implementation for direct peer-to-peer communication
//! between rovers using data channels. It includes both a signaling server and peer
//! functionality, designed to support seamless network handovers for resilient
//! connections in changing network environments.

pub mod model;
pub mod peer;
pub mod server;

use std::env;

mod util;

/// Entry point for the Rover RTC application.
///
/// Parses command-line arguments to determine whether to run as a server or peer.
///
/// # Usage
///
/// ```bash
/// cargo run server  # Start the WebRTC signaling server
/// cargo run peer    # Start a WebRTC peer client
/// ```
fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "server" => {
                println!("Starting server...");
                let _ = server::main();
            }
            "peer" => {
                println!("Starting WebRTC peer...");
                match peer::main() {
                    Ok(_) => println!("Peer completed successfully"),
                    Err(e) => println!("Peer error:\n{}", e),
                }
            }
            _ => {
                print_usage();
            }
        }
    } else {
        print_usage();
    }
}

/// Prints usage information for the application.
fn print_usage() {
    println!("Rover RTC");
    println!("Usage:");
    println!("  cargo run server  - Start the WebRTC server");
    println!("  cargo run client  - Start the WebRTC client");
}
