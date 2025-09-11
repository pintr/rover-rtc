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
