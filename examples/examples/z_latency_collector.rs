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
//! Demonstrates per-hop latency profiling using `TsStack`.
//!
//! Publishes [`N_SAMPLES`] messages with Send+Route+Receive instrumentation,
//! collects `TsStack` records in a subscriber, and prints p50/p95/p99 latency
//! for each hop (Send→Receive, Send→Route, Route→Receive).
//!
//! Route timestamps only appear when messages pass through a Zenoh router.
//! In the default loopback configuration only Send→Receive is populated.
//!
//! Run with:
//! ```
//! cargo run --example z_latency_collector --features unstable
//! ```

use std::sync::{Arc, Mutex};
use std::time::Duration;

use zenoh::timestamp_stack::{InstrumentationTimestamp, InterceptionPoint, TimestampInstrumentationBuilder};
use zenoh::Config;

const N_SAMPLES: usize = 500;

// NTP64 layout: upper 32 bits = seconds since epoch, lower 32 bits = sub-second fraction.
fn ntp_to_ns(ntp: u64) -> u64 {
    let secs = ntp >> 32;
    let frac = ntp & 0xFFFF_FFFF;
    secs.saturating_mul(1_000_000_000) + ((frac * 1_000_000_000) >> 32)
}

fn percentile(sorted: &[i64], p: f64) -> i64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() as f64 - 1.0) * p / 100.0).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

#[derive(Default)]
struct HopAccumulator {
    send_receive: Vec<i64>,
    send_route: Vec<i64>,
    route_receive: Vec<i64>,
}

impl HopAccumulator {
    fn ingest(&mut self, stack: &zenoh::timestamp_stack::TimestampStack) {
        let mut send_ns: Option<u64> = None;
        let mut route_ns: Option<u64> = None;
        let mut receive_ns: Option<u64> = None;

        for rec in stack.records() {
            let InstrumentationTimestamp::UHLC(ts) = rec.timestamp() else {
                continue;
            };
            let ns = ntp_to_ns(ts.get_time().as_u64());
            match rec.point() {
                InterceptionPoint::Send => send_ns = Some(ns),
                InterceptionPoint::Route => route_ns = Some(ns),
                InterceptionPoint::Receive => receive_ns = Some(ns),
                _ => {}
            }
        }

        if let (Some(s), Some(r)) = (send_ns, receive_ns) {
            self.send_receive.push(r as i64 - s as i64);
        }
        if let (Some(s), Some(r)) = (send_ns, route_ns) {
            self.send_route.push(r as i64 - s as i64);
        }
        if let (Some(r), Some(rcv)) = (route_ns, receive_ns) {
            self.route_receive.push(rcv as i64 - r as i64);
        }
    }

    fn print_hop(label: &str, data: &[i64]) {
        if data.is_empty() {
            return;
        }
        let mut sorted = data.to_vec();
        sorted.sort_unstable();
        println!(
            "  {:<22}  p50={:>7} µs   p95={:>7} µs   p99={:>7} µs   n={}",
            label,
            percentile(&sorted, 50.0) / 1_000,
            percentile(&sorted, 95.0) / 1_000,
            percentile(&sorted, 99.0) / 1_000,
            sorted.len(),
        );
    }

    fn print_stats(&self) {
        Self::print_hop("Send → Receive", &self.send_receive);
        Self::print_hop("Send → Route", &self.send_route);
        Self::print_hop("Route → Receive", &self.route_receive);
        if self.send_route.is_empty() {
            println!("  (no Route records — run via a router to see per-hop breakdown)");
        }
    }
}

#[tokio::main]
async fn main() {
    zenoh::init_log_from_env_or("error");

    println!(
        "Collecting {} samples with Send+Route+Receive instrumentation...\n",
        N_SAMPLES
    );

    let session = zenoh::open(Config::default()).await.unwrap();
    let instr = TimestampInstrumentationBuilder::new()
        .set_send(true)
        .set_route(true)
        .set_receive(true)
        .build()
        .unwrap();

    let acc: Arc<Mutex<HopAccumulator>> = Arc::new(Mutex::new(HopAccumulator::default()));
    let acc2 = acc.clone();

    let _sub = session
        .declare_subscriber("demo/latency/**")
        .callback(move |sample| {
            if let Some(stack) = sample.timestamp_stack() {
                acc2.lock().unwrap().ingest(stack);
            }
        })
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    for i in 0..N_SAMPLES {
        session
            .put(format!("demo/latency/{i}"), "payload")
            .timestamp_instrumentation(Some(instr))
            .await
            .unwrap();
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    let received = acc.lock().unwrap().send_receive.len();
    println!("Received {}/{} samples. Per-hop latency:\n", received, N_SAMPLES);
    acc.lock().unwrap().print_stats();
}
