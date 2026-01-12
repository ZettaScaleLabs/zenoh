use zenoh::config::Config;

#[tokio::main]
async fn main() {
    env_logger::init();

    println!("Loading client configuration from client_config.json5...");
    let config = Config::from_file("client_config.json5").unwrap();

    println!("Opening Zenoh session as CLIENT...");
    let session = zenoh::open(config).await.unwrap();

    println!("Connected to Zenoh router!");
    println!("Subscribing to building/floor1/room_a/temperature\n");
    
    let mut subscriber = session
        .declare_subscriber("building/floor1/room_a/temperature")
        .await
        .unwrap();

    println!("Monitor started. Waiting for temperature readings...\n");

    let mut count = 0;
    while let Ok(sample) = subscriber.recv_async().await {
        let temperature = sample
            .payload()
            .try_to_string()
            .unwrap_or_else(|_| "unknown".into());
        println!("[Monitor] Temperature: {}°C", temperature);

        count += 1;
        if count >= 20 {
            println!("\nMonitor: Received 20 readings. Exiting.");
            break;
        }
    }
}
