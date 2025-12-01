# Docker-Based Network Switching Test Environment

This directory contains a complete Docker-based testing environment for simulating network changes and testing WebRTC reconnection capabilities.

## Architecture

The setup creates:
- **1 Server container**: Connected to both networks (acts as relay)
- **1 Peer container**: Can switch between networks
- **2 Docker networks**: `network_a` (172.20.0.0/16) and `network_b` (172.21.0.0/16)

The server is connected to both networks simultaneously, so the peer can always reach it regardless of which network the peer is on. This simulates a real-world scenario where the server has a stable connection while the mobile peer switches networks (e.g., from WiFi to cellular).

## Quick Start

### 1. Build and Start Containers

```bash
# Build and start in detached mode
docker-compose up --build -d

# Or build and see logs
docker-compose up --build
```

### 2. Run Interactive Network Switch Test

```bash
# Make scripts executable (first time only)
chmod +x test-network-switch.sh test-rapid-switching.sh

# Run the interactive test
./test-network-switch.sh
```

This script will:
1. Show initial network status
2. Prompt you to disconnect peer from Network A
3. Show offline state
4. Prompt you to connect peer to Network B
5. Verify connectivity and reconnection
6. Optionally switch back

### 3. Run Automated Rapid Switching Test

```bash
# Run 5 network switches with 15 seconds between each
./test-rapid-switching.sh

# Custom: 10 switches with 20 seconds between
./test-rapid-switching.sh 10 20
```

## Manual Testing Commands

### View Logs

```bash
# Follow peer logs
docker logs -f rover-peer

# Follow server logs
docker logs -f rover-server

# View both side by side
docker-compose logs -f
```

### Manual Network Switching

```bash
# Disconnect peer from Network A
docker network disconnect rover-rtc_network_a rover-peer

# Connect peer to Network B
docker network connect rover-rtc_network_b rover-peer

# Disconnect from Network B
docker network disconnect rover-rtc_network_b rover-peer

# Reconnect to Network A
docker network connect rover-rtc_network_a rover-peer
```

### Check Network Status

```bash
# Show peer's network interfaces
docker exec rover-peer ip addr show

# Show peer's routing table
docker exec rover-peer ip route

# Show all network connections
docker exec rover-peer netstat -tunap

# Get server IP on Network A
docker inspect -f '{{range .NetworkSettings.Networks}}{{.NetworkID}} {{.IPAddress}}{{end}}' rover-server

# Get peer IP
docker inspect -f '{{range .NetworkSettings.Networks}}{{.NetworkID}} {{.IPAddress}}{{end}}' rover-peer
```

### Test Connectivity

```bash
# Ping server from peer (use actual server IP)
docker exec rover-peer ping -c 3 172.20.0.2

# Test from host
docker exec rover-peer curl -v http://rover-server:3000
```

### Shell Access

```bash
# Get shell in peer container
docker exec -it rover-peer bash

# Get shell in server container
docker exec -it rover-server bash

# Inside container, you can use tools like:
ip addr              # Show network interfaces
ip route             # Show routing table
ping <ip>            # Test connectivity
netstat -tunap       # Show network connections
tcpdump -i any       # Capture network traffic
```

## Advanced Testing Scenarios

### Simulate Packet Loss

```bash
# Add 30% packet loss on peer
docker exec rover-peer tc qdisc add dev eth0 root netem loss 30%

# Remove packet loss
docker exec rover-peer tc qdisc del dev eth0 root netem
```

### Simulate Network Latency

```bash
# Add 200ms delay
docker exec rover-peer tc qdisc add dev eth0 root netem delay 200ms

# Add variable delay (100ms ± 20ms)
docker exec rover-peer tc qdisc add dev eth0 root netem delay 100ms 20ms

# Remove delay
docker exec rover-peer tc qdisc del dev eth0 root netem
```

### Simulate Bandwidth Limitation

```bash
# Limit bandwidth to 1mbit
docker exec rover-peer tc qdisc add dev eth0 root tbf rate 1mbit burst 32kbit latency 400ms

# Remove limit
docker exec rover-peer tc qdisc del dev eth0 root
```

### Complete Network Outage Simulation

```bash
# Disconnect from all networks
docker network disconnect rover-rtc_network_a rover-peer
docker network disconnect rover-rtc_network_b rover-peer

# Peer is now completely offline
# Wait 15+ seconds to trigger health check

# Reconnect
docker network connect rover-rtc_network_b rover-peer
```

## What to Look For

When testing network switches, watch for these in the logs:

### Server Logs
- `Connection health check triggered for client`
- `Initiating ICE restart for client`
- `ICE restart offer sent to client`
- `ICE restart answer received from client`
- Changes in `consecutive_failures` counter

### Peer Logs
- `ICE Connection State: Disconnected`
- `ICE Connection State: Checking`
- `ICE Connection State: Connected`
- Message send/receive activity resuming after reconnection

### Expected Behavior
1. **Network switch detected**: Server sees failures, peer sees ICE state change to Disconnected
2. **ICE restart initiated**: Server creates new offer with ICE restart flag
3. **Reconnection**: New ICE candidates exchanged, connection re-established on new network
4. **Data channel recovery**: Messages resume flowing

### Timing
- Health check runs every 5 seconds
- Reconnection triggers after 10+ seconds of inactivity with 3+ failures
- ICE restart has max 3 attempts
- Full reconnection typically takes 5-15 seconds after network is restored

## Troubleshooting

### Containers won't start
```bash
# Clean up and rebuild
docker-compose down
docker-compose up --build
```

### Can't switch networks
```bash
# Check if peer is connected to network
docker network inspect rover-rtc_network_a

# Force disconnect/reconnect
docker network disconnect -f rover-rtc_network_a rover-peer
docker network connect rover-rtc_network_a rover-peer
```

### No reconnection happening
- Check server logs for health check activity
- Verify peer actually disconnected: `docker exec rover-peer ip addr`
- Ensure enough time passed (10+ seconds with failures)
- Check if max restart attempts (3) exceeded

## Cleanup

```bash
# Stop and remove containers
docker-compose down

# Remove containers and networks
docker-compose down --volumes

# Remove images too
docker-compose down --rmi all --volumes
```

## Environment Variables

You can customize behavior with environment variables:

```bash
# Set log level
RUST_LOG=debug docker-compose up

# Or edit docker-compose.yml and set:
environment:
  - RUST_LOG=debug
```

## Network Configuration Details

- **Network A**: 172.20.0.0/16, Gateway 172.20.0.1
- **Network B**: 172.21.0.0/16, Gateway 172.21.0.1
- **Server**: Connected to both networks, accessible from either
- **Peer**: Starts on Network A, can switch to Network B

This simulates a realistic scenario where:
- Server has stable public IP (or DNS)
- Peer is mobile and switches between WiFi/cellular networks
- Server remains reachable on both networks
