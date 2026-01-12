# Chapter 4: Distribution - Zenoh Router & Multi-Client Architecture

**Time**: 25 minutes | **Level**: Intermediate | **Concepts**: Zenoh Router, Network Config, Multi-Process Architecture

## Overview

In Chapters 1-3, publishers and subscribers ran on the same machine or automatically found each other. In real systems, services are distributed across different machines and networks. This chapter shows you how to:

- Deploy a **Zenoh router** (central coordinator)
- Connect multiple **client applications** to the router
- Configure network connections
- Build scalable distributed architectures

## Architecture

```
WITHOUT ROUTER (Peer-to-Peer):
┌─────────┐      ┌─────────┐      ┌─────────┐
│ Room 1  │ ←──→ │ Room 2  │ ←──→ │ Monitor │
│Sensor   │      │ Sensor  │      │  (Sub)  │
└─────────┘      └─────────┘      └─────────┘
(Each must know about all others)

WITH ROUTER (Centralized):
┌─────────┐
│ Room 1  │
│ Sensor  │
└────┬────┘
     │
     ├─────────────┐
     │             │
     ▼             ▼
┌──────────────────────────┐
│   ZENOH ROUTER           │
│  (Central Coordinator)   │
└──────────────────────────┘
     ▲             ▲
     │             │
     └─────────────┤
                   │
        ┌──────────┴──────────┐
        │                     │
    ┌───▼────┐         ┌─────▼────┐
    │ Room 2 │         │ Monitor  │
    │ Sensor │         │  (Sub)   │
    └────────┘         └──────────┘
```

## Key Concepts

### Zenoh Router
A **router** is a centralized process that:
- Receives connections from clients
- Routes messages between publishers and subscribers
- Provides discovery and coordination
- Can run on dedicated hardware (server, edge gateway)

### Client Connection
**Clients** (publishers, subscribers, queryables) connect to a router via TCP, UDP, or other protocols.

### Network Configuration
Configure where clients connect, what protocols to use, and security settings via configuration files.

## Step-by-Step Guide

### Step 1: Understand Zenoh Router

The Zenoh router is already included in your Zenoh installation as `zenohd`. Let's learn about it:

```bash
# If zenohd is in your path:
zenohd --help

# Or build it from source:
cd path/to/zenoh
cargo build --release --bin zenohd
```

### Step 2: Create Router Configuration

Create `router.json5` in your `chapter_4` directory:

```jsonc
{
    // Listen for client connections
    listeners: {
        default: {
            type: "tcp",
            bind: "127.0.0.1:7447",
        }
    },

    // Administrative API (optional)
    admin: {
        auth: {
            default_permission: "allow",
        }
    },

    // Logging level
    log: {
        level: "info",
        format: "compact",
    }
}
```

This configuration:
- Listens on TCP port 7447
- Allows connections from localhost
- Logs at INFO level

### Step 3: Create Client Configuration

Create `client_config.json5` for connecting to a router:

```jsonc
{
    // Connect to router instead of peer-to-peer
    mode: "client",
    connect: {
        endpoints: ["tcp/127.0.0.1:7447"]
    }
}
```

### Step 4: Modify Examples to Use Router Config

Create `src/bin/sensor_with_router.rs`:

```rust
use std::time::Duration;
use zenoh::config::Config;

#[tokio::main]
async fn main() {
    env_logger::init();

    println!("Loading client configuration from client_config.json5...");
    
    // Load configuration from file
    let config = Config::from_file("client_config.json5").unwrap();
    
    println!("Opening Zenoh session as CLIENT...");
    let session = zenoh::open(config).await.unwrap();
    
    println!("Connected to Zenoh router!");
    println!("Declaring publisher for building/floor1/room_a/temperature\n");
    
    let publisher = session
        .declare_publisher("building/floor1/room_a/temperature")
        .await
        .unwrap();

    println!("Sensor started. Publishing temperature readings...\n");

    let mut temperature = 22.0;
    for i in 0..20 {
        temperature += (rand::random::<f32>() - 0.5) * 0.2;

        let message = format!("{:.1}", temperature);
        println!("[Sensor] Publishing: {}°C (reading #{})", message, i + 1);

        publisher.put(message).await.unwrap();

        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    println!("\nSensor: Done publishing 20 readings.");
}
```

