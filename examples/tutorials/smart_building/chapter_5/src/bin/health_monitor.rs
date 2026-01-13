use zenoh::config::Config;
use serde_json::json;
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    env_logger::init();

    println!("Opening Zenoh session for health monitor...");
    let session = zenoh::open(Config::default()).await.unwrap();

    // Subscribe to all health updates
    let mut health_sub = session
        .declare_subscriber("health/**")
        .await
        .unwrap();

    // Publish alerts
    let alert_pub = session
        .declare_publisher("alerts/health")
        .await
        .unwrap();

    println!("Health Monitor started. Monitoring system health...\n");

    let mut sensor_states: HashMap<i32, String> = HashMap::new();

    loop {
        if let Ok(sample) = health_sub.recv_async().await {
            if let Ok(health_str) = sample.payload().try_to_string() {
                if let Ok(health) = serde_json::from_str::<serde_json::Value>(&health_str) {
                    if let Some(sensor_id) = health["sensor_id"].as_i64() {
                        let status = health["status"].as_str().unwrap_or("unknown");
                        let prev_status = sensor_states.get(&(sensor_id as i32)).cloned();

                        // Check for status changes
                        if prev_status != Some(status.to_string()) {
                            if status == "degraded" || status == "unhealthy" {
                                // Alert on degradation
                                let alert = json!({
                                    "level": "warning",
                                    "sensor_id": sensor_id,
                                    "status": status,
                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                });
                                alert_pub.put(alert.to_string()).await.unwrap();

                                println!(
                                    "⚠️  [Alert] Sensor {} degraded to: {}",
                                    sensor_id, status
                                );
                            } else {
                                println!("✓ [Monitor] Sensor {} recovered to: {}", sensor_id, status);
                            }

                            sensor_states.insert(sensor_id as i32, status.to_string());
                        }
                    }
                }
            }
        }
    }
}
