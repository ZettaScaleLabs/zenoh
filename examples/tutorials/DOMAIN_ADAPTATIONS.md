# Domain Adaptation Guides

The Smart Building tutorial teaches core Zenoh patterns using a relatable example. These domain adaptation guides show how to apply those same patterns to different industries and applications.

## Quick Navigation

| Domain | Focus | Use Case | Complexity |
|--------|-------|----------|-----------|
| **[Robotics](./DOMAIN_ADAPTATION_ROBOTICS.md)** | Fleet coordination | Multi-robot systems, autonomous teams | Medium |
| **[IoT Sensors](./DOMAIN_ADAPTATION_IOT.md)** | Distributed monitoring | Thousands of sensors, edge gateways | Medium-High |
| **[Market Data](./DOMAIN_ADAPTATION_MARKET.md)** | High-frequency feeds | Real-time trading, price distribution | High |
| **[Manufacturing](./DOMAIN_ADAPTATION_MANUFACTURING.md)** | Production systems | Factory automation, OEE, maintenance | Medium |

## How to Use These Guides

### Step 1: Complete the Smart Building Tutorial
Each domain guide assumes you're familiar with the core Smart Building tutorial:
- Chapter 1-2: Basic pub/sub and key expressions
- Chapter 3-4: Query/reply and router setup
- Chapter 5: Multi-tier architecture

Estimated time: 1.5-2 hours

### Step 2: Choose Your Domain
Pick the guide that matches your use case or area of interest.

### Step 3: Follow the Adaptation Pattern
Each guide provides:
1. **Quick Mapping** table showing how Smart Building concepts apply to your domain
2. **Architecture Pattern** showing the system layout
3. **Key Expression Hierarchy** for organizing data
4. **Chapter Adaptations** showing how each chapter's patterns translate
5. **Complete Example** with working code
6. **Exercises** to practice the patterns
7. **Common Patterns** specific to the domain
8. **References** back to the tutorial

### Step 4: Build Your Application
Use the examples as starting points for your own systems.

## Comparison by Characteristics

### Robotics Fleet Management
**Best for:** Multi-robot coordination, autonomous systems, distributed teams

**Key Zenoh patterns:**
- Pub/sub for telemetry (position, battery, health)
- Query/reply for task assignment and status
- Router for team coordination
- Hierarchical organization: fleet → team → robot → subsystem

**Example workflow:**
```
Robots publish telemetry (position, battery)
└─ Team aggregator collects metrics
   └─ Fleet coordinator assigns tasks
      └─ Dashboard displays team status
```

**Typical scale:** 3-100 robots per fleet

### IoT Sensor Networks
**Best for:** Distributed sensor deployments, edge computing, cloud integration

**Key Zenoh patterns:**
- Pub/sub for sensor readings (high volume)
- Query/reply for device discovery and metadata
- Storage for time-series data
- Selective subscriptions for filtering
- Router as edge gateway

**Example workflow:**
```
Thousands of sensors publish data
└─ Regional gateway aggregates and filters
   └─ Edge analytics process locally
      └─ Cloud connector persists to database
```

**Typical scale:** 100s-10,000s of sensors per region

### Market Data Distribution
**Best for:** Financial systems, real-time price feeds, trading infrastructure

**Key Zenoh patterns:**
- High-frequency pub/sub (1000s msgs/sec)
- Query/reply for historical quotes
- Storage for audit trail
- Hierarchical organization: exchange → asset class → instrument
- Low-latency emphasis

**Example workflow:**
```
Exchange feeds publish prices
└─ Market gateway deduplicates and normalizes
   └─ Trading systems subscribe to watchlists
      └─ Risk monitor queries portfolio status
```

**Typical throughput:** 1,000-100,000 messages/second

### Manufacturing Systems
**Best for:** Factory automation, production tracking, predictive maintenance

**Key Zenoh patterns:**
- Pub/sub for machine telemetry (RPM, temperature, etc.)
- Query/reply for machine status and specifications
- Router as Manufacturing Execution System (MES)
- Storage for production history and audit trail

**Example workflow:**
```
Machines publish telemetry
└─ MES aggregates and triggers alerts
   └─ Predictive maintenance monitors health
      └─ Dashboard displays OEE and production status
```

**Typical scale:** 10-200 machines per factory

## Domain Mapping Quick Reference

All domains use these core Zenoh patterns from the tutorial:

| Tutorial Pattern | Robotics | IoT | Market Data | Manufacturing |
|---|---|---|---|---|
| **Chapter 1: Pub/Sub** | Robot telemetry | Sensor readings | Price ticks | Machine metrics |
| **Chapter 2: Key Expr** | fleet/team/robot | sensors/region/facility | market/exchange/symbol | manufacturing/line/machine |
| **Chapter 3: Query/Reply** | Task assignment | Device registry | Historical quotes | Machine status |
| **Chapter 4: Router** | Team hub | Edge gateway | Market gateway | MES |
| **Chapter 5: Multi-tier** | Fleet coordination | Cloud integration | Risk monitoring | OEE calculation |

