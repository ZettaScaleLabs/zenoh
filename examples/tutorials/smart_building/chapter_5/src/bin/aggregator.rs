use zenoh::config::Config;
use serde_json::json;

#[tokio::main]
async fn main() {
    env_logger::init();

    println!("Opening Zenoh session for aggregator...");
    let session = zenoh::open(Config::default()).await.unwrap();

    // Subscribe to sensor data
    let mut sensor_sub = session
        .declare_subscriber("sensors/data/*")
        .await
        .unwrap();

    // Declare queryable for aggregated status
    let mut status_queryable = session
        .declare_queryable("services/aggregator/status")
        .await
        .unwrap();

    // Publish aggregated metrics
    let metrics_pub = session
        .declare_publisher("metrics/aggregated")
        .await
        .unwrap();

    println!("Aggregator service started. Waiting for sensor data...\n");

    let mut sensor_count = 0;
    let mut total_temp = 0.0;
    let mut total_humidity = 0.0;

    // Spawn query handler
    let session_clone = session.clone();
    tokio::spawn(async move {
        loop {
            if let Ok(query) = status_queryable.recv_async().await {
                let status = json!({
                    "service": "aggregator",
                    "status": "running",
                    "sensors_active": sensor_count,
                    "last_update": chrono::Utc::now().to_rfc3339()
                });

                query
                    .reply(query.key_expr().clone(), status.to_string())
                    .await
                    .unwrap();
            }
        }
    });

    // Process sensor data
    loop {
        if let Ok(sample) = sensor_sub.recv_async().await {
            if let Ok(data_str) = sample.payload().try_to_string() {
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&data_str) {
                    sensor_count = (sensor_count + 1) % 100;

                    // Parse and aggregate
                    if let (Some(temp), Some(humidity)) = (data["temperature"].as_str(), data["humidity"].as_str())
                    {
                        if let (Ok(t), Ok(h)) = (temp.parse::<f32>(), humidity.parse::<f32>()) {
                            total_temp += t;
                            total_humidity += h;

                            // Publish aggregated metric every 5 readings
                            if sensor_count % 5 == 0 {
                                let avg = json!({
                                    "avg_temperature": format!("{:.1}", total_temp / 5.0),
                                    "avg_humidity": format!("{:.0}", total_humidity / 5.0),
                                    "sample_count": sensor_count
                                });
                                metrics_pub.put(avg.to_string()).await.unwrap();

                                println!("[Aggregator] Updated metrics - Avg Temp: {:.1}°C, Avg Humidity: {:.0}%",
                                    total_temp / 5.0, total_humidity / 5.0);

                                total_temp = 0.0;
                                total_humidity = 0.0;
                            }
                        }
                    }
                }
            }
        }
    }
}
