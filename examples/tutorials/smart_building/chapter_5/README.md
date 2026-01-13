# Chapter 5: Multi-Tier Architecture & Observability

**Time**: 30 minutes | **Level**: Intermediate | **Concepts**: Distributed Systems, Observability, Monitoring

## Overview

In this chapter, you'll learn how to build a production-ready multi-tier Zenoh system with comprehensive observability. You'll understand:

- Building scalable multi-tier architectures
- Implementing observability and monitoring
- Coordinating multiple services across tiers
- Handling service discovery and health checks
- Collecting and analyzing metrics
- Creating dashboards and alerts

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    PRESENTATION TIER                    │
│         (Dashboards, Web UIs, Mobile Apps)              │
│                                                          │
│  ┌──────────────────────────────────────────────────┐   │
│  │   Dashboard Service (Queries & Displays Data)    │   │
│  └──────────────────┬───────────────────────────────┘   │
└─────────────────────┼──────────────────────────────────┘
                      │
        ┌─────────────┼─────────────┐
        │             │             │
┌───────▼───┐  ┌──────▼───┐  ┌─────▼──────┐
│   Query   │  │ Metrics  │  │   Health   │
│  Channel  │  │ Channel  │  │  Channel   │
└───────┬───┘  └──────┬───┘  └─────┬──────┘
        │             │             │
┌───────▼─────────────▼─────────────▼──────────────────┐
│            APPLICATION/SERVICE TIER                   │
│  (Business Logic, Aggregation, Coordination)          │
│                                                       │
│  ┌──────────────┐  ┌─────────────┐  ┌──────────────┐ │
│  │ Status Service│  │Aggregator   │  │Health Monitor│ │
│  │ (Queryable)  │  │(Publisher)  │  │ (Monitor)    │ │
│  └──────────────┘  └─────────────┘  └──────────────┘ │
└───────┬──────────────────┬──────────────────┬────────┘
        │                  │                  │
        │ Status Requests  │ Metric Data      │ Health Checks
        │                  │                  │
┌───────▼──────────────────▼──────────────────▼────────┐
│              SENSOR/DATA TIER                         │
│  (Raw Data Collection, Edge Devices)                 │
│                                                      │
│  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐       │
│  │Sens 1│ │Sens 2│ │Sens 3│ │Sens 4│ │Sens 5│  ...  │
│  └──────┘ └──────┘ └──────┘ └──────┘ └──────┘       │
└──────────────────────────────────────────────────────┘
```

## Key Concepts

### Multi-Tier Architecture

A **multi-tier architecture** separates concerns into distinct layers:

- **Sensor Tier**: Raw data collection from devices
- **Service Tier**: Business logic, aggregation, coordination
- **Presentation Tier**: User-facing interfaces and dashboards

```rust
// Data flows from sensors through services to presentation
// Each tier can scale independently
// Services communicate via Zenoh pub/sub and queries
```

### Observability

**Observability** means you can understand system behavior from external outputs:

- **Metrics**: Numerical measurements (temperature, count, latency)
- **Logs**: Detailed event records with context
- **Traces**: Request paths through system components

```rust
// Example: Publishing observability data alongside business data
publisher.put("sensor/temp").await.unwrap();  // Business data
publisher.put("metrics/temp/samples").await.unwrap();  // Metric
publisher.put("health/sensor").await.unwrap();  // Health indicator
```

### Service Discovery

**Service discovery** automatically detects and locates services:

```rust
// Services announce their availability
publisher.put("services/status_service/available").await.unwrap();

// Clients discover available services via key expressions
subscriber.declare_subscriber("services/*/available").await.unwrap();
```

### Health Checks

**Health checks** monitor service availability and status:

```rust
// Service publishes health status periodically
let health_status = json!({
    "service": "status_service",
    "status": "healthy",
    "timestamp": now(),
    "uptime_ms": 45000
});
publisher.put("health/services/status_service").await.unwrap();
```

## Step-by-Step Implementation

### Step 1: Create Chapter 5 Project Structure

```bash
cd chapter_5
cargo init --name smart-building-ch5
cd smart-building-ch5
```

Edit `Cargo.toml`:

```toml
[package]
name = "smart-building-ch5"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"

