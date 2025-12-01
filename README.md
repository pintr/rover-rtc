# Rover RTC

A robust WebRTC-based peer-to-peer communication system built in Rust, designed for direct rover-to-rover communication with resilient network recovery capabilities.

## Overview

Rover RTC provides a complete WebRTC implementation featuring direct P2P data channels, automatic connection health monitoring, and network recovery mechanisms. The system is designed to maintain stable connections in dynamic network environments, making it suitable for mobile robotics, embedded systems, and other scenarios requiring reliable peer-to-peer communication.

## Architecture

The system consists of two main components:

### Server
An HTTP-based signaling server that:
- Handles SDP offer/answer exchange between peers
- Manages multiple concurrent WebRTC client connections
- Monitors connection health and triggers automatic recovery
- Relays UDP packets between connected clients
- Broadcasts periodic status messages to all clients

### Peer
A WebRTC client that:
- Establishes direct P2P connections via the signaling server
- Creates bidirectional data channels for communication
- Handles complete ICE candidate negotiation
- Supports network interface changes and reconnection
- Processes real-time data exchange with other peers

## Features

### Core Capabilities

- **Direct P2P Communication**: Establishes secure, end-to-end encrypted data channels between peers using WebRTC
- **Network Recovery**: Automatic detection and recovery from network failures using ICE restart mechanisms
- **Connection Health Monitoring**: Tracks connection status, activity, and failures for each client
- **Multi-Client Support**: Server can manage multiple simultaneous peer connections
- **Lightweight & Efficient**: Built on str0m for minimal overhead and direct socket control
- **Fully Asynchronous**: Leverages Tokio runtime for non-blocking I/O operations

### Network Resilience

The system includes comprehensive network recovery mechanisms to maintain connections in unstable network conditions:

#### Connection Health Monitoring

- **Activity Tracking**: Records timestamps of last successful communication for each client
- **Failure Detection**: Counts consecutive packet delivery failures
- **Automatic Health Checks**: Periodic monitoring every 5 seconds to identify degraded connections
- **Recovery Triggers**: Initiates recovery when no activity for >10 seconds with >3 consecutive failures
- **Attempt Limiting**: Maximum 3 ICE restart attempts to prevent infinite recovery loops

#### Graceful Degradation

- **UDP Failure Tolerance**: Temporary packet send failures don't immediately disconnect clients
- **Enhanced State Tracking**: Monitors all ICE connection state transitions (Checking, Connected, Disconnected)
- **Transient Issue Handling**: Only disconnects when connection is explicitly closed, not on temporary problems
- **Detailed Logging**: Comprehensive logging of connection health and recovery attempts

#### Recovery Mechanisms

The server implements automatic recovery through:

- `ConnectionHealth` struct tracking activity, failures, and restart attempts
- `check_client_health()` function for periodic health assessment
- `attempt_connection_recovery()` for automatic ICE restart when connections degrade
- Activity marking on successful polls and received packets
- Failure marking when packets aren't accepted by any client

Client-side recovery support:

- `add_new_candidate()` method to add new ICE candidates when network interfaces change
- `create_ice_restart_offer()` to generate ICE restart offers for signaling
- Enhanced event handling that tolerates temporary failures

## Technology Stack

