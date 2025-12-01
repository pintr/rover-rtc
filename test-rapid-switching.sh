#!/bin/bash

# Script for rapid network switching to test reconnection stability

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}=== Rapid Network Switching Test ===${NC}\n"

# Check if containers are running
if ! docker ps | grep -q rover-server || ! docker ps | grep -q rover-peer; then
    echo -e "${RED}Error: Containers not running. Please start with:${NC}"
    echo "  docker-compose up --build"
    exit 1
fi

CYCLES=${1:-5}
DELAY=${2:-15}

echo -e "This test will switch between networks ${GREEN}$CYCLES${NC} times"
echo -e "with ${GREEN}$DELAY${NC} seconds between switches.\n"
echo -e "${YELLOW}Press ENTER to start...${NC}"
read

for ((i=1; i<=$CYCLES; i++)); do
    echo -e "\n${BLUE}=== Cycle $i/$CYCLES ===${NC}"
    
    # Switch to Network B
    echo -e "${YELLOW}[$i] Switching to Network B...${NC}"
    docker network disconnect rover-rtc_network_a rover-peer 2>/dev/null
    docker network connect rover-rtc_network_b rover-peer 2>/dev/null
    echo -e "${GREEN}[$i] On Network B${NC}"
    
    # Wait
    for ((j=$DELAY; j>0; j--)); do
        echo -ne "\r  Waiting: ${j}s  "
        sleep 1
    done
    echo ""
    
    # Switch to Network A
    echo -e "${YELLOW}[$i] Switching to Network A...${NC}"
    docker network disconnect rover-rtc_network_b rover-peer 2>/dev/null
    docker network connect rover-rtc_network_a rover-peer 2>/dev/null
    echo -e "${GREEN}[$i] On Network A${NC}"
    
    # Wait (except on last cycle)
    if [ $i -lt $CYCLES ]; then
        for ((j=$DELAY; j>0; j--)); do
            echo -ne "\r  Waiting: ${j}s  "
            sleep 1
        done
        echo ""
    fi
done

echo -e "\n${GREEN}=== Test Complete ===${NC}"
echo -e "Switched networks ${CYCLES} times."
echo -e "Check logs with: ${BLUE}docker logs rover-peer${NC}\n"