[workspace]

[[bin]]
name = "sensor_network"
path = "src/bin/sensor_network.rs"

[[bin]]
name = "aggregator"
path = "src/bin/aggregator.rs"

[[bin]]
name = "dashboard"
path = "src/bin/dashboard.rs"

[[bin]]
name = "health_monitor"
path = "src/bin/health_monitor.rs"

[dependencies]
zenoh = { version = "1.7.2", features = ["default"] }
tokio = { version = "1", features = ["full"] }
serde_json = "1.0"
chrono = "0.4"
```

### Step 2: Sensor Network Publisher

Create `src/bin/sensor_network.rs`:

```rust
use std::time::Duration;
use zenoh::config::Config;
use serde_json::json;

#[tokio::main]
async fn main() {
    env_logger::init();

    println!("Opening Zenoh session for sensor network...");
    let session = zenoh::open(Config::default()).await.unwrap();

    // Create publishers for different data channels
    let sensor_pub = session
        .declare_publisher("sensors/data/*")
        .await
        .unwrap();
    let metrics_pub = session
        .declare_publisher("metrics/sensors/*")
        .await
        .unwrap();
    let health_pub = session
        .declare_publisher("health/sensors/*")
        .await
        .unwrap();

    println!("Sensor network started. Publishing 5 sensors...\n");

    let mut readings = 0;

    loop {
        // Simulate 5 sensors
        for sensor_id in 1..=5 {
            let temp = 20.0 + (sensor_id as f32 * 2.0) + (rand::random::<f32>() - 0.5) * 2.0;
            let humidity = 40.0 + (rand::random::<f32>() - 0.5) * 20.0;

            // Publish sensor data
            let sensor_data = json!({
                "sensor_id": sensor_id,
                "temperature": format!("{:.1}", temp),
                "humidity": format!("{:.0}", humidity),
                "timestamp": chrono::Utc::now().to_rfc3339()
            });
            sensor_pub.put(sensor_data.to_string()).await.unwrap();

            // Publish metrics
            let metrics = json!({
                "sensor_id": sensor_id,
                "samples": readings * 5 + sensor_id,
                "latency_ms": 5 + (rand::random::<u32>() % 10)
            });
            metrics_pub.put(metrics.to_string()).await.unwrap();

            // Publish health status
            let health = json!({
                "sensor_id": sensor_id,
                "status": if rand::random::<f32>() > 0.95 { "degraded" } else { "healthy" },
                "uptime_sec": readings * 5 + sensor_id
            });
            health_pub.put(health.to_string()).await.unwrap();

            println!(
                "[Sensor {}] Temp: {:.1}°C, Humidity: {:.0}%, Status: {}",
                sensor_id,
                temp,
                humidity,
                health["status"]
            );
        }

        readings += 1;
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
```

### Step 3: Aggregator Service

Create `src/bin/aggregator.rs`:

```rust
use zenoh::config::Config;
use serde_json::json;

#[tokio::main]
async fn main() {
    env_logger::init();

    println!("Opening Zenoh session for aggregator...");
    let session = zenoh::open(Config::default()).await.unwrap();

    // Subscribe to sensor data
    let mut sensor_sub = session
        .declare_subscriber("sensors/data/*")
        .await
        .unwrap();

    // Declare queryable for aggregated status
    let mut status_queryable = session
        .declare_queryable("services/aggregator/status")
        .await
        .unwrap();

    // Publish aggregated metrics
    let metrics_pub = session
        .declare_publisher("metrics/aggregated")
        .await
        .unwrap();

    println!("Aggregator service started. Waiting for sensor data...\n");

    let mut sensor_count = 0;
    let mut total_temp = 0.0;
    let mut total_humidity = 0.0;

    // Spawn query handler
    let session_clone = session.clone();
    tokio::spawn(async move {
        loop {
            if let Ok(query) = status_queryable.recv_async().await {
                let status = json!({
                    "service": "aggregator",
                    "status": "running",
                    "sensors_active": sensor_count,
                    "last_update": chrono::Utc::now().to_rfc3339()
                });

                query
                    .reply(query.key_expr().clone(), status.to_string())
                    .await
                    .unwrap();
            }
        }
    });

    // Process sensor data
    loop {
        if let Ok(sample) = sensor_sub.recv_async().await {
            if let Ok(data_str) = sample.payload().try_to_string() {
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&data_str) {
                    sensor_count = (sensor_count + 1) % 100;

                    // Parse and aggregate
                    if let (Some(temp), Some(humidity)) = (data["temperature"].as_str(), data["humidity"].as_str())
                    {
                        if let (Ok(t), Ok(h)) = (temp.parse::<f32>(), humidity.parse::<f32>()) {
                            total_temp += t;
                            total_humidity += h;

                            // Publish aggregated metric every 5 readings
                            if sensor_count % 5 == 0 {
                                let avg = json!({
                                    "avg_temperature": format!("{:.1}", total_temp / 5.0),
                                    "avg_humidity": format!("{:.0}", total_humidity / 5.0),
                                    "sample_count": sensor_count
                                });
                                metrics_pub.put(avg.to_string()).await.unwrap();

                                println!("[Aggregator] Updated metrics - Avg Temp: {:.1}°C, Avg Humidity: {:.0}%",
                                    total_temp / 5.0, total_humidity / 5.0);

                                total_temp = 0.0;
                                total_humidity = 0.0;
                            }
                        }
                    }
                }
            }
        }
    }
}
```

### Step 4: Dashboard/Monitoring Service

Create `src/bin/dashboard.rs`:

```rust
use zenoh::config::Config;

#[tokio::main]
async fn main() {
    env_logger::init();

    println!("Opening Zenoh session for dashboard...");
    let session = zenoh::open(Config::default()).await.unwrap();

    // Subscribe to metrics
    let mut metrics_sub = session
        .declare_subscriber("metrics/**")
        .await
        .unwrap();

    // Subscribe to health data
    let mut health_sub = session
        .declare_subscriber("health/**")
        .await
        .unwrap();

    println!("Dashboard started. Displaying data...\n");

    // Spawn metrics display task
    let session_clone = session.clone();
    tokio::spawn(async move {
        loop {
            if let Ok(sample) = health_sub.recv_async().await {
                if let Ok(data) = sample.payload().try_to_string() {
                    println!("[Dashboard] Health Update: {}", data);
                }
            }
        }
    });

    // Display metrics
    loop {
        if let Ok(sample) = metrics_sub.recv_async().await {
            if let Ok(data) = sample.payload().try_to_string() {
                println!("[Dashboard] Metric: {}", data);
            }
        }
    }
}
```

### Step 5: Health Monitor Service

Create `src/bin/health_monitor.rs`:

```rust
use zenoh::config::Config;
use serde_json::json;
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    env_logger::init();

    println!("Opening Zenoh session for health monitor...");
    let session = zenoh::open(Config::default()).await.unwrap();

    // Subscribe to all health updates
    let mut health_sub = session
        .declare_subscriber("health/**")
        .await
        .unwrap();

    // Publish alerts
    let alert_pub = session
        .declare_publisher("alerts/health")
        .await
        .unwrap();

    println!("Health Monitor started. Monitoring system health...\n");

    let mut sensor_states: HashMap<i32, String> = HashMap::new();

    loop {
        if let Ok(sample) = health_sub.recv_async().await {
            if let Ok(health_str) = sample.payload().try_to_string() {
                if let Ok(health) = serde_json::from_str::<serde_json::Value>(&health_str) {
                    if let Some(sensor_id) = health["sensor_id"].as_i64() {
                        let status = health["status"].as_str().unwrap_or("unknown");
                        let prev_status = sensor_states.get(&(sensor_id as i32)).cloned();

                        // Check for status changes
                        if prev_status != Some(status.to_string()) {
                            if status == "degraded" || status == "unhealthy" {
                                // Alert on degradation
                                let alert = json!({
                                    "level": "warning",
                                    "sensor_id": sensor_id,
                                    "status": status,
                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                });
                                alert_pub.put(alert.to_string()).await.unwrap();

                                println!(
                                    "⚠️  [Alert] Sensor {} degraded to: {}",
                                    sensor_id, status
                                );
                            } else {
                                println!("✓ [Monitor] Sensor {} recovered to: {}", sensor_id, status);
                            }

                            sensor_states.insert(sensor_id as i32, status.to_string());
                        }
                    }
                }
            }
        }
    }
}
```

## Compilation and Running

```bash
cd chapter_5
cargo build --release

