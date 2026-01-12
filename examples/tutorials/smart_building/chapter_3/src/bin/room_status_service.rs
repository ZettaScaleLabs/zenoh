use std::sync::{Arc, Mutex};
use std::time::Duration;
use zenoh::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("Opening Zenoh session...");
    let session = zenoh::open(Config::default()).await?;

    // Simulated room state
    let state = Arc::new(Mutex::new((22.5, 45.0, 2)));

    println!("Declaring queryable for building/floor1/room_a/status\n");
    let mut queryable = session
        .declare_queryable("building/floor1/room_a/status")
        .await?;

    println!("Room A Status Service started. Waiting for queries...\n");

    while let Ok(query) = queryable.recv_async().await {
        let (temp, humidity, occupancy) = *state.lock().unwrap();

        // Build JSON response
        let response = format!(
            r#"{{"temperature": {:.1}, "humidity": {:.0}, "occupancy": {}}}"#,
            temp, humidity, occupancy
        );

        println!("[Status Service] Received query: {}", query.selector());
        println!("[Status Service] Sending response: {}", response);

        query.reply(Ok(response.into())).await?;

        // Simulate changing state
        *state.lock().unwrap() = (
            temp + (rand::random::<f32>() - 0.5) * 0.2,
            (humidity + (rand::random::<f32>() - 0.5) * 2.0).max(0.0).min(100.0),
            (rand::random::<f32>() * 5.0) as i32,
        );

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    Ok(())
}
