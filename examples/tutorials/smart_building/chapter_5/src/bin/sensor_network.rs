use std::time::Duration;
use zenoh::config::Config;
use serde_json::json;

#[tokio::main]
async fn main() {
    env_logger::init();

    println!("Opening Zenoh session for sensor network...");
    let session = zenoh::open(Config::default()).await.unwrap();

    // Create publishers for different data channels
    let sensor_pub = session
        .declare_publisher("sensors/data/*")
        .await
        .unwrap();
    let metrics_pub = session
        .declare_publisher("metrics/sensors/*")
        .await
        .unwrap();
    let health_pub = session
        .declare_publisher("health/sensors/*")
        .await
        .unwrap();

    println!("Sensor network started. Publishing 5 sensors...\n");

    let mut readings = 0;

    loop {
        // Simulate 5 sensors
        for sensor_id in 1..=5 {
            let temp = 20.0 + (sensor_id as f32 * 2.0) + (rand::random::<f32>() - 0.5) * 2.0;
            let humidity = 40.0 + (rand::random::<f32>() - 0.5) * 20.0;

            // Publish sensor data
            let sensor_data = json!({
                "sensor_id": sensor_id,
                "temperature": format!("{:.1}", temp),
                "humidity": format!("{:.0}", humidity),
                "timestamp": chrono::Utc::now().to_rfc3339()
            });
            sensor_pub.put(sensor_data.to_string()).await.unwrap();

            // Publish metrics
            let metrics = json!({
                "sensor_id": sensor_id,
                "samples": readings * 5 + sensor_id,
                "latency_ms": 5 + (rand::random::<u32>() % 10)
            });
            metrics_pub.put(metrics.to_string()).await.unwrap();

            // Publish health status
            let health = json!({
                "sensor_id": sensor_id,
                "status": if rand::random::<f32>() > 0.95 { "degraded" } else { "healthy" },
                "uptime_sec": readings * 5 + sensor_id
            });
            health_pub.put(health.to_string()).await.unwrap();

            println!(
                "[Sensor {}] Temp: {:.1}°C, Humidity: {:.0}%, Status: {}",
                sensor_id,
                temp,
                humidity,
                health["status"]
            );
        }

        readings += 1;
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
