use std::time::Duration;
use zenoh::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("Opening Zenoh session...");
    let session = zenoh::open(Config::default()).await?;

    // Declare publishers for different sensors in Room A
    println!("Declaring publishers for temperature, humidity, and occupancy...");
    let pub_temp = session
        .declare_publisher("building/floor1/room_a/temperature")
        .await?;
    let pub_humidity = session
        .declare_publisher("building/floor1/room_a/humidity")
        .await?;
    let pub_occupancy = session
        .declare_publisher("building/floor1/room_a/occupancy")
        .await?;

    println!("Multi-Sensor Publisher started.\n");

    let mut temp = 22.0;
    let mut humidity = 45.0;

    for i in 0..15 {
        // Simulate sensor readings
        temp += (rand::random::<f32>() - 0.5) * 0.4;
        humidity += (rand::random::<f32>() - 0.5) * 2.0;
        let occupancy = (rand::random::<f32>() * 5.0) as u32;

        println!("[Room A Sensors] Publishing reading #{}", i + 1);
        println!("  Temperature: {:.1}°C", temp);
        println!("  Humidity: {:.0}%", humidity.max(0.0).min(100.0));
        println!("  Occupancy: {} people\n", occupancy);

        pub_temp.put(format!("{:.1}", temp)).await?;
        pub_humidity.put(format!("{:.0}", humidity)).await?;
        pub_occupancy.put(occupancy.to_string()).await?;

        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    println!("Multi-Sensor Publisher: Done.");
    Ok(())
}
