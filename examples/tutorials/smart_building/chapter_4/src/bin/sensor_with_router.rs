use std::time::Duration;
use zenoh::config::Config;

#[tokio::main]
async fn main() {
    env_logger::init();

    println!("Loading client configuration from client_config.json5...");
    
    // Load configuration from file
    let config = Config::from_file("client_config.json5").unwrap();
    
    println!("Opening Zenoh session as CLIENT...");
    let session = zenoh::open(config).await.unwrap();
    
    println!("Connected to Zenoh router!");
    println!("Declaring publisher for building/floor1/room_a/temperature\n");
    
    let publisher = session
        .declare_publisher("building/floor1/room_a/temperature")
        .await
        .unwrap();

    println!("Sensor started. Publishing temperature readings...\n");

    let mut temperature = 22.0;
    for i in 0..20 {
        temperature += (rand::random::<f32>() - 0.5) * 0.2;

        let message = format!("{:.1}", temperature);
        println!("[Sensor] Publishing: {}°C (reading #{})", message, i + 1);

        publisher.put(message).await.unwrap();

        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    println!("\nSensor: Done publishing 20 readings.");
}