# Terminal 1: Start sensor network
./target/release/sensor_network

# Terminal 2: Start aggregator
./target/release/aggregator

# Terminal 3: Start dashboard
./target/release/dashboard

# Terminal 4: Start health monitor (optional)
./target/release/health_monitor
```

## Expected Output

### Sensor Network:
```
Opening Zenoh session for sensor network...
Sensor network started. Publishing 5 sensors...

[Sensor 1] Temp: 22.1°C, Humidity: 50%, Status: "healthy"
[Sensor 2] Temp: 24.2°C, Humidity: 45%, Status: "healthy"
[Sensor 3] Temp: 26.3°C, Humidity: 42%, Status: "degraded"
[Sensor 4] Temp: 28.0°C, Humidity: 55%, Status: "healthy"
[Sensor 5] Temp: 30.1°C, Humidity: 48%, Status: "healthy"
```

### Aggregator:
```
Opening Zenoh session for aggregator...
Aggregator service started. Waiting for sensor data...

[Aggregator] Updated metrics - Avg Temp: 26.1°C, Avg Humidity: 48%
[Aggregator] Updated metrics - Avg Temp: 25.8°C, Avg Humidity: 49%
```

### Dashboard:
```
Opening Zenoh session for dashboard...
Dashboard started. Displaying data...

