use zenoh::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("Opening Zenoh session...");
    let session = zenoh::open(Config::default()).await?;

    println!("Subscribing to building/floor1/room_a/temperature\n");
    let mut subscriber = session
        .declare_subscriber("building/floor1/room_a/temperature")
        .await?;

    println!("Monitor started. Waiting for temperature readings...\n");

    let mut count = 0;
    while let Ok(sample) = subscriber.recv_async().await {
        let temperature = String::from_utf8_lossy(&sample.payload);
        println!("[Monitor] Room A Temperature: {}°C", temperature);

        count += 1;
        if count >= 20 {
            println!("\nMonitor: Received 20 readings. Exiting.");
            break;
        }
    }

    Ok(())
}
