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
mod demux;
mod mux;

pub use demux::*;
pub use mux::*;
use zenoh_buffers::ZBuf;
use zenoh_protocol::{
    core::{Channel, CongestionControl, ExprId, WireExpr, ZenohId},
    zenoh::{
        ConsolidationMode, DataInfo, PullId, QueryBody, QueryId, QueryTarget, QueryableInfo,
        RoutingContext, SubInfo,
    },
};

pub trait Primitives: Send + Sync {
    fn decl_resource(&self, expr_id: ExprId, key_expr: &WireExpr);
    fn forget_resource(&self, expr_id: ExprId);

    fn decl_publisher(&self, key_expr: &WireExpr, routing_context: Option<RoutingContext>);
    fn forget_publisher(&self, key_expr: &WireExpr, routing_context: Option<RoutingContext>);

    fn decl_subscriber(
        &self,
        key_expr: &WireExpr,
        sub_info: &SubInfo,
        routing_context: Option<RoutingContext>,
    );
    fn forget_subscriber(&self, key_expr: &WireExpr, routing_context: Option<RoutingContext>);

    fn decl_queryable(
        &self,
        key_expr: &WireExpr,
        qabl_info: &QueryableInfo,
        routing_context: Option<RoutingContext>,
    );
    fn forget_queryable(&self, key_expr: &WireExpr, routing_context: Option<RoutingContext>);

    fn send_data(
        &self,
        key_expr: &WireExpr,
        payload: ZBuf,
        channel: Channel,
        cogestion_control: CongestionControl,
        data_info: Option<DataInfo>,
        routing_context: Option<RoutingContext>,
    );

    #[allow(clippy::too_many_arguments)]
    fn send_query(
        &self,
        key_expr: &WireExpr,
        parameters: &str,
        qid: QueryId,
        target: QueryTarget,
        consolidation: ConsolidationMode,
        body: Option<QueryBody>,
        routing_context: Option<RoutingContext>,
    );

    fn send_reply_data(
        &self,
        qid: QueryId,
        replier_id: ZenohId,
        key_expr: WireExpr,
        info: Option<DataInfo>,
        payload: ZBuf,
    );

    fn send_reply_final(&self, qid: QueryId);

    fn send_pull(
        &self,
        is_final: bool,
        key_expr: &WireExpr,
        pull_id: PullId,
        max_samples: &Option<u16>,
    );

    fn send_close(&self);
}

#[derive(Default)]
pub struct DummyPrimitives;

impl DummyPrimitives {
    pub fn new() -> Self {
        Self
    }
}

impl Primitives for DummyPrimitives {
    fn decl_resource(&self, _expr_id: ExprId, _key_expr: &WireExpr) {}
    fn forget_resource(&self, _expr_id: ExprId) {}

    fn decl_publisher(&self, _key_expr: &WireExpr, _routing_context: Option<RoutingContext>) {}
    fn forget_publisher(&self, _key_expr: &WireExpr, _routing_context: Option<RoutingContext>) {}

    fn decl_subscriber(
        &self,
        _key_expr: &WireExpr,
        _sub_info: &SubInfo,
        _routing_context: Option<RoutingContext>,
    ) {
    }
    fn forget_subscriber(&self, _key_expr: &WireExpr, _routing_context: Option<RoutingContext>) {}

    fn decl_queryable(
        &self,
        _key_expr: &WireExpr,
        _qable_info: &QueryableInfo,
        _routing_context: Option<RoutingContext>,
    ) {
    }
    fn forget_queryable(&self, _key_expr: &WireExpr, _routing_context: Option<RoutingContext>) {}

    fn send_data(
        &self,
        _key_expr: &WireExpr,
        _payload: ZBuf,
        _channel: Channel,
        _cogestion_control: CongestionControl,
        _info: Option<DataInfo>,
        _routing_context: Option<RoutingContext>,
    ) {
    }
    fn send_query(
        &self,
        _key_expr: &WireExpr,
        _parameters: &str,
        _qid: QueryId,
        _target: QueryTarget,
        _consolidation: ConsolidationMode,
        _body: Option<QueryBody>,
        _routing_context: Option<RoutingContext>,
    ) {
    }
    fn send_reply_data(
        &self,
        _qid: QueryId,
        _replier_id: ZenohId,
        _key_expr: WireExpr,
        _info: Option<DataInfo>,
        _payload: ZBuf,
    ) {
    }
    fn send_reply_final(&self, _qid: QueryId) {}
    fn send_pull(
        &self,
        _is_final: bool,
        _key_expr: &WireExpr,
        _pull_id: PullId,
        _max_samples: &Option<u16>,
    ) {
    }

    fn send_close(&self) {}
}
