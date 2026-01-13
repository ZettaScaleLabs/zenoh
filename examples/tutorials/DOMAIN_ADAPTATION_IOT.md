# Domain Adaptation Guide: IoT Sensor Networks

This guide shows how to adapt the Smart Building tutorial patterns to large-scale distributed IoT deployments, where hundreds or thousands of heterogeneous sensors send data to processing pipelines.

## Overview

The Smart Building tutorial uses a controlled environment (single building) with organized hierarchies. IoT sensor networks face different challenges: massive scale, geographic distribution, intermittent connectivity, and heterogeneous sensor types. The same Zenoh patterns apply with emphasis on:

- **Efficient hierarchies** for geographic or functional organization
- **Selective subscriptions** to handle high data volumes
- **Query patterns** for on-demand sensor discovery
- **Storage and persistence** for cloud integration

## Quick Mapping

| Smart Building Concept | IoT Application |
|---|---|
| Building zones | Geographic regions or sensor networks |
| Room sensors | Individual IoT sensors (temperature, humidity, soil moisture, etc.) |
| Hierarchical key expressions | Device type / location / measurement |
| Monitoring display | Cloud dashboard or analytics pipeline |
| Query/Reply | Sensor metadata lookup, device configuration |
| Router for scale | Regional gateway or edge hub |
| Persistence | Time-series storage for analytics |

## Architecture Pattern

```
Zenoh Router (Edge Gateway)
├─ region-north
│  ├─ facility-01
│  │  ├─ temp-sensor-01 (Publisher)
│  │  ├─ humidity-sensor-01 (Publisher)
│  │  └─ pressure-sensor-01 (Publisher)
│  └─ facility-02
│     ├─ temp-sensor-01
│     └─ temp-sensor-02
├─ region-south
│  └─ facility-03
│     └─ temp-sensor-01
├─ Cloud-Connector (Subscriber: filtered data)
├─ Analytics-Pipeline (Subscriber: aggregated)
└─ Device-Registry (Queryable: sensor metadata)
```

## Key Expression Hierarchy

Choose hierarchy based on your IoT deployment structure:

### Geographic Hierarchy
```
sensors/region-us-west/city-seattle/facility-123/device-abc/temperature
sensors/region-us-west/city-seattle/facility-123/device-abc/humidity
sensors/region-eu-east/city-prague/facility-456/device-def/temperature
```

### Functional Hierarchy
```
sensors/device-type-thermometer/geographic-zone-1/building-a/floor-3/room-101/temperature
sensors/device-type-hygrometer/geographic-zone-1/building-a/floor-3/room-101/humidity
sensors/device-type-power-meter/building-a/panel-5/current
```

### Time-Series Hierarchy
```
metrics/temperature/2024-01-15/region-west/facility-01/device-sensor-01
metrics/humidity/2024-01-15/region-west/facility-01/device-sensor-01
alerts/high-temperature/2024-01-15T14-32-10Z/region-west/facility-01
```

## Channel Organization

For massive sensor networks, organize into channels to balance workload:

```rust
// Instead of one global subscriber
// Use multiple subscriptions for different sensor types
let temp_sub = session.declare_subscriber("sensors/*/*/*/temperature").res().await?;
let humidity_sub = session.declare_subscriber("sensors/*/*/*/humidity").res().await?;
let power_sub = session.declare_subscriber("sensors/*/power/**").res().await?;

// Or geographic channels
let region_north = session.declare_subscriber("sensors/region-north/**").res().await?;
let region_south = session.declare_subscriber("sensors/region-south/**").res().await?;
```

## Chapter Adaptation Examples

### Chapter 1: Hello Zenoh → Simple IoT Sensor

**Original:** Room temperature sensor
**Adapted:** Weather station with multiple sensor types

```rust
use zenoh::prelude::*;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let device_id = "ws-001";  // Weather station 001
    let region = "us-west";
    let facility = "seattle-downtown";
    
    let session = zenoh::open(Default::default()).res().await.unwrap();
    
    // Publish multiple sensor types from same device
    let temp_pub = session
        .declare_publisher(format!("sensors/{}/{}//{}//temperature", region, facility, device_id))
        .res()
        .await
        .unwrap();
    
    let humidity_pub = session
        .declare_publisher(format!("sensors/{}/{}//{}//humidity", region, facility, device_id))
        .res()
        .await
        .unwrap();
    
    let mut counter = 0;
    loop {
        let temp = 20.0 + (counter as f32 * 0.1).sin();
        let humidity = 50.0 + (counter as f32 * 0.05).cos() * 10.0;
        
        temp_pub.put(format!("{:.2}", temp)).res().await.ok();
        humidity_pub.put(format!("{:.2}", humidity)).res().await.ok();
        
        println!("Published: temp={:.2}°C, humidity={:.2}%", temp, humidity);
        
        counter += 1;
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}
```

