# Chapter 3: Request/Reply - Query & Queryable

**Time**: 20 minutes | **Level**: Beginner | **Concepts**: Queryables, Get, Query/Reply Pattern

## Overview

In Chapters 1-2, you learned **pub/sub** - where publishers send data and subscribers listen continuously. But what if you want to **request data on-demand**? That's where **Query/Reply** (request/response) comes in.

This chapter teaches you how to:
- Create a **Queryable** service that responds to requests
- Send a **Query** (get request) and receive responses
- Build request/response communication patterns

## When to Use Query/Reply vs Pub/Sub

| Scenario | Use |
|----------|-----|
| Sensor publishes readings every second | Pub/Sub ✓ |
| Dashboard asks "What's the current temperature?" | Query/Reply ✓ |
| Multiple subscribers need latest readings | Pub/Sub ✓ |
| Request specific information from a service | Query/Reply ✓ |
| Stream of continuous data | Pub/Sub ✓ |
| Point-in-time snapshot or status check | Query/Reply ✓ |

## Architecture

```
Request/Response Pattern:

┌──────────────────────┐
│   Dashboard          │
│  (Querier)           │
│  "What's the temp?"  │
└──────────┬───────────┘
           │
           │ Query
           │
           ▼
┌──────────────────────┐
│  Room Service        │
│  (Queryable)         │
│  "It's 22.5°C"       │
└──────────┬───────────┘
           │
           │ Reply
           │
           ▼
┌──────────────────────┐
│   Dashboard          │
│   (Querier)          │
│   Display: 22.5°C    │
└──────────────────────┘
```

## Key Concepts

### Queryable

A **Queryable** is a service that listens for queries on a specific key and sends back replies.

```rust
let mut queryable = session
    .declare_queryable("building/floor1/room_a/status")
    .await?;

while let Ok(query) = queryable.recv_async().await {
    let response = "{ temperature: 22.5, humidity: 45 }";
    query.reply(Ok(response.into())).await?;
}
```

### Query / Get

A **Query** (via `.get()`) sends a request and waits for replies.

```rust
let results = session
    .get("building/floor1/room_a/status")
    .await?;

while let Ok(reply) = results.recv_async().await {
    let sample = reply.sample?;
    println!("Response: {}", String::from_utf8_lossy(&sample.payload));
}
```

### Selectors

A **Selector** allows querying with parameters or filters.

```rust
// Query with selector
let results = session
    .get("building/floor1/room_a/status?type=detailed")
    .await?;
```

## Step-by-Step Guide

### Step 1: Create a Status Service (Queryable)

Create `src/bin/room_status_service.rs`:

```rust
use std::time::Duration;
use zenoh::config::Config;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("Opening Zenoh session...");
    let session = zenoh::open(Config::default()).await?;

    // Simulated room state
    let state = Arc::new(Mutex::new((22.5, 45.0, 2)));

    println!("Declaring queryable for building/floor1/room_a/status\n");
    let mut queryable = session
        .declare_queryable("building/floor1/room_a/status")
        .await?;

    println!("Room A Status Service started. Waiting for queries...\n");

    while let Ok(query) = queryable.recv_async().await {
        let (temp, humidity, occupancy) = *state.lock().unwrap();

        // Build JSON response
        let response = format!(
            r#"{{"temperature": {:.1}, "humidity": {:.0}, "occupancy": {}}}"#,
            temp, humidity, occupancy
        );

        println!("[Status Service] Received query: {}", query.selector());
        println!("[Status Service] Sending response: {}", response);

        query.reply(Ok(response.into())).await?;

        // Simulate changing state
        *state.lock().unwrap() = (
            temp + (rand::random::<f32>() - 0.5) * 0.2,
            (humidity + (rand::random::<f32>() - 0.5) * 2.0).max(0.0).min(100.0),
            (rand::random::<f32>() * 5.0) as i32,
        );
    }

    Ok(())
}
```

### Step 2: Create a Dashboard (Querier)

Create `src/bin/dashboard.rs`:

