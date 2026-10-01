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
#![cfg(all(unix, feature = "shared-memory"))]

//! A process that exits with an open session must not panic when a peer
//! connects while its SHM statics are being finalized. The window is a few
//! hundred microseconds per exit, so many short lived peers run concurrently:
//! each new one scouts and connects to the others, some of which are exiting.

use std::{process::Command, time::Duration};

use zenoh::Config;

const PROBE: &str = "PROBE_SHM_EXIT_PEER";
const PEERS_MARKER: &str = "shm_exit_peer connected peers: ";
const CONCURRENCY: usize = 16;
const ITERATIONS: usize = 15;
// Peers only find each other through multicast scouting. Without it (no
// multicast on the runner, a restricted network namespace) the test would pass
// without ever staging the race, so require that most peers connected to
// another one.
const MIN_CONNECTED: usize = CONCURRENCY * ITERATIONS / 2;

fn config() -> Config {
    let mut config = Config::default();
    config.scouting.set_delay(Some(0)).unwrap();
    config
}

#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn shm_exit_peer() {
    if std::env::var(PROBE).is_err() {
        return;
    }
    let session = zenoh::open(config()).await.unwrap();
    let queryable = session
        .declare_queryable("shm_exit/service")
        .callback(|_| {})
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    let peers = session.info().peers_zid().await.count();
    println!("{PEERS_MARKER}{peers}");
    // Leak the session: the process exits with it open, like an application
    // that relies on process exit for cleanup.
    std::mem::forget(queryable);
    std::mem::forget(session);
}

/// Runs one peer and returns the number of peers it was connected to.
fn run_peer() -> Result<usize, String> {
    let output = Command::new(std::env::current_exe().unwrap())
        .arg("shm_exit_peer")
        .arg("--nocapture")
        .arg("--exact")
        .arg("--include-ignored")
        .env(PROBE, "true")
        .env("RUST_BACKTRACE", "1")
        .output()
        .expect("Failed to run peer in separate process");
    let stderr = String::from_utf8_lossy(&output.stderr);
    // The static_init access error is the failure this test exists for.
    // Other panics at exit, such as routing panics under peer churn, are
    // reported but do not fail the test, so that unrelated bugs do not
    // hide this one.
    if stderr.contains("AccessError") {
        return Err(format!("peer panicked at exit:\n{stderr}"));
    }
    // A peer that crashes (signal) or fails to open its session must fail the
    // test: it did not exercise the race. Panics in unrelated threads leave
    // the exit status at 0 and stay warnings.
    if !output.status.success() {
        return Err(format!("peer exited with {}:\n{stderr}", output.status));
    }
    if stderr.contains("panicked") {
        eprintln!("peer panicked at exit for another reason:\n{stderr}");
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        // libtest may print its own status text on the same line, so do not
        // require the marker at the start of the line.
        .find_map(|l| l.split_once(PEERS_MARKER))
        .and_then(|(_, n)| n.trim().parse().ok())
        .ok_or_else(|| format!("peer did not report its connected peers:\n{stderr}"))
}

#[test]
fn shm_exit_race() {
    let workers: Vec<_> = (0..CONCURRENCY)
        .map(|_| std::thread::spawn(|| (0..ITERATIONS).map(|_| run_peer()).collect::<Vec<_>>()))
        .collect();
    let results: Vec<Result<usize, String>> = workers
        .into_iter()
        .flat_map(|w| w.join().unwrap())
        .collect();
    let connected = results
        .iter()
        .filter(|r| matches!(r, Ok(n) if *n > 0))
        .count();
    let failures: Vec<String> = results.into_iter().filter_map(Result::err).collect();
    assert!(
        failures.is_empty(),
        "{} of {} peers panicked or crashed at exit, first:\n{}",
        failures.len(),
        CONCURRENCY * ITERATIONS,
        failures[0]
    );
    assert!(
        connected >= MIN_CONNECTED,
        "only {} of {} peers connected to another peer, the race was not exercised",
        connected,
        CONCURRENCY * ITERATIONS
    );
}