### Chapter 2: Key Expressions → Sensor Organization

**Key insight:** Design hierarchies for efficient filtering

```rust
// Subscribe to all temperature sensors in a region
let subscriber = session
    .declare_subscriber("sensors/region-us-west/*/*/temperature")
    .res()
    .await
    .unwrap();

// Subscribe to all metrics from a specific facility
let subscriber = session
    .declare_subscriber("sensors/*/facility-downtown-01/**")
    .res()
    .await
    .unwrap();

// Extract metadata from key expression
while let Ok(sample) = subscriber.recv_async().await {
    let parts: Vec<&str> = sample.key_expr().as_str().split('/').collect();
    let region = parts[1];
    let facility = parts[2];
    let device = parts[3];
    let metric = parts[4];
    
    println!("Received from {}/{}/{}: {} = {}", 
        region, facility, device, metric, sample.value());
}
```

### Chapter 3: Query/Reply → Sensor Discovery

**Original:** Thermostat responds to queries
**Adapted:** Each sensor responds with device metadata

```rust
use zenoh::prelude::*;
use serde_json::{json, Value};

#[tokio::main]
async fn main() {
    let device_id = "temp-sensor-001";
    let session = zenoh::open(Default::default()).res().await.unwrap();
    
    // Publish sensor readings
    let pub_key = "sensors/region-us-west/seattle/building-a/temp-sensor-001/temperature";
    let publisher = session
        .declare_publisher(pub_key)
        .res()
        .await
        .unwrap();
    
    // Queryable: respond to metadata requests
    let query_key = "device-registry/region-us-west/seattle/building-a/temp-sensor-001";
    let mut queries = session
        .declare_queryable(query_key)
        .res()
        .await
        .unwrap();
    
    // Spawn publisher task
    let pub_session = session.clone();
    tokio::spawn(async move {
        for i in 0..100 {
            let temp = 20.0 + (i as f32 * 0.1).sin();
            let _ = pub_session
                .put(pub_key, format!("{:.2}", temp))
                .res()
                .await;
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    });
    
    // Handle queries for device metadata
    while let Ok(query) = queries.recv_async().await {
        let metadata = json!({
            "device_id": device_id,
            "type": "temperature_sensor",
            "unit": "celsius",
            "accuracy": 0.5,
            "update_interval_s": 5,
            "firmware_version": "1.2.3",
            "location": {
                "region": "us-west",
                "city": "seattle",
                "building": "a",
                "floor": 3,
                "room": 301
            },
            "capabilities": ["temperature", "diagnostics"],
            "online": true,
            "battery_level_pct": 87
        });
        
        let payload = serde_json::to_string(&metadata).unwrap();
        let _ = query.reply(Ok(Sample::new(query.key_expr().clone(), payload))).res().await;
    }
}
```

### Chapter 4: Router Setup → Edge Gateway

**Original:** Router connects multiple buildings
**Adapted:** Router acts as edge gateway for regional sensors

```bash
# Edge gateway configuration (edge_gateway.json5)
router: {
    storages: [
        {
            key_prefix: "sensors/region-us-west/**",
            volume: "filesystem",
            dir: "/data/sensors/region-west",
            keep_last: 100,  // Keep last 100 samples per sensor
        },
        {
            key_prefix: "alerts/**",
            volume: "memory",
            capacity: 10000,
        }
    ]
}

# Run edge gateway
zenohd -c edge_gateway.json5 --listen 127.0.0.1:7447
```

Sensors connect:
```bash
# Sensor device connects to gateway
ZENOH_CONNECT=127.0.0.1:7447 cargo run --bin iot_sensor

# Cloud connector pulls data from gateway
ZENOH_CONNECT=127.0.0.1:7447 cargo run --bin cloud_connector
```

### Chapter 5: Multi-Tier System → Cloud-Connected IoT

