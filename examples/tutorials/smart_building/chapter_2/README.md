# Chapter 2: Organizing Data - Key Expressions & Hierarchies

**Time**: 20 minutes | **Level**: Beginner | **Concepts**: Key Expressions, Wildcards, Multiple Sensors

## Overview

In Chapter 1, we published one sensor's data to one key. In real systems, you have many sensors across many rooms. This chapter teaches you how to organize data hierarchically and use wildcards to efficiently subscribe to multiple sensors at once.

## Architecture

```
Building Hierarchy:
building/
├── floor1/
│   ├── room_a/
│   │   ├── temperature
│   │   ├── humidity
│   │   └── occupancy
│   ├── room_b/
│   │   ├── temperature
│   │   ├── humidity
│   │   └── occupancy
└── floor2/
    ├── room_c/
    │   ├── temperature
    │   ├── humidity
    │   └── occupancy
```

Subscribers can watch:
- Single sensor: `building/floor1/room_a/temperature`
- All room sensors: `building/floor1/room_a/*`
- All sensors in room: `building/floor1/room_a/**`
- All floor1 temperatures: `building/floor1/*/temperature`
- Everything: `building/**`

## Key Concepts

### Key Expression Syntax

A **key expression** is a hierarchical path with special syntax:

- **Exact match**: `building/floor1/room_a/temperature`
  - Matches exactly this key
  
- **Wildcard `*`**: `building/floor1/room_a/*`
  - Matches one segment (e.g., temperature, humidity, occupancy)
  - Does NOT match segments with `/` in them
  
- **Recursive wildcard `**`**: `building/floor1/room_a/**`
  - Matches multiple segments and nested structures
  - Matches `temperature`, `sub/nested/data`, etc.

### Examples

| Pattern | Matches | Doesn't Match |
|---------|---------|---------------|
| `building/floor1/room_a/temperature` | Exactly this | `building/floor2/room_a/temperature` |
| `building/floor1/room_a/*` | `humidity`, `occupancy`, `temperature` | `status/alert`, `data/sub/value` |
| `building/floor1/*/temperature` | All room temps on floor1 | Temps from other floors |
| `building/*/room_a/*` | All rooms named `room_a` on any floor | Rooms with different names |
| `building/**` | Everything under building | Nothing (catches all) |
| `building/floor1/**` | All data on floor1 | Data from floor2 or floor3 |

## Step-by-Step Guide

### Step 1: Create Multi-Sensor Publisher

Create `src/bin/multi_sensor.rs`:

```rust
use std::time::Duration;
use zenoh::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("Opening Zenoh session...");
    let session = zenoh::open(Config::default()).await?;

    // Declare publishers for different sensors in Room A
    println!("Declaring publishers...");
    let pub_temp = session
        .declare_publisher("building/floor1/room_a/temperature")
        .await?;
    let pub_humidity = session
        .declare_publisher("building/floor1/room_a/humidity")
        .await?;
    let pub_occupancy = session
        .declare_publisher("building/floor1/room_a/occupancy")
        .await?;

    println!("Multi-Sensor Publisher started.\n");

    let mut temp = 22.0;
    let mut humidity = 45.0;

    for i in 0..15 {
        // Simulate sensor readings
        temp += (rand::random::<f32>() - 0.5) * 0.4;
        humidity += (rand::random::<f32>() - 0.5) * 2.0;
        let occupancy = (rand::random::<f32>() * 5.0) as u32;

        println!("[Room A Sensors] Publishing reading #{}", i + 1);
        println!("  Temperature: {:.1}°C", temp);
        println!("  Humidity: {:.0}%", humidity.max(0.0).min(100.0));
        println!("  Occupancy: {} people\n", occupancy);

        pub_temp.put(format!("{:.1}", temp)).await?;
        pub_humidity.put(format!("{:.0}", humidity)).await?;
        pub_occupancy.put(occupancy.to_string()).await?;

        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    println!("Multi-Sensor Publisher: Done.");
    Ok(())
}
```

### Step 2: Create Wildcard Subscriber

Create `src/bin/floor_monitor.rs` - subscribes to all Room A sensors:

