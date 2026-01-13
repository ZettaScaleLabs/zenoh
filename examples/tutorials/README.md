# Zenoh Tutorials

Welcome to the Zenoh Tutorials! These hands-on guides will teach you how to build distributed systems with Zenoh by constructing a complete, production-inspired application from the ground up.

## Available Tutorials

### [Smart Building System](./smart_building/README.md) ⭐ **START HERE**

Learn Zenoh by building a real-time monitoring system for a smart building. This progressive tutorial introduces concepts step-by-step through a relatable, practical application.

**What you'll build**: A complete system that monitors temperature, humidity, and occupancy across multiple rooms and floors, with data aggregation, historical queries, alerting, and device management.

**Duration**: ~3-4 hours (can be done chapter by chapter)

**Prerequisites**: 
- Basic Rust knowledge
- Rust 1.75.0+
- 30 minutes to read this tutorial overview

---

## Tutorial Structure

Each tutorial is organized into **9 progressive chapters**:

| Chapter | Focus | Duration | Concepts |
|---------|-------|----------|----------|
| 1 | Foundations | 20 min | Pub/Sub, Sessions, Publishers, Subscribers |
| 2 | Organization | 20 min | Key Expressions, Hierarchies, Multiple Topics |
| 3 | Request/Reply | 20 min | Queryables, Gets, Query/Reply Pattern |
| 4 | Distributed Arch | 25 min | Zenoh Router, Network Config, Multi-Client |
| 5 | Real Systems | 30 min | Multi-tier Architecture, Data Aggregation |
| 6 | Monitoring | 30 min | Logging, Metrics, Health Checks, Observability |
| 7 | Persistence | 30 min | Storage Backends, Historical Queries, Caching |
| 8 | Management | 30 min | Dynamic Registration, Configuration, Security |
| 9 | Troubleshooting | 30 min | Debugging, Common Issues, Performance Tips |

---

## How to Use This Tutorial

### Option 1: Follow Chapter by Chapter (Recommended)
```bash
# Chapter 1: Hello Zenoh
cd smart_building/chapter_1
cargo run --example hello_zenoh

# Chapter 2: Hierarchical Topics
cd ../chapter_2
cargo run --example room_sensors

# ... continue through chapters
```

### Option 2: Study the Full Application
```bash
# Build the complete Smart Building System
cd smart_building/final_application
cargo build --release

# Run all components
./run_demo.sh
```

### Option 3: Reference Specific Concepts
Jump to any chapter that covers the concept you need. Each chapter is self-contained but builds on previous knowledge.

---

## Key Concepts Covered

### Core Zenoh Patterns
- **Pub/Sub**: Publish data for multiple subscribers to consume
- **Query/Reply**: Request data from services and receive responses
- **Key Expressions**: Hierarchical naming and filtering
- **Sessions**: Connections to Zenoh network
- **Routers**: Distributed network infrastructure

### Real-World Patterns
- Multi-tier architecture (edge → aggregation → data center)
- Data serialization and format handling
- Monitoring and observability
- Device discovery and management
- Historical data queries and persistence
- Security (TLS, authentication)
- Performance optimization
- Debugging and troubleshooting

---

## The Smart Building Example

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         DATA CENTER                             │
│  ┌──────────────────┐  ┌──────────────┐  ┌─────────────────┐  │
│  │  Query API       │  │  Storage     │  │  Alert Service  │  │
│  │  (Web/CLI)       │  │  Backend     │  │                 │  │
│  └────────┬─────────┘  └──────┬───────┘  └────────┬────────┘  │
└───────────┼──────────────────┼────────────────────┼────────────┘
            │                  │                    │
        ┌───┴──────────────────┴────────────────────┴───┐
        │        ZENOH ROUTER (Coordinator)            │
        └───┬──────────────────┬────────────────────┬───┘
            │                  │                    │
┌───────────┴────────┐  ┌──────┴──────────┐  ┌─────┴─────────────┐
│   FLOOR COLLECTOR  │  │ FLOOR COLLECTOR │  │ FLOOR COLLECTOR   │
│   (Floor 1)        │  │   (Floor 2)     │  │    (Floor 3)      │
└───┬────────────────┘  └──────┬──────────┘  └─────┬─────────────┘
    │                          │                    │
┌───┴──────┬──────────┐  ┌─────┴──────┐      ┌─────┴──────┐
│  Room 1  │  Room 2  │  │  Room 3    │      │  Room 4    │
│ Sensors  │ Sensors  │  │ Sensors    │ ...  │ Sensors    │
└──────────┴──────────┘  └────────────┘      └────────────┘

EDGE (Sensors)        │  AGGREGATION        │  CENTRAL (Query/Storage)
- Temperature        │  - Statistics       │  - Analytics
- Humidity           │  - Alerts           │  - History
- Occupancy          │  - Aggregation      │  - API
```

### Data Flow Example

```
Room Temperature Sensor (25.3°C)
        ↓
