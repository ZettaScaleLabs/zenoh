# Domain Adaptation Guide: Robotics Fleet Management

This guide shows how to adapt the Smart Building tutorial patterns to a robotics fleet management system, where multiple mobile robots communicate their status, coordinate tasks, and share sensor data.

## Overview

The Smart Building tutorial teaches Zenoh fundamentals using a hierarchical building structure (zones, floors, rooms). The same patterns apply directly to robot fleets with a natural hierarchy: fleet → teams → individual robots → robot subsystems (actuators, sensors).

## Quick Mapping

| Smart Building Concept | Robotics Application |
|---|---|
| Building with zones | Fleet with teams |
| Room sensors | Robot sensors (position, battery, health) |
| Thermostat control | Robot task assignment |
| Monitoring display | Fleet dashboard |
| Multi-reader queryable | Centralized fleet state query |
| Router for scale | Multi-robot coordination hub |
| Persistence | Robot state history/telemetry logging |

## Architecture Pattern

```
Zenoh Router
├─ Team-1
│  ├─ Robot-01 (Publisher: telemetry, Queryable: status)
│  ├─ Robot-02 (Publisher: telemetry, Queryable: status)
│  └─ Robot-03 (Publisher: telemetry, Queryable: status)
├─ Team-2
│  ├─ Robot-04
│  ├─ Robot-05
│  └─ Robot-06
├─ Fleet-Dashboard (Subscriber: all telemetry, Querier: all status)
└─ Task-Coordinator (Queryable: task assignments)
```

## Key Expression Hierarchy

```
fleet/team-1/robot-01/telemetry/position
fleet/team-1/robot-01/telemetry/battery_level
fleet/team-1/robot-01/telemetry/temperature
fleet/team-1/robot-01/health/error_count
fleet/team-2/robot-04/telemetry/**

# Queries
query: fleet/*/robot-*/status       # All robot status
query: fleet/team-1/*/status        # Team 1 only
```

## Chapter Adaptation Examples

### Chapter 1: Hello Zenoh → Robot Telemetry Publisher

**Original:** Simple room temperature sensor
**Adapted:** Robot position and battery publisher

```rust
use zenoh::prelude::*;
use std::time::Duration;
use rand::Rng;

#[tokio::main]
async fn main() {
    let session = zenoh::open(Default::default()).res().await.unwrap();
    let publisher = session.declare_publisher("fleet/team-1/robot-01/telemetry/position").res().await.unwrap();
    let mut rng = rand::thread_rng();
    
    loop {
        let x = rng.gen_range(0.0..100.0);
        let y = rng.gen_range(0.0..100.0);
        let battery = rng.gen_range(20.0..100.0);
        
        let data = format!(
            "{{\"x\": {:.2}, \"y\": {:.2}, \"battery_pct\": {:.1}}}",
            x, y, battery
        );
        
        publisher.put(data).res().await.unwrap();
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
```

### Chapter 2: Key Expressions → Fleet Organization

**Key insight:** Use fleet/team hierarchy for natural key expression organization

```rust
// Publish to hierarchical paths
let team_path = format!("fleet/team-{}/robot-{}/telemetry/position", team_id, robot_id);
let subscriber = session.declare_subscriber(team_path).res().await.unwrap();

// Use wildcards for fleet-wide queries
session.get("fleet/*/robot-*/health/**").res().await  // All robot health data
session.get("fleet/team-1/*/status/**").res().await   // Team 1 only
```

### Chapter 3: Query/Reply → Robot Status System

**Original:** Thermostat responds to queries
**Adapted:** Each robot responds with detailed status

```rust
// Robot publishes its own status via queryable
let mut queries = session
    .declare_queryable("fleet/team-1/robot-01/status")
    .res()
    .await
    .unwrap();

while let Ok(query) = queries.recv_async().await {
    let status = RobotStatus {
        position: (x, y),
        battery_pct: 85.5,
        motors_ok: true,
        last_task: "navigation",
        error_count: 0,
    };
    
    let payload = serde_json::to_string(&status).unwrap();
    query.reply(Ok(Sample::new(query.key_expr().clone(), payload))).res().await.ok();
}
```

### Chapter 4: Router Setup → Multi-Team Coordination

**Original:** Router connects multiple buildings
**Adapted:** Router connects multiple robot teams

```bash
# In zenohd config (or router_config.json5)
# Enable storage for robot telemetry history
# Enable routing between teams
# Set up security boundaries if needed

# Run router
zenohd --listen 127.0.0.1:7447

# Robot team 1 connects to router
cargo run --bin robot -- --connect 127.0.0.1:7447 --team 1

# Robot team 2 connects to router  
cargo run --bin robot -- --connect 127.0.0.1:7447 --team 2

# Dashboard connects to router
cargo run --bin fleet_dashboard -- --connect 127.0.0.1:7447
```

### Chapter 5: Multi-Tier System → Production Fleet Monitoring

**Original:** Multi-reader storage with aggregation
**Adapted:** Fleet management with task coordination

**Architecture:**
- **Layer 1 (Edge):** Individual robot publishers (telemetry, health)
- **Layer 2 (Teams):** Team aggregators collect and relay
- **Layer 3 (Fleet):** Central coordinator assigns tasks, stores history

```rust
// Team aggregator: collect all robot telemetry
let mut subscriber = session.declare_subscriber("fleet/team-1/robot-*/telemetry/**")
    .res()
    .await
    .unwrap();

while let Ok(sample) = subscriber.recv_async().await {
    let robot_id = extract_robot_id(sample.key_expr());
    let team_status = update_team_state(robot_id, sample.payload());
    
    // Publish aggregated team status
    team_publisher.put(team_status).res().await.ok();
}

// Fleet coordinator: handle task assignments
let mut task_queries = session
    .declare_queryable("fleet/task-assignment/**")
    .res()
    .await
    .unwrap();

while let Ok(query) = task_queries.recv_async().await {
    let robot_id = extract_robot_from_query(&query);
    let task = assign_next_task(robot_id);
    query.reply(Ok(Sample::new(query.key_expr().clone(), task))).res().await.ok();
}
```

