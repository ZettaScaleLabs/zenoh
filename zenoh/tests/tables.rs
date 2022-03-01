//
// Copyright (c) 2017, 2020 ADLINK Technology Inc.
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the Apache License, Version 2.0
// which is available at https://www.apache.org/licenses/LICENSE-2.0.
//
// SPDX-License-Identifier: EPL-2.0 OR Apache-2.0
//
// Contributors:
//   ADLINK zenoh team, <zenoh@adlink-labs.tech>
//
use async_std::sync::Arc;
use std::convert::TryInto;
use std::time::Duration;
use uhlc::HLC;
use zenoh::net::routing::router::*;
use zenoh_config::ZN_QUERIES_DEFAULT_TIMEOUT_DEFAULT;
use zenoh_core::zlock;
use zenoh_protocol::io::ZBuf;
use zenoh_protocol::proto::{DataInfo, RoutingContext};
use zenoh_protocol_core::key_expr::intersect;
use zenoh_protocol_core::{
    Channel, CongestionControl, ConsolidationStrategy, KeyExpr, PeerId, QueryTarget, QueryableInfo,
    Reliability, SubInfo, SubMode, WhatAmI, ZInt, EMPTY_EXPR_ID,
};
use zenoh_transport::{DummyPrimitives, Primitives};

#[test]
fn base_test() {
    let mut tables = Tables::new(
        PeerId::new(0, [0; 16]),
        WhatAmI::Client,
        Some(Arc::new(HLC::default())),
        Duration::from_millis(ZN_QUERIES_DEFAULT_TIMEOUT_DEFAULT.parse().unwrap()),
    );
    let primitives = Arc::new(DummyPrimitives::new());
    let face = tables.open_face(PeerId::new(0, [0; 16]), WhatAmI::Client, primitives);
    tables.register_expr(&face.upgrade().unwrap(), 1, &"/one/two/three".into());
    tables.register_expr(&face.upgrade().unwrap(), 2, &"/one/deux/trois".into());

    let sub_info = SubInfo {
        reliability: Reliability::Reliable,
        mode: SubMode::Push,
        period: None,
    };
    declare_client_subscription(
        &mut tables,
        &face.upgrade().unwrap(),
        &KeyExpr::from(1).with_suffix("/four/five"),
        &sub_info,
    );

    // Tables::print(&tables);
}

#[test]
fn match_test() {
    let key_exprs = [
        "/",
        "/a",
        "/a/",
        "/a/b",
        "/*",
        "/abc",
        "/abc/",
        "/*/",
        "xxx",
        "/ab*",
        "/abcd",
        "/ab*d",
        "/ab",
        "/ab/*",
        "/a/*/c/*/e",
        "/a/b/c/d/e",
        "/a/*b/c/*d/e",
        "/a/xb/c/xd/e",
        "/a/c/e",
        "/a/b/c/d/x/e",
        "/ab*cd",
        "/abxxcxxd",
        "/abxxcxxcd",
        "/abxxcxxcdx",
        "/**",
        "/a/b/c",
        "/a/b/c/",
        "/**/",
        "/ab/**",
        "/**/xyz",
        "/a/b/xyz/d/e/f/xyz",
        "/**/xyz*xyz",
        "/a/b/xyz/d/e/f/xyz",
        "/a/**/c/**/e",
        "/a/b/b/b/c/d/d/d/e",
        "/a/**/c/*/e/*",
        "/a/b/b/b/c/d/d/c/d/e/f",
        "/a/**/c/*/e/*",
        "/x/abc",
        "/x/*",
        "/x/abc*",
        "/x/*abc",
        "/x/a*",
        "/x/a*de",
        "/x/abc*de",
        "/x/a*d*e",
        "/x/a*e",
        "/x/a*c*e",
        "/x/ade",
        "/x/c*",
        "/x/*d",
        "/x/*e",
    ];

    let mut tables = Tables::new(
        PeerId::new(0, [0; 16]),
        WhatAmI::Client,
        Some(Arc::new(HLC::default())),
        Duration::from_millis(ZN_QUERIES_DEFAULT_TIMEOUT_DEFAULT.parse().unwrap()),
    );
    let primitives = Arc::new(DummyPrimitives::new());
    let face = tables.open_face(PeerId::new(0, [0; 16]), WhatAmI::Client, primitives);
    for (i, key_expr) in key_exprs.iter().enumerate() {
        tables.register_expr(
            &face.upgrade().unwrap(),
            i.try_into().unwrap(),
            &(*key_expr).into(),
        );
    }

    for key_expr1 in key_exprs.iter() {
        let matches = tables._matches(key_expr1);
        for key_expr2 in key_exprs.iter() {
            if matches.iter().any(|x| x == *key_expr2) {
                assert!(intersect(key_expr1, key_expr2));
            } else {
                assert!(!intersect(key_expr1, key_expr2));
            }
        }
    }
}