[Dashboard] Metric: {"sensor_id":1,"samples":1,"latency_ms":8}
[Dashboard] Health Update: {"sensor_id":1,"status":"healthy","uptime_sec":1}
```

## Exercises

### Exercise 1: Add Service Discovery
**Task:** Implement service discovery where services announce their availability and capabilities.

**Hint:** Services should publish to `services/<service_name>/available` with their capabilities (e.g., supported queries).

**Expected Result:** Dashboard can query `services/*/available` to find available services.

### Exercise 2: Implement Alerting Thresholds
**Task:** Add temperature/humidity thresholds and alert when exceeded.

**Hint:** Add configuration at program start:
```rust
const TEMP_HIGH: f32 = 30.0;
const TEMP_LOW: f32 = 15.0;
const HUMIDITY_HIGH: f32 = 70.0;
```

### Exercise 3: Add Service-to-Service Communication
**Task:** Implement service queryables so services can ask each other questions.

**Hint:** Aggregator can query health status of sensors:
```rust
let query = session.get("health/sensors/*").await.unwrap();
```

### Exercise 4: Create System Status Dashboard
**Task:** Build a service that shows overall system health.

**Hint:** Query all services for their status and compute overall health:
```rust
let healthy = active_sensors / total_sensors;
let status = if healthy > 0.8 { "healthy" } else { "degraded" };
```

## Troubleshooting

### Services not communicating
**Problem:** Aggregator can't subscribe to sensor data.
**Solution:** Ensure sensor network is running first; check key expressions match.

### No data appearing in dashboard
**Problem:** Dashboard is running but receiving no data.
**Solution:** Verify sensors are publishing to `metrics/**` and `health/**`.

### High latency between tiers
**Problem:** Data takes too long to flow through system.
**Solution:** Check that services don't buffer data; use `async` methods properly.

### Memory usage growing
**Problem:** Dashboards or aggregators consuming memory over time.
**Solution:** Ensure subscriptions have reasonable key expressions and don't collect unbounded history.

## Key Takeaways

1. **Multi-tier architectures scale better** - Each tier can be deployed independently
2. **Observability is built-in** - Zenoh's pub/sub makes it easy to monitor everything
3. **Service discovery simplifies operations** - Systems can find each other automatically
4. **Health checks enable resilience** - Detect and respond to failures early
5. **Aggregation reduces traffic** - Raw sensor data can be filtered and summarized

## Next Steps

- **Chapter 6:** Storage & Persistence - Add data storage and retrieval
- **Chapter 7:** Device Management - Add configuration and remote control
- **Chapter 8:** Troubleshooting - Diagnose and fix system issues
- **Chapter 9:** Production Deployment - Deploy to real infrastructure