### Step 5: Create Monitor with Router

Create `src/bin/monitor_with_router.rs`:

```rust
use zenoh::config::Config;

#[tokio::main]
async fn main() {
    env_logger::init();

    println!("Loading client configuration from client_config.json5...");
    let config = Config::from_file("client_config.json5").unwrap();

    println!("Opening Zenoh session as CLIENT...");
    let session = zenoh::open(config).await.unwrap();

    println!("Connected to Zenoh router!");
    println!("Subscribing to building/floor1/room_a/temperature\n");
    
    let mut subscriber = session
        .declare_subscriber("building/floor1/room_a/temperature")
        .await
        .unwrap();

    println!("Monitor started. Waiting for temperature readings...\n");

    let mut count = 0;
    while let Ok(sample) = subscriber.recv_async().await {
        let temperature = sample
            .payload()
            .try_to_string()
            .unwrap_or_else(|_| "unknown".into());
        println!("[Monitor] Temperature: {}°C", temperature);

        count += 1;
        if count >= 20 {
            println!("\nMonitor: Received 20 readings. Exiting.");
            break;
        }
    }
}
```

### Step 6: Create Multi-Process Demo Script

Create `run_demo.sh`:

```bash
#!/bin/bash

# Colors for output
BLUE='\033[0;34m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Zenoh Multi-Client Demo ===${NC}\n"

# Build project
echo -e "${YELLOW}Building project...${NC}"
cargo build --release

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
MONITOR_PID=$!

# Cleanup
echo -e "\n${YELLOW}Cleaning up processes...${NC}"
kill $ROUTER_PID $SENSOR_PID $MONITOR_PID 2>/dev/null

echo -e "${GREEN}✓ All processes stopped${NC}"
```

Make it executable:
```bash
chmod +x run_demo.sh
```

### Step 7: Update Cargo.toml

```toml
[[bin]]
name = "sensor_with_router"
path = "src/bin/sensor_with_router.rs"

[[bin]]
name = "monitor_with_router"
path = "src/bin/monitor_with_router.rs"
```

### Step 8: Run the Multi-Client System

**Option 1: Manual (Separate Terminals)**

Terminal 1 - Start Router:
```bash
cargo run --release --bin zenohd -- --config router.json5
```

Expected output:
```
2025-01-12T19:30:00.000Z INFO zenohd - Zenoh router startup
2025-01-12T19:30:00.001Z INFO zenohd - Listening on: tcp/127.0.0.1:7447
```

Terminal 2 - Start Sensor:
```bash
cargo run --release --bin sensor_with_router
```

Expected output:
```
Loading client configuration from client_config.json5...
Opening Zenoh session as CLIENT...
Connected to Zenoh router!
Declaring publisher for building/floor1/room_a/temperature

[Sensor] Publishing: 22.0°C (reading #1)
[Sensor] Publishing: 22.1°C (reading #2)
```

Terminal 3 - Start Monitor:
```bash
cargo run --release --bin monitor_with_router
```

Expected output:
```
Loading client configuration from client_config.json5...
Opening Zenoh session as CLIENT...
Connected to Zenoh router!
Subscribing to building/floor1/room_a/temperature

[Monitor] Temperature: 22.0°C
[Monitor] Temperature: 22.1°C
```

**Option 2: Automated (Using Script)**

```bash
./run_demo.sh
```

This runs all components automatically, waits for sensor/monitor to complete, then cleans up.

## Network Topologies

### Simple Star Topology

```
      Router
     /  |  \
   S1  S2   Sub
```

Configuration:
- Router on machine A
- Sensors S1, S2 on machines B, C
- Subscriber on machine D

All connect to router.

### Hierarchical Topology

```
       Router1 (Data Center)
       /        \
    Router2    Router3
    /  |       |  \
  S1  S2      S3  S4
```

