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
use zenoh::{config::ZenohId, prelude::*};
use zenoh_examples::CommonArgs;

#[tokio::main]
async fn main() {
    initialize_logging();

    let config = parse_args();

    println!("Opening session...");
    let session = zenoh::open(config).await.unwrap();

    let info = session.info();
    println!("zid: {}", info.zid().await);
    println!(
        "routers zid: {:?}",
        info.routers_zid().await.collect::<Vec<ZenohId>>()
    );
    println!(
        "peers zid: {:?}",
        info.peers_zid().await.collect::<Vec<ZenohId>>()
    );
}

fn initialize_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init()
}

#[derive(clap::Parser, Clone, PartialEq, Eq, Hash, Debug)]
struct Args {
    #[command(flatten)]
    common: CommonArgs,
}

fn parse_args() -> zenoh::Config {
    let args = Args::parse();
    args.common.into()
}
