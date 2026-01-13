# Chapter 1: Hello Zenoh - Pub/Sub Basics

**Time**: 20 minutes | **Level**: Beginner | **Concepts**: Sessions, Publishers, Subscribers

## Overview

In this chapter, you'll learn the fundamentals of Zenoh by building a simple temperature sensor and monitor. You'll understand:

- What a Zenoh session is
- How to publish data
- How to subscribe to data
- How to run multiple processes that communicate

## Architecture

```
┌──────────────────┐
│  Room A Sensor   │
│  (Publisher)     │
│ publishes temp   │
└────────┬─────────┘
         │
    Zenoh Network
         │
    ┌────┴────────┐
    │             │
┌───▼──────┐  ┌──▼───────┐
│ Monitor1 │  │ Monitor2  │
│(Subscribe)│ │(Subscribe)│
└──────────┘  └───────────┘
```

## Key Concepts

### Zenoh Session
A **session** is your connection to the Zenoh network. It's the entry point for publishing and subscribing.

```rust
let session = zenoh::open(Config::default()).await.unwrap();
```

### Publisher
A **publisher** sends data to a specific key expression. Multiple subscribers can listen to the same key.

```rust
let pub_temp = session.declare_publisher("building/floor1/room_a/temperature").await.unwrap();
pub_temp.put(message).await.unwrap();
```

### Subscriber
A **subscriber** listens for data published to a specific key expression.

```rust
let subscriber = session.declare_subscriber("building/floor1/room_a/temperature").await.unwrap();
while let Ok(sample) = subscriber.recv_async().await {
    let temperature = sample
        .payload()
        .try_to_string()
        .unwrap_or_else(|_| "unknown".into());
    println!("Received: {}", temperature);
}
```

### Key Expression
A **key expression** is the address/topic where data is published. In Zenoh, they're hierarchical strings separated by `/`.

Example: `building/floor1/room_a/temperature`

## Step-by-Step Guide

### Step 1: Create a New Project

```bash
mkdir smart-building-ch1
cd smart-building-ch1
cargo init --name smart-building-ch1
```

This creates a new Rust project with the following structure:
```
smart-building-ch1/
├── Cargo.toml
└── src/
    └── main.rs
```

### Step 2: Add Zenoh Dependency

Edit `Cargo.toml`:

```toml
[package]
name = "smart-building-ch1"
version = "0.1.0"
edition = "2021"

[dependencies]
zenoh = { version = "1.7.2", features = ["default"] }
tokio = { version = "1", features = ["full"] }
rand = "0.8"
env_logger = "0.11"
```

### Step 3: Create the Publisher (Sensor)

Create `src/bin/room_sensor.rs`:

```rust
use std::time::Duration;
use zenoh::config::Config;

#[tokio::main]
async fn main() {
    env_logger::init();

    println!("Opening Zenoh session...");
    let session = zenoh::open(Config::default()).await.unwrap();
    
    println!("Declaring publisher for building/floor1/room_a/temperature");
    let publisher = session
        .declare_publisher("building/floor1/room_a/temperature")
        .await
        .unwrap();

    println!("Room A Sensor started. Publishing temperature readings...\n");
    
    let mut temperature = 22.0;
    for i in 0..20 {
        // Simulate temperature variations
        temperature += (rand::random::<f32>() - 0.5) * 0.2;
        
        let message = format!("{:.1}", temperature);
        println!("[Room A Sensor] Publishing temperature: {}°C (reading #{})", message, i + 1);
        
        publisher.put(message).await.unwrap();
        
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    println!("\nRoom A Sensor: Done publishing 20 readings.");
}
```

### Step 4: Create the Subscriber (Monitor)

Create `src/bin/monitor.rs`:

```rust
use zenoh::config::Config;

#[tokio::main]
async fn main() {
    env_logger::init();

    println!("Opening Zenoh session...");
    let session = zenoh::open(Config::default()).await.unwrap();
    
    println!("Subscribing to building/floor1/room_a/temperature\n");
    let subscriber = session
        .declare_subscriber("building/floor1/room_a/temperature")
        .await
        .unwrap();

    println!("Monitor started. Waiting for temperature readings...\n");

    while let Ok(sample) = subscriber.recv_async().await {
        let temperature = sample
            .payload()
            .try_to_string()
            .unwrap_or_else(|_| "unknown".into());
        println!("[Monitor] Room A Temperature: {}°C", temperature);
    }
}
```

