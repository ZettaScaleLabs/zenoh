# Domain Adaptation Guide: Manufacturing Systems

This guide shows how to adapt the Smart Building tutorial patterns to industrial manufacturing and Industry 4.0 scenarios, where machines report telemetry, quality metrics, and production status across factory floors.

## Overview

Manufacturing systems require coordination similar to Smart Buildings, but with emphasis on:

- **Real-time production tracking** (machine state, throughput, quality)
- **Predictive maintenance** (monitoring equipment health, detecting anomalies)
- **Traceability** (tracking products through multiple stages)
- **Safety and compliance** (emergency stops, safety interlocks)

The Zenoh hierarchy organizes naturally: plant → production line → machine → subsystem (spindle, conveyor, etc.). The same Zenoh patterns handle equipment coordination, quality monitoring, and production dashboards.

## Quick Mapping

| Smart Building Concept | Manufacturing Application |
|---|---|
| Building with zones | Factory with production lines |
| Room sensors | Machine sensors (RPM, temperature, vibration) |
| Thermostat control | Machine state commands (start/stop/pause) |
| Monitoring display | Production dashboard (OEE, throughput) |
| Multi-reader queryable | Machine specification and maintenance schedule |
| Router for scale | Factory MES (Manufacturing Execution System) |
| Persistence | Production history and audit trail |

## Architecture Pattern

```
MES (Manufacturing Execution System) - Zenoh Router
├─ Factory-Plant-A
│  ├─ Production-Line-1
│  │  ├─ Machine-001 (CNC Mill)
│  │  │  ├─ spindle_rpm (telemetry)
│  │  │  ├─ tool_temperature (telemetry)
│  │  │  ├─ vibration (telemetry/alerts)
│  │  │  └─ state (online/offline/error)
│  │  ├─ Machine-002 (Hydraulic Press)
│  │  └─ Machine-003 (Assembly Robot)
│  ├─ Production-Line-2
│  │  └─ ... more machines
│  └─ Quality-Inspection (Queryable: product results)
├─ Production-Dashboard (Subscriber: metrics)
├─ Predictive-Maintenance (Subscriber: anomalies)
├─ Supply-Chain (Querier: inventory status)
└─ ERP-System (Persistence: production history)
```

## Key Expression Hierarchy

```
# By line and machine
manufacturing/plant-a/line-1/machine-001/telemetry/spindle_rpm
manufacturing/plant-a/line-1/machine-001/telemetry/temperature
manufacturing/plant-a/line-1/machine-001/health/vibration_alert
manufacturing/plant-a/line-1/machine-002/state
manufacturing/plant-a/line-2/machine-005/telemetry/**

# Alternative: By machine type
manufacturing/cnc/plant-a/line-1/machine-001/telemetry/spindle_rpm
manufacturing/robot/plant-b/line-3/machine-007/telemetry/joint_temp
manufacturing/conveyor/plant-a/line-1/segment-02/speed

# Product tracking
manufacturing/production/order-12345/stage-1/start-time
manufacturing/production/order-12345/stage-2/end-time
manufacturing/production/order-12345/quality-check/pass
```

## Chapter Adaptation Examples

### Chapter 1: Hello Zenoh → Machine Telemetry Publisher

**Original:** Room temperature sensor
**Adapted:** CNC machine spindle RPM monitor

```rust
use zenoh::prelude::*;
use std::time::Duration;
use rand::Rng;

#[tokio::main]
async fn main() {
    let plant = "plant-a";
    let line = "line-1";
    let machine = "machine-001";  // CNC Mill
    
    let session = zenoh::open(Default::default()).res().await.unwrap();
    
    // Publisher for spindle RPM
    let rpm_pub = session
        .declare_publisher(format!(
            "manufacturing/{}/{}/{}/telemetry/spindle_rpm",
            plant, line, machine
        ))
        .res()
        .await
        .unwrap();
    
    // Publisher for tool temperature
    let temp_pub = session
        .declare_publisher(format!(
            "manufacturing/{}/{}/{}/telemetry/tool_temperature",
            plant, line, machine
        ))
        .res()
        .await
        .unwrap();
    
    let mut rng = rand::thread_rng();
    let mut rpm = 0.0;
    let mut temp = 20.0;
    
    loop {
        // Simulate machine operation
        rpm = if rng.gen::<f32>() < 0.3 {
            0.0  // Machine idle 30% of time
        } else {
            rng.gen_range(500.0..8000.0)
        };
        
        temp = 20.0 + (rpm / 1000.0) * 50.0 + rng.gen_range(-5.0..5.0);
        
        rpm_pub.put(format!("{:.0}", rpm)).res().await.ok();
        temp_pub.put(format!("{:.1}", temp)).res().await.ok();
        
        println!("[{}] RPM={:.0}, Temp={:.1}°C", machine, rpm, temp);
        
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
```

