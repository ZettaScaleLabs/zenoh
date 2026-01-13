# Smart Building Tutorial: Complete Guide

Welcome! In this tutorial, you'll build a complete Smart Building monitoring system with Zenoh. This system demonstrates real-world distributed systems patterns while progressively introducing Zenoh concepts.

## What You'll Build

By the end of this tutorial, you'll have a fully functional system that:

- ✅ Reads sensor data from multiple rooms (temperature, humidity, occupancy)
- ✅ Aggregates data across floors
- ✅ Queries historical data from a central database
- ✅ Generates alerts for anomalies
- ✅ Manages device discovery and registration
- ✅ Monitors system health and performance
- ✅ Debugs issues in a distributed system

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                        CHAPTER 9                                  │
│              FULL PRODUCTION SYSTEM                              │
│  Device Management | Security | Performance | Monitoring         │
└──────────────────────────────────────────────────────────────────┘
                            ▲
┌──────────────────────────────────────────────────────────────────┐
│                        CHAPTER 8                                  │
│         Dynamic Registration | Configuration | Security           │
└──────────────────────────────────────────────────────────────────┘
                            ▲
┌──────────────────────────────────────────────────────────────────┐
│                        CHAPTER 7                                  │
│        Storage Backends | Historical Queries | Persistence       │
└──────────────────────────────────────────────────────────────────┘
                            ▲
┌──────────────────────────────────────────────────────────────────┐
│                        CHAPTER 6                                  │
│     Logging | Metrics | Health Checks | Observability            │
└──────────────────────────────────────────────────────────────────┘
                            ▲
┌──────────────────────────────────────────────────────────────────┐
│                        CHAPTER 5                                  │
│    Multi-Tier Architecture | Data Aggregation | Real Systems     │
└──────────────────────────────────────────────────────────────────┘
                            ▲
┌──────────────────────────────────────────────────────────────────┐
│                        CHAPTER 4                                  │
│        Zenoh Router | Network Config | Multi-Client              │
└──────────────────────────────────────────────────────────────────┘
                            ▲
┌──────────────────────────────────────────────────────────────────┐
│                        CHAPTER 3                                  │
│        Query/Reply Pattern | Queryables | Gets                   │
└──────────────────────────────────────────────────────────────────┘
                            ▲
┌──────────────────────────────────────────────────────────────────┐
│                        CHAPTER 2                                  │
│    Key Expressions | Hierarchies | Multiple Topics               │
└──────────────────────────────────────────────────────────────────┘
                            ▲