```rust
use zenoh::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("Opening Zenoh session...");
    let session = zenoh::open(Config::default()).await?;

    // Subscribe to ALL Room A sensors with wildcard
    println!("Subscribing to building/floor1/room_a/* (all sensors)\n");
    let mut subscriber = session
        .declare_subscriber("building/floor1/room_a/*")
        .await?;

    println!("Floor Monitor started. Listening to all Room A sensors...\n");

    let mut count = 0;
    while let Ok(sample) = subscriber.recv_async().await {
        let key = sample.key_expr.to_string();
        let value = String::from_utf8_lossy(&sample.payload);

        // Extract sensor type from key (last segment)
        let sensor_type = key.split('/').last().unwrap_or("unknown");

        match sensor_type {
            "temperature" => println!("🌡️  [Floor Monitor] Room A Temperature: {}°C", value),
            "humidity" => println!("💧 [Floor Monitor] Room A Humidity: {}%", value),
            "occupancy" => println!("👥 [Floor Monitor] Room A Occupancy: {} people", value),
            _ => println!("[Floor Monitor] {}: {}", sensor_type, value),
        }

        count += 1;
        if count >= 45 {
            // 15 readings * 3 sensors
            println!("\nFloor Monitor: Received 45 total readings. Exiting.");
            break;
        }
    }

    Ok(())
}
```

### Step 3: Create Multi-Room Publisher

Create `src/bin/building_sensors.rs` - multiple rooms:

```rust
use std::time::Duration;
use zenoh::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("Opening Zenoh session...");
    let session = zenoh::open(Config::default()).await?;

    // Publishers for Room A
    let pub_a_temp = session
        .declare_publisher("building/floor1/room_a/temperature")
        .await?;
    let pub_a_humidity = session
        .declare_publisher("building/floor1/room_a/humidity")
        .await?;

    // Publishers for Room B
    let pub_b_temp = session
        .declare_publisher("building/floor1/room_b/temperature")
        .await?;
    let pub_b_humidity = session
        .declare_publisher("building/floor1/room_b/humidity")
        .await?;

    // Publishers for Room C (Floor 2)
    let pub_c_temp = session
        .declare_publisher("building/floor2/room_c/temperature")
        .await?;
    let pub_c_humidity = session
        .declare_publisher("building/floor2/room_c/humidity")
        .await?;

    println!("Building Sensors started (3 rooms, 2 sensors each).\n");

    let mut a_temp = 22.0;
    let mut b_temp = 21.5;
    let mut c_temp = 23.0;

    for i in 0..10 {
        a_temp += (rand::random::<f32>() - 0.5) * 0.4;
        b_temp += (rand::random::<f32>() - 0.5) * 0.4;
        c_temp += (rand::random::<f32>() - 0.5) * 0.4;

        println!("[Building Sensors] Publishing round #{}", i + 1);

        pub_a_temp.put(format!("{:.1}", a_temp)).await?;
        pub_a_humidity.put("42").await?;

        pub_b_temp.put(format!("{:.1}", b_temp)).await?;
        pub_b_humidity.put("45").await?;

        pub_c_temp.put(format!("{:.1}", c_temp)).await?;
        pub_c_humidity.put("48").await?;

        println!("  Room A: {:.1}°C, Room B: {:.1}°C, Room C: {:.1}°C\n", a_temp, b_temp, c_temp);

        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    println!("Building Sensors: Done.");
    Ok(())
}
```

### Step 4: Create Selective Monitor

Create `src/bin/selective_monitor.rs` - watch only temperatures:

```rust
use zenoh::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("Opening Zenoh session...");
    let session = zenoh::open(Config::default()).await?;

    // Subscribe to ALL temperatures across the building
    println!("Subscribing to building/*/*/temperature (all temps)\n");
    let mut subscriber = session
        .declare_subscriber("building/*/*/temperature")
        .await?;

    println!("Temperature Monitor started.\n");

    let mut count = 0;
    while let Ok(sample) = subscriber.recv_async().await {
        let key = sample.key_expr.to_string();
        let value = String::from_utf8_lossy(&sample.payload);

        println!("[Temp Monitor] {}: {}°C", key, value);

        count += 1;
        if count >= 30 {
            println!("\nTemperature Monitor: Done.");
            break;
        }
    }

    Ok(())
}
```

### Step 5: Update Cargo.toml

Add the new binaries to `Cargo.toml`:

```toml
[[bin]]
name = "multi_sensor"
path = "src/bin/multi_sensor.rs"

[[bin]]
name = "floor_monitor"
path = "src/bin/floor_monitor.rs"

[[bin]]
name = "building_sensors"
path = "src/bin/building_sensors.rs"

[[bin]]
name = "selective_monitor"
path = "src/bin/selective_monitor.rs"
```

