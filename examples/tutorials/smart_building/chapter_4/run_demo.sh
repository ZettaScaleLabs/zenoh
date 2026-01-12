#!/bin/bash

# Colors for output
BLUE='\033[0;34m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Zenoh Multi-Client Demo ===${NC}\n"

# Build project
echo -e "${YELLOW}Building project...${NC}"
cargo build --release 2>&1 | grep -v "warning:" || true

if [ $? -ne 0 ]; then
    echo -e "${RED}✗ Build failed${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Build complete${NC}\n"

# Start router in background
echo -e "${YELLOW}Starting Zenoh Router on tcp/127.0.0.1:7447${NC}"
cargo run --release --bin zenohd -- --config router.json5 &
ROUTER_PID=$!

# Give router time to start
sleep 2

echo -e "${GREEN}✓ Router started (PID: $ROUTER_PID)${NC}\n"

# Start sensor
echo -e "${YELLOW}Starting Sensor (in background)${NC}"
cargo run --release --bin sensor_with_router &
SENSOR_PID=$!

# Give sensor time to connect
sleep 1

echo -e "${GREEN}✓ Sensor started (PID: $SENSOR_PID)${NC}\n"

# Start monitor
echo -e "${YELLOW}Starting Monitor${NC}"
cargo run --release --bin monitor_with_router

# Cleanup
echo -e "\n${YELLOW}Cleaning up processes...${NC}"
kill $ROUTER_PID $SENSOR_PID 2>/dev/null

echo -e "${GREEN}✓ All processes stopped${NC}"
