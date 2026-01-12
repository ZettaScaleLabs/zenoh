use zenoh::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("Opening Zenoh session...");
    let session = zenoh::open(Config::default()).await?;

    // Subscribe to ALL Room A sensors with wildcard
    println!("Subscribing to building/floor1/room_a/* (all sensors)\n");
    let mut subscriber = session
        .declare_subscriber("building/floor1/room_a/*")
        .await?;

    println!("Floor Monitor started. Listening to all Room A sensors...\n");

    let mut count = 0;
    while let Ok(sample) = subscriber.recv_async().await {
        let key = sample.key_expr.to_string();
        let value = String::from_utf8_lossy(&sample.payload);

        // Extract sensor type from key (last segment)
        let sensor_type = key.split('/').last().unwrap_or("unknown");

        match sensor_type {
            "temperature" => println!("🌡️  [Floor Monitor] Room A Temperature: {}°C", value),
            "humidity" => println!("💧 [Floor Monitor] Room A Humidity: {}%", value),
            "occupancy" => println!("👥 [Floor Monitor] Room A Occupancy: {} people", value),
            _ => println!("[Floor Monitor] {}: {}", sensor_type, value),
        }

        count += 1;
        if count >= 45 {
            // 15 readings * 3 sensors
            println!("\nFloor Monitor: Received 45 total readings. Exiting.");
            break;
        }
    }

    Ok(())
}