## Complete Example: Mini Fleet System

Save as `robot.rs` in chapter_1:

```rust
use zenoh::prelude::*;
use std::time::Duration;
use rand::Rng;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
struct RobotState {
    team_id: u32,
    robot_id: u32,
    x: f32,
    y: f32,
    battery_pct: f32,
}

#[tokio::main]
async fn main() {
    let team_id = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let robot_id = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);

    let state = Arc::new(Mutex::new(RobotState {
        team_id,
        robot_id,
        x: 0.0,
        y: 0.0,
        battery_pct: 100.0,
    }));

    let session = zenoh::open(Default::default()).res().await.unwrap();

    // Publisher: send telemetry
    let pub_state = state.clone();
    let pub_session = session.clone();
    tokio::spawn(async move {
        let publisher = pub_session
            .declare_publisher(format!("fleet/team-{}/robot-{}/telemetry/position",
                pub_state.lock().await.team_id, pub_state.lock().await.robot_id))
            .res()
            .await
            .unwrap();

        let mut rng = rand::thread_rng();
        loop {
            let mut s = pub_state.lock().await;
            s.x += rng.gen_range(-5.0..5.0);
            s.y += rng.gen_range(-5.0..5.0);
            s.battery_pct -= rng.gen_range(0.1..0.5);

            let data = format!(
                "{{\"x\": {:.2}, \"y\": {:.2}, \"battery_pct\": {:.1}}}",
                s.x, s.y, s.battery_pct
            );
            publisher.put(data).res().await.ok();
            drop(s);
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });

    // Queryable: respond to status queries
    let qa_state = state.clone();
    let qa_session = session.clone();
    tokio::spawn(async move {
        let key_expr = format!("fleet/team-{}/robot-{}/status",
            qa_state.lock().await.team_id, qa_state.lock().await.robot_id);
        let mut queries = qa_session
            .declare_queryable(&key_expr)
            .res()
            .await
            .unwrap();

        while let Ok(query) = queries.recv_async().await {
            let s = qa_state.lock().await;
            let status = format!(
                "{{\"x\": {:.2}, \"y\": {:.2}, \"battery_pct\": {:.1}, \"online\": true}}",
                s.x, s.y, s.battery_pct
            );
            let _ = query.reply(Ok(Sample::new(query.key_expr().clone(), status))).res().await;
        }
    });

    // Keep running
    tokio::time::sleep(Duration::from_secs(3600)).await;
}
```

## Exercises

### Exercise 1: Multi-Robot Simulation
Extend the example above to run 3 robots simultaneously, each with different team IDs. Verify they publish to separate key expressions.

**Solution approach:**
```bash
# Terminal 1
cargo run --release --bin robot -- team 1 robot 1

# Terminal 2
cargo run --release --bin robot -- team 1 robot 2

# Terminal 3
cargo run --release --bin robot -- team 2 robot 3

# Terminal 4: Query all robot status
cargo run --example z_querier -- --selector 'fleet/*/robot-*/status'
```

### Exercise 2: Team Aggregator
Create a subscriber that listens to `fleet/team-1/robot-*/telemetry/**` and periodically prints a summary of all robots in team 1.

**Key concepts to practice:**
- Wildcard subscriptions
- Extracting robot ID from key expression
- Aggregating JSON payloads

### Exercise 3: Task Coordinator
Implement a queryable that responds to `fleet/task-assignment/robot-01` with task assignments. Create a simple load balancing algorithm.

**Key concepts to practice:**
- Implementing queryables
- State management across queries
- JSON serialization for task data

## Common Patterns

### Hierarchical State Queries
```rust
// Query specific team status
session.get("fleet/team-1/**/status").res().await

// Query specific robot
session.get("fleet/team-1/robot-01/telemetry/**").res().await
```

### Scaling to Many Robots
- **Per-robot publishers:** Keep telemetry local, aggregate at team level
- **Selective subscriptions:** Dashboard subscribes to `fleet/team-*/robot-*/health/critical` only
- **Use storage for history:** Router stores last N samples for telemetry queries
- **Batching:** Aggregate multiple sensor readings before publishing

### Fault Tolerance
```rust
// Publish heartbeat
loop {
    publisher.put(format!("{{ \"alive\": true, \"timestamp\": {} }}", now()))
        .res().await.ok();
    tokio::time::sleep(Duration::from_secs(5)).await;
}

// Subscriber detects robots offline if no heartbeat in 15 seconds
// Use subscriber with liveliness token for automatic cleanup
```

## Next Steps

1. Start with **Chapter 1: Hello Zenoh** using the robot telemetry example above
2. Modify **Chapter 2: Key Expressions** to use fleet/team/robot hierarchy
3. Extend **Chapter 3: Query/Reply** to implement robot status queries
4. Build **Chapter 4: Router Setup** connecting multiple robot teams
5. Design **Chapter 5: Multi-Tier** with fleet coordinator

## References

- [Smart Building Tutorial](./smart_building/README.md) - Original patterns
- [Chapter 1: Hello Zenoh](./smart_building/chapter_1/README.md) - Pub/sub basics
- [Chapter 3: Query/Reply](./smart_building/chapter_3/README.md) - Queryable pattern
- [Zenoh Documentation](https://zenoh.io/docs/)
