pub mod model;
pub mod peer;
pub mod server;

use std::env;

mod util;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "server" => {
                println!("Starting server...");
                server::main();
            }
            "client" => {
                println!("Starting WebRTC peer...");
                peer::main();
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