### Chapter 2: Key Expressions → Factory Organization

**Key insight:** Design hierarchies for natural factory structure

```rust
// Subscribe to all machine telemetry on line 1
let line_sub = session
    .declare_subscriber("manufacturing/plant-a/line-1/*/telemetry/**")
    .res()
    .await
    .unwrap();

// Subscribe to machine alerts only
let alert_sub = session
    .declare_subscriber("manufacturing/plant-a/*/*/health/alerts")
    .res()
    .await
    .unwrap();

// Subscribe to production stage changes
let stage_sub = session
    .declare_subscriber("manufacturing/production/*/stage-*/end-time")
    .res()
    .await
    .unwrap();

while let Ok(sample) = line_sub.recv_async().await {
    let parts: Vec<&str> = sample.key_expr().as_str().split('/').collect();
    let plant = parts[1];
    let line = parts[2];
    let machine = parts[3];
    let metric = parts[5];
    
    println!("{}:{} [{}]: {} = {}", plant, line, machine, metric, sample.value());
}
```

### Chapter 3: Query/Reply → Machine Status & Specification

**Original:** Thermostat responds with status
**Adapted:** Each machine responds with current status and specification

```rust
use zenoh::prelude::*;
use serde_json::{json, Value};
use std::time::Duration;

#[tokio::main]
async fn main() {
    let machine_id = "machine-001";
    let machine_type = "CNC Mill";
    
    let session = zenoh::open(Default::default()).res().await.unwrap();
    
    // Publisher: machine telemetry
    let rpm_pub = session
        .declare_publisher("manufacturing/plant-a/line-1/machine-001/telemetry/spindle_rpm")
        .res()
        .await
        .unwrap();
    
    // Queryable: respond to status requests
    let query_key = "manufacturing/plant-a/line-1/machine-001/status";
    let mut queries = session
        .declare_queryable(query_key)
        .res()
        .await
        .unwrap();
    
    // Spawn telemetry publisher
    let pub_session = session.clone();
    tokio::spawn(async move {
        let mut rng = rand::thread_rng();
        for i in 0..1000 {
            let rpm = if i % 5 == 0 { 0.0 } else { rng.gen_range(1000.0..8000.0) };
            let _ = rpm_pub.put(format!("{:.0}", rpm)).res().await;
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
    
    // Handle status queries
    while let Ok(query) = queries.recv_async().await {
        let status = json!({
            "machine_id": machine_id,
            "type": machine_type,
            "state": "running",
            "uptime_hours": 12345.5,
            "maintenance_due_hours": 500,
            "error_code": null,
            "last_error": "none",
            "utilization_pct": 78.5,
            "parts_produced_today": 1247,
            "quality_pass_rate_pct": 99.8,
            "estimated_time_to_failure_hours": 450,
            "specification": {
                "max_rpm": 10000,
                "max_temperature_c": 120,
                "manufacturer": "Haas",
                "model": "UMC-750",
                "year_built": 2018
            }
        });
        
        let payload = serde_json::to_string(&status).unwrap();
        let _ = query.reply(Ok(Sample::new(query.key_expr().clone(), payload))).res().await;
    }
}
```

### Chapter 4: Router Setup → Factory MES

**Original:** Router connects multiple buildings
**Adapted:** MES acts as central hub for multiple production lines