- **WebRTC**: [str0m](https://github.com/algesten/str0m) 0.11.1 - Minimal WebRTC implementation with direct socket control
- **HTTP Server**: [rouille](https://github.com/tomaka/rouille) 3.6.2 - Lightweight HTTP server for signaling
- **Async Runtime**: [tokio](https://tokio.rs/) 1.48.0 - Asynchronous runtime for peer operations
- **Serialization**: [serde_json](https://github.com/serde-rs/json) 1.0.145 - JSON serialization for SDP exchange
- **HTTP Client**: [reqwest](https://github.com/seanmonstar/reqwest) 0.11.22 - Async HTTP client for signaling
- **Logging**: [tracing](https://github.com/tokio-rs/tracing) 0.1.37 - Structured logging and diagnostics
- **Binary Serialization**: [bincode](https://github.com/bincode-org/bincode) 2.0.1 - Efficient binary encoding

## Getting Started

### Prerequisites

- Rust 1.56 or later
- Cargo (comes with Rust)

### Installation

Clone the repository:

```bash
git clone https://github.com/pintr/rover-rtc.git
cd rover-rtc
```

### Running the System

You'll need two terminal windows to run a complete setup:

#### Terminal 1: Start the Signaling Server

```bash
cargo run server
```

The server will:
- Bind to a random UDP port for WebRTC traffic
- Start an HTTP server on `0.0.0.0:3000` for signaling
- Display the local address and port for connections

Expected output:
```
Bound UDP port: 192.168.1.100:54321
Connect a browser to http://192.168.1.100:3000
```

#### Terminal 2: Start a Peer Client

```bash
cargo run peer
```

The peer will:
- Create a WebRTC connection through the signaling server
- Establish ICE candidates and negotiate the connection
- Open a data channel named "test"
- Begin exchanging messages once connected

Expected output:
```
Starting modern str0m peer...
Offer SDP: [SDP details]
Answer SDP: [SDP details]
Event: IceConnectionStateChange(Connected)
Event: ChannelOpen(ChannelId(0), "test")
```

## Project Structure

```
rover-rtc/
├── src/
│   ├── main.rs           # Entry point and command-line argument handling
│   ├── server.rs         # WebRTC signaling server implementation
│   ├── peer.rs           # WebRTC peer client implementation
│   ├── model/
│   │   ├── client.rs     # Client connection management
│   │   ├── payload.rs    # Message payload structures
│   │   ├── propagated.rs # Propagated message handling
│   │   └── tracks.rs     # Media track management
│   └── util/
│       └── mod.rs        # Utility functions (logging, networking)
├── Cargo.toml            # Project dependencies and metadata
├── README.md             # This file
└── LICENSE               # Apache License 2.0
```

## How It Works

### Connection Flow

1. **Peer Initialization**: Peer creates an RTC instance, binds a UDP socket, and discovers local ICE candidates
2. **Offer Generation**: Peer creates a data channel and generates an SDP offer containing connection parameters
3. **Signaling Exchange**: Peer sends the offer to the signaling server via HTTP POST
4. **Server Processing**: Server receives the offer, creates its own RTC instance, and generates an SDP answer
5. **Answer Acceptance**: Peer receives the answer and completes the WebRTC negotiation
6. **ICE Negotiation**: Peers exchange ICE candidates to find the best connection path
7. **Connection Established**: Once ICE completes, a direct P2P UDP connection is established
8. **Data Exchange**: The data channel opens, enabling bidirectional message exchange

### Health Monitoring and Recovery

The server implements a comprehensive connection health monitoring system that continuously tracks the state of all connected peers.

#### Monitoring Process

For each connected client, the server maintains a `ConnectionHealth` record containing:

- **Last Activity Timestamp**: Updated on every successful packet exchange
- **Consecutive Failures**: Incremented when packets fail to reach any client
- **ICE Restart Attempts**: Counter tracking recovery attempts for this connection

Every 5 seconds, the server runs `check_client_health()` which:

1. Examines each client's health record
2. Identifies connections meeting recovery criteria
3. Initiates automatic recovery for degraded connections
4. Cleans up health records for disconnected clients

#### Recovery Criteria

Automatic recovery is triggered when ALL conditions are met:

- No activity for more than 10 seconds
- More than 3 consecutive packet failures
- Fewer than 3 previous restart attempts

#### Recovery Process

When triggered, `attempt_connection_recovery()` performs:

1. Calls client's `add_new_candidate()` to refresh ICE candidates
2. Increments the restart attempt counter
3. Resets the consecutive failure count
4. Logs the recovery attempt for monitoring

The system automatically re-establishes connections through available network paths using ICE restart mechanisms, allowing peers to maintain connectivity despite network changes or temporary disruptions.

#### Graceful Failure Handling

The implementation includes several safeguards:

- UDP send failures generate warnings but don't immediately kill connections
- Only explicit connection closure triggers disconnection
- Activity and failure counters provide objective health assessment
- Maximum attempt limits prevent infinite recovery loops
- Comprehensive logging enables debugging and monitoring

## API Overview

### Server Functions

- `server::main()` - Initializes and runs the signaling server
- `server::web_request()` - Handles HTTP signaling requests (SDP exchange)
- `server::run()` - Main event loop managing multiple clients
- `server::spawn_new_client()` - Creates new client instances from RTC connections
- `server::poll_client()` - Processes client output and returns next timeout
- `server::check_client_health()` - Periodic health monitoring of all clients

### Peer Functions

- `peer::main()` - Async entry point for peer client
- Creates data channels with `rtc.sdp_api().add_channel()`
- Generates offers with `change.apply()`
- Handles connection events through `rtc.poll_output()`
- Processes incoming data via `Event::ChannelData`

### Client Management

The `Client` struct (in `model/client.rs`) encapsulates:
- Unique client identification
- RTC instance management
- Data channel state tracking
- ICE connection monitoring
- Message handling capabilities

Key methods:
- `Client::new()` - Creates a new client instance
- `client.accepts()` - Checks if a packet belongs to this client
- `client.handle_input()` - Processes incoming UDP packets
- `client.poll_output()` - Drives the WebRTC state machine
- `client.send_message()` - Sends data through the channel

## Configuration

### Server Configuration

The server binds to:
- UDP: Random port on the selected host address (for WebRTC traffic)
- HTTP: Port 3000 on all interfaces (for signaling)

To modify the HTTP port, edit `server.rs`:
```rust
let server = Server::new("0.0.0.0:3000", move |request| {
    web_request(request, addr, tx.clone())
})
```

### Peer Configuration

The peer connects to the signaling server at `http://172.17.0.1:3000` by default. To change this, modify the URL in `peer.rs`:
```rust
let answer: SdpAnswer = client
    .post("http://YOUR_SERVER_IP:3000")
    .body(serde_json::to_string(&offer)?)
    .send()
    .await?
    .json()
    .await?;
```

### Logging

Logging is configured via the `RUST_LOG` environment variable:

```bash
# Info level (default)
RUST_LOG=info cargo run server

# Debug level for detailed logs
RUST_LOG=debug cargo run peer

# Module-specific logging
RUST_LOG=rover_rtc::peer=debug,rover_rtc::server=info cargo run server
```

## Troubleshooting

### Common Issues

**Server fails to bind**
- Ensure port 3000 is not already in use
- Check firewall settings allow incoming connections

**Peer cannot connect to server**
- Verify the server IP address in `peer.rs` is correct
- Ensure the server is running before starting the peer
- Check network connectivity between peer and server

**ICE connection fails**
- Verify UDP traffic is not blocked by firewall
- Check that NAT traversal is not required (current implementation uses host candidates only)
- Review logs for ICE state changes using `RUST_LOG=debug`

**Connection drops frequently**
- Enable debug logging to see health monitoring in action
- Check network stability and packet loss
- Review ICE restart attempts in server logs

## Development

### Building from Source

```bash
# Build in debug mode
cargo build

# Build in release mode (optimized)
cargo build --release

# Run tests
cargo test

# Check code without building
cargo check
```

### Code Structure

- `main.rs` - CLI entry point and argument parsing
- `server.rs` - Signaling server and client management (421 lines)
- `peer.rs` - WebRTC peer client implementation (278 lines)
- `model/client.rs` - Client abstraction for server-side connections (266 lines)
- `model/payload.rs` - Message payload structures
- `model/propagated.rs` - Propagated message handling
- `model/tracks.rs` - Media track management
- `util/mod.rs` - Utility functions for networking and logging

## Future Enhancements

- Support for TURN/STUN servers for NAT traversal
- Multiple data channels per connection
- Media track support (audio/video)
- Enhanced metrics and monitoring dashboard
- Automatic network interface switching
- Connection quality metrics (RTT, packet loss)
- Persistent storage of connection state
- Web-based control panel

## Contributing

Contributions are welcome. Please ensure:
- Code follows Rust idioms and formatting (`cargo fmt`)
- All tests pass (`cargo test`)
- New features include appropriate documentation
- Commit messages are clear and descriptive

## License

This project is licensed under the Apache License 2.0. See the [LICENSE](LICENSE) file for details.

## Acknowledgments

Built with [str0m](https://github.com/algesten/str0m), a minimal and powerful WebRTC implementation in Rust.

- **`server::read_socket_input()`**: Attempts to read from the UDP socket with timeout handling, converting received data into `Input::Receive` events for the RTC state machine.

- **`client::accepts()`**: Checks if a client's RTC instance should handle a given input based on the 5-tuple (source/destination addresses and protocol).

- **`client::send_message()`**: Sends a text message over the established data channel by writing bytes to the channel if it's open.

### Key Data Structures

- **`Client`**: Wraps an `Rtc` instance with a unique ID and optional channel ID for server-side connection management
- **`ClientId`**: Atomically-generated unique identifier for each client connection
- **`TrackIn/TrackOut`**: Structures for managing incoming and outgoing media tracks (audio/video)
- **`Propagated`**: Events propagated between clients for media relay scenarios
- **`WebrtcError`**: Error types for peer operations (server, SDP, network, send errors)

### ICE and Media Flow

```
Peer                          Server                        Other Peers
 |                               |                               |
 |---(1) POST SDP Offer--------> |                               |
 |<--(2) Return SDP Answer------ |                               |
 |                               |                               |
 |---(3) STUN/UDP packets------->|                               |
 |<--(4) STUN/UDP packets--------|                               |
 |                               |                               |
 | (ICE Connection Established)  |                               |
 |                               |                               |
 |---(5) Data Channel Open------>|                               |
 |<--(6) Data Channel Messages---|<----(7) Broadcast messages----|
 |                               |                               |
```

The server acts as a signaling relay and can forward messages between multiple connected peers, while the actual media/data flows peer-to-peer after ICE negotiation completes.

