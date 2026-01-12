# Zenoh Smart Building Tutorial - Quick Reference Guide

## Running the Tutorials

### Prerequisites
```bash
# Install Rust (if needed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update
```

### Chapter 1: Hello Zenoh (Pub/Sub)
```bash
cd examples/tutorials/smart_building/chapter_1

# Terminal 1: Publisher
cargo run --release --bin room_sensor

# Terminal 2: Subscriber
cargo run --release --bin monitor
```

### Chapter 2: Key Expressions (Hierarchies & Wildcards)
```bash
cd examples/tutorials/smart_building/chapter_2

# Example 1: Single room, multiple sensors
# Terminal 1: Multi-sensor publisher
cargo run --release --bin multi_sensor

# Terminal 2: Wildcard subscriber (all room sensors)
cargo run --release --bin floor_monitor

# Example 2: Multiple rooms
# Terminal 1: Building sensors (3 rooms)
cargo run --release --bin building_sensors

# Terminal 2: Temperature-only monitor
cargo run --release --bin selective_monitor
```

### Chapter 3: Query/Reply Pattern
```bash
cd examples/tutorials/smart_building/chapter_3

# Example 1: Simple request/response
# Terminal 1: Service that responds to queries
cargo run --release --bin room_status_service

# Terminal 2: Client querying for data
cargo run --release --bin dashboard

# Example 2: Multiple services with wildcard queries
# Terminal 1: Service serving multiple rooms
cargo run --release --bin building_status

# Terminal 2: Client querying with wildcards
cargo run --release --bin selective_query
```

### Chapter 4: Zenoh Router (Distributed)
```bash
cd examples/tutorials/smart_building/chapter_4

# Terminal 1: Start Zenoh Router
cargo run --release --bin zenohd -- --config router.json5

# Terminal 2: Start sensor (connects to router)
cargo run --release --bin sensor_with_router

# Terminal 3: Start monitor (connects to router)
cargo run --release --bin monitor_with_router

# OR run automated demo (all in one)
./run_demo.sh
```

## Key Concepts Quick Reference

### Pub/Sub Pattern
```rust
// Publisher
let pub = session.declare_publisher("key/expression").await?;
pub.put(data).await?;

// Subscriber
let mut sub = session.declare_subscriber("key/expression").await?;
while let Ok(sample) = sub.recv_async().await {
    println!("{}", String::from_utf8_lossy(&sample.payload));
}
```

### Key Expression Wildcards
```
building/floor1/room_a/temperature     # Exact match
building/floor1/room_a/*               # Any sensor in room_a
building/floor1/*/temperature          # All temps on floor1
building/**/temperature                # All temps everywhere
building/**                            # Everything under building
```

### Query/Reply Pattern
```rust
// Queryable (Service)
let mut qb = session.declare_queryable("key/expression").await?;
while let Ok(query) = qb.recv_async().await {
    query.reply(Ok(response.into())).await?;
}

// Query (Client)
let results = session.get("key/expression").await?;
while let Ok(reply) = results.recv_async().await {
    match reply.sample {
        Ok(sample) => {
            // Handle response
        }
        Err(e) => {
            // Handle error
        }
    }
}
```

### Router Configuration
```jsonc
// router.json5 - Router listens here
{
    listeners: {
        default: {
            type: "tcp",
            bind: "127.0.0.1:7447",
        }
    }
}

// client_config.json5 - Client connects here
{
    mode: "client",
    connect: {
        endpoints: ["tcp/127.0.0.1:7447"]
    }
}
```

## Data Flow Patterns

### Single Room, Single Sensor
```
Sensor → Zenoh Network → Monitor
```

### Multiple Sensors in One Room
```
Temp Sensor  ┐
Humidity Sensor ├→ Zenoh Network → Floor Monitor (subscribes to *)
Occupancy Sensor┘
```

### Multiple Rooms Through Router
```
Room A Sensor ─┐
Room B Sensor ─┼→ Zenoh Router ←─ Monitor 1
Room C Sensor ─┤                ←─ Monitor 2
               └───────────────────── Dashboard (queries)
```

### Aggregation Pipeline
```
Edge (Sensors)      → Aggregator (process) → Central (storage/query)
Raw readings (1s)      Statistics (5s)         History + API
```

## Common Code Patterns

