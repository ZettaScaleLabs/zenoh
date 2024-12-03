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
use std::time::Duration;

use clap::Parser;
use zenoh::{bytes::Encoding, key_expr::KeyExpr, Config};
use zenoh_examples::CommonArgs;

#[tokio::main]
async fn main() {
    // Initiate logging
    zenoh::init_log_from_env_or("error");

    #[cfg(feature = "unstable")]
    let (config, key_expr, payload, attachment, add_matching_listener) = parse_args();
    #[cfg(not(feature = "unstable"))]
    let (config, key_expr, payload, _attachment, _) = parse_args();

    println!("Opening session...");
    let session = zenoh::open(config).await.unwrap();

    println!("Declaring Publisher on '{key_expr}'...");
    let publisher = session.declare_publisher(&key_expr).await.unwrap();

    #[cfg(feature = "unstable")]
    if add_matching_listener {
        publisher
            .matching_listener()
            .callback(|matching_status| {
                if matching_status.matching() {
                    println!("Publisher has matching subscribers.");
                } else {
                    println!("Publisher has NO MORE matching subscribers.");
                }
            })
            .background()
            .await
            .unwrap();
    }

    println!("Press CTRL-C to quit...");
    use zenoh_ext::ZSerializer;
    use std::time::{SystemTime, UNIX_EPOCH};

    let start = SystemTime::now();

    for idx in 0..u32::MAX {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let context = format!("[{idx:4}] {payload}");

        println!("Putting Data ('{}': '{}')...", &key_expr, &context);

        // Serialize the string with CDR.
        use cdr::{CdrLe, Infinite};
        let buf = cdr::serialize::<_, _, CdrLe>(&context, Infinite).unwrap();

        // Construct the required attachment for rmw_zenoh.
        let mut ser = ZSerializer::new();
        ser.serialize("sequence_number");
        ser.serialize(idx as i64);
        ser.serialize("source_timestamp");
        ser.serialize(start.duration_since(UNIX_EPOCH).unwrap().as_millis() as i64);
        ser.serialize("source_gid");
        ser.serialize([1; 16]);
        let bytes = ser.finish();

        // Publish the serialized data with bytes encoding
        publisher
            .put(buf)
            .encoding(Encoding::ZENOH_BYTES)
            .attachment(bytes)
            .await
            .unwrap();
    }
}

#[derive(clap::Parser, Clone, PartialEq, Eq, Hash, Debug)]
struct Args {
    #[arg(short, long, default_value = "demo/example/zenoh-rs-pub")]
    /// The key expression to write to.
    key: KeyExpr<'static>,
    #[arg(short, long, default_value = "Pub from Rust!")]
    /// The payload to write.
    payload: String,
    #[arg(short, long)]
    /// The attachments to add to each put.
    attach: Option<String>,
    /// Enable matching listener.
    #[cfg(feature = "unstable")]
    #[arg(long)]
    add_matching_listener: bool,
    #[command(flatten)]
    common: CommonArgs,
}

fn parse_args() -> (Config, KeyExpr<'static>, String, Option<String>, bool) {
    let args = Args::parse();
    (
        args.common.into(),
        args.key,
        args.payload,
        args.attach,
        #[cfg(feature = "unstable")]
        args.add_matching_listener,
        #[cfg(not(feature = "unstable"))]
        false,
    )
}
