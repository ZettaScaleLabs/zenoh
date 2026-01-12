use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use zenoh::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("Opening Zenoh session...");
    let session = zenoh::open(Config::default()).await?;

    // Room data
    let mut rooms: HashMap<String, (f32, f32)> = HashMap::new();
    rooms.insert("room_a".to_string(), (22.5, 45.0));
    rooms.insert("room_b".to_string(), (21.8, 42.0));
    rooms.insert("room_c".to_string(), (23.1, 48.0));
    let rooms = Arc::new(Mutex::new(rooms));

    println!("Declaring queryable for building/floor1/*/status\n");
    let mut queryable = session
        .declare_queryable("building/floor1/*/status")
        .await?;

    println!("Building Status Service started.\n");

    while let Ok(query) = queryable.recv_async().await {
        let selector = query.selector().to_string();
        println!("[Status Service] Received query: {}", selector);

        // Extract room name from selector
        // E.g., "building/floor1/room_a/status" -> "room_a"
        let room_name = selector
            .split('/')
            .nth(2)
            .unwrap_or("unknown")
            .to_string();

        let rooms_lock = rooms.lock().unwrap();
        match rooms_lock.get(&room_name) {
            Some((temp, humidity)) => {
                let response = format!(
                    r#"{{"room": "{}", "temperature": {:.1}, "humidity": {:.0}}}"#,
                    room_name, temp, humidity
                );
                println!("[Status Service] Sending: {}", response);
                query.reply(Ok(response.into())).await?;
            }
            None => {
                let error_msg = format!("Room {} not found", room_name);
                println!("[Status Service] Error: {}", error_msg);
                query.reply(Err(error_msg.into())).await?;
            }
        }
        println!();
    }

    Ok(())
}