┌──────────────────────────────────────────────────────────────────┐
│                        CHAPTER 1                                  │
│   Pub/Sub | Sessions | Publishers | Subscribers                  │
└──────────────────────────────────────────────────────────────────┘
```

## Chapter Overview

### [Chapter 1: Hello Zenoh - Pub/Sub Basics](./chapter_1/README.md)
**Time**: 20 minutes | **Level**: Beginner

Learn the fundamentals of Zenoh by building a simple temperature publisher and subscriber.

**Key Concepts**:
- Creating a Zenoh session
- Publishing data with `.put()`
- Subscribing to data with `.declare_subscriber()`
- Running multiple processes that communicate

**What You'll Create**:
- `room_sensor.rs`: A temperature sensor that publishes readings
- `monitor.rs`: A subscriber that displays sensor readings

**Expected Output**:
```
[Monitor] Received: building/floor1/room_a/temperature = 22.5°C
[Monitor] Received: building/floor1/room_a/temperature = 22.6°C
[Monitor] Received: building/floor1/room_a/temperature = 22.4°C
```

---

### [Chapter 2: Organizing Data - Key Expressions](./chapter_2/README.md)
**Time**: 20 minutes | **Level**: Beginner

Organize sensor data hierarchically and learn about key expressions and wildcards.

**Key Concepts**:
- Hierarchical key expressions (e.g., `building/floor/room/sensor`)
- Wildcard subscriptions (`building/floor1/*/temperature`)
- Filtering and matching
- Multiple sensor types (temperature, humidity, occupancy)

**What You'll Create**:
- `room_sensor.rs`: Multiple sensor types in a single room
- `floor_monitor.rs`: Subscriber that watches entire floors with wildcards
- `selective_monitor.rs`: Subscriber filtering specific sensors

**Expected Output**:
```
[Floor Monitor] Room A Temp: 22.5°C
[Floor Monitor] Room A Humidity: 45%
[Floor Monitor] Room B Temp: 21.8°C
[Floor Monitor] Room B Humidity: 42%
```

---

### [Chapter 3: Queries & Responses - Request/Reply](./chapter_3/README.md)
**Time**: 20 minutes | **Level**: Beginner

Implement the query/reply pattern for on-demand data retrieval.

**Key Concepts**:
- Declaring queryables with `.declare_queryable()`
- Sending queries with `.get()`
- Handling query requests and sending replies
- Selector expressions
- Use case: Status checks and data lookup

**What You'll Create**:
- `room_status_service.rs`: Queryable that responds with room status
- `dashboard.rs`: Application that queries room status on demand
- `multi_service.rs`: Multiple queryables responding to same query

**Expected Output**:
```
[Dashboard] Query: building/floor1/room_a?status=*
[Status Service] Received query from dashboard
[Dashboard] Response: Room A - Temp: 22.5°C, Occupancy: 2, Light: ON
```

---

### [Chapter 4: Distribution - Zenoh Router Setup](./chapter_4/README.md)
**Time**: 25 minutes | **Level**: Intermediate

Connect multiple applications through a Zenoh router for distributed communication.

**Key Concepts**:
- Zenoh router deployment and configuration
- Client connection to routers
- Network topologies
- Multiple subscribers across processes
- Router as coordinator

**What You'll Create**:
- Router configuration file
- Multi-process architecture
- Room sensors (separate processes)
- Multiple monitoring services (separate processes)
- Test scripts to run everything

**Architecture**:
```
Room 1 Sensor --\
Room 2 Sensor -- --> [Zenoh Router] <-- Monitor 1
Room 3 Sensor --/                    <-- Monitor 2
                                     <-- Dashboard
```

**Expected Output**:
```
[Router] Listening on tcp/127.0.0.1:7447
[Room 1] Connected to router
[Monitor 1] Connected to router
[Monitor 1] Received data from Room 1
```

---

### [Chapter 5: Real Systems - Multi-Tier Architecture](./chapter_5/README.md)
**Time**: 30 minutes | **Level**: Intermediate

Build a complete, realistic multi-tier system with edge devices, aggregation, and central services.

**Key Concepts**:
- Edge layer: Sensor devices publishing raw data
- Aggregation layer: Collecting and processing data
- Central layer: Storage and API services
- Data transformation pipelines
- Scalable architectures

**What You'll Create**:
- **Edge Tier**:
  - Multiple room sensors (simulate with configurable rates)
  - Each publishes temperature, humidity, occupancy
  
- **Aggregation Tier**:
  - Floor aggregators (one per floor)
  - Calculate statistics (avg temp, max humidity, occupancy count)
  - Subscribe to room data, publish aggregate data
  
- **Central Tier**:
  - Data center query API
  - In-memory cache of latest data
  - Status queryable

**Data Flow**:
```
Edge (Sensors)      Aggregation         Central
T: 22.5°C  ──┐
H: 45%     ──> [Floor 1 Agg] ──> Avg: 22.3°C  ──┐
O: 2       ──┘  Max: 23.1°C      ────────────────> [Query API]
T: 21.8°C  ──┐      Total: 7     ──────────────→ [Cache]
H: 42%     ──> [Floor 2 Agg] ──> Avg: 22.1°C  ──┘
O: 0       ──┘
```

**Expected Output**:
```
[Floor 1 Aggregator] Avg Temp: 22.3°C | Max Humidity: 48% | Occupancy: 7
[Floor 2 Aggregator] Avg Temp: 22.1°C | Max Humidity: 45% | Occupancy: 0
[Data Center] Query: building/*/avg_temperature
[Data Center] Response: Floor1=22.3, Floor2=22.1
```

---

### [Chapter 6: Observability - Monitoring & Logging](./chapter_6/README.md)
**Time**: 30 minutes | **Level**: Intermediate

Add comprehensive logging, metrics, and health checks to monitor system health.

**Key Concepts**:
- Structured logging
- Metrics collection and reporting
- Health checks and liveness
- Debugging techniques
- Performance measurement

**What You'll Create**:
- Logging infrastructure (tracing-subscriber)
- Metrics collection (counters, gauges)
- Health check service
- Performance profiling
- Metrics dashboard simulator

**Monitoring Metrics**:
- Message count and throughput
- Latency histograms
- Active subscribers/publishers
- Error rates
- System health status

**Expected Output**:
```
[Metrics] Published: 1250 msg/s | Subscribed: 850 msg/s | Latency: 2.3ms
[Health] All services: OK | Memory: 125MB | CPU: 12%
[Logs] WARN: Room A sensor offline for 5 seconds
[Logs] ERROR: Floor 2 aggregator connection lost
```

---

### [Chapter 7: Persistence - Storage & History](./chapter_7/README.md)
**Time**: 30 minutes | **Level**: Intermediate

Implement persistent storage and historical data queries.

**Key Concepts**:
- Zenoh storage backends
- Time-series data management
- Historical queries (e.g., "What was the temperature 1 hour ago?")
- Data retention policies
- Caching strategies

**What You'll Create**:
- In-memory storage backend (SQLite-based)
- Storage subscribers for persistence
- Query interface for historical data
- Data retention and cleanup
- Metrics storage

**Capabilities**:
- Query data from any time in the past
- Aggregate historical data (hourly averages)
- Trend analysis
- Alerting based on historical patterns

**Expected Output**:
```
[Storage] Room A temperature (2025-01-12 14:30): 22.5°C
[Query] Historical avg temp (last 24h): 22.1°C
[Trend] Temperature is rising: +0.3°C in last hour
```

---

### [Chapter 8: Management - Device Onboarding & Config](./chapter_8/README.md)
**Time**: 30 minutes | **Level**: Advanced

Implement dynamic device discovery, configuration management, and security.

**Key Concepts**:
- Device registration and discovery
- Configuration management
- TLS/Authentication
- Dynamic device addition/removal
- Life cycle management

**What You'll Create**:
- Device registry service
- Configuration distribution service
- Security setup (TLS certificates)
- Device health monitoring
- Graceful shutdown and restart

**Capabilities**:
- Add/remove devices without restarting system
- Push configuration updates to devices
- Secure communication with TLS
- Device authentication
- Audit logging

**Expected Output**:
```
[Registry] New device registered: building/floor1/room_a
[Config] Pushed update to room_a: poll_interval=1000ms
[Security] Device authenticated with certificate
[Registry] Device offline: building/floor2/room_c
```

---

### [Chapter 9: Troubleshooting & Optimization](./chapter_9/README.md)
**Time**: 30 minutes | **Level**: Advanced

Learn debugging techniques, common pitfalls, and performance optimization.

**Key Concepts**:
- Debugging distributed systems
- Common Zenoh issues and solutions
- Performance profiling
- Latency optimization
- Reliability patterns

**Topics Covered**:
- Debug logging and tracing
- Network troubleshooting
- Router configuration issues
- Memory profiling
- Load testing and benchmarking
- Error recovery patterns

**Tools & Techniques**:
- `RUST_LOG` for detailed debugging
- Message timing analysis
- Connection status monitoring
- Throughput benchmarking
- Failure scenario testing

**Expected Output**:
```
[Debug] Subscriber not receiving: Check key expression match
[Perf] Throughput: 50k msg/s (baseline) → 200k msg/s (optimized)
[Latency] P99: 10ms (before) → 2ms (after optimization)
```

---

## Quick Start

### Option 1: Follow Sequentially (Recommended)

```bash
# Navigate to the tutorial
cd examples/tutorials/smart_building

