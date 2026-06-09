//
// Copyright (c) 2026 ZettaScale Technology
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the Apache License, Version 2.0
// which is available at https://www.apache.org/licenses/LICENSE-2.0.
//
// SPDX-License-Identifier: EPL-2.0 OR Apache-2.0
//
// Contributors:
//   ZettaScale Zenoh Team, <zenoh@zettascale.tech>
//
//! Demonstrates the `TsStack` timestamp instrumentation feature.
//!
//! Publishes a few messages with Send+Route+Receive instrumentation enabled
//! and prints the per-message timestamp stack in the subscriber.
//!
//! Run with:
//! ```
//! cargo run --example z_timestamp_instrumentation --features unstable
//! ```

use std::time::Duration;

use zenoh::timestamp_stack::{InstrumentationTimestamp, TimestampInstrumentation};
use zenoh::Config;

fn ntp_to_ns(ntp: u64) -> u64 {
    let secs = ntp >> 32;
    let frac = ntp & 0xFFFF_FFFF;
    secs.saturating_mul(1_000_000_000) + ((frac * 1_000_000_000) >> 32)
}

#[tokio::main]
async fn main() {
    zenoh::init_log_from_env_or("error");

    let session = zenoh::open(Config::default()).await.unwrap();

    // Enable Send + Route + Receive instrumentation.
    let instr = TimestampInstrumentation::new(true, true, true).unwrap();

    let _sub = session
        .declare_subscriber("demo/timestamps/**")
        .callback(|sample| {
            let Some(stack) = sample.timestamp_stack() else {
                return;
            };
            println!("key: {}", sample.key_expr());
            for rec in stack.records() {
                match rec.timestamp() {
                    InstrumentationTimestamp::UHLC(ts) => {
                        let ns = ntp_to_ns(ts.get_time().as_u64());
                        println!("  {:?}  hlc_ns={}", rec.point(), ns);
                    }
                    InstrumentationTimestamp::Custom(bytes) => {
                        println!(
                            "  {:?}  custom  {} bytes",
                            rec.point(),
                            bytes.len()
                        );
                    }
                }
            }
            println!();
        })
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    for i in 0..3u32 {
        session
            .put(format!("demo/timestamps/{i}"), format!("payload-{i}"))
            .timestamp_instrumentation(Some(instr))
            .await
            .unwrap();
    }

    tokio::time::sleep(Duration::from_millis(200)).await;
}