```rust
use zenoh::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("Opening Zenoh session...");
    let session = zenoh::open(Config::default()).await?;

    println!("Dashboard ready. Querying room status...\n");

    for i in 1..=5 {
        println!("--- Query {} ---", i);
        println!("[Dashboard] Querying: building/floor1/room_a/status");

        let results = session
            .get("building/floor1/room_a/status")
            .await?;

        let mut found = false;
        while let Ok(reply) = results.recv_async().await {
            match reply.sample {
                Ok(sample) => {
                    let response = String::from_utf8_lossy(&sample.payload);
                    println!("[Dashboard] Response: {}\n", response);
                    found = true;
                }
                Err(e) => {
                    println!("[Dashboard] Error: {}\n", e);
                }
            }
        }

        if !found {
            println!("[Dashboard] No response received (service may be offline)\n");
        }

        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }

    println!("Dashboard: Done with 5 queries.");
    Ok(())
}
```

### Step 3: Create Multi-Service Queryable

Create `src/bin/building_status.rs`:

```rust
use zenoh::config::Config;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("Opening Zenoh session...");
    let session = zenoh::open(Config::default()).await?;

    // Room data
    let mut rooms: HashMap<String, (f32, f32)> = HashMap::new();
    rooms.insert("room_a".to_string(), (22.5, 45.0));
    rooms.insert("room_b".to_string(), (21.8, 42.0));
    rooms.insert("room_c".to_string(), (23.1, 48.0));
    let rooms = Arc::new(Mutex::new(rooms));

    println!("Declaring queryable for building/floor1/*/status\n");
    let mut queryable = session
        .declare_queryable("building/floor1/*/status")
        .await?;

    println!("Building Status Service started.\n");

    while let Ok(query) = queryable.recv_async().await {
        let selector = query.selector().to_string();
        println!("[Status Service] Received query: {}", selector);

        // Extract room name from selector
        // E.g., "building/floor1/room_a/status" -> "room_a"
        let room_name = selector
            .split('/')
            .nth(2)
            .unwrap_or("unknown")
            .to_string();

        let rooms_lock = rooms.lock().unwrap();
        match rooms_lock.get(&room_name) {
            Some((temp, humidity)) => {
                let response = format!(
                    r#"{{"room": "{}", "temperature": {:.1}, "humidity": {:.0}}}"#,
                    room_name, temp, humidity
                );
                println!("[Status Service] Sending: {}", response);
                query.reply(Ok(response.into())).await?;
            }
            None => {
                let error_msg = format!("Room {} not found", room_name);
                println!("[Status Service] Error: {}", error_msg);
                query.reply(Err(error_msg.into())).await?;
            }
        }
        println!();
    }

    Ok(())
}
```

### Step 4: Create Query with Selector

Create `src/bin/selective_query.rs`:

```rust
use zenoh::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("Opening Zenoh session...");
    let session = zenoh::open(Config::default()).await?;

    println!("Querying with selectors...\n");

    // Query with wildcard
    println!("[Query] Getting all room statuses with wildcard:");
    let results = session
        .get("building/floor1/*/status")
        .await?;

    let mut count = 0;
    while let Ok(reply) = results.recv_async().await {
        match reply.sample {
            Ok(sample) => {
                let key = sample.key_expr.to_string();
                let response = String::from_utf8_lossy(&sample.payload);
                println!("  {} -> {}", key, response);
                count += 1;
            }
            Err(e) => println!("  Error: {}", e),
        }
    }

    println!("\nReceived {} responses.\n", count);

    // Another query with different selector
    println!("[Query] Querying single room:");
    let results = session
        .get("building/floor1/room_a/status")
        .await?;

    while let Ok(reply) = results.recv_async().await {
        match reply.sample {
            Ok(sample) => {
                let response = String::from_utf8_lossy(&sample.payload);
                println!("  Response: {}", response);
            }
            Err(e) => println!("  Error: {}", e),
        }
    }

    Ok(())
}
```

### Step 5: Update Cargo.toml

```toml
[[bin]]
name = "room_status_service"
path = "src/bin/room_status_service.rs"

[[bin]]
name = "dashboard"
path = "src/bin/dashboard.rs"

[[bin]]
name = "building_status"
path = "src/bin/building_status.rs"

[[bin]]
name = "selective_query"
path = "src/bin/selective_query.rs"
```

### Step 6: Run Examples

**Example 1: Simple Query/Response**

Terminal 1:
```bash
cargo run --release --bin room_status_service
```

Terminal 2:
```bash
cargo run --release --bin dashboard
```