Publishes to: "building/floor1/room_a/temperature"
        ↓
Collected by: Floor 1 Aggregator
        ↓
Stores in: Local cache
        ↓
Queried by: Dashboard or Analytics Service
        ↓
Retrieved from: Central Storage Backend
        ↓
Displayed: Web Dashboard / Alert System
```

---

## Running the Examples

### Prerequisites
```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update

# Clone or navigate to the zenoh repository
cd zenoh/examples/tutorials/smart_building
```

### Build All Examples
```bash
# Build the tutorial workspace
cargo build --release

# Run a specific chapter
cargo run --release --example ch1_hello_zenoh
```

### Run Interactive Demo
```bash
# In chapter_5 (or later chapters with a full setup)
./run_demo.sh  # Runs all components with proper timing
```

---

## Learning Path

**Beginner** (Chapters 1-3)
- Learn Zenoh pub/sub and query/reply basics
- Build familiarity with the API
- Understand hierarchical data organization

**Intermediate** (Chapters 4-6)
- Deploy distributed systems with routers
- Build realistic multi-tier architectures
- Add monitoring and observability

**Advanced** (Chapters 7-9)
- Implement persistence and storage backends
- Manage devices at scale
- Debug and optimize performance

---

## Example Output

After completing Chapter 5, running the full demo will show:

```
[Floor 1 Aggregator] Room A: Temp=22.3°C Humidity=45% Occupancy=2
[Floor 1 Aggregator] Room B: Temp=21.8°C Humidity=42% Occupancy=0
[Floor 2 Aggregator] Room C: Temp=23.1°C Humidity=48% Occupancy=5
[Data Center] Received floor aggregation: Floor 1 avg temp: 22.05°C
[Alert Service] ⚠️  Room C exceeds max temperature threshold!
[Query] GET building/floor1/* → 2 samples received
```

---

## Troubleshooting

### Build Issues
See [Chapter 1](./smart_building/chapter_1/README.md) - Troubleshooting section

### Runtime Issues
See [Chapter 9](./smart_building/chapter_9/README.md) - Debugging Guide

### Zenoh Router Not Connecting
See [Chapter 4](./smart_building/chapter_4/README.md) - Router Configuration section

---

## Domain-Specific Adaptation Guides

After completing the Smart Building tutorial, learn how to apply the same patterns to your specific domain:

### [📚 Domain Adaptations Index](./DOMAIN_ADAPTATIONS.md)

Complete guide to adapting Zenoh patterns for different industries and applications.

### Available Domains

| Domain | Best For | Complexity | Time to Adapt |
|--------|----------|-----------|---------------|
| **[Robotics Fleet Management](./DOMAIN_ADAPTATION_ROBOTICS.md)** | Multi-robot coordination, autonomous teams | Medium | 1-2 days |
| **[IoT Sensor Networks](./DOMAIN_ADAPTATION_IOT.md)** | Distributed sensors, edge gateways, cloud integration | Medium-High | 2-3 days |
| **[Market Data Distribution](./DOMAIN_ADAPTATION_MARKET.md)** | Real-time trading, financial feeds, high-frequency | High | 3-5 days |
| **[Manufacturing Systems](./DOMAIN_ADAPTATION_MANUFACTURING.md)** | Factory automation, OEE, predictive maintenance | Medium | 2-3 days |

### How Domain Guides Work

Each domain guide includes:
1. **Quick Mapping** showing how Smart Building concepts apply to your domain
2. **Architecture Pattern** with concrete system layout
3. **Key Expression Hierarchy** for organizing your data
4. **Adapted Examples** for each tutorial chapter
5. **Working Code** you can copy and modify
6. **Exercises** to practice the patterns
7. **Common Patterns** and best practices for the domain

### Quick Start

1. ✅ Complete the Smart Building tutorial (1.5-2 hours)
2. 📖 Pick your domain from the list above
3. 💻 Follow the adapted examples
4. 🔨 Build your application using the provided templates
5. 🚀 Deploy and scale!

---

## Next Steps After Tutorials

Once you complete the tutorials:
1. **Read the [Zenoh API Docs](https://docs.rs/zenoh/)** for comprehensive reference
2. **Follow a [Domain Adaptation Guide](./DOMAIN_ADAPTATIONS.md)** for your specific use case
3. **Join the [Zenoh Community](https://github.com/eclipse-zenoh/roadmap)** for questions and discussions
4. **Build Your Application** using the patterns you've learned

---

## Contributing

Found an issue or have a suggestion? Please open an issue or PR in the [Zenoh repository](https://github.com/eclipse-zenoh/zenoh).

---

## License

These tutorials and examples are licensed under the same license as Zenoh: **EPL 2.0 OR Apache 2.0**