### Step 5: Add Cargo Binary Targets

Edit `Cargo.toml` to add binaries:

```toml
[[bin]]
name = "room_sensor"
path = "src/bin/room_sensor.rs"

[[bin]]
name = "monitor"
path = "src/bin/monitor.rs"
```

### Step 6: Build the Project

```bash
cargo build --release
```

### Step 7: Run the Example

**Terminal 1** - Start the sensor:
```bash
cargo run --release --bin room_sensor
```

Expected output:
```
Opening Zenoh session...
Declaring publisher for building/floor1/room_a/temperature
Room A Sensor started. Publishing temperature readings...

[Room A Sensor] Publishing temperature: 22.0°C
[Room A Sensor] Publishing temperature: 22.1°C
[Room A Sensor] Publishing temperature: 21.9°C
```

**Terminal 2** - Start the monitor:
```bash
cargo run --release --bin monitor
```

Expected output:
```
Opening Zenoh session...
Subscribing to building/floor1/room_a/temperature

Monitor started. Waiting for temperature readings...

[Monitor] Room A Temperature: 22.0°C
[Monitor] Room A Temperature: 22.1°C
[Monitor] Room A Temperature: 21.9°C
```

## Code Walkthrough

### Opening a Session

```rust
let session = zenoh::open(Config::default()).await.unwrap();
```

This creates a connection to Zenoh. By default, it tries to connect to a local Zenoh instance or creates a peer-to-peer connection with other Zenoh instances on the network.

### Declaring a Publisher

```rust
let publisher = session
    .declare_publisher("building/floor1/room_a/temperature")
    .await
    .unwrap();
```

This announces to Zenoh that this session will publish data under the key `building/floor1/room_a/temperature`.

### Publishing Data

```rust
publisher.put(message).await.unwrap();
```

This sends a message to all subscribers listening to this key.

### Declaring a Subscriber

```rust
let subscriber = session
    .declare_subscriber("building/floor1/room_a/temperature")
    .await
    .unwrap();
```

This announces interest in receiving data from the specified key.

### Receiving Data

```rust
while let Ok(sample) = subscriber.recv_async().await {
    let temperature = sample
        .payload()
        .try_to_string()
        .unwrap_or_else(|_| "unknown".into());
    println!("[Monitor] Room A Temperature: {}°C", temperature);
}
```

This waits for incoming samples and processes them as they arrive.

## Exercises

### Exercise 1: Multiple Monitors
Create a second monitor that subscribes to the same temperature data. Run it in a third terminal. You should see both monitors receiving the same data.

### Exercise 2: Different Rooms
Modify the sensor to publish two different rooms:
- `building/floor1/room_a/temperature`
- `building/floor1/room_b/temperature`

Create sensors for both rooms and a monitor for each.

**Hint**: Use different publishers in the same program.

### Exercise 3: Multiple Data Types
Extend the sensor to also publish humidity:
- `building/floor1/room_a/temperature`
- `building/floor1/room_a/humidity`

Create separate publishers for each and a monitor that subscribes to both.

## Common Issues

### Issue: "Address already in use"
**Cause**: A previous instance is still running.

**Solution**: 
```bash
# Kill any previous processes
pkill -f room_sensor
pkill -f monitor
```

### Issue: "Connection refused"
**Cause**: No Zenoh router or peer is available.

**Solution**: 
In Chapter 4 we'll add a router. For now, make sure sensor and monitor are running on the same machine.

### Issue: No messages received
**Cause**: Subscriber started after publisher, but no data was published yet.

**Solution**: 
The publisher needs to be running and sending data when the subscriber starts listening. Start the sensor first, then the monitor.

## Key Takeaways

✅ A **session** is your connection to Zenoh
✅ **Publishers** send data to key expressions
✅ **Subscribers** receive data from key expressions
✅ Multiple subscribers can listen to the same key
✅ Key expressions are hierarchical (e.g., `building/floor1/room_a/temperature`)

## Next Steps

Now that you understand basic pub/sub, let's organize your data better using **Key Expressions** and **wildcards** in [Chapter 2](../chapter_2/README.md).

You'll learn how to:
- Use wildcard subscriptions to listen to multiple sensors at once
- Organize data hierarchically
- Filter data efficiently

---

**[← Back to Main Tutorial](../README.md)** | **[Next: Chapter 2 →](../chapter_2/README.md)**
