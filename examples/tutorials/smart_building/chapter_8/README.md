# Chapter 8: Troubleshooting & Monitoring

**Time**: 25 minutes | **Level**: Advanced | **Concepts**: Debugging, Performance Analysis, System Health

## Overview

Diagnose and fix production issues. Learn about:

- Performance monitoring
- Latency analysis
- Error tracking
- System diagnostics
- Capacity planning

## Key Patterns

### Latency Monitoring

```rust
// Track message latencies
let start = std::time::Instant::now();
publisher.put(data).await.unwrap();
let latency = start.elapsed();

// Publish latency metric
metrics_pub.put(json!({
    "latency_ms": latency.as_millis(),
    "timestamp": Utc::now().to_rfc3339()
}).to_string()).await.unwrap();
```

### Error Rate Tracking

```rust
let mut errors = 0;
let mut successes = 0;

loop {
    match publisher.put(data).await {
        Ok(_) => successes += 1,
        Err(e) => {
            errors += 1;
            println!("Error: {}", e);
        }
    }
    
    let error_rate = errors as f32 / (errors + successes) as f32;
    if error_rate > 0.01 {
        println!("⚠️  Error rate: {:.2}%", error_rate * 100.0);
    }
}
```

### System Health Check

```rust
let health_check = async {
    let session = zenoh::open(Config::default()).await.ok()?;
    let subscriber = session.declare_subscriber("sensors/data/*").await.ok()?;
    
    // Try receiving a message within timeout
    tokio::time::timeout(
        Duration::from_secs(5),
        subscriber.recv_async()
    ).await.ok().flatten().ok()
};
```

## Diagnostic Queries

```rust
// Query all service status
match session.get("services/*/status").await {
    Ok(mut results) => {
        while let Ok(result) = results.recv_async().await {
            println!("Service status: {:?}", result);
        }
    }
    Err(e) => println!("Diagnostic failed: {}", e),
}
```

## Common Issues & Solutions

### Issue: High Latency
**Symptoms:** Messages taking long time to arrive
**Diagnosis:** Check network conditions, subscriber count
**Solution:** Add batching, increase buffer sizes

### Issue: Message Loss
**Symptoms:** Expected messages not arriving
**Diagnosis:** Check subscriber alignment, replay buffer
**Solution:** Enable history, verify subscriptions match publishers

### Issue: Memory Growing
**Symptoms:** Process memory increases over time
**Diagnosis:** Check for unbounded collections
**Solution:** Implement retention policies, cleanup routines

### Issue: CPU Spike
**Symptoms:** Unexpected CPU usage
**Diagnosis:** Profile with tools, check message rate
**Solution:** Rate limit, add processing delays

## Exercises

### Exercise 1: Create Diagnostic Dashboard
Build a service that shows real-time system health metrics.

### Exercise 2: Implement Rate Limiting
Add automatic backpressure when message rates exceed thresholds.

### Exercise 3: Performance Profiling
Measure end-to-end latency for message journeys through system.

### Exercise 4: Anomaly Detection
Detect unusual patterns (e.g., sudden temperature jumps).

## Troubleshooting Checklist

- [ ] All services publishing on correct key expressions?
- [ ] All subscribers subscribing to correct patterns?
- [ ] Zenoh session connectivity verified?
- [ ] Network connectivity between nodes?
- [ ] Sufficient disk space for logs/storage?
- [ ] CPU/memory within acceptable limits?
- [ ] Clock synchronization across nodes?
- [ ] Firewall rules allowing Zenoh traffic?

## Next Steps

- **Chapter 9:** Production Deployment - Deploy to real systems