Expected output (Terminal 2):
```
--- Query 1 ---
[Dashboard] Querying: building/floor1/room_a/status
[Dashboard] Response: {"temperature": 22.5, "humidity": 45, "occupancy": 2}

--- Query 2 ---
[Dashboard] Querying: building/floor1/room_a/status
[Dashboard] Response: {"temperature": 22.7, "humidity": 44, "occupancy": 1}
```

**Example 2: Multiple Services with Wildcards**

Terminal 1:
```bash
cargo run --release --bin building_status
```

Terminal 2:
```bash
cargo run --release --bin selective_query
```

Expected output (Terminal 2):
```
[Query] Getting all room statuses with wildcard:
  building/floor1/room_a/status -> {"room": "room_a", "temperature": 22.5, "humidity": 45.0}
  building/floor1/room_b/status -> {"room": "room_b", "temperature": 21.8, "humidity": 42.0}
  building/floor1/room_c/status -> {"room": "room_c", "temperature": 23.1, "humidity": 48.0}

Received 3 responses.

[Query] Querying single room:
  Response: {"room": "room_a", "temperature": 22.5, "humidity": 45.0}
```

## Query Timeout & Multiple Replies

### Handling Multiple Replies

A queryable can have **multiple instances** responding to the same query:

```rust
// Multiple services can declare same queryable
let mut queryable1 = session.declare_queryable("building/floor1/room_a/status").await?;
let mut queryable2 = session.declare_queryable("building/floor1/room_a/status").await?;

// A query will receive replies from both
let results = session.get("building/floor1/room_a/status").await?;
// This will contain 2 replies
```

### Default Timeout

By default, a query waits a short time for replies. You can customize:

```rust
let results = session
    .get("building/floor1/room_a/status")
    .timeout(std::time::Duration::from_millis(500))
    .await?;
```

## Exercises

### Exercise 1: Error Handling
Modify `room_status_service.rs` to:
1. Make the service go offline for 3 seconds
2. In `dashboard.rs`, handle the timeout properly

**Hint**: Use `tokio::time::sleep()` to simulate offline periods.

### Exercise 2: Conditional Responses
Modify `building_status.rs` to respond differently based on the selector:
```
Query: building/floor1/room_a/status?type=simple
Response: {"temp": 22.5}

Query: building/floor1/room_a/status?type=detailed  
Response: {"temp": 22.5, "humidity": 45, "occupancy": 2, "timestamp": ...}
```

**Hint**: Parse `query.selector()` to extract parameters.

### Exercise 3: Aggregating Responses
Create a new querier that:
1. Queries all rooms: `building/floor1/*/status`
2. Collects all responses
3. Calculates average temperature across all rooms

## Common Issues

### Issue: "No reply received"
**Cause**: Queryable not running or key expression doesn't match.

**Solution**:
- Ensure queryable is running before sending queries
- Check key expressions match exactly (or use wildcards correctly)

### Issue: "Multiple replies for one query"
**Cause**: Multiple queryables declared with same key.

**Solution**: 
This is actually valid! Collect all replies:
```rust
while let Ok(reply) = results.recv_async().await {
    // Process each reply
}
```

## Pub/Sub vs Query/Reply Summary

| Aspect | Pub/Sub | Query/Reply |
|--------|---------|------------|
| **Initiator** | Publisher | Querier |
| **Data Flow** | Unidirectional (pub → sub) | Bidirectional (query → reply) |
| **Timing** | Continuous stream | On-demand |
| **Subscribers** | Multiple can listen | Multiple can respond |
| **Use Case** | Sensor data, events | Status, lookups, requests |

## Key Takeaways

✅ **Queryable** creates a service that responds to requests
✅ **Query/Get** sends a request and receives replies
✅ Multiple queryables can respond to the same query
✅ Wildcards work in queries too
✅ Query/Reply is for on-demand data; Pub/Sub is for streams

## Next Steps

Now you understand both **Pub/Sub** and **Query/Reply** patterns! Time to build a **distributed system** with multiple applications communicating through a **Zenoh Router** in [Chapter 4](../chapter_4/README.md).

You'll learn:
- How to deploy a Zenoh router
- How to configure multiple clients to connect to it
- How to build network topologies
- How to debug multi-process systems

---

**[← Back to Main Tutorial](../README.md)** | **[← Chapter 2](../chapter_2/README.md)** | **[Next: Chapter 4 →](../chapter_4/README.md)**