```bash
# MES configuration (mes_config.json5)
router: {
    storages: [
        {
            key_prefix: "manufacturing/*/*/*/telemetry/**",
            volume: "memory",
            capacity: 100000,
            keep_last: 100,  # Keep last 100 readings per machine
        },
        {
            key_prefix: "manufacturing/production/**",
            volume: "filesystem",
            dir: "/data/production",
            keep_last: 1000,  # Keep all production records
        },
        {
            key_prefix: "manufacturing/*/*/*/health/alerts",
            volume: "memory",
            capacity: 10000,
            keep_last: 50,  # Alert history
        }
    ]
}

# Run MES
zenohd -c mes_config.json5 --listen 0.0.0.0:7447
```

Configuration for high-availability MES:
```bash
# Plant-A machines connect to MES
ZENOH_CONNECT=192.168.1.100:7447 cargo run --bin cnc_machine
ZENOH_CONNECT=192.168.1.100:7447 cargo run --bin robot_arm
ZENOH_CONNECT=192.168.1.100:7447 cargo run --bin conveyor

# Production dashboard queries MES
ZENOH_CONNECT=192.168.1.100:7447 cargo run --bin production_dashboard

# Predictive maintenance subscribes to alerts
ZENOH_CONNECT=192.168.1.100:7447 cargo run --bin predictive_maintenance
```

### Chapter 5: Multi-Tier System → Production Management

**Original:** Multi-reader storage with aggregation
**Adapted:** OEE calculation, quality tracking, maintenance

**Architecture:**
- **Layer 1 (Machines):** Individual machines publish telemetry
- **Layer 2 (MES):** Central coordination, aggregation, alarms
- **Layer 3 (Analytics):** OEE calculation, trend analysis, predictive maintenance
- **Layer 4 (ERP):** Production history, inventory, supply chain

```rust
// OEE (Overall Equipment Effectiveness) Calculator
#[tokio::main]
async fn main() {
    let session = zenoh::open(Default::default()).res().await.unwrap();
    
    // Subscribe to all production line activity
    let mut subscriber = session
        .declare_subscriber("manufacturing/plant-a/line-1/*/telemetry/**")
        .res()
        .await
        .unwrap();
    
    let mut metrics = ProductionMetrics::default();
    let start_time = std::time::Instant::now();
    
    while let Ok(sample) = subscriber.recv_async().await {
        let value_str = sample.value().to_string();
        
        match sample.key_expr().as_str() {
            k if k.contains("spindle_rpm") => {
                if let Ok(rpm) = value_str.parse::<f32>() {
                    if rpm > 0.0 {
                        metrics.running_time += 1;
                    }
                }
            },
            k if k.contains("good_parts") => {
                if let Ok(count) = value_str.parse::<u32>() {
                    metrics.good_parts += count;
                }
            },
            k if k.contains("defective_parts") => {
                if let Ok(count) = value_str.parse::<u32>() {
                    metrics.defective_parts += count;
                }
            },
            _ => {}
        }
        
        // Calculate and publish OEE every minute
        if start_time.elapsed().as_secs() % 60 == 0 {
            let availability = metrics.running_time as f32 / 60.0;
            let quality = metrics.good_parts as f32 / (metrics.good_parts + metrics.defective_parts) as f32;
            let oee = availability * quality;
            
            println!("OEE: {:.1}% (Availability: {:.1}%, Quality: {:.1}%)", 
                oee * 100.0, availability * 100.0, quality * 100.0);
        }
    }
}

#[derive(Default)]
struct ProductionMetrics {
    running_time: u32,
    good_parts: u32,
    defective_parts: u32,
}

// Predictive Maintenance: detect anomalies in vibration
#[tokio::main]
async fn main() {
    let session = zenoh::open(Default::default()).res().await.unwrap();
    
    // Subscribe to vibration data from all machines
    let mut subscriber = session
        .declare_subscriber("manufacturing/plant-a/*/*/telemetry/vibration")
        .res()
        .await
        .unwrap();
    
    let alert_pub = session
        .declare_publisher("manufacturing/alerts/maintenance")
        .res()
        .await
        .unwrap();
    
    let mut vibration_history = Vec::new();
    const WINDOW_SIZE: usize = 100;
    const ANOMALY_THRESHOLD: f32 = 3.0;  // 3 std devs
    
    while let Ok(sample) = subscriber.recv_async().await {
        let vibration: f32 = sample.value().to_string().parse().unwrap_or(0.0);
        vibration_history.push(vibration);
        
        if vibration_history.len() > WINDOW_SIZE {
            vibration_history.remove(0);
            
            let mean = vibration_history.iter().sum::<f32>() / vibration_history.len() as f32;
            let variance = vibration_history.iter()
                .map(|v| (v - mean).powi(2))
                .sum::<f32>() / vibration_history.len() as f32;
            let std_dev = variance.sqrt();
            
            // Detect anomaly
            if (vibration - mean).abs() > ANOMALY_THRESHOLD * std_dev {
                let alert = json!({
                    "machine": extract_machine_id(sample.key_expr()),
                    "anomaly_type": "high_vibration",
                    "value": vibration,
                    "expected_range": (mean - 2.0 * std_dev, mean + 2.0 * std_dev),
                    "recommendation": "schedule_maintenance",
                    "timestamp": chrono::Utc::now().to_rfc3339()
                });
                
                let _ = alert_pub.put(serde_json::to_string(&alert).unwrap()).res().await;
            }
        }
    }
}

// Production Dashboard: display real-time metrics
#[tokio::main]
async fn main() {
    let session = zenoh::open(Default::default()).res().await.unwrap();
    
    // Query all machine status
    let statuses = session
        .get("manufacturing/plant-a/*/*/status")
        .res()
        .await
        .unwrap();
    
    println!("=== Production Dashboard ===\n");
    
    while let Ok(reply) = statuses.recv_async().await {
        match reply.sample() {
            Ok(sample) => {
                if let Ok(status) = serde_json::from_str::<serde_json::Value>(&sample.value().to_string()) {
                    let machine = extract_machine_id(sample.key_expr());
                    let state = status["state"].as_str().unwrap_or("unknown");
                    let utilization = status["utilization_pct"].as_f64().unwrap_or(0.0);
                    let quality = status["quality_pass_rate_pct"].as_f64().unwrap_or(0.0);
                    
                    println!("{}: {} | Util: {:.1}% | Quality: {:.1}%", 
                        machine, state, utilization, quality);
                }
            },
            Err(_) => {}
        }
    }
}

use serde_json::json;

fn extract_machine_id(key_expr: &str) -> &str {
    key_expr.split('/').nth(3).unwrap_or("unknown")
}
```