Multiple routers at different levels. More on this in advanced chapters.

## Configuration Options

### Common Router Settings

```jsonc
{
    // Listener on specific interface
    listeners: {
        default: {
            type: "tcp",
            bind: "0.0.0.0:7447",  // Listen on all interfaces
        }
    },

    // Multiple listeners
    listeners: {
        tcp: {
            type: "tcp",
            bind: "0.0.0.0:7447",
        },
        udp: {
            type: "udp",
            bind: "0.0.0.0:7448",
        }
    }
}
```

### Common Client Settings

```jsonc
{
    mode: "client",
    
    // Single endpoint
    connect: {
        endpoints: ["tcp/router-host:7447"]
    },

    // Multiple endpoints (failover)
    connect: {
        endpoints: [
            "tcp/router1:7447",
            "tcp/router2:7447"
        ]
    }
}
```

## Debugging Multi-Client Systems

### Check Router Status

Terminal with router running:
```bash
# Look for connection messages
# Example output:
# 2025-01-12T19:30:01.000Z INFO zenohd - Peer connected: client1
# 2025-01-12T19:30:02.000Z INFO zenohd - Peer connected: client2
```

### Enable Debug Logging

Set environment variable:
```bash
RUST_LOG=zenoh=debug cargo run --release --bin sensor_with_router
```

### Check Network Connectivity

```bash
# Test connectivity to router
nc -zv 127.0.0.1 7447
# Output: Connection to 127.0.0.1 7447 port [tcp/*] succeeded!
```

## Exercises

### Exercise 1: Multiple Sensors
Modify the demo to have **3 sensors** running in parallel:
- room_a, room_b, room_c
- Each publishes to `building/floor1/{room}/temperature`
- One monitor subscribes to all: `building/floor1/*/temperature`

**Hint**: Start multiple sensor processes before the monitor.

### Exercise 2: Remote Router
Change the router config to listen on `0.0.0.0:7447` (all interfaces) instead of just localhost. This allows connections from other machines.

Client config becomes:
```jsonc
connect: {
    endpoints: ["tcp/your-router-ip:7447"]
}
```

### Exercise 3: Multi-Floor System
Create a complete building:
```
building/
├── floor1/
│   ├── room_a/ (sensor + monitor)
│   └── room_b/ (sensor + monitor)
└── floor2/
    ├── room_c/ (sensor + monitor)
    └── room_d/ (sensor + monitor)
```

All connected through a single central router.

## Common Issues

### Issue: "Connection refused"
**Cause**: Router not running or wrong address.

**Solution**:
```bash
# Make sure router is running
cargo run --release --bin zenohd -- --config router.json5

# Check the listening address in output
# Should see: "Listening on: tcp/127.0.0.1:7447"
```

### Issue: "Cannot find zenohd"
**Cause**: zenohd binary not in PATH.

**Solution**:
```bash
# Build zenohd first (if not already built)
cd path/to/zenoh
cargo build --release --bin zenohd

# Then use full path in demo script
./target/release/zenohd --config router.json5
```

### Issue: Port already in use
**Cause**: Previous router still running or another service using port 7447.

**Solution**:
```bash
# Find what's using the port
lsof -i :7447

# Kill the process
kill <PID>
```

## Key Takeaways

✅ **Zenoh Router** is a central coordinator for distributed systems
✅ **Clients** (publishers, subscribers) connect to routers
✅ Configuration files specify connection details
✅ Multiple clients can communicate through one router
✅ Same pub/sub and query/reply patterns work across the network

## Next Steps

Now that you can build distributed systems with routers, let's build a **complete multi-tier application** with edge devices, aggregation services, and data center components in [Chapter 5](../chapter_5/README.md).

You'll build:
- Edge sensor tier
- Aggregation tier
- Central data center tier
- Complete working system

---

**[← Back to Main Tutorial](../README.md)** | **[← Chapter 3](../chapter_3/README.md)** | **[Next: Chapter 5 →](../chapter_5/README.md)**
