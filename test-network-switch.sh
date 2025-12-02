#!/bin/bash

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Rover RTC Network Switch Test ===${NC}\n"

# Function to print section headers
print_header() {
    echo -e "\n${GREEN}>>> $1${NC}\n"
}

# Function to show current network status
show_network_status() {
    print_header "Current Network Status"
    
    echo -e "${YELLOW}Server networks:${NC}"
    podman exec rover-server ip addr show | grep -E "inet |^[0-9]+: " | grep -v "127.0.0.1"
    
    echo -e "\n${YELLOW}Peer networks:${NC}"
    podman exec rover-peer ip addr show | grep -E "inet |^[0-9]+: " | grep -v "127.0.0.1"
    
    echo ""
}

# Function to test connectivity
test_connectivity() {
    local target=$1
    echo -e "${YELLOW}Testing connectivity to $target...${NC}"
    
    if podman exec rover-peer ping -c 2 -W 2 $target > /dev/null 2>&1; then
        echo -e "${GREEN}✓ Connectivity OK${NC}"
        return 0
    else
        echo -e "${RED}✗ No connectivity${NC}"
        return 1
    fi
}

# Check if containers are running
if ! podman ps | grep -q rover-server || ! podman ps | grep -q rover-peer; then
    echo -e "${RED}Error: Containers not running. Please start with:${NC}"
    echo "  podman-compose up -d"
    exit 1
fi

# Show initial state
show_network_status

# Get server IP on network_a
SERVER_IP_A=$(podman inspect -f '{{range .NetworkSettings.Networks}}{{if eq .NetworkID "'$(podman network inspect -f '{{.Id}}' rover-rtc_network_a)'"}}{{.IPAddress}}{{end}}{{end}}' rover-server)
# Get server IP on network_b
SERVER_IP_B=$(podman inspect -f '{{range .NetworkSettings.Networks}}{{if eq .NetworkID "'$(podman network inspect -f '{{.Id}}' rover-rtc_network_b)'"}}{{.IPAddress}}{{end}}{{end}}' rover-server)

echo -e "${BLUE}Server IPs:${NC}"
echo -e "  Network A: ${GREEN}$SERVER_IP_A${NC}"
echo -e "  Network B: ${GREEN}$SERVER_IP_B${NC}"
echo ""

# Test initial connectivity
print_header "Step 1: Testing initial connectivity on Network A"
test_connectivity $SERVER_IP_A

echo -e "\n${YELLOW}Press ENTER to simulate network switch...${NC}"
read

# Disconnect from network_a
print_header "Step 2: Disconnecting peer from Network A"
podman network disconnect rover-rtc_network_a rover-peer
echo -e "${GREEN}✓ Disconnected from Network A${NC}"
sleep 2

show_network_status

echo -e "\n${RED}Peer is now OFFLINE - no network connectivity${NC}"
echo -e "${YELLOW}This simulates complete network loss (e.g., WiFi disconnect)${NC}"
echo -e "${YELLOW}Press ENTER to reconnect on Network B...${NC}"
read

# Connect to network_b
print_header "Step 3: Connecting peer to Network B"
podman network connect rover-rtc_network_b rover-peer
echo -e "${GREEN}✓ Connected to Network B${NC}"
sleep 2

show_network_status

print_header "Step 4: Testing connectivity on Network B"
test_connectivity $SERVER_IP_B

echo -e "\n${BLUE}Network switch complete!${NC}"
echo -e "The peer should now reconnect via the new network."
echo -e "Check the container logs to see ICE restart and reconnection.\n"

# Offer to switch back
echo -e "${YELLOW}Press ENTER to switch back to Network A (optional)...${NC}"
read

print_header "Step 5: Switching back to Network A"
podman network disconnect rover-rtc_network_b rover-peer
echo -e "${GREEN}✓ Disconnected from Network B${NC}"
sleep 1

podman network connect rover-rtc_network_a rover-peer
echo -e "${GREEN}✓ Connected back to Network A${NC}"
sleep 2

show_network_status

print_header "Step 6: Testing connectivity on Network A (again)"
test_connectivity $SERVER_IP_A

echo -e "\n${GREEN}=== Test Complete ===${NC}"
echo -e "\nUseful commands:"
echo -e "  ${BLUE}podman logs -f rover-peer${NC}   - Follow peer logs"
echo -e "  ${BLUE}podman logs -f rover-server${NC} - Follow server logs"
echo -e "  ${BLUE}podman exec -it rover-peer bash${NC} - Shell into peer container"
