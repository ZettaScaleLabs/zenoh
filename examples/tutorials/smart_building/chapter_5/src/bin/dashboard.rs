use zenoh::config::Config;

#[tokio::main]
async fn main() {
    env_logger::init();

    println!("Opening Zenoh session for dashboard...");
    let session = zenoh::open(Config::default()).await.unwrap();

    // Subscribe to metrics
    let mut metrics_sub = session
        .declare_subscriber("metrics/**")
        .await
        .unwrap();

    // Subscribe to health data
    let mut health_sub = session
        .declare_subscriber("health/**")
        .await
        .unwrap();

    println!("Dashboard started. Displaying data...\n");

    // Spawn health display task
    let health_task = {
        let health_sub_clone = health_sub.clone();
        tokio::spawn(async move {
            loop {
                if let Ok(sample) = health_sub_clone.recv_async().await {
                    if let Ok(data) = sample.payload().try_to_string() {
                        println!("[Dashboard] Health Update: {}", data);
                    }
                }
            }
        })
    };

    // Display metrics
    loop {
        if let Ok(sample) = metrics_sub.recv_async().await {
            if let Ok(data) = sample.payload().try_to_string() {
                println!("[Dashboard] Metric: {}", data);
            }
        }
    }
}
