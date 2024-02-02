use std::sync::{Arc, Mutex};

struct IntervalCounter {
    first_tick: bool,
    last_time: std::time::Instant,
    count: u32,
    total_time: std::time::Duration,
}

impl IntervalCounter {
    fn new() -> IntervalCounter {
        IntervalCounter {
            first_tick: true,
            last_time: std::time::Instant::now(),
            count: 0,
            total_time: std::time::Duration::from_secs(0),
        }
    }

    fn tick(&mut self) {
        let curr_time = std::time::Instant::now();
        if self.first_tick {
            self.first_tick = false;
        } else {
            self.total_time += curr_time - self.last_time;
            self.count += 1;
        }
        self.last_time = curr_time;
    }

    fn get_middle(&self) -> u32 {
        self.total_time.as_millis() as u32 / self.count
    }

    fn check_middle(&self, ms: u32) {
        let middle = self.get_middle();
        println!("Interval {}, count: {}, middle: {}", ms, self.count, middle);
        assert!(middle + 1 >= ms);
    }
}

#[test]
fn downsampling_by_keyexpr() {
    let _ = env_logger::builder().is_test(true).try_init();

    use zenoh::prelude::sync::*;

    // declare subscriber
    let zenoh_sub = zenoh::open(Config::default()).res().unwrap();

    let counter_r100 = Arc::new(Mutex::new(IntervalCounter::new()));
    let counter_r100_clone = counter_r100.clone();
    let counter_r50 = Arc::new(Mutex::new(IntervalCounter::new()));
    let counter_r50_clone = counter_r50.clone();

    let total_count = Arc::new(Mutex::new(0));
    let total_count_clone = total_count.clone();

    let _sub = zenoh_sub
        .declare_subscriber("test/downsamples_by_keyexp/*")
        .callback(move |sample| {
            let mut count = total_count_clone.lock().unwrap();
            *count += 1;
            if sample.key_expr.as_str() == "test/downsamples_by_keyexp/r100" {
                counter_r100.lock().unwrap().tick();
            } else if sample.key_expr.as_str() == "test/downsamples_by_keyexp/r50" {
                counter_r50.lock().unwrap().tick();
            }
        })
        .res()
        .unwrap();

    // declare publisher
    let mut config = Config::default();
    config
        .insert_json5(
            "downsampling/downsamples",
            r#"
              [
                {
                  keyexprs: ["test/downsamples_by_keyexp/r100"],
                  threshold_ms: 100,
                },
                {
                  keyexprs: ["test/downsamples_by_keyexp/r50"],
                  threshold_ms: 50,
                },
              ]
            "#,
        )
        .unwrap();
    let zenoh_pub = zenoh::open(config).res().unwrap();
    let publisher_r100 = zenoh_pub
        .declare_publisher("test/downsamples_by_keyexp/r100")
        .res()
        .unwrap();

    let publisher_r50 = zenoh_pub
        .declare_publisher("test/downsamples_by_keyexp/r50")
        .res()
        .unwrap();

    let publisher_all = zenoh_pub
        .declare_publisher("test/downsamples_by_keyexp/all")
        .res()
        .unwrap();

    let interval = std::time::Duration::from_millis(1);
    let messages_count = 1000;
    for i in 0..messages_count {
        println!("message {}", i);
        publisher_r100.put(format!("message {}", i)).res().unwrap();
        publisher_r50.put(format!("message {}", i)).res().unwrap();
        publisher_all.put(format!("message {}", i)).res().unwrap();

        std::thread::sleep(interval);
    }

    for _ in 0..100 {
        if *(total_count.lock().unwrap()) >= messages_count {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert!(*(total_count.lock().unwrap()) >= messages_count);

    counter_r50_clone.lock().unwrap().check_middle(50);
    counter_r100_clone.lock().unwrap().check_middle(100);
}

#[cfg(unix)]
#[test]
fn downsampling_by_interface() {
    let _ = env_logger::builder().is_test(true).try_init();

    use zenoh::prelude::sync::*;

    // declare subscriber
    let mut config_sub = Config::default();
    config_sub
        .insert_json5("listen/endpoints", r#"["tcp/127.0.0.1:7447"]"#)
        .unwrap();
    let zenoh_sub = zenoh::open(config_sub).res().unwrap();

    let counter_r100 = Arc::new(Mutex::new(IntervalCounter::new()));
    let counter_r100_clone = counter_r100.clone();

    let total_count = Arc::new(Mutex::new(0));
    let total_count_clone = total_count.clone();

    let _sub = zenoh_sub
        .declare_subscriber("test/downsamples_by_interface/*")
        .callback(move |sample| {
            let mut count = total_count_clone.lock().unwrap();
            *count += 1;
            if sample.key_expr.as_str() == "test/downsamples_by_interface/r100" {
                counter_r100.lock().unwrap().tick();
            }
        })
        .res()
        .unwrap();

    // declare publisher
    let mut config_pub = Config::default();
    config_pub
        .insert_json5("connect/endpoints", r#"["tcp/127.0.0.1:7447"]"#)
        .unwrap();
    config_pub
        .insert_json5(
            "downsampling/downsamples",
            r#"
              [
                {
                  keyexprs: ["test/downsamples_by_interface/r100"],
                  interfaces: ["lo", "lo0"],
                  threshold_ms: 100,
                },
                {
                  keyexprs: ["test/downsamples_by_interface/all"],
                  interfaces: ["some_unknown_interface"],
                  threshold_ms: 100,
                },
              ]
            "#,
        )
        .unwrap();

    let zenoh_pub = zenoh::open(config_pub).res().unwrap();
    let publisher_r100 = zenoh_pub
        .declare_publisher("test/downsamples_by_interface/r100")
        .res()
        .unwrap();

    let publisher_all = zenoh_pub
        .declare_publisher("test/downsamples_by_interface/all")
        .res()
        .unwrap();

    let interval = std::time::Duration::from_millis(1);
    let messages_count = 1000;
    for i in 0..messages_count {
        println!("message {}", i);
        publisher_r100.put(format!("message {}", i)).res().unwrap();
        publisher_all.put(format!("message {}", i)).res().unwrap();

        std::thread::sleep(interval);
    }

    for _ in 0..100 {
        if *(total_count.lock().unwrap()) >= messages_count {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert!(*(total_count.lock().unwrap()) >= messages_count);

    counter_r100_clone.lock().unwrap().check_middle(100);
}
