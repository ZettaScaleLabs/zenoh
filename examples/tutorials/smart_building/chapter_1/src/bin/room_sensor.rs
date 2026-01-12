use std::time::Duration;
use zenoh::config::Config;

#[tokio::main]
async fn main() {
    env_logger::init();

    println!("Opening Zenoh session...");
    let session = zenoh::open(Config::default()).await.unwrap();

    println!("Declaring publisher for building/floor1/room_a/temperature");
    let publisher = session
        .declare_publisher("building/floor1/room_a/temperature")
        .await
        .unwrap();

    println!("Room A Sensor started. Publishing temperature readings...\n");

    let mut temperature = 22.0;
    for i in 0..20 {
        // Simulate temperature variations
        temperature += (rand::random::<f32>() - 0.5) * 0.2;

        let message = format!("{:.1}", temperature);
        println!(
            "[Room A Sensor] Publishing temperature: {}°C (reading #{})",
            message, i + 1
        );

        publisher.put(message).await.unwrap();

        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    println!("\nRoom A Sensor: Done publishing 20 readings.");
}
