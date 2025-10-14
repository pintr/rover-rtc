pub mod model;
pub mod peer;
pub mod server;

use std::env;
use tokio::runtime::Runtime;

mod util;

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
                let rt = Runtime::new().unwrap();
                rt.block_on(peer::main()).unwrap();
            }
            _ => {
                print_usage();
            }
        }
    } else {
        print_usage();
    }
}

fn print_usage() {
    println!("Rover RTC");
    println!("Usage:");
    println!("  cargo run server  - Start the WebRTC server");
    println!("  cargo run client  - Start the WebRTC client");
}