### Step 6: Run the Examples

**Example 1: Single Room with Multiple Sensors**

Terminal 1:
```bash
cargo run --release --bin multi_sensor
```

Terminal 2:
```bash
cargo run --release --bin floor_monitor
```

Expected output (Terminal 2):
```
🌡️  [Floor Monitor] Room A Temperature: 22.3°C
💧 [Floor Monitor] Room A Humidity: 44%
👥 [Floor Monitor] Room A Occupancy: 2 people
🌡️  [Floor Monitor] Room A Temperature: 22.4°C
💧 [Floor Monitor] Room A Humidity: 45%
👥 [Floor Monitor] Room A Occupancy: 1 people
```

**Example 2: Multiple Rooms with Selective Monitoring**

Terminal 1:
```bash
cargo run --release --bin building_sensors
```

Terminal 2:
```bash
cargo run --release --bin selective_monitor
```

Expected output (Terminal 2):
```
[Temp Monitor] building/floor1/room_a/temperature: 22.3°C
[Temp Monitor] building/floor1/room_b/temperature: 21.8°C
[Temp Monitor] building/floor2/room_c/temperature: 23.1°C
[Temp Monitor] building/floor1/room_a/temperature: 22.4°C
[Temp Monitor] building/floor1/room_b/temperature: 21.9°C
[Temp Monitor] building/floor2/room_c/temperature: 23.2°C
```

## Wildcard Matching Guide

### Pattern: `building/floor1/room_a/*`

Matches:
- `building/floor1/room_a/temperature` ✓
- `building/floor1/room_a/humidity` ✓
- `building/floor1/room_a/occupancy` ✓

Does NOT match:
- `building/floor1/room_b/temperature` ✗
- `building/floor1/room_a/data/nested` ✗

### Pattern: `building/floor1/*/temperature`

Matches:
- `building/floor1/room_a/temperature` ✓
- `building/floor1/room_b/temperature` ✓
- `building/floor1/room_c/temperature` ✓

Does NOT match:
- `building/floor2/room_a/temperature` ✗
- `building/floor1/room_a/humidity` ✗

### Pattern: `building/**/temperature`

Matches:
- `building/floor1/room_a/temperature` ✓
- `building/floor2/room_c/temperature` ✓
- `building/floor1/wing_a/room_x/temperature` ✓ (nested)

## Exercises

### Exercise 1: Watch Entire Building
Create a subscriber that watches ALL data across the building:
```rust
let mut subscriber = session.declare_subscriber("building/**").await?;
```

### Exercise 2: Temperature Threshold Alert
Modify `selective_monitor.rs` to:
1. Subscribe to all temperatures
2. Alert if any temperature exceeds 25°C

**Hint**:
```rust
let value: f32 = value.parse().unwrap_or(0.0);
if value > 25.0 {
    println!("⚠️  ALERT: High temperature!");
}
```

### Exercise 3: Multi-Floor Aggregation
Create a monitor that:
1. Subscribes to all temperatures with `building/**/temperature`
2. Groups readings by floor
3. Calculates average temperature per floor

**Hint**: Parse the key to extract floor information.

## Common Issues

### Issue: Wildcard not matching expected keys
**Cause**: Wildcard syntax confusion.

**Solution**: 
- `*` matches exactly one segment
- `**` matches any number of segments
- No matching happens across `/` boundaries with `*`

### Issue: Multiple subscribers interfering
**Cause**: In same process, subscriptions can overlap.

**Solution**: 
Each subscriber receives independent copies of all matching messages. This is expected and fine.

## Key Takeaways

✅ Key expressions are hierarchical (`building/floor/room/sensor`)
✅ `*` matches a single segment
✅ `**` matches multiple segments (recursive)
✅ Subscribers can use wildcards to listen to multiple keys
✅ Multiple subscribers can listen to same or overlapping keys
✅ Each subscriber gets all matching messages independently

## Next Steps

Now that you can organize and subscribe to multiple sensors, let's learn how to **request data on-demand** using the **Query/Reply pattern** in [Chapter 3](../chapter_3/README.md).

You'll learn how to:
- Create queryable services that respond to requests
- Query for specific data
- Build request/response communication patterns

---

**[← Back to Main Tutorial](../README.md)** | **[← Chapter 1](../chapter_1/README.md)** | **[Next: Chapter 3 →](../chapter_3/README.md)**
