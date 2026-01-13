# Chapter 6: Storage & Persistence

**Time**: 30 minutes | **Level**: Intermediate/Advanced | **Concepts**: Storage, Data Persistence, Historical Data, Queries

## Overview

In this chapter, you'll learn how to add persistent storage to your Zenoh system. You'll understand:

- Implementing storage backends
- Querying historical data
- Time-series data storage
- Data retention policies
- Backup and recovery
- Database integration

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     QUERY CLIENTS                        │
│    (Request Historical Data, Aggregations, Analytics)    │
└──────────────┬─────────────────────────────────────────┘
               │ Query: sensor/*/data?start=T1&end=T2
┌──────────────▼─────────────────────────────────────────┐
│            STORAGE LAYER (Storage Service)              │
│  ┌────────────────────────────────────────────────────┐ │
│  │  Queryable: sensors/*/data/query                   │ │
│  │  Subscriber: sensors/*/data (incoming)             │ │
│  │  Publisher: sensors/*/data/results (responses)     │ │
│  └────────────────────────────────────────────────────┘ │
└──────┬───────────────────────────────────────────────┬──┘
       │                                               │
       │ ┌─────────────────────────────────────────┐   │
       └─│   Persistent Data Store (SQLite/DB)    │───┘
         │  • Time-series table                    │
         │  • Indexes on (sensor_id, timestamp)   │
         │  • Retention policies                  │
         └─────────────────────────────────────────┘
               │
         ┌─────▼─────┐
         │  Backups  │
         │  Archives │
         └───────────┘
```

## Key Concepts

### Storage Service Pattern

```rust
// Service subscribes to data streams and stores them
let mut data_sub = session.declare_subscriber("sensors/data/*").await.unwrap();
loop {
    if let Ok(sample) = data_sub.recv_async().await {
        // Store in database
        db.insert(&sample).await.unwrap();
    }
}

// Service exposes queryable interface for retrieval
let mut query = session.declare_queryable("sensors/data/query").await.unwrap();
loop {
    if let Ok(q) = query.recv_async().await {
        // Query database based on parameters
        let results = db.query(&q.selector()).await.unwrap();
        q.reply(q.key_expr().clone(), results).await.unwrap();
    }
}
```

### Time-Series Data

```rust
// Store data with timestamp for historical analysis
let record = DataRecord {
    sensor_id: 1,
    temperature: 22.5,
    timestamp: chrono::Utc::now(),
};
db.store(record).await.unwrap();

// Query time ranges efficiently
let records = db.query_range(
    "sensor_1",
    start_time,
    end_time,
).await.unwrap();
```

### Data Retention

```rust
// Clean up old data periodically
tokio::spawn(async {
    loop {
        // Delete records older than 7 days
        db.delete_older_than(
            chrono::Utc::now() - Duration::days(7)
        ).await.unwrap();
        
        tokio::time::sleep(Duration::hours(1)).await;
    }
});
```

## Step-by-Step Implementation

### Step 1: Create Chapter 6 Project

```bash
cd chapter_6
cargo init --name smart-building-ch6
```

Edit `Cargo.toml`:

```toml
[package]
name = "smart-building-ch6"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"

[workspace]

[[bin]]
name = "storage_service"
path = "src/bin/storage_service.rs"

[[bin]]
name = "data_client"
path = "src/bin/data_client.rs"

[[bin]]
name = "sensor_with_storage"
path = "src/bin/sensor_with_storage.rs"

[dependencies]
zenoh = { path = "../../../../zenoh", features = ["default"] }
tokio = { version = "1", features = ["full"] }
serde_json = "1.0"
chrono = "0.4"
```

### Step 2: Storage Service Implementation

Create `src/bin/storage_service.rs` - Stores all incoming sensor data and provides query interface for historical retrieval.

```rust
use zenoh::config::Config;
use serde_json::json;
use std::collections::VecDeque;
use chrono::{DateTime, Utc, Duration};

#[derive(Clone, Debug)]
struct SensorReading {
    sensor_id: u32,
    value: f32,
    timestamp: DateTime<Utc>,
}

#[tokio::main]
async fn main() {
    env_logger::init();

    println!("Opening Zenoh session for storage service...");
    let session = zenoh::open(Config::default()).await.unwrap();

    // In-memory store (VecDeque with max 1000 readings for tutorial)
    let store = std::sync::Arc::new(tokio::sync::Mutex::new(
        VecDeque::<SensorReading>::with_capacity(1000)
    ));

    // Subscribe to sensor data
    let mut sensor_sub = session
        .declare_subscriber("sensors/data/*")
        .await
        .unwrap();

    // Declare queryable for data retrieval
    let mut query = session
        .declare_queryable("sensors/data/query")
        .await
        .unwrap();

    println!("Storage service started. Listening for data...\n");

    // Spawn query handler
    let store_clone = store.clone();
    tokio::spawn(async move {
        loop {
            if let Ok(q) = query.recv_async().await {
                let selector = q.selector().to_string();
                let store_lock = store_clone.lock().await;
                
                // Filter stored data based on query
                let results: Vec<_> = store_lock
                    .iter()
                    .filter(|r| selector.contains(&r.sensor_id.to_string()))
                    .collect();

                let response = json!({
                    "query": selector,
                    "count": results.len(),
                    "latest": results.last().map(|r| r.value)
                });

                q.reply(q.key_expr().clone(), response.to_string())
                    .await
                    .unwrap();
            }
        }
    });

    // Store incoming data
    loop {
        if let Ok(sample) = sensor_sub.recv_async().await {
            if let Ok(data_str) = sample.payload().try_to_string() {
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&data_str) {
                    let reading = SensorReading {
                        sensor_id: data["sensor_id"].as_u64().unwrap_or(0) as u32,
                        value: data["value"].as_f64().unwrap_or(0.0) as f32,
                        timestamp: Utc::now(),
                    };

                    let mut store_lock = store.lock().await;
                    store_lock.push_back(reading.clone());
                    
                    if store_lock.len() > 1000 {
                        store_lock.pop_front();
                    }

                    println!("[Storage] Stored: Sensor {}, Value: {:.1}, Total: {}",
                        reading.sensor_id, reading.value, store_lock.len());
                }
            }
        }
    }
}
```

### Step 3: Data Query Client

Create `src/bin/data_client.rs` - Queries historical data from storage service.

```rust
use zenoh::config::Config;