#[test]
fn clean_test() {
    let mut tables = Tables::new(
        PeerId::new(0, [0; 16]),
        WhatAmI::Client,
        Some(Arc::new(HLC::default())),
        Duration::from_millis(ZN_QUERIES_DEFAULT_TIMEOUT_DEFAULT.parse().unwrap()),
    );

    let primitives = Arc::new(DummyPrimitives::new());
    let face0 = tables.open_face(PeerId::new(0, [0; 16]), WhatAmI::Client, primitives);
    assert!(face0.upgrade().is_some());

    // --------------
    tables.register_expr(&face0.upgrade().unwrap(), 1, &"/todrop1".into());
    assert!(tables._contains_res("/todrop1"));

    tables.register_expr(&face0.upgrade().unwrap(), 2, &"/todrop1/todrop11".into());
    assert!(tables._contains_res("/todrop1/todrop11"));

    tables.register_expr(&face0.upgrade().unwrap(), 3, &"/**".into());
    assert!(tables._contains_res("/**"));

    tables.unregister_expr(&face0.upgrade().unwrap(), 1);
    assert!(tables._contains_res("/todrop1"));
    assert!(tables._contains_res("/todrop1/todrop11"));
    assert!(tables._contains_res("/**"));

    tables.unregister_expr(&face0.upgrade().unwrap(), 2);
    assert!(!tables._contains_res("/todrop1"));
    assert!(!tables._contains_res("/todrop1/todrop11"));
    assert!(tables._contains_res("/**"));

    tables.unregister_expr(&face0.upgrade().unwrap(), 3);
    assert!(!tables._contains_res("/todrop1"));
    assert!(!tables._contains_res("/todrop1/todrop11"));
    assert!(!tables._contains_res("/**"));

    // --------------
    tables.register_expr(&face0.upgrade().unwrap(), 1, &"/todrop1".into());
    assert!(tables._contains_res("/todrop1"));

    let sub_info = SubInfo {
        reliability: Reliability::Reliable,
        mode: SubMode::Push,
        period: None,
    };

    declare_client_subscription(
        &mut tables,
        &face0.upgrade().unwrap(),
        &"/todrop1/todrop11".into(),
        &sub_info,
    );
    assert!(tables._contains_res("/todrop1/todrop11"));

    declare_client_subscription(
        &mut tables,
        &face0.upgrade().unwrap(),
        &KeyExpr::from(1).with_suffix("/todrop12"),
        &sub_info,
    );
    assert!(tables._contains_res("/todrop1/todrop12"));

    forget_client_subscription(
        &mut tables,
        &face0.upgrade().unwrap(),
        &KeyExpr::from(1).with_suffix("/todrop12"),
    );
    assert!(tables._contains_res("/todrop1"));
    assert!(tables._contains_res("/todrop1/todrop11"));
    assert!(!tables._contains_res("/todrop1/todrop12"));

    forget_client_subscription(
        &mut tables,
        &face0.upgrade().unwrap(),
        &"/todrop1/todrop11".into(),
    );
    assert!(tables._contains_res("/todrop1"));
    assert!(!tables._contains_res("/todrop1/todrop11"));
    assert!(!tables._contains_res("/todrop1/todrop12"));

    tables.unregister_expr(&face0.upgrade().unwrap(), 1);
    assert!(!tables._contains_res("/todrop1"));
    assert!(!tables._contains_res("/todrop1/todrop11"));
    assert!(!tables._contains_res("/todrop1/todrop12"));

    // --------------
    tables.register_expr(&face0.upgrade().unwrap(), 2, &"/todrop3".into());
    declare_client_subscription(
        &mut tables,
        &face0.upgrade().unwrap(),
        &"/todrop3".into(),
        &sub_info,
    );
    assert!(tables._contains_res("/todrop3"));

    forget_client_subscription(&mut tables, &face0.upgrade().unwrap(), &"/todrop3".into());
    assert!(tables._contains_res("/todrop3"));

    tables.unregister_expr(&face0.upgrade().unwrap(), 2);
    assert!(!tables._contains_res("/todrop3"));

    // --------------
    tables.register_expr(&face0.upgrade().unwrap(), 3, &"/todrop4".into());
    tables.register_expr(&face0.upgrade().unwrap(), 4, &"/todrop5".into());
    declare_client_subscription(
        &mut tables,
        &face0.upgrade().unwrap(),
        &"/todrop5".into(),
        &sub_info,
    );
    declare_client_subscription(
        &mut tables,
        &face0.upgrade().unwrap(),
        &"/todrop6".into(),
        &sub_info,
    );
    assert!(tables._contains_res("/todrop4"));
    assert!(tables._contains_res("/todrop5"));
    assert!(tables._contains_res("/todrop6"));

    tables.close_face(&face0);
    assert!(face0.upgrade().is_none());
    assert!(!tables._contains_res("/todrop4"));
    assert!(!tables._contains_res("/todrop5"));
    assert!(!tables._contains_res("/todrop6"));
}

