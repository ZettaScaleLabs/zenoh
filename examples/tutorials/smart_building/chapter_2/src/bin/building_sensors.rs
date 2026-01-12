use std::time::Duration;
use zenoh::config::Config;

#[tokio::main]
async fn main() {
    env_logger::init();

    println!("Opening Zenoh session...");
    let session = zenoh::open(Config::default()).await.unwrap();

    // Publishers for Room A
    let pub_a_temp = session
        .declare_publisher("building/floor1/room_a/temperature")
        .await
        .unwrap();
    let pub_a_humidity = session
        .declare_publisher("building/floor1/room_a/humidity")
        .await
        .unwrap();

    // Publishers for Room B
    let pub_b_temp = session
        .declare_publisher("building/floor1/room_b/temperature")
        .await
        .unwrap();
    let pub_b_humidity = session
        .declare_publisher("building/floor1/room_b/humidity")
        .await
        .unwrap();

    // Publishers for Room C (Floor 2)
    let pub_c_temp = session
        .declare_publisher("building/floor2/room_c/temperature")
        .await
        .unwrap();
    let pub_c_humidity = session
        .declare_publisher("building/floor2/room_c/humidity")
        .await
        .unwrap();

    println!("Building Sensors started (3 rooms, 2 sensors each).\n");

    let mut a_temp = 22.0;
    let mut b_temp = 21.5;
    let mut c_temp = 23.0;

    for i in 0..10 {
        a_temp += (rand::random::<f32>() - 0.5) * 0.4;
        b_temp += (rand::random::<f32>() - 0.5) * 0.4;
        c_temp += (rand::random::<f32>() - 0.5) * 0.4;

        println!("[Building Sensors] Publishing round #{}", i + 1);

        pub_a_temp.put(format!("{:.1}", a_temp)).await.unwrap();
        pub_a_humidity.put("42").await.unwrap();

        pub_b_temp.put(format!("{:.1}", b_temp)).await.unwrap();
        pub_b_humidity.put("45").await.unwrap();

        pub_c_temp.put(format!("{:.1}", c_temp)).await.unwrap();
        pub_c_humidity.put("48").await.unwrap();

        println!(
            "  Room A: {:.1}°C, Room B: {:.1}°C, Room C: {:.1}°C\n",
            a_temp, b_temp, c_temp
        );

        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    println!("Building Sensors: Done.");
}
