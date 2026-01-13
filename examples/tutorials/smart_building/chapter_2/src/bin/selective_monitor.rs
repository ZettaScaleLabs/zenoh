use zenoh::config::Config;

#[tokio::main]
async fn main() {
    env_logger::init();

    println!("Opening Zenoh session...");
    let session = zenoh::open(Config::default()).await.unwrap();

    // Subscribe to ALL temperatures across the building
    println!("Subscribing to building/*/*/temperature (all temps)\n");
    let subscriber = session
        .declare_subscriber("building/*/*/temperature")
        .await
        .unwrap();

    println!("Temperature Monitor started.\n");

    let mut count = 0;
    while let Ok(sample) = subscriber.recv_async().await {
        let key = sample.key_expr().to_string();
        let value = sample
            .payload()
            .try_to_string()
            .unwrap_or_else(|_| "unknown".into());

        println!("[Temp Monitor] {}: {}°C", key, value);

        count += 1;
        if count >= 30 {
            println!("\nTemperature Monitor: Done.");
            break;
        }
    }
}