pub struct ClientPrimitives {
    data: std::sync::Mutex<Option<KeyExpr<'static>>>,
    mapping: std::sync::Mutex<std::collections::HashMap<ZInt, String>>,
}

impl ClientPrimitives {
    pub fn new() -> ClientPrimitives {
        ClientPrimitives {
            data: std::sync::Mutex::new(None),
            mapping: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    pub fn clear_data(&self) {
        *self.data.lock().unwrap() = None;
    }
}

impl Default for ClientPrimitives {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientPrimitives {
    fn get_name(&self, key_expr: &KeyExpr) -> String {
        let mapping = self.mapping.lock().unwrap();
        let (scope, suffix) = key_expr.as_id_and_suffix();
        if scope == EMPTY_EXPR_ID {
            suffix.to_string()
        } else if suffix.is_empty() {
            mapping.get(&scope).unwrap().clone()
        } else {
            format!("{}{}", mapping.get(&scope).unwrap(), suffix)
        }
    }

    fn get_last_name(&self) -> Option<String> {
        self.data
            .lock()
            .unwrap()
            .as_ref()
            .map(|data| self.get_name(data))
    }

    #[allow(dead_code)]
    fn get_last_key(&self) -> Option<KeyExpr> {
        self.data.lock().unwrap().as_ref().cloned()
    }
}

impl Primitives for ClientPrimitives {
    fn decl_resource(&self, expr_id: ZInt, key_expr: &KeyExpr) {
        let name = self.get_name(key_expr);
        zlock!(self.mapping).insert(expr_id, name);
    }

    fn forget_resource(&self, expr_id: ZInt) {
        zlock!(self.mapping).remove(&expr_id);
    }

    fn decl_publisher(&self, _key_expr: &KeyExpr, _routing_context: Option<RoutingContext>) {}
    fn forget_publisher(&self, _key_expr: &KeyExpr, _routing_context: Option<RoutingContext>) {}

    fn decl_subscriber(
        &self,
        _key_expr: &KeyExpr,
        _sub_info: &SubInfo,
        _routing_context: Option<RoutingContext>,
    ) {
    }
    fn forget_subscriber(&self, _key_expr: &KeyExpr, _routing_context: Option<RoutingContext>) {}

    fn decl_queryable(
        &self,
        _key_expr: &KeyExpr,
        _kind: ZInt,
        _qabl_info: &QueryableInfo,
        _routing_context: Option<RoutingContext>,
    ) {
    }
    fn forget_queryable(
        &self,
        _key_expr: &KeyExpr,
        _kind: ZInt,
        _routing_context: Option<RoutingContext>,
    ) {
    }

    fn send_data(
        &self,
        key_expr: &KeyExpr,
        _payload: ZBuf,
        _channel: Channel,
        _congestion_control: CongestionControl,
        _info: Option<DataInfo>,
        _routing_context: Option<RoutingContext>,
    ) {
        *zlock!(self.data) = Some(key_expr.to_owned());
    }

    fn send_query(
        &self,
        _key_expr: &KeyExpr,
        _value_selector: &str,
        _qid: ZInt,
        _target: QueryTarget,
        _consolidation: ConsolidationStrategy,
        _routing_context: Option<RoutingContext>,
    ) {
    }

    fn send_reply_data(
        &self,
        _qid: ZInt,
        _replier_kind: ZInt,
        _replier_id: PeerId,
        _key_expr: KeyExpr,
        _info: Option<DataInfo>,
        _payload: ZBuf,
    ) {
    }
    fn send_reply_final(&self, _qid: ZInt) {}

    fn send_pull(
        &self,
        _is_final: bool,
        _key_expr: &KeyExpr,
        _pull_id: ZInt,
        _max_samples: &Option<ZInt>,
    ) {
    }

    fn send_close(&self) {}
}

#[test]
fn client_test() {
    let mut tables = Tables::new(
        PeerId::new(0, [0; 16]),
        WhatAmI::Client,
        Some(Arc::new(HLC::default())),
        Duration::from_millis(ZN_QUERIES_DEFAULT_TIMEOUT_DEFAULT.parse().unwrap()),
    );
    let sub_info = SubInfo {
        reliability: Reliability::Reliable,
        mode: SubMode::Push,
        period: None,
    };

    let primitives0 = Arc::new(ClientPrimitives::new());
    let face0 = tables.open_face(
        PeerId::new(0, [0; 16]),
        WhatAmI::Client,
        primitives0.clone(),
    );
    tables.register_expr(&face0.upgrade().unwrap(), 11, &"/test/client".into());
    primitives0.decl_resource(11, &"/test/client".into());
    declare_client_subscription(
        &mut tables,
        &face0.upgrade().unwrap(),
        &KeyExpr::from(11).with_suffix("/**"),
        &sub_info,
    );
    tables.register_expr(
        &face0.upgrade().unwrap(),
        12,
        &KeyExpr::from(11).with_suffix("/z1_pub1"),
    );
    primitives0.decl_resource(12, &KeyExpr::from(11).with_suffix("/z1_pub1"));

    let primitives1 = Arc::new(ClientPrimitives::new());
    let face1 = tables.open_face(
        PeerId::new(0, [0; 16]),
        WhatAmI::Client,
        primitives1.clone(),
    );
    tables.register_expr(&face1.upgrade().unwrap(), 21, &"/test/client".into());
    primitives1.decl_resource(21, &"/test/client".into());
    declare_client_subscription(
        &mut tables,
        &face1.upgrade().unwrap(),
        &KeyExpr::from(21).with_suffix("/**"),
        &sub_info,
    );
    tables.register_expr(
        &face1.upgrade().unwrap(),
        22,
        &KeyExpr::from(21).with_suffix("/z2_pub1"),
    );
    primitives1.decl_resource(22, &KeyExpr::from(21).with_suffix("/z2_pub1"));

    let primitives2 = Arc::new(ClientPrimitives::new());
    let face2 = tables.open_face(
        PeerId::new(0, [0; 16]),
        WhatAmI::Client,
        primitives2.clone(),
    );
    tables.register_expr(&face2.upgrade().unwrap(), 31, &"/test/client".into());
    primitives2.decl_resource(31, &"/test/client".into());
    declare_client_subscription(
        &mut tables,
        &face2.upgrade().unwrap(),
        &KeyExpr::from(31).with_suffix("/**"),
        &sub_info,
    );

    primitives0.clear_data();
    primitives1.clear_data();
    primitives2.clear_data();
    route_data(
        &tables,
        &face0.upgrade().unwrap(),
        &"/test/client/z1_wr1".into(),
        Channel::default(),
        CongestionControl::default(),
        None,
        ZBuf::default(),
        None,
    );

    // functionnal check
    assert!(primitives1.get_last_name().is_some());
    assert_eq!(primitives1.get_last_name().unwrap(), "/test/client/z1_wr1");
    // mapping strategy check
    // assert_eq!(primitives1.get_last_key().unwrap(), KeyExpr::IdWithSuffix(21, "/z1_wr1".to_string()));

    // functionnal check
    assert!(primitives2.get_last_name().is_some());
    assert_eq!(primitives2.get_last_name().unwrap(), "/test/client/z1_wr1");
    // mapping strategy check
    // assert_eq!(primitives2.get_last_key().unwrap(), KeyExpr::IdWithSuffix(31, "/z1_wr1".to_string()));

    primitives0.clear_data();
    primitives1.clear_data();
    primitives2.clear_data();
    route_data(
        &tables,
        &face0.upgrade().unwrap(),
        &KeyExpr::from(11).with_suffix("/z1_wr2"),
        Channel::default(),
        CongestionControl::default(),
        None,
        ZBuf::default(),
        None,
    );

    // functionnal check
    assert!(primitives1.get_last_name().is_some());
    assert_eq!(primitives1.get_last_name().unwrap(), "/test/client/z1_wr2");
    // mapping strategy check
    // assert_eq!(primitives1.get_last_key().unwrap(), KeyExpr::IdWithSuffix(21, "/z1_wr2".to_string()));

    // functionnal check
    assert!(primitives2.get_last_name().is_some());
    assert_eq!(primitives2.get_last_name().unwrap(), "/test/client/z1_wr2");
    // mapping strategy check
    // assert_eq!(primitives2.get_last_key().unwrap(), KeyExpr::IdWithSuffix(31, "/z1_wr2".to_string()));

    primitives0.clear_data();
    primitives1.clear_data();
    primitives2.clear_data();
    route_data(
        &tables,
        &face1.upgrade().unwrap(),
        &"/test/client/**".into(),
        Channel::default(),
        CongestionControl::default(),
        None,
        ZBuf::default(),
        None,
    );

    // functionnal check
    assert!(primitives0.get_last_name().is_some());
    assert_eq!(primitives0.get_last_name().unwrap(), "/test/client/**");
    // mapping strategy check
    // assert_eq!(primitives1.get_last_key().unwrap(), KeyExpr::IdWithSuffix(11, "/**".to_string()));

    // functionnal check
    assert!(primitives2.get_last_name().is_some());
    assert_eq!(primitives2.get_last_name().unwrap(), "/test/client/**");
    // mapping strategy check
    // assert_eq!(primitives2.get_last_key().unwrap(), KeyExpr::IdWithSuffix(31, "/**".to_string()));

    primitives0.clear_data();
    primitives1.clear_data();
    primitives2.clear_data();
    println!("ROUTE_DATA");
    route_data(
        &tables,
        &face0.upgrade().unwrap(),
        &12.into(),
        Channel::default(),
        CongestionControl::default(),
        None,
        ZBuf::default(),
        None,
    );

    // functionnal check
    assert!(primitives1.get_last_name().is_some());
    assert_eq!(primitives1.get_last_name().unwrap(), "/test/client/z1_pub1");
    // mapping strategy check
    // assert_eq!(primitives1.get_last_key().unwrap(), KeyExpr::IdWithSuffix(21, "/z1_pub1".to_string()));

    // functionnal check
    assert!(primitives2.get_last_name().is_some());
    assert_eq!(primitives2.get_last_name().unwrap(), "/test/client/z1_pub1");
    // mapping strategy check
    // assert_eq!(primitives2.get_last_key().unwrap(), KeyExpr::IdWithSuffix(31, "/z1_pub1".to_string()));

    primitives0.clear_data();
    primitives1.clear_data();
    primitives2.clear_data();
    route_data(
        &tables,
        &face1.upgrade().unwrap(),
        &22.into(),
        Channel::default(),
        CongestionControl::default(),
        None,
        ZBuf::default(),
        None,
    );

    // functionnal check
    assert!(primitives0.get_last_name().is_some());
    assert_eq!(primitives0.get_last_name().unwrap(), "/test/client/z2_pub1");
    // mapping strategy check
    // assert_eq!(primitives1.get_last_key().unwrap(), KeyExpr::IdWithSuffix(11, "/z2_pub1".to_string()));

    // functionnal check
    assert!(primitives2.get_last_name().is_some());
    assert_eq!(primitives2.get_last_name().unwrap(), "/test/client/z2_pub1");
    // mapping strategy check
    // assert_eq!(primitives2.get_last_key().unwrap(), KeyExpr::IdWithSuffix(31, "/z2_pub1".to_string()));
}
