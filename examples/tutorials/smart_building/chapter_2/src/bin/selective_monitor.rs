use zenoh::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("Opening Zenoh session...");
    let session = zenoh::open(Config::default()).await?;

    // Subscribe to ALL temperatures across the building
    println!("Subscribing to building/*/*/temperature (all temps)\n");
    let mut subscriber = session
        .declare_subscriber("building/*/*/temperature")
        .await?;

    println!("Temperature Monitor started.\n");

    let mut count = 0;
    while let Ok(sample) = subscriber.recv_async().await {
        let key = sample.key_expr.to_string();
        let value = String::from_utf8_lossy(&sample.payload);

        println!("[Temp Monitor] {}: {}°C", key, value);

        count += 1;
        if count >= 30 {
            println!("\nTemperature Monitor: Done.");
            break;
        }
    }

    Ok(())
}