## Common Implementation Tasks

### Task: Set Up Hierarchical Organization
**In Smart Building:** Building → Floor → Room
**In Your Domain:** Choose your hierarchy

**Example: Robotics**
```rust
let key_expr = format!("fleet/{}/robot-{}/telemetry/position", team_id, robot_id);
```

**Example: IoT**
```rust
let key_expr = format!("sensors/{}/{}/facility-{}/device-{}/temperature", region, city, facility_id, device_id);
```

### Task: Configure Selective Subscriptions
**In Smart Building:** Subscribe to "building/floor-1/**"
**In Your Domain:** Subscribe to relevant data

**Example: Market Data (trading system needs only tech stocks)**
```rust
session.declare_subscriber("market/nasdaq/tech/*/quote").res().await?
```

**Example: Manufacturing (dashboard needs all metrics)**
```rust
session.declare_subscriber("manufacturing/plant-a/*/*/telemetry/**").res().await?
```

### Task: Implement Queryable Services
**In Smart Building:** Thermostat responds with status
**In Your Domain:** Service responds with domain info

**Example: IoT (sensor responds with metadata)**
```rust
while let Ok(query) = queries.recv_async().await {
    let metadata = json!({
        "type": "temperature_sensor",
        "accuracy": 0.5,
        "update_interval": 5,
        "location": "building-a/floor-3/room-301"
    });
    query.reply(Ok(Sample::new(query.key_expr().clone(), 
        serde_json::to_string(&metadata).unwrap()))).res().await?;
}
```

## Choosing Your Domain

### Choose Robotics if you're working with:
- Multi-robot systems
- Autonomous vehicle fleets
- Drone swarms
- Collaborative robots (cobots)
- Robot-as-a-Service platforms

**Time to adapt:** 1-2 days

### Choose IoT if you're working with:
- Environmental monitoring
- Smart cities
- Agricultural sensors
- Building automation (beyond single building)
- Connected devices at scale

**Time to adapt:** 2-3 days

### Choose Market Data if you're working with:
- Financial trading systems
- Price/quote distribution
- Real-time market data feeds
- High-frequency applications
- Low-latency messaging

**Time to adapt:** 3-5 days (requires performance testing)

### Choose Manufacturing if you're working with:
- Factory floor automation
- Production monitoring
- Equipment maintenance
- Industrial IoT
- Supply chain coordination

**Time to adapt:** 2-3 days

## Next Steps After Completing a Domain

1. **Build a prototype** of your specific system using the domain's example code
2. **Scale it up** by adding more devices/machines/sensors
3. **Add persistence** using Chapter 7 patterns (storage backends)
4. **Add monitoring** using Chapter 6 patterns (logging, metrics)
5. **Deploy to production** using Chapter 8-9 patterns (configuration, security)

## Tips for Success

1. **Start small**: Begin with one publisher and one subscriber
2. **Test locally first**: Run sender and receiver on same machine
3. **Add complexity gradually**: Add more senders, then aggregators, then storage
4. **Use the quick reference**: Keep QUICK_REFERENCE.md handy while coding
5. **Check the examples**: Copy working patterns from the provided code
6. **Debug with subscribers**: Use simple subscribers to see data flowing
7. **Monitor key expressions**: Use `zenoh-probe` or similar tools to observe the network

## References

- **[Smart Building Tutorial](./smart_building/README.md)** - Core tutorial (start here)
- **[Chapter 1-5 READMEs](./smart_building/chapter_1/README.md)** - Detailed pattern explanations
- **[Quick Reference Guide](./smart_building/QUICK_REFERENCE.md)** - Copy-paste code patterns
- **[Zenoh Official Docs](https://zenoh.io/docs/)** - Complete Zenoh documentation
- **[Zenoh GitHub](https://github.com/eclipse-zenoh/zenoh)** - Source code and examples

## Feedback and Questions

If you have questions or suggestions about these domain guides:

1. Check if the Smart Building tutorial covers your pattern
2. Review the specific domain guide's exercises section
3. Look at the provided example code for working implementations
4. Consult the Zenoh documentation for API details

## Contributing New Domains

Have another domain you'd like to see adapted? The pattern is:

1. Start with Smart Building tutorial fundamentals
2. Create a mapping of Smart Building concepts to your domain
3. Adapt architecture and key expression hierarchy
4. Provide chapter-by-chapter examples
5. Include working code examples and exercises

See CONTRIBUTING.md in the main Zenoh repository for submission guidelines.
