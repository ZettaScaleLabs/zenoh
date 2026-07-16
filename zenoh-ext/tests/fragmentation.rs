//
// Copyright (c) 2024 ZettaScale Technology
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

use zenoh::{internal::ztimeout, Config};
use zenoh_config::{EndPoint, EndPoints, WhatAmI};
use zenoh_ext::{AdvancedPublisherBuilderExt, AdvancedSubscriberBuilderExt, MissDetectionConfig};

const SLEEP: Duration = Duration::from_secs(1);
const TIMEOUT: Duration = Duration::from_secs(60);

async fn create_peer_pair() -> (zenoh::Session, zenoh::Session) {
    let locator = format!("tcp/127.0.0.1:{}", zenoh_test::get_free_tcp_port());
    let peer1 = {
        let mut c = Config::default();
        c.listen
            .endpoints
            .set(vec![locator.parse::<EndPoint>().unwrap()])
            .unwrap();
        c.scouting.multicast.set_enabled(Some(false)).unwrap();
        let _ = c.set_mode(Some(WhatAmI::Peer));
        let s = ztimeout!(zenoh::open(c)).unwrap();
        tracing::info!("Peer (1) ZID: {}", s.zid());
        s
    };

    let peer2 = {
        let mut c = Config::default();
        c.connect
            .endpoints
            .set(vec![locator.parse::<EndPoints>().unwrap()])
            .unwrap();
        c.scouting.multicast.set_enabled(Some(false)).unwrap();
        let _ = c.set_mode(Some(WhatAmI::Peer));
        let s = ztimeout!(zenoh::open(c)).unwrap();
        tracing::info!("Peer (2) ZID: {}", s.zid());
        s
    };

    (peer1, peer2)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_fragmentation_reassembly() {
    zenoh_util::init_log_from_env_or("error");
    let (peer1, peer2) = create_peer_pair().await;

    let sub = ztimeout!(peer2
        .declare_subscriber("test/fragmentation/reassembly")
        .advanced())
    .unwrap();
    tokio::time::sleep(SLEEP).await;

    let publ = ztimeout!(peer1
        .declare_publisher("test/fragmentation/reassembly")
        .advanced()
        .fragmentation(4)
        .sample_miss_detection(MissDetectionConfig::default()))
    .unwrap();

    let payload = "HelloWorldFromFragmentationTest";
    ztimeout!(publ.put(payload)).unwrap();
    tokio::time::sleep(SLEEP).await;

    let sample = ztimeout!(sub.recv_async()).unwrap();
    assert_eq!(sample.payload().try_to_string().unwrap().as_ref(), payload);
    assert!(sub.try_recv().unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_max_fragments_enforced() {
    zenoh_util::init_log_from_env_or("error");
    let (peer1, peer2) = create_peer_pair().await;

    let sub = ztimeout!(peer2
        .declare_subscriber("test/fragmentation/max")
        .advanced()
        .max_fragments(4))
    .unwrap();
    tokio::time::sleep(SLEEP).await;

    let publ = ztimeout!(peer1
        .declare_publisher("test/fragmentation/max")
        .advanced()
        .fragmentation(1)
        .sample_miss_detection(MissDetectionConfig::default()))
    .unwrap();

    // 10 bytes -> 10 fragments, but subscriber only accepts up to 4.
    ztimeout!(publ.put("HelloWorld")).unwrap();
    tokio::time::sleep(SLEEP).await;

    assert!(sub.try_recv().unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_fragmentation_builder_order_independent() {
    zenoh_util::init_log_from_env_or("error");
    let (peer1, _peer2) = create_peer_pair().await;

    // `.fragmentation()` before `.sample_miss_detection()` must resolve
    // successfully now that validation is deferred.
    let _publ = ztimeout!(peer1
        .declare_publisher("test/fragmentation/order")
        .advanced()
        .fragmentation(4)
        .sample_miss_detection(MissDetectionConfig::default()))
    .unwrap();
}