#[tokio::main]
async fn main() {
    env_logger::init();

    println!("Opening Zenoh session for data client...");
    let session = zenoh::open(Config::default()).await.unwrap();

    // Query storage for specific sensor
    let selector = "sensors/data/query";

    println!("Querying storage service for sensor data...\n");

    loop {
        // Query for sensor 1 data
        match session.get(selector).await {
            Ok(mut results) = {
                while let Ok(result) = results.recv_async().await {
                    match result.sample {
                        Ok(sample) => {
                            if let Ok(data) = sample.payload().try_to_string() {
                                println!("[Query Result] {}", data);
                            }
                        }
                        Err(e) => println!("[Query Error] {}", e),
                    }
                }
            }
            Err(e) => println!("[Query Failed] {}", e),
        }

        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}
```

### Step 4: Sensor with Storage Publisher

Create `src/bin/sensor_with_storage.rs` - Publishes data that gets stored.

```rust
use std::time::Duration;
use zenoh::config::Config;
use serde_json::json;

#[tokio::main]
async fn main() {
    env_logger::init();

    println!("Opening Zenoh session...");
    let session = zenoh::open(Config::default()).await.unwrap();

    let pub_data = session
        .declare_publisher("sensors/data/*")
        .await
        .unwrap();

    println!("Publishing sensor data for storage...\n");

    for i in 0..100 {
        let data = json!({
            "sensor_id": (i % 5) + 1,
            "value": 20.0 + (i as f32 / 10.0),
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        pub_data.put(data.to_string()).await.unwrap();
        println!("[Publisher] Sent data #{}: {}", i + 1, data);

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    println!("\nPublishing complete.");
}
```

## Compilation and Running

```bash
cd chapter_6
cp ../../../../Cargo.lock .
cargo build --release

# Terminal 1: Start storage service
./target/release/storage_service

# Terminal 2: Start sensor publisher
./target/release/sensor_with_storage

# Terminal 3: Query historical data
./target/release/data_client
```

## Expected Output

### Storage Service:
```
Opening Zenoh session for storage service...
Storage service started. Listening for data...

[Storage] Stored: Sensor 1, Value: 20.1, Total: 1
[Storage] Stored: Sensor 2, Value: 20.2, Total: 2
```

### Data Client:
```
Querying storage service for sensor data...

[Query Result] {"query":"sensors/data/query","count":50,"latest":25.5}
```

## Exercises

### Exercise 1: Implement Time-Range Queries
**Task:** Add date range parameters to query historical data within specific times.

**Hint:** Extend query format: `sensors/data/query?start=2024-01-01T00:00:00Z&end=2024-01-01T01:00:00Z`

### Exercise 2: Add Data Aggregation
**Task:** Compute aggregates (min, max, average) for historical data.

**Hint:** Store aggregation results for common ranges (hourly, daily).

### Exercise 3: Implement Backup System
**Task:** Periodically backup stored data to a file.

**Hint:** Use JSON serialization to save/load data.

### Exercise 4: Add Retention Policy
**Task:** Automatically delete data older than a specified duration.

**Hint:** Use `chrono::Utc::now() - Duration::days(7)` to find cutoff.

## Troubleshooting

### Storage growing unbounded
**Problem:** Memory usage keeps increasing.
**Solution:** Implement retention policy or use persistent database.

### Queries slow with large datasets
**Problem:** Historical queries taking too long.
**Solution:** Add indexes on (sensor_id, timestamp) if using real database.

### Data lost on restart
**Problem:** Data not persisted across restarts.
**Solution:** Use SQLite or similar for persistence beyond program lifetime.

## Key Takeaways

1. **Storage services centralize data** - Single query interface for all clients
2. **Historical analysis enables insights** - Patterns only visible over time
3. **Time-series specific** - Optimize for timestamp ranges
4. **Queryable pattern scales** - One service handles many queries
5. **Retention is important** - Storage grows unbounded without policy

## Next Steps

- **Chapter 7:** Device Management - Configure and control devices
- **Chapter 8:** Troubleshooting - Diagnose system issues
- **Chapter 9:** Production Deployment - Deploy to real systems