## Complete Example: Production Line Simulator

```rust
use zenoh::prelude::*;
use std::time::Duration;
use rand::Rng;
use serde_json::json;

#[tokio::main]
async fn main() {
    let plant = std::env::args().nth(1).unwrap_or_else(|| "plant-a".into());
    let line = std::env::args().nth(2).unwrap_or_else(|| "line-1".into());
    let machine = std::env::args().nth(3).unwrap_or_else(|| "machine-001".into());
    
    let session = zenoh::open(Default::default()).res().await.unwrap();
    
    let base_path = format!("manufacturing/{}/{}/{}", plant, line, machine);
    
    // Telemetry publishers
    let rpm_pub = session.declare_publisher(format!("{}/telemetry/spindle_rpm", base_path)).res().await.unwrap();
    let temp_pub = session.declare_publisher(format!("{}/telemetry/tool_temperature", base_path)).res().await.unwrap();
    let vibration_pub = session.declare_publisher(format!("{}/telemetry/vibration", base_path)).res().await.unwrap();
    let parts_pub = session.declare_publisher(format!("{}/production/parts_count", base_path)).res().await.unwrap();
    
    // Status queryable
    let query_path = format!("{}/status", base_path);
    let mut queries = session.declare_queryable(&query_path).res().await.unwrap();
    
    // Spawn telemetry publisher
    let tel_session = session.clone();
    tokio::spawn(async move {
        let mut rng = rand::thread_rng();
        let mut rpm = 0.0;
        let mut temp = 20.0;
        let mut parts_made = 0;
        
        loop {
            // Simulate machine operation (idle 30% of time)
            if rng.gen::<f32>() > 0.3 {
                rpm = rng.gen_range(1000.0..8000.0);
                temp = 20.0 + (rpm / 1000.0) * 50.0 + rng.gen_range(-10.0..10.0);
                let vibration = (rpm / 1000.0) + rng.gen_range(-0.5..0.5);
                
                let _ = rpm_pub.put(format!("{:.0}", rpm)).res().await;
                let _ = temp_pub.put(format!("{:.1}", temp)).res().await;
                let _ = vibration_pub.put(format!("{:.2}", vibration)).res().await;
                
                // Increment parts made
                if rng.gen::<f32>() < 0.8 {  // 80% pass rate
                    parts_made += 1;
                }
                let _ = parts_pub.put(format!("{}", parts_made)).res().await;
            } else {
                rpm = 0.0;
                temp = 20.0;
                let _ = rpm_pub.put("0").res().await;
                let _ = temp_pub.put("20.0").res().await;
                let _ = vibration_pub.put("0.0").res().await;
            }
            
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });
    
    // Handle status queries
    while let Ok(query) = queries.recv_async().await {
        let status = json!({
            "state": "running",
            "uptime_hours": 12345,
            "maintenance_due_hours": 500,
            "quality_pass_rate_pct": 99.2,
            "parts_produced_today": 1247
        });
        
        let _ = query.reply(Ok(Sample::new(
            query.key_expr().clone(),
            serde_json::to_string(&status).unwrap()
        ))).res().await;
    }
}
```