### Connect to Zenoh
```rust
use zenoh::config::Config;

// Peer mode (default)
let session = zenoh::open(Config::default()).await?;

// Client mode (connect to router)
let config = Config::from_file("client_config.json5")?;
let session = zenoh::open(config).await?;
```

### Parse Key Expression
```rust
let key = sample.key_expr.to_string();
let room = key.split('/').nth(2);  // Extract "room_a" from "building/floor1/room_a/temperature"
let sensor = key.split('/').last();  // Extract "temperature"
```

### Handle Errors
```rust
match query.sample {
    Ok(sample) => { /* success */ }
    Err(e) => {
        eprintln!("Error: {}", e);
        // Continue to next reply
    }
}
```

### Add Logging
```rust
env_logger::init();
println!("[ComponentName] Message");

// Enable debug logging
RUST_LOG=zenoh=debug cargo run --release --bin program
```

## Troubleshooting Checklist

### Publisher not receiving data
- [ ] Publisher created before subscriber
- [ ] Key expressions match exactly (or use wildcards)
- [ ] Session is open and connected

### Subscriber receives nothing
- [ ] Publisher is running and sending data
- [ ] Key expression matches (test with exact match first)
- [ ] If using wildcard, test with exact match

### Query returns no replies
- [ ] Queryable service is running
- [ ] Key expression matches
- [ ] Service is responding (check logs)
- [ ] Query timeout didn't expire

### Can't connect to router
- [ ] Router is running: `cargo run --release --bin zenohd -- --config router.json5`
- [ ] Check listening port: `nc -zv 127.0.0.1 7447`
- [ ] Client config has correct endpoint

### Port already in use
```bash
# Find process using port 7447
lsof -i :7447

# Kill the process
kill -9 <PID>
```

## File Organization

```
Each chapter has:
├── README.md          # Full tutorial text
├── Cargo.toml         # Dependencies & binaries
├── *.json5            # Config files (if needed)
└── src/
    ├── main.rs        # Main program (if applicable)
    └── bin/           # Example programs
        ├── program1.rs
        ├── program2.rs
        └── program3.rs
```

## Building & Running

```bash
# Build a chapter
cd chapter_N
cargo build --release

# Run an example
cargo run --release --bin example_name

# Run with logging
RUST_LOG=debug cargo run --release --bin example_name

# Clean build
cargo clean
cargo build --release
```

## Testing Your Knowledge

After each chapter, you should be able to:

**Chapter 1**
- [ ] Create a publisher that sends data
- [ ] Create a subscriber that receives data
- [ ] Modify the data type (number, string, JSON)

**Chapter 2**
- [ ] Use wildcards to subscribe to multiple keys
- [ ] Organize data hierarchically
- [ ] Extract parts of a key expression

**Chapter 3**
- [ ] Create a queryable service
- [ ] Query a service and handle responses
- [ ] Use wildcard queries

**Chapter 4**
- [ ] Start and configure a Zenoh router
- [ ] Configure clients to connect to router
- [ ] Communicate across process boundaries
- [ ] Kill and restart components safely

## Next Chapter Preview

| Chapter | Preview |
|---------|---------|
| 1 | ✓ Basics ⟶ 2 |
| 2 | ✓ Organization ⟶ 3 |
| 3 | ✓ Request/Reply ⟶ 4 |
| 4 | ✓ Distribution ⟶ 5: Building complete system |
| 5 | ⟶ 6: Add monitoring |
| 6 | ⟶ 7: Add storage |
| 7 | ⟶ 8: Add device management |
| 8 | ⟶ 9: Optimization & debugging |

## Resources

- **Zenoh Documentation**: https://docs.rs/zenoh/
- **Zenoh Repository**: https://github.com/eclipse-zenoh/zenoh
- **API Reference**: https://docs.rs/zenoh/latest/zenoh/
- **Discussions**: https://github.com/eclipse-zenoh/roadmap/discussions

## Getting Help

1. Check the chapter's "Common Issues" section
2. Look at the troubleshooting checklist above
3. Enable debug logging: `RUST_LOG=zenoh=debug`
4. Check key expressions match with exact match first
5. Start fresh: `cargo clean && cargo build --release`
6. Join the Zenoh community for questions

---

**Tips**: 
- Keep terminals organized (one per component)
- Start services in order (router, then services, then clients)
- Always check that connections are established before troubleshooting data flow
- Use `RUST_LOG=debug` when stuck to see detailed logs

Happy learning! 🚀