**Original:** Multi-reader storage with aggregation
**Adapted:** Edge aggregation + cloud persistence

**Architecture:**
- **Layer 1 (Sensors):** Individual IoT devices publish raw data
- **Layer 2 (Edge):** Local aggregator processes and filters
- **Layer 3 (Cloud):** Cloud connector persists to time-series database

```rust
// Edge aggregator: filter and aggregate sensor data
#[tokio::main]
async fn main() {
    let session = zenoh::open(Default::default()).res().await.unwrap();
    
    // Subscribe to all temperature sensors
    let mut subscriber = session
        .declare_subscriber("sensors/region-us-west/*/*/temperature")
        .res()
        .await
        .unwrap();
    
    // Publisher for alerts
    let alert_pub = session
        .declare_publisher("alerts/temperature-anomaly")
        .res()
        .await
        .unwrap();
    
    // Aggregate readings
    let mut readings_window = Vec::new();
    let window_size = 10;
    
    while let Ok(sample) = subscriber.recv_async().await {
        let value: f32 = sample.value().to_string().parse().unwrap_or(0.0);
        readings_window.push(value);
        
        if readings_window.len() >= window_size {
            let avg = readings_window.iter().sum::<f32>() / readings_window.len() as f32;
            let max = readings_window.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            
            // Alert if anomaly detected
            if max > 35.0 {
                let alert = format!(
                    "{{\"type\": \"temperature_high\", \"max\": {}, \"avg\": {}, \"device\": \"{}\"}}",
                    max, avg, sample.key_expr()
                );
                alert_pub.put(alert).res().await.ok();
            }
            
            readings_window.clear();
        }
    }
}

// Cloud connector: persist to database
#[tokio::main]
async fn main() {
    let session = zenoh::open(Default::default()).res().await.unwrap();
    
    let mut subscriber = session
        .declare_subscriber("sensors/**")
        .res()
        .await
        .unwrap();
    
    while let Ok(sample) = subscriber.recv_async().await {
        let timestamp = chrono::Utc::now().to_rfc3339();
        let key = sample.key_expr().as_str();
        let value = sample.value().to_string();
        
        // Write to time-series database
        let datapoint = CloudDatapoint {
            timestamp,
            key: key.to_string(),
            value,
        };
        
        write_to_database(datapoint).await.ok();
    }
}
```

## Complete Example: Multi-Facility Weather Network

```rust
use zenoh::prelude::*;
use std::time::Duration;
use rand::Rng;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let region = args.get(1).map(|s| s.as_str()).unwrap_or("us-west");
    let facility_id = args.get(2).map(|s| s.as_str()).unwrap_or("facility-001");
    
    let session = zenoh::open(Default::default()).res().await.unwrap();
    
    let mut rng = rand::thread_rng();
    let mut temp = 20.0;
    let mut humidity = 50.0;
    
    // Publishers for different metrics
    let temp_pub = session
        .declare_publisher(format!("sensors/{}/{}/temperature", region, facility_id))
        .res()
        .await
        .unwrap();
    
    let humidity_pub = session
        .declare_publisher(format!("sensors/{}/{}/humidity", region, facility_id))
        .res()
        .await
        .unwrap();
    
    let pressure_pub = session
        .declare_publisher(format!("sensors/{}/{}/pressure", region, facility_id))
        .res()
        .await
        .unwrap();
    
    // Queryable for device info
    let info_key = format!("device-info/{}/{}", region, facility_id);
    let mut queries = session
        .declare_queryable(&info_key)
        .res()
        .await
        .unwrap();
    
    // Spawn publisher loop
    let pub_session = session.clone();
    tokio::spawn(async move {
        let mut counter = 0;
        loop {
            // Simulate environmental changes
            temp += rng.gen_range(-1.0..1.0);
            temp = temp.max(10.0).min(35.0);
            humidity += rng.gen_range(-5.0..5.0);
            humidity = humidity.max(20.0).min(90.0);
            let pressure = 1013.25 + rng.gen_range(-10.0..10.0);
            
            let _ = temp_pub.put(format!("{:.2}", temp)).res().await;
            let _ = humidity_pub.put(format!("{:.2}", humidity)).res().await;
            let _ = pressure_pub.put(format!("{:.2}", pressure)).res().await;
            
            println!("[{}] T={:.1}°C H={:.0}% P={:.1}hPa", facility_id, temp, humidity, pressure);
            
            counter += 1;
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    });
    
    // Handle metadata queries
    while let Ok(query) = queries.recv_async().await {
        let info = format!(
            r#"{{"facility_id": "{}", "region": "{}", "sensors": ["temperature", "humidity", "pressure"], "status": "online"}}"#,
            facility_id, region
        );
        let _ = query.reply(Ok(Sample::new(query.key_expr().clone(), info))).res().await;
    }
}
```