# Chapter 1: Hello Zenoh (20 min)
cd chapter_1
cargo run --example hello_zenoh --release
# Follow instructions in chapter_1/README.md

# Chapter 2: Key Expressions (20 min)
cd ../chapter_2
cargo run --example hierarchical_sensors --release

# ... continue through all 9 chapters
```

### Option 2: Run Complete Demo

```bash
cd examples/tutorials/smart_building
./run_complete_demo.sh  # Runs all components with proper timing
```

### Option 3: Jump to Specific Topic

Each chapter is self-contained. Jump to the chapter covering concepts you need:
- Want to learn pub/sub? → Chapter 1
- Need distributed architecture? → Chapter 4
- Interested in storage? → Chapter 7

---

## Prerequisites

- **Rust**: 1.75.0 or later
- **Cargo**: Latest (comes with Rust)
- **System**: Linux, macOS, or Windows (WSL2)
- **Time**: 3-4 hours to complete all chapters (or do chapter-by-chapter)

### Verify Prerequisites

```bash
rustc --version  # Should be 1.75.0+
cargo --version
```

---

## Estimated Time Breakdown

| Phase | Chapters | Time | Topics |
|-------|----------|------|--------|
| **Foundations** | 1-3 | 60 min | Pub/Sub, Key Expressions, Query/Reply |
| **Distributed Systems** | 4-5 | 55 min | Router, Multi-tier Architecture |
| **Operations** | 6-7 | 60 min | Monitoring, Storage, Persistence |
| **Advanced** | 8-9 | 60 min | Management, Troubleshooting, Optimization |
| **Total** | 1-9 | ~235 min | Complete Smart Building System |

---

## Learning Outcomes

After completing this tutorial, you'll be able to:

✅ **Understand Core Zenoh Concepts**
- Pub/Sub messaging patterns
- Query/Reply request patterns
- Key expressions and hierarchical naming
- Sessions and connections

✅ **Design Distributed Systems**
- Multi-tier architectures
- Edge-to-cloud data flow
- Aggregation and processing pipelines
- Scalable designs

✅ **Build Production-Ready Applications**
- Monitoring and observability
- Data persistence and history
- Device management
- Security and authentication

✅ **Debug & Optimize**
- Troubleshoot distributed systems
- Profile performance
- Identify bottlenecks
- Optimize for throughput and latency

✅ **Transfer Knowledge**
- Apply patterns to your own domain
- Adapt architecture for different use cases
- Design systems with Zenoh

---

## File Structure

```
smart_building/
├── README.md                    # This file
├── chapter_1/
│   ├── README.md               # Chapter 1 instructions
│   ├── Cargo.toml
│   └── examples/
│       ├── hello_zenoh.rs
│       └── room_sensor.rs
├── chapter_2/
│   ├── README.md
│   ├── Cargo.toml
│   └── examples/
│       ├── hierarchical_sensors.rs
│       └── floor_monitor.rs
├── ...
├── chapter_9/
│   ├── README.md
│   ├── Cargo.toml
│   └── examples/
│       ├── debugging.rs
│       └── profiling.rs
├── shared_lib/                 # Shared utilities
│   ├── lib.rs
│   └── models.rs
└── run_complete_demo.sh        # Runs full demo
```

---

## Common Questions

**Q: Do I need to complete all chapters?**
A: No! Each chapter builds on the previous, but you can skip ahead if you're familiar with earlier concepts. Chapters 1-5 are essential; 6-9 are optional but recommended for production systems.

**Q: Can I run this on Windows?**
A: Yes! Use Windows Subsystem for Linux (WSL2) or install Rust natively. The examples are platform-independent.

**Q: What if I get stuck?**
A: Each chapter has a troubleshooting section. Also check the [Zenoh documentation](https://docs.rs/zenoh/) and [community discussions](https://github.com/eclipse-zenoh/roadmap).

**Q: How long does each chapter take?**
A: 20-30 minutes depending on your pace and Rust familiarity.

**Q: Can I adapt this to my use case?**
A: Absolutely! See the [Domain Adaptation Guides](../DOMAIN_ADAPTATIONS.md) for patterns adapted to robotics, IoT sensors, financial data, and more.

---

## Next Steps

1. **Start with Chapter 1**: [Hello Zenoh](./chapter_1/README.md)
2. **Join the Community**: [Zenoh Discussions](https://github.com/eclipse-zenoh/roadmap/discussions)
3. **Build Your Application**: Use the patterns from this tutorial
4. **Explore Advanced Topics**: See the [API Documentation](https://docs.rs/zenoh/)

---

## License

These tutorials are part of Zenoh and are licensed under: **EPL 2.0 OR Apache 2.0**

---

**Ready to begin? → [Go to Chapter 1](./chapter_1/README.md)**
