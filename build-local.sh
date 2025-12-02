#!/bin/bash

# Quick build script for local development

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}=== Building Rover RTC Locally ===${NC}\n"

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: cargo not found. Please install Rust.${NC}"
    exit 1
fi

# Build release binary
echo -e "${BLUE}Building release binary...${NC}"
cargo build --release

if [ $? -ne 0 ]; then
    echo -e "${RED}Build failed!${NC}"
    exit 1
fi

echo -e "\n${GREEN}✓ Binary built successfully${NC}\n"

# Build Podman images
echo -e "${BLUE}Building Podman images with local binary...${NC}"
podman-compose build

if [ $? -ne 0 ]; then
    echo -e "${RED}Podman build failed!${NC}"
    exit 1
fi

echo -e "\n${GREEN}✓ Podman images built successfully${NC}\n"
echo -e "Start with: ${BLUE}podman-compose up -d${NC}"
