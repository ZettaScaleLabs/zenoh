# Chapter 9: Production Deployment

**Time**: 30 minutes | **Level**: Advanced | **Concepts**: Infrastructure, Scaling, Operations, Reliability

## Overview

Deploy the Smart Building system to production. Learn about:

- Multi-machine deployments
- Router configuration
- Security and authentication
- High availability setup
- Monitoring and alerting
- Backup and disaster recovery

## Production Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Internet/WAN                              │
└────────────────────────┬────────────────────────────────────┘
                         │
        ┌────────────────┼────────────────┐
        │                │                │
   ┌────▼────┐      ┌────▼────┐      ┌───▼─────┐
   │ Router 1 │      │ Router 2 │      │ Router 3│
   │(Primary) │      │(Secondary)      │(Backup) │
   └────┬────┘      └────┬────┘      └────┬────┘
        │                │                │
   ┌────┴────────────────┼────────────────┴────┐
   │                     │                     │
┌──▼────┐  ┌──────────┐  │  ┌────────┐  ┌─────▼──┐
│Building A Service │  │  │Storage │  │Building B
│Tier Devices   Tier│  │  │Service │  │Devices
└─────────────────┘  │  └────────┘  └────────┘
                  ┌──▼──────────┐
                  │Monitoring & │
                  │Logging      │
                  └─────────────┘
```

## Configuration for Multi-Machine Setup

### Router Configuration (`zenoh_router_prod.json5`)

```json5
{
  mode: "router",
  listen: {
    endpoints: [
      "tcp/0.0.0.0:7447",
      "tcp/0.0.0.0:7448",
    ]
  },
  admin: {
    enabled: true,
    endpoint: "tcp/127.0.0.1:8000"
  }
}
```

### Client Configuration (`zenoh_client_prod.json5`)

```json5
{
  mode: "client",
  connect: {
    endpoints: [
      "tcp/router1.domain.com:7447",
      "tcp/router2.domain.com:7447",  // Fallback
    ]
  }
}
```

## Deployment Checklist

### Pre-Deployment
- [ ] All services built with `--release`
- [ ] Configurations tested in staging
- [ ] Monitoring and logging configured
- [ ] Backup procedures tested
- [ ] Disaster recovery plan documented
- [ ] Performance baselines established

### Deployment
- [ ] Deploy routers first
- [ ] Deploy service tier services
- [ ] Deploy sensor/device tier
- [ ] Deploy monitoring and logging
- [ ] Run smoke tests
- [ ] Verify data flowing correctly

### Post-Deployment
- [ ] Monitor resource usage
- [ ] Check service health metrics
- [ ] Verify backups running
- [ ] Document actual topology
- [ ] Schedule maintenance windows
- [ ] Plan for scaling

## High Availability Setup

### Router Redundancy

```
Primary Router (Active)
   ↓
   ↓ (Heartbeat)
   ↓
Secondary Router (Standby)
   ↓ (Takes over if primary fails)
   ↓
Clients automatically failover
```

### Service Redundancy

```rust
// Deploy multiple instances of critical services
// Clients distribute queries across them

// Service 1 (Active)
let queryable1 = session.declare_queryable("services/aggregator/primary").await.unwrap();

// Service 2 (Standby)
let queryable2 = session.declare_queryable("services/aggregator/standby").await.unwrap();

