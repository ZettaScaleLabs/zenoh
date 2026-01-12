use zenoh::config::Config;

#[tokio::main]
async fn main() {
    env_logger::init();

    println!("Opening Zenoh session...");
    let session = zenoh::open(Config::default()).await.unwrap();

    println!("Querying with selectors...\n");

    // Query with wildcard
    println!("[Query] Getting all room statuses with wildcard:");
    let results = session
        .get("building/floor1/*/status")
        .await
        .unwrap();

    let mut count = 0;
    while let Ok(reply) = results.recv_async().await {
        match reply.result() {
            Ok(sample) => {
                let key = sample.key_expr().to_string();
                let response = sample
                    .payload()
                    .try_to_string()
                    .unwrap_or_else(|_| "unknown".into());
                println!("  {} -> {}", key, response);
                count += 1;
            }
            Err(e) => println!("  Error: {}", e),
        }
    }

    println!("\nReceived {} responses.\n", count);

    // Another query with different selector
    println!("[Query] Querying single room:");
    let results = session
        .get("building/floor1/room_a/status")
        .await
        .unwrap();

    while let Ok(reply) = results.recv_async().await {
        match reply.result() {
            Ok(sample) => {
                let response = sample
                    .payload()
                    .try_to_string()
                    .unwrap_or_else(|_| "unknown".into());
                println!("  Response: {}", response);
            }
            Err(e) => println!("  Error: {}", e),
        }
    }
}
