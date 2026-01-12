use zenoh::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("Opening Zenoh session...");
    let session = zenoh::open(Config::default()).await?;

    println!("Querying with selectors...\n");

    // Query with wildcard
    println!("[Query] Getting all room statuses with wildcard:");
    let results = session
        .get("building/floor1/*/status")
        .await?;

    let mut count = 0;
    while let Ok(reply) = results.recv_async().await {
        match reply.sample {
            Ok(sample) => {
                let key = sample.key_expr.to_string();
                let response = String::from_utf8_lossy(&sample.payload);
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
        .await?;

    while let Ok(reply) = results.recv_async().await {
        match reply.sample {
            Ok(sample) => {
                let response = String::from_utf8_lossy(&sample.payload);
                println!("  Response: {}", response);
            }
            Err(e) => println!("  Error: {}", e),
        }
    }

    Ok(())
}
