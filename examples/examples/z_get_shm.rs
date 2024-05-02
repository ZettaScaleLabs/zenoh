//
// Copyright (c) 2023 ZettaScale Technology
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
use clap::Parser;
use std::time::Duration;
use zenoh::prelude::*;
use zenoh_examples::CommonArgs;

const N: usize = 10;

#[tokio::main]
async fn main() {
    // initiate logging
    zenoh_util::try_init_log_from_env();

    let (mut config, selector, mut value, target, timeout) = parse_args();

    // A probing procedure for shared memory is performed upon session opening. To enable `z_pub_shm` to operate
    // over shared memory (and to not fallback on network mode), shared memory needs to be enabled also on the
    // subscriber side. By doing so, the probing procedure will succeed and shared memory will operate as expected.
    config.transport.shared_memory.set_enabled(true).unwrap();

    println!("Opening session...");
    let session = zenoh::open(config).await.unwrap();

    println!("Creating POSIX SHM provider...");
    // create an SHM backend...
    // NOTE: For extended PosixSharedMemoryProviderBackend API please check z_posix_shm_provider.rs
    let backend = PosixSharedMemoryProviderBackend::builder()
        .with_size(N * 1024)
        .unwrap()
        .res()
        .unwrap();
    // ...and an SHM provider
    let provider = SharedMemoryProviderBuilder::builder()
        .protocol_id::<POSIX_PROTOCOL_ID>()
        .backend(backend)
        .res();

    // Allocate an SHM buffer
    // NOTE: For allocation API please check z_alloc_shm.rs example
    // NOTE: For buf's API please check z_bytes_shm.rs example
    println!("Allocating Shared Memory Buffer...");
    let mut sbuf = provider
        .alloc_layout()
        .size(1024)
        .res()
        .unwrap()
        .alloc()
        .with_policy::<BlockOn<GarbageCollect>>()
        .res_async()
        .await
        .unwrap();

    let content = value
        .take()
        .unwrap_or_else(|| "Get from SharedMemory Rust!".to_string());
    sbuf[0..content.len()].copy_from_slice(content.as_bytes());

    println!("Sending Query '{selector}'...");
    let replies = session
        .get(&selector)
        .value(sbuf)
        .target(target)
        .timeout(timeout)
        .await
        .unwrap();

    while let Ok(reply) = replies.recv_async().await {
        match reply.result() {
            Ok(sample) => {
                print!(">> Received ('{}': ", sample.key_expr().as_str());
                match sample.payload().deserialize::<&zshm>() {
                    Ok(payload) => println!("'{}')", String::from_utf8_lossy(payload),),
                    Err(e) => println!("'Not a SharedMemoryBuf: {:?}')", e),
                }
            }
            Err(err) => {
                let payload = err
                    .payload()
                    .deserialize::<String>()
                    .unwrap_or_else(|e| format!("{}", e));
                println!(">> Received (ERROR: '{}')", payload);
            }
        }
    }
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
#[value(rename_all = "SCREAMING_SNAKE_CASE")]
enum Qt {
    BestMatching,
    All,
    AllComplete,
}

#[derive(Parser, Clone, Debug)]
struct Args {
    #[arg(short, long, default_value = "demo/example/**")]
    /// The selection of resources to query
    selector: Selector<'static>,
    /// The value to publish.
    value: Option<String>,
    #[arg(short, long, default_value = "BEST_MATCHING")]
    /// The target queryables of the query.
    target: Qt,
    #[arg(short = 'o', long, default_value = "10000")]
    /// The query timeout in milliseconds.
    timeout: u64,
    #[command(flatten)]
    common: CommonArgs,
}

fn parse_args() -> (
    Config,
    Selector<'static>,
    Option<String>,
    QueryTarget,
    Duration,
) {
    let args = Args::parse();
    (
        args.common.into(),
        args.selector,
        args.value,
        match args.target {
            Qt::BestMatching => QueryTarget::BestMatching,
            Qt::All => QueryTarget::All,
            Qt::AllComplete => QueryTarget::AllComplete,
        },
        Duration::from_millis(args.timeout),
    )
}