## Exercises

### Exercise 1: Multi-Facility Network
Run the example with 3 different facilities in same region. Verify each publishes to separate key expressions.

```bash
# Terminal 1-3: Different facilities
cargo run -- us-west facility-001
cargo run -- us-west facility-002
cargo run -- us-west facility-003

# Terminal 4: Subscribe to all
cargo run --example z_sub -- --selector 'sensors/us-west/*/**'
```

### Exercise 2: Selective Filtering
Create a subscriber that only receives temperature alerts (values > 30°C) from any facility.

**Key concepts:**
- Parse JSON payloads
- Conditional alert generation
- Key expression pattern matching

### Exercise 3: Device Registry
Implement a query handler that tracks device availability. Query returns list of all currently online sensors.

**Key concepts:**
- State management across queries
- Filtering active devices (heartbeat timeout)
- JSON array serialization

## Common Patterns

### Hierarchical Filtering
```rust
// Subscribe to specific region only
session.declare_subscriber("sensors/region-us-west/**")

// Subscribe to specific facility
session.declare_subscriber("sensors/*/facility-downtown-01/**")

// Subscribe to temperature from all facilities in region
session.declare_subscriber("sensors/region-us-west/*/*/temperature")
```

### Scaling to Thousands of Sensors
- **Channel organization:** Separate subscriptions per region/facility
- **Buffering:** Use async queues to handle bursty publishing
- **Compression:** Send delta values instead of full readings
- **Sampling:** Reduce frequency for non-critical metrics
- **Storage:** Use TTL and retention policies for historical data

### Edge Computing
```rust
// Edge device: aggregate and filter locally
// Only send if value exceeds threshold
let subscriber = session.declare_subscriber("sensors/local/**").res().await?;

while let Ok(sample) = subscriber.recv_async().await {
    let value: f32 = parse_value(&sample);
    if value > THRESHOLD {
        // Send to cloud only if significant
        cloud_publisher.put(sample).res().await.ok();
    }
}
```

### Time-Series Analytics
```rust
// Group readings into time windows
// Compute aggregates: min, max, average, stddev
let readings = readings_window
    .iter()
    .copied()
    .collect::<Vec<_>>();

let stats = json!({
    "min": readings.iter().cloned().fold(f32::INFINITY, f32::min),
    "max": readings.iter().cloned().fold(f32::NEG_INFINITY, f32::max),
    "avg": readings.iter().sum::<f32>() / readings.len() as f32,
});
```

## Integration with Cloud Platforms

The IoT pattern integrates naturally with cloud time-series databases:

```rust
// Example: Sending to InfluxDB
use influxdb2::Client;

let client = Client::new("http://influxdb:8086", "org", "bucket", "token");

while let Ok(sample) = subscriber.recv_async().await {
    let datapoint = DataPoint::builder("sensor_reading")
        .field("value", sample.value().to_string().parse::<f64>()?)
        .tag("sensor", extract_sensor_id(sample.key_expr()))
        .tag("facility", extract_facility_id(sample.key_expr()))
        .timestamp(Utc::now().timestamp_nanos_u128())
        .build()?;
    
    client.write("bucket", futures::stream::once(async { datapoint })).await?;
}
```

## Next Steps

1. Start with **Chapter 1: Hello Zenoh** using the weather station example
2. Modify **Chapter 2: Key Expressions** to design your facility hierarchy
3. Extend **Chapter 3: Query/Reply** for device registry/discovery
4. Build **Chapter 4: Router Setup** for edge gateway
5. Design **Chapter 5: Multi-Tier** with cloud integration

## References

- [Smart Building Tutorial](./smart_building/README.md)
- [Chapter 2: Key Expressions](./smart_building/chapter_2/README.md)
- [Chapter 3: Query/Reply](./smart_building/chapter_3/README.md)
- [Zenoh Storage Documentation](https://zenoh.io/docs/apis/storage/)
