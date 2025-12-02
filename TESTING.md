# Rover RTC - Network Switching Test Environment

A WebRTC-based peer-to-peer communication system with Docker/Podman support for testing network switching and reconnection scenarios.

## Quick Start

### 1. Build the Application

```bash
# Build Rust binary locally (fast)
./build-local.sh
```

This compiles the Rust application on your host machine and creates container images with the binary.

### 2. Start Containers

```bash
# Start server and peer containers
podman-compose up -d

# Or with Docker
docker-compose up -d
```

### 3. Run Network Switch Tests

```bash
# Interactive network switch test
./test-network-switch.sh

# Automated rapid switching test (5 cycles, 15 seconds each)
./test-rapid-switching.sh
```

## Architecture

The test environment creates:
- **Server container**: Connected to both networks (172.20.0.0/16 and 172.21.0.0/16)
- **Peer container**: Starts on network_a, can switch to network_b
- Simulates real-world scenarios where a mobile peer switches between WiFi/cellular

## Testing Network Reconnection

### Interactive Test

The `test-network-switch.sh` script guides you through:
1. Initial connectivity check
2. Disconnecting from network A (simulates WiFi loss)
3. Reconnecting on network B (simulates switching to cellular)
4. Verifying reconnection and data flow
5. Optionally switching back to network A

### Rapid Switching Test

```bash
# Custom: 10 switches with 20 seconds between each
./test-rapid-switching.sh 10 20
```

## Monitoring

```bash
# View peer logs
podman logs -f rover-peer

# View server logs
podman logs -f rover-server

# View both
podman-compose logs -f
```

## Manual Network Operations

```bash
# Disconnect peer from network A
podman network disconnect rover-rtc_network_a rover-peer

# Connect peer to network B
podman network connect rover-rtc_network_b rover-peer

# Check peer's network interfaces
podman exec rover-peer ip addr show

# Ping server from peer
podman exec rover-peer ping -c 3 rover-server
```

## Advanced Testing

### Simulate Packet Loss
```bash
podman exec rover-peer tc qdisc add dev eth0 root netem loss 30%
# Remove: podman exec rover-peer tc qdisc del dev eth0 root netem
```

### Simulate Network Latency
```bash
podman exec rover-peer tc qdisc add dev eth0 root netem delay 200ms
# Remove: podman exec rover-peer tc qdisc del dev eth0 root netem
```

### Simulate Bandwidth Limitation
```bash
podman exec rover-peer tc qdisc add dev eth0 root tbf rate 1mbit burst 32kbit latency 400ms
# Remove: podman exec rover-peer tc qdisc del dev eth0 root
```

## What to Look For

When testing network switches, watch for these in the logs:

**Server Logs:**
- `Connection health check triggered for client`
- `Initiating ICE restart for client`
- `ICE restart offer sent to client`

**Peer Logs:**
- `ICE Connection State: Disconnected`
- `ICE Connection State: Checking`
- `ICE Connection State: Connected`
- Message send/receive resuming after reconnection

## Cleanup

```bash
# Stop and remove containers
podman-compose down

# Remove containers and volumes
podman-compose down --volumes
```

## Podman Setup (Recommended)

### Install Podman
```bash
# Ubuntu/Debian
sudo apt-get install podman podman-compose

# Fedora
sudo dnf install podman podman-compose

# macOS
brew install podman podman-compose
```

### Configure SQLite Database (Important)

Podman 5.x defaults to BoltDB which is deprecated. Migrate to SQLite:

```bash
# Create config directory
mkdir -p ~/.config/containers

# Create storage configuration
cat > ~/.config/containers/storage.conf << 'EOF'
[storage]
driver = "overlay"

[storage.options.overlay]
mountopt = "nodev,metacopy=on"
EOF

# Migrate to SQLite
podman system migrate

# Verify (should show: databaseBackend: sqlite)
podman info | grep databaseBackend
```

## Files

- **`README.md`** - This file
- **`Dockerfile`** - Container image definition
- **`docker-compose.yml`** - Multi-container setup
- **`build-local.sh`** - Build script (compiles locally, then builds images)
- **`test-network-switch.sh`** - Interactive network switching test
- **`test-rapid-switching.sh`** - Automated rapid switching test

## Troubleshooting

### Containers won't start
```bash
podman-compose down --volumes
./build-local.sh
podman-compose up -d
```

### Network switch doesn't work
```bash
# Check networks exist
podman network ls | grep rover-rtc

# Manually test
podman network disconnect rover-rtc_network_a rover-peer
podman network connect rover-rtc_network_b rover-peer
```

### BoltDB warning
See Podman Setup section above to migrate to SQLite.

## Project Structure

```
rover-rtc/
├── README.md                    # This file
├── Dockerfile                   # Container definition
├── docker-compose.yml           # Multi-container orchestration
├── build-local.sh              # Build script
├── test-network-switch.sh      # Interactive test
├── test-rapid-switching.sh     # Automated test
├── Cargo.toml                  # Rust dependencies
├── src/                        # Rust source code
│   ├── main.rs
│   ├── server.rs              # Signaling server
│   ├── peer.rs                # WebRTC peer client
│   └── ...
└── certs/                      # SSL certificates

```

## License

See LICENSE file.