## Exercises

### Exercise 1: Multi-Machine Factory
Run 3 machines (CNC, Robot, Conveyor) on same production line. Subscribe to all and display summary.

```bash
cargo run -- plant-a line-1 machine-001
cargo run -- plant-a line-1 machine-002
cargo run -- plant-a line-1 machine-003

# Terminal 4: Query all status
cargo run --example z_querier -- --selector 'manufacturing/plant-a/line-1/*/status'
```

### Exercise 2: Quality Alert System
Subscribe to good/defective parts, calculate pass rate, alert if drops below 95%.

**Key concepts:**
- Part count aggregation
- Percentage calculations
- Alert thresholds

### Exercise 3: Predictive Maintenance
Track vibration over time, calculate moving average and standard deviation, alert on anomalies.

**Key concepts:**
- Statistical analysis
- Anomaly detection
- Maintenance alerts

## Common Patterns

### Production Tracking
```rust
// Start of production order
publisher.put(json!({
    "order_id": "order-12345",
    "stage": "stage-1",
    "action": "start",
    "timestamp": now()
})).res().await?;

// Completion
publisher.put(json!({
    "order_id": "order-12345",
    "stage": "stage-1",
    "action": "complete",
    "quality_check": "pass",
    "timestamp": now()
})).res().await?;
```

### Safety Interlocks
```rust
// Emergency stop alert
let emergency_stop_pub = session
    .declare_publisher("manufacturing/plant-a/emergency-stop")
    .res()
    .await?;

// All machines subscribe to emergency stop and react
let mut sub = session.declare_subscriber("manufacturing/plant-a/emergency-stop")
    .res()
    .await?;

while let Ok(_) = sub.recv_async().await {
    stop_machine_immediately();
}
```

### Machine Coordination
```rust
// Conveyor waits for CNC to finish
let mut cnc_status = session.declare_subscriber("manufacturing/plant-a/line-1/machine-001/status")
    .res()
    .await?;

while let Ok(sample) = cnc_status.recv_async().await {
    if status.contains("complete") {
        trigger_conveyor_move();
    }
}
```

## Next Steps

1. Start with **Chapter 1: Hello Zenoh** using machine telemetry example
2. Modify **Chapter 2: Key Expressions** to organize by production line
3. Extend **Chapter 3: Query/Reply** for machine status/specification
4. Build **Chapter 4: Router Setup** for factory MES
5. Design **Chapter 5: Multi-Tier** with OEE and maintenance

## References

- [Smart Building Tutorial](./smart_building/README.md)
- [Chapter 2: Key Expressions](./smart_building/chapter_2/README.md)
- [Chapter 3: Query/Reply](./smart_building/chapter_3/README.md)
- [Zenoh Documentation](https://zenoh.io/docs/)
- [OEE (Overall Equipment Effectiveness)](https://en.wikipedia.org/wiki/Overall_equipment_effectiveness)
