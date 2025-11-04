# Rust P2P Rover Communication Prototype

A Rust prototype for rover-to-rover communication using WebRTC data channels. It establishes a direct P2P link and is designed to perform seamless network handovers to maintain a resilient connection in changing network environments.

This project serves as a foundation for building robust, decentralized communication systems for mobile or embedded hardware.

---

### Core Features

* **Direct P2P Communication:** Establishes a secure, end-to-end encrypted data channel between two peers using WebRTC.
* **Handover-Ready Architecture:** Specifically designed to support seamless network handovers. The system can be extended to switch between network interfaces (e.g., Wi-Fi, Cellular, Ethernet) without dropping the connection by using the WebRTC ICE Restart mechanism.
* **Lightweight & Controllable:** Built with [`str0m`](https://github.com/algesten/str0m), a minimal WebRTC implementation that gives the application direct control over network sockets, which is essential for the handover logic.
* **Fully Asynchronous:** Uses the `tokio` runtime for efficient, non-blocking I/O, making it suitable for resource-constrained environments.

---

### How It Works

The system consists of two parts: a simple signaling server and the main rover client application.

1.  **Signaling Rendezvous:** The two rover clients connect to a WebSocket server. This server's only job is to act as a temporary message relay, helping the two peers find each other.
2.  **WebRTC Negotiation:** The clients exchange session information (SDP offers/answers) and network addresses (ICE candidates) through the signaling server.
3.  **P2P Connection:** Once negotiation is complete, `str0m` establishes a direct, encrypted UDP connection between the two rovers. The signaling server is no longer needed for communication.
4.  **Data Exchange:** A reliable data channel is established over the P2P connection, allowing the rovers to exchange messages directly.

The next development phase involves monitoring this P2P link's quality (latency, packet loss) to automatically trigger an ICE Restart for a network handover when the connection degrades.

---

### Technology Stack

* **WebRTC Implementation:** [`str0m`](https://github.com/algesten/str0m)
* **Asynchronous Runtime:** [`tokio`](https://tokio.rs/)
* **WebSocket Signaling:** [`tokio-tungstenite`](https://github.com/snapview/tokio-tungstenite)
* **Serialization (JSON):** [`serde`](https://serde.rs/)

---

### How to Run the Prototype

You will need three separate terminal windows.

1.  **Clone the Repository (if applicable):**
    ```bash
    git clone <your-repo-url>
    cd <your-repo-directory>
    ```

2.  **Terminal 1: Start the Signaling Server**
    This server simply relays messages between the two clients.
    ```bash
    cargo run --bin server
    ```
    *You should see a "Server listening on 127.0.0.1:3001" message.*

3.  **Terminal 2: Start the First Rover (Offerer)**
    This client will initiate the WebRTC offer.
    ```bash
    cargo run --main main_strom.rs -- offerer
    ```

4.  **Terminal 3: Start the Second Rover (Answerer)**
    This client will wait for the offer and respond.
    ```bash
    cargo run --main main_strom.rs
    ```

After the answerer starts, you will see logs in all terminals indicating that the connection is being established. Shortly after, the two rover clients will confirm that the data channel is open and will begin exchanging messages.

---

### License

This project is licensed under the **Apache License 2.0**.

---

## Sequence of Operations

This section documents the flow of operations and key functions involved in establishing and maintaining WebRTC connections in the Rover RTC system.

### Server Mode

When running as a server (`cargo run server`):

1. **Initialization** (`server::main`)
   - Initialize logging with `init_log()`
   - Select host address using `select_host_address()`
   - Bind UDP socket on random port for WebRTC traffic
   - Spawn background thread with `run()` function
   - Start HTTP server on port 3000 for signaling

2. **Signaling** (`server::web_request`)
   - Receive HTTP POST request with SDP offer from client
   - Deserialize offer using `serde_json::from_reader()`
   - Create new `Rtc` instance
   - Add host candidate with `rtc.add_local_candidate()`
   - Accept offer and generate answer with `rtc.sdp_api().accept_offer()`
   - Send `Rtc` instance to main loop via channel
   - Return SDP answer as JSON response

3. **Client Management** (`server::run`)
   - Poll channel for new clients with `spawn_new_client()`
   - Wrap each `Rtc` in `Client` instance using `Client::new()`
   - Remove disconnected clients with `clients.retain(|c| c.rtc.is_alive())`
   - Broadcast messages every 5 seconds using `client.send_message()`

4. **Event Loop** (`server::run`)
   - Poll each client with `poll_client()` to get next timeout
   - Read UDP socket with `read_socket_input()` using configurable timeout
   - Demultiplex packets to clients with `client.accepts(&input)`
   - Handle input with `client.handle_input()`
   - Drive time forward with `Input::Timeout(now)`

5. **Client Output Handling** (`client::poll_output`)
   - Poll RTC instance with `rtc.poll_output()`
   - Handle output types with `handle_output()`:
     - `Output::Transmit`: Send UDP packet via socket
     - `Output::Timeout`: Return timeout instant
     - `Output::Event`: Process WebRTC events
   - Handle events:
     - `Event::IceConnectionStateChange`: Monitor connection state
     - `Event::ChannelOpen`: Store channel ID for messaging
     - `Event::ChannelData`: Log received messages

### Peer Mode

When running as a peer (`cargo run peer`):

1. **Initialization** (`peer::main`)
   - Initialize logging with `init_log()`
   - Create new `Rtc` instance using `Rtc::new()`
   - Bind UDP socket on port 0 (random)
   - Discover local candidates with `get_candidates()`
   - Add candidates with `rtc.add_local_candidate()`

2. **Channel Creation**
   - Get SDP API with `rtc.sdp_api()`
   - Add data channel with `change.add_channel("test")`
   - Apply changes and create offer with `change.apply()`
   - Returns `(SdpOffer, SdpPending)`

3. **Signaling Exchange**
   - Serialize offer to JSON using `serde_json::to_string()`
   - POST offer to server at `http://172.17.0.1:3000`
   - Receive and deserialize answer as `SdpAnswer`
   - Accept answer with `rtc.sdp_api().accept_answer()`

4. **Connection Establishment Loop**
   - Poll RTC for output with `rtc.poll_output()`
   - Handle three output types:
     - `Output::Timeout`: Wait until timeout instant
     - `Output::Transmit`: Send UDP packet to destination
     - `Output::Event`: Process connection events
   - Set socket read timeout with `socket.set_read_timeout()`
   - Read incoming UDP packets with `socket.recv_from()`
   - Convert to `Input::Receive` with network details
   - Drive state forward with `rtc.handle_input()`

5. **ICE Connection States** (monitored via `Event::IceConnectionStateChange`)
   - `IceConnectionState::New`: ICE starting
   - `IceConnectionState::Checking`: Testing candidates
   - `IceConnectionState::Connected`: Connection established
   - `IceConnectionState::Completed`: ICE complete
   - `IceConnectionState::Disconnected`: Connection lost (exit loop)

6. **Data Channel Operations**
   - Wait for `Event::ChannelOpen` with matching channel ID
   - Receive data via `Event::ChannelData` events
   - Send data using `channel.write()`

### Utility Functions

The following utility functions support the WebRTC operations:

- **`util::select_host_address()`**: Discovers the first routable IPv4 address on the system by iterating through network interfaces and filtering out loopback, link-local, and broadcast addresses.

- **`util::get_candidates()`**: Generates a list of ICE host candidates for all available network interfaces (excluding loopback and link-local), each bound to the specified UDP socket port.

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
 |---(1) POST SDP Offer-------->|                               |
 |<--(2) Return SDP Answer------|                               |
 |                               |                               |
 |---(3) STUN/UDP packets------->|                               |
 |<--(4) STUN/UDP packets--------|                               |
 |                               |                               |
 |    (ICE Connection Established)                              |
 |                               |                               |
 |---(5) Data Channel Open------>|                               |
 |<--(6) Data Channel Messages---|<---(7) Broadcast messages----|
 |                               |                               |
```

The server acts as a signaling relay and can forward messages between multiple connected peers, while the actual media/data flows peer-to-peer after ICE negotiation completes.

