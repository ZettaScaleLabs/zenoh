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
//! Demonstrates the proprietary 24-byte token pattern via `SessionTimestampCallback`.
//!
//! The token embeds hardware nanoseconds, a per-session sequence counter, a node
//! fingerprint derived from the session ZenohId, and a checksum. Generic tooling sees
//! `is_custom = true` and skips these records; only code that holds the `decode_token`
//! function can extract timing data.
//!
//! This is the "data-layer moat" pattern: the wire transport is open source, but only
//! the SDK that knows the proprietary format produces actionable analytics.
//!
//! In production, replace the additive checksum with HMAC-SHA256 (truncated to 4 bytes),
//! keyed per customer deployment. That makes each record tamper-evident — a malicious
//! intermediary node cannot forge a valid Send timestamp.
//!
//! Run with:
//! ```
//! cargo run --example z_proprietary_token --features unstable
//! ```

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use zenoh::timestamp_stack::{
    InstrumentationTimestamp, InterceptionPoint, SessionTimestampCallback,
    TimestampInstrumentation, TsStackContext,
};
use zenoh::Config;

// ── Token layout (24 bytes) ───────────────────────────────────────────────────
// [ 0.. 8]  hardware_ns   u64 LE — wall-clock ns (swap for PTP clock in production)
// [ 8..16]  sequence      u64 LE — monotonic per-session counter
// [16..20]  fingerprint   4 bytes — first 4 bytes of session ZenohId (LE)
// [20..24]  checksum      u32 LE — wrapping sum of bytes [0..20]
//
// PRODUCTION NOTE: bytes [20..24] should be HMAC-SHA256(key, bytes[0..20])[..4].
// The key is provisioned per customer at SDK license issuance time.

#[derive(Debug)]
struct ProprietaryToken {
    hardware_ns: u64,
    sequence: u64,
    node_fingerprint: [u8; 4],
}

fn build_callback() -> SessionTimestampCallback {
    let seq = Arc::new(AtomicU64::new(0));
    Arc::new(move |ctx: TsStackContext| {
        let hardware_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        let sequence = seq.fetch_add(1, Ordering::Relaxed);
        let zid_bytes = ctx.zid.to_le_bytes();

        let mut buf = [0u8; 24];
        buf[0..8].copy_from_slice(&hardware_ns.to_le_bytes());
        buf[8..16].copy_from_slice(&sequence.to_le_bytes());
        buf[16..20].copy_from_slice(&zid_bytes[..4]);
        // additive checksum over the first 20 bytes
        let checksum = buf[..20]
            .iter()
            .fold(0u32, |acc, &b| acc.wrapping_add(b as u32));
        buf[20..24].copy_from_slice(&checksum.to_le_bytes());
        buf.to_vec()
    })
}

fn decode_token(bytes: &[u8]) -> Option<ProprietaryToken> {
    if bytes.len() != 24 {
        return None;
    }
    let expected = bytes[..20]
        .iter()
        .fold(0u32, |acc, &b| acc.wrapping_add(b as u32));
    let stored = u32::from_le_bytes(bytes[20..24].try_into().ok()?);
    if expected != stored {
        return None; // tampered or wrong format
    }
    Some(ProprietaryToken {
        hardware_ns: u64::from_le_bytes(bytes[0..8].try_into().ok()?),
        sequence: u64::from_le_bytes(bytes[8..16].try_into().ok()?),
        node_fingerprint: bytes[16..20].try_into().ok()?,
    })
}

fn print_token_record(rec: &zenoh::timestamp_stack::TimestampStackRecord) {
    match rec.timestamp() {
        InstrumentationTimestamp::Custom(bytes) => match decode_token(bytes) {
            Some(tok) => println!(
                "  {:?}  custom  hw_ns={:<20}  seq={:<5}  node={:08x}",
                rec.point(),
                tok.hardware_ns,
                tok.sequence,
                u32::from_le_bytes(tok.node_fingerprint),
            ),
            None => println!("  {:?}  custom  [unrecognised format or tampered]", rec.point()),
        },
        InstrumentationTimestamp::UHLC(ts) => println!("  {:?}  hlc={}", rec.point(), ts),
    }
}

#[tokio::main]
async fn main() {
    zenoh::init_log_from_env_or("error");

    println!("── Proprietary 24-byte token via SessionTimestampCallback ───────────────\n");

    // ── Session with custom callback ──────────────────────────────────────────
    let session = zenoh::open(Config::default())
        .with_timestamp_callback(build_callback())
        .await
        .unwrap();

    let instr = TimestampInstrumentation::new(true, false, true).unwrap();

    let samples: Arc<Mutex<Vec<_>>> = Arc::new(Mutex::new(vec![]));
    let samples2 = samples.clone();
    let _sub = session
        .declare_subscriber("demo/token/**")
        .callback(move |s| samples2.lock().unwrap().push(s))
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    for i in 0..3u32 {
        session
            .put(format!("demo/token/{i}"), format!("message-{i}"))
            .timestamp_instrumentation(Some(instr))
            .await
            .unwrap();
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    println!("Received samples (decoded with proprietary format):\n");
    for sample in samples.lock().unwrap().iter() {
        let Some(stack) = sample.timestamp_stack() else {
            continue;
        };
        println!("key: {}", sample.key_expr());
        for rec in stack.records() {
            print_token_record(rec);
        }
        println!();
    }

    // ── Demonstrate what generic tooling sees ─────────────────────────────────
    println!("── What generic tooling sees (no decode key) ────────────────────────────\n");
    for sample in samples.lock().unwrap().iter() {
        let Some(stack) = sample.timestamp_stack() else {
            continue;
        };
        println!("key: {}", sample.key_expr());
        for rec in stack.records() {
            match rec.timestamp() {
                InstrumentationTimestamp::Custom(bytes) => println!(
                    "  {:?}  is_custom=true  raw_bytes=[{}]  (skipped — unknown format)",
                    rec.point(),
                    bytes
                        .iter()
                        .take(8)
                        .map(|b| format!("{b:02x}"))
                        .collect::<Vec<_>>()
                        .join(" "),
                ),
                InstrumentationTimestamp::UHLC(ts) => println!("  {:?}  hlc={}", rec.point(), ts),
            }
        }
        println!();
    }

    // ── Side-by-side latency from custom timestamps ───────────────────────────
    println!("── Send→Receive latency from proprietary hardware_ns field ─────────────\n");
    {
        let guard = samples.lock().unwrap();
        let mut latencies: Vec<i64> = Vec::new();
        for sample in guard.iter() {
            let Some(stack) = sample.timestamp_stack() else {
                continue;
            };
            let mut send_ns: Option<u64> = None;
            let mut receive_ns: Option<u64> = None;
            for rec in stack.records() {
                let InstrumentationTimestamp::Custom(bytes) = rec.timestamp() else {
                    continue;
                };
                if let Some(tok) = decode_token(bytes) {
                    match rec.point() {
                        InterceptionPoint::Send => send_ns = Some(tok.hardware_ns),
                        InterceptionPoint::Receive => receive_ns = Some(tok.hardware_ns),
                        _ => {}
                    }
                }
            }
            if let (Some(s), Some(r)) = (send_ns, receive_ns) {
                latencies.push(r as i64 - s as i64);
            }
        }
        for (i, lat_ns) in latencies.iter().enumerate() {
            println!("  message-{i}  Send→Receive = {} µs", lat_ns / 1_000);
        }
    }
}
