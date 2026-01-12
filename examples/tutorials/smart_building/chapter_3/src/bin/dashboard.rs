use std::time::Duration;
use zenoh::config::Config;

#[tokio::main]
async fn main() {
    env_logger::init();

    println!("Opening Zenoh session...");
    let session = zenoh::open(Config::default()).await.unwrap();

    println!("Dashboard ready. Querying room status...\n");

    for i in 1..=5 {
        println!("--- Query {} ---", i);
        println!("[Dashboard] Querying: building/floor1/room_a/status");

        let results = session
            .get("building/floor1/room_a/status")
            .await
            .unwrap();

        let mut found = false;
        while let Ok(reply) = results.recv_async().await {
            match reply.result() {
                Ok(sample) => {
                    let response = sample
                        .payload()
                        .try_to_string()
                        .unwrap_or_else(|_| "unknown".into());
                    println!("[Dashboard] Response: {}\n", response);
                    found = true;
                }
                Err(e) => {
                    println!("[Dashboard] Error: {}\n", e);
                }
            }
        }

        if !found {
            println!("[Dashboard] No response received (service may be offline)\n");
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    println!("Dashboard: Done with 5 queries.");
}