// Client tries primary, falls back to standby
match session.get("services/aggregator/primary").await {
    Ok(result) => { /* use primary */ }
    Err(_) => {
        match session.get("services/aggregator/standby").await {
            Ok(result) => { /* use standby */ }
            Err(e) => { /* handle error */ }
        }
    }
}
```

## Monitoring in Production

### Key Metrics to Track

1. **Message Latency**: End-to-end time for data flow
2. **Error Rate**: Percentage of failed operations
3. **Throughput**: Messages per second
4. **Service Availability**: Uptime percentage
5. **Resource Usage**: CPU, Memory, Network bandwidth
6. **Storage Usage**: Database/log storage growth

### Example Monitoring Script

```rust
#[tokio::main]
async fn monitor() {
    let session = zenoh::open(Config::default()).await.unwrap();
    
    loop {
        // Collect metrics
        let start = Instant::now();
        
        // Query all services for status
        if let Ok(mut results) = session.get("services/*/status").await {
            while let Ok(result) = results.recv_async().await {
                println!("Service status: {:?}", result);
            }
        }
        
        let latency = start.elapsed();
        println!("Query latency: {:?}", latency);
        
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}
```

## Scaling Strategy

### Vertical Scaling
- Increase resources (CPU, RAM) on existing machines
- Suitable for: Storage services, message aggregators

### Horizontal Scaling
- Add more machines with same services
- Suitable for: Sensors, data collectors

### Tier-Based Scaling
```
Sensors:           1000s (horizontal)
Service Tier:      10s  (vertical + horizontal)
Aggregation:       5    (vertical)
Storage:           2-3  (HA pair + backup)
```

## Operational Procedures

### Starting System

```bash
# 1. Start routers
zenohd -c zenoh_router_prod.json5 &

# 2. Start storage service (wait for router)
sleep 5
./storage_service &

# 3. Start aggregator services
./aggregator &

# 4. Start monitoring
./monitor &

# 5. Start devices
for i in {1..100}; do
    ./sensor_device $i &
done
```

### Maintenance Window

```bash
# 1. Announce maintenance
echo "MAINTENANCE" | pub maintenance/status

# 2. Stop accepting new connections
# (configure router)

# 3. Wait for in-flight messages
sleep 10

# 4. Graceful shutdown
pkill storage_service
pkill aggregator

# 5. Maintenance tasks...
# (backup, upgrade, etc)

# 6. Restart services
```

### Disaster Recovery

```bash
# 1. Detect failure
# (monitoring detects no heartbeat)

# 2. Promote backup
./promote_backup.sh

# 3. Alert operations team
# (send notification)

# 4. Investigate root cause
# (analyze logs)

# 5. Restore data from backup
./restore_backup.sh backup_from_yesterday.tar.gz

# 6. Verify system health
./health_check.sh
```

## Security Considerations

### Network Security
- Use TLS for all connections
- Firewall Zenoh ports
- Use VPNs for remote access

### Access Control
- Authenticate Zenoh connections
- Limit data access by role
- Audit all queries and changes

### Data Protection
- Encrypt data at rest
- Encrypt data in transit
- Regular backups encrypted

## Performance Tuning

```json5
// In production config
performance: {
  batch_size: 65536,     // Larger batches
  queue_size: 2097152,   // Larger queues
  flow_control: true,    // Enable backpressure
},
network: {
  tcp_keepalive_time: 30,
  tcp_keepalive_interval: 10,
}
```

## Exercises

### Exercise 1: Multi-Router Setup
Set up 3 routers in HA configuration and test failover.

### Exercise 2: Performance Test
Generate load and measure throughput, latency, CPU usage.

### Exercise 3: Failure Scenarios
Simulate device failures and verify system recovery.

### Exercise 4: Capacity Planning
Based on production metrics, plan for 10x growth.

## Exercises

### Exercise 1: Deploy to 3 Machines
Set up system across multiple machines with router coordination.

### Exercise 2: Test Failover
Simulate router failure and verify automatic client failover.

### Exercise 3: Performance Tuning
Adjust batch sizes and queue depths for target throughput.

### Exercise 4: Backup/Recovery
Test full system backup and recovery procedure.

## Summary

Congratulations! You've completed the Smart Building Tutorial. You now understand:

✓ Pub/Sub messaging (Chapter 1-2)
✓ Query/Reply patterns (Chapter 3)
✓ Router coordination (Chapter 4)
✓ Multi-tier architectures (Chapter 5)
✓ Data persistence (Chapter 6)
✓ Device management (Chapter 7)
✓ Troubleshooting (Chapter 8)
✓ Production deployment (Chapter 9)

You're ready to build Zenoh-based systems!

## What's Next?

- Explore Zenoh documentation: https://zenoh.io
- Build your own domain-specific system
- Join the Zenoh community
- Contribute examples and patterns
- Scale to your specific use cases
