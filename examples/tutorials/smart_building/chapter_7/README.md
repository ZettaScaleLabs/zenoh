# Chapter 7: Device Management & Configuration

**Time**: 25 minutes | **Level**: Advanced | **Concepts**: Device Control, Configuration Management, Remote Updates

## Overview

Manage and configure distributed devices via Zenoh. Learn about:

- Remote configuration updates
- Device control commands
- State synchronization
- Configuration versioning
- Device grouping and targeting

## Key Patterns

### Configuration Service

```rust
// Devices query for their configuration
let config_query = session.get("config/device/*/current").await.unwrap();

// Config service provides configuration
let mut config_queryable = session
    .declare_queryable("config/device/*/current")
    .await
    .unwrap();
```

### Command Execution

```rust
// Control center publishes commands
let cmd_pub = session.declare_publisher("commands/device/*").await.unwrap();
cmd_pub.put(json!({"action": "reboot"}).to_string()).await.unwrap();

// Devices execute commands
let mut cmd_sub = session
    .declare_subscriber("commands/device/*")
    .await
    .unwrap();
```

## Architecture

```
┌──────────────────────┐
│  Control Center      │
│  (Publish Commands)  │
└──────────┬───────────┘
           │ commands/device/*
    ┌──────▼──────────────────────┐
    │   Device Management Tier    │
    │ • Config Distribution       │
    │ • Command Routing           │
    │ • State Aggregation         │
    └──────┬──────────────────────┘
           │ config/device/*/current
    ┌──────▼──────────────────────┐
    │    Device Tier              │
    │ Device 1  Device 2  Device 3│
    │ (Query Config, Execute Cmds)│
    └─────────────────────────────┘
```

## Basic Implementation

### Device Configuration Management

```rust
// Device requests current configuration
#[tokio::main]
async fn main() {
    let session = zenoh::open(Config::default()).await.unwrap();
    
    // Query config for this device
    let device_id = std::env::args().nth(1).unwrap_or("1".to_string());
    
    loop {
        match session.get(&format!("config/device/{}/current", device_id)).await {
            Ok(mut results) => {
                while let Ok(result) = results.recv_async().await {
                    if let Ok(sample) = result.sample {
                        println!("Received config: {}", sample.payload().try_to_string().unwrap_or_default());
                    }
                }
            }
            Err(e) => println!("Config query failed: {}", e),
        }
        
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}
```

### Configuration Distribution

```rust
// Management service distributes configurations
let mut query = session.declare_queryable("config/device/*/current").await.unwrap();

loop {
    if let Ok(q) = query.recv_async().await {
        let device_id = q.selector().split('/').nth(2).unwrap_or("unknown");
        
        let config = json!({
            "polling_interval_sec": 10,
            "max_buffer_size": 1000,
            "log_level": "info"
        });
        
        q.reply(q.key_expr().clone(), config.to_string()).await.unwrap();
    }
}
```

## Exercises

### Exercise 1: Implement Configuration Versioning
Add version tracking to configurations for rollback capability.

### Exercise 2: Add Device Groups
Support targeting commands to groups of devices (e.g., "floor1_sensors").

### Exercise 3: State Reporting
Devices report their current state and config version back to management.

### Exercise 4: Config Rollback
Implement ability to revert devices to previous configurations.

## Next Steps

- **Chapter 8:** Troubleshooting - Diagnose and fix issues
- **Chapter 9:** Production Deployment - Deploy to real infrastructure
