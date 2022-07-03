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
use petgraph::graph::NodeIndex;
use std::collections::HashMap;
use std::sync::RwLock;
use zenoh_collections::{vector_map::set::VecSet, VecMapWalker, VecWalker, Walker};
use zenoh_core::zread;
use zenoh_sync::get_mut_unchecked;

use zenoh_protocol::io::ZBuf;
use zenoh_protocol::proto::{DataInfo, RoutingContext};
use zenoh_protocol_core::{
    Channel, CongestionControl, KeyExpr, PeerId, Priority, Reliability, SubInfo, SubMode, WhatAmI,
    ZInt,
};

use crate::net::routing::router::Matches;

use super::face::{FaceId, FaceState};
use super::network::Network;
use super::restree::Strengthen;
use super::router::{
    net, PullCaches, ResourceTree, ResourceTreeIndex, Route, SessionContext, Tables,
};

#[allow(clippy::too_many_arguments)]
#[inline]
fn send_sourced_subscription_to_net_childs(
    restree: &mut ResourceTree,
    faces: &HashMap<FaceId, Arc<FaceState>>,
    net: &Network,
    childs: &[NodeIndex],
    res: &ResourceTreeIndex,
    src_face: Option<&Arc<FaceState>>,
    sub_info: &SubInfo,
    routing_context: Option<RoutingContext>,
) {
    for child in childs {
        if net.graph.contains_node(*child) {
            match faces
                .values()
                .find(|face| face.pid == net.graph[*child].pid)
                .cloned()
            {
                Some(someface) => {
                    if src_face.is_none() || someface.id != src_face.unwrap().id {
                        let key_expr = Tables::decl_key(restree, res, &someface);

                        log::debug!("Send subscription {} on {}", restree.expr(res), someface);

                        someface
                            .primitives
                            .decl_subscriber(&key_expr, sub_info, routing_context);
                    }
                }
                None => log::trace!("Unable to find face for pid {}", net.graph[*child].pid),
            }
        }
    }
}

fn propagate_simple_subscription(
    tables: &mut Tables,
    res: &ResourceTreeIndex,
    sub_info: &SubInfo,
    src_face: &Arc<FaceState>,
) {
    let faces = &mut tables.faces;
    let restree = &mut tables.restree;
    for dst_face in faces.values_mut() {
        if src_face.id != dst_face.id
            && !dst_face.local_subs.contains(res)
            && match tables.whatami {
                WhatAmI::Router => dst_face.whatami == WhatAmI::Client,
                WhatAmI::Peer => dst_face.whatami == WhatAmI::Client,
                _ => (src_face.whatami == WhatAmI::Client || dst_face.whatami == WhatAmI::Client),
            }
        {
            get_mut_unchecked(dst_face).local_subs.insert(res.clone());
            let key_expr = Tables::decl_key(restree, res, dst_face);
            dst_face
                .primitives
                .decl_subscriber(&key_expr, sub_info, None);
        }
    }
}

fn propagate_sourced_subscription(
    tables: &mut Tables,
    res: &ResourceTreeIndex,
    sub_info: &SubInfo,
    src_face: Option<&Arc<FaceState>>,
    source: &PeerId,
    net_type: WhatAmI,
) {
    let restree = &mut tables.restree;
    let net = net!(tables, net_type).unwrap();

    match net.get_idx(source) {
        Some(tree_sid) => {
            if net.trees.len() > tree_sid.index() {
                send_sourced_subscription_to_net_childs(
                    restree,
                    &tables.faces,
                    net,
                    &net.trees[tree_sid.index()].childs,
                    res,
                    src_face,
                    sub_info,
                    Some(RoutingContext::new(tree_sid.index() as ZInt)),
                );
            } else {
                log::trace!(
                    "Propagating sub {}: tree for node {} sid:{} not yet ready",
                    tables.restree.expr(res),
                    tree_sid.index(),
                    source
                );
            }
        }
        None => log::error!(
            "Error propagating sub {}: cannot get index of {}!",
            tables.restree.expr(res),
            source
        ),
    }
}

fn register_router_subscription(
    tables: &mut Tables,
    face: &Arc<FaceState>,
    res: &ResourceTreeIndex,
    sub_info: &SubInfo,
    router: PeerId,
) {
    if !tables.restree.weight(res).router_subs.contains(&router) {
        // Register router subscription
        {
            log::debug!(
                "Register router subscription {} (router: {})",
                tables.restree.expr(res),
                router
            );
            tables.restree.weight_mut(res).router_subs.insert(router);
            tables.router_subs.insert(res.clone());
        }

        // Propagate subscription to routers
        propagate_sourced_subscription(tables, res, sub_info, Some(face), &router, WhatAmI::Router);

        // Propagate subscription to peers
        if face.whatami != WhatAmI::Peer {
            register_peer_subscription(tables, face, res, sub_info, tables.pid)
        }
    }

    // Propagate subscription to clients
    propagate_simple_subscription(tables, res, sub_info, face);
}

pub fn declare_router_subscription(
    tables: &mut Tables,
    face: &Arc<FaceState>,
    expr: &KeyExpr,
    sub_info: &SubInfo,
    router: PeerId,
) {
    match tables.get_mapping(face, &expr.scope).cloned() {
        Some(prefix) => {
            let res = tables.restree.get_or_insert(&prefix, expr.suffix.as_ref());
            tables.match_resource(&res);
            register_router_subscription(tables, face, &res, sub_info, router);

            compute_matches_data_routes(tables, &res);
        }
        None => log::error!(
            "Declare router subscription for unknown scope {}!",
            expr.scope
        ),
    }
}

fn register_peer_subscription(
    tables: &mut Tables,
    face: &Arc<FaceState>,
    res: &ResourceTreeIndex,
    sub_info: &SubInfo,
    peer: PeerId,
) {
    if !tables.restree.weight(res).peer_subs.contains(&peer) {
        // Register peer subscription
        {
            log::debug!(
                "Register peer subscription {} (peer: {})",
                tables.restree.expr(res),
                peer
            );
            tables.restree.weight_mut(res).peer_subs.insert(peer);
            tables.peer_subs.insert(res.clone());
        }

        // Propagate subscription to peers
        propagate_sourced_subscription(tables, res, sub_info, Some(face), &peer, WhatAmI::Peer);
    }
}

pub fn declare_peer_subscription(
    tables: &mut Tables,
    face: &Arc<FaceState>,
    expr: &KeyExpr,
    sub_info: &SubInfo,
    peer: PeerId,
) {
    match tables.get_mapping(face, &expr.scope).cloned() {
        Some(prefix) => {
            let res = tables.restree.get_or_insert(&prefix, expr.suffix.as_ref());
            tables.match_resource(&res);
            register_peer_subscription(tables, face, &res, sub_info, peer);

            if tables.whatami == WhatAmI::Router {
                let mut propa_sub_info = sub_info.clone();
                propa_sub_info.mode = SubMode::Push;
                register_router_subscription(tables, face, &res, &propa_sub_info, tables.pid);
            }

            compute_matches_data_routes(tables, &res);
        }
        None => log::error!(
            "Declare router subscription for unknown scope {}!",
            expr.scope
        ),
    }
}

fn register_client_subscription(
    tables: &mut Tables,
    face: &Arc<FaceState>,
    res: &ResourceTreeIndex,
    sub_info: &SubInfo,
) {
    // Register subscription
    {
        log::debug!(
            "Register subscription {} for {}",
            tables.restree.expr(res),
            face
        );
        match tables
            .restree
            .weight_mut(res)
            .session_ctxs
            .get_mut(&face.id)
        {
            Some(ctx) => match &ctx.subs {
                Some(info) => {
                    if SubMode::Pull == info.mode {
                        get_mut_unchecked(ctx).subs = Some(sub_info.clone());
                    }
                }
                None => {
                    get_mut_unchecked(ctx).subs = Some(sub_info.clone());
                }
            },
            None => {
                tables.restree.weight_mut(res).session_ctxs.insert(
                    face.id,
                    Arc::new(SessionContext {
                        face: face.clone(),
                        local_expr_id: None,
                        remote_expr_id: None,
                        subs: Some(sub_info.clone()),
                        qabl: HashMap::new(),
                        last_values: HashMap::new(),
                    }),
                );
            }
        }
    }
    get_mut_unchecked(face).remote_subs.insert(res.clone());
}

pub fn declare_client_subscription(
    tables: &mut Tables,
    face: &Arc<FaceState>,
    expr: &KeyExpr,
    sub_info: &SubInfo,
) {
    match tables.get_mapping(face, &expr.scope).cloned() {
        Some(prefix) => {
            let res = tables.restree.get_or_insert(&prefix, expr.suffix.as_ref());
            tables.match_resource(&res);

            register_client_subscription(tables, face, &res, sub_info);
            match tables.whatami {
                WhatAmI::Router => {
                    let mut propa_sub_info = sub_info.clone();
                    propa_sub_info.mode = SubMode::Push;
                    register_router_subscription(tables, face, &res, &propa_sub_info, tables.pid);
                }
                WhatAmI::Peer => {
                    let mut propa_sub_info = sub_info.clone();
                    propa_sub_info.mode = SubMode::Push;
                    register_peer_subscription(tables, face, &res, &propa_sub_info, tables.pid);
                }
                _ => {
                    propagate_simple_subscription(tables, &res, sub_info, face);
                }
            }

            compute_matches_data_routes(tables, &res);
        }
        None => log::error!("Declare subscription for unknown scope {}!", expr.scope),
    }
}

#[inline]
fn remote_router_subs(tables: &Tables, res: &ResourceTreeIndex) -> bool {
    tables
        .restree
        .weight(res)
        .router_subs
        .iter()
        .any(|peer| peer != &tables.pid)
}

#[inline]
fn remote_peer_subs(tables: &Tables, res: &ResourceTreeIndex) -> bool {
    tables
        .restree
        .weight(res)
        .peer_subs
        .iter()
        .any(|peer| peer != &tables.pid)
}

#[inline]
fn client_subs(tables: &Tables, res: &ResourceTreeIndex) -> Vec<Arc<FaceState>> {
    tables
        .restree
        .weight(res)
        .session_ctxs
        .values()
        .filter_map(|ctx| {
            if ctx.subs.is_some() {
                Some(ctx.face.clone())
            } else {
                None
            }
        })
        .collect()
}

#[inline]
fn send_forget_sourced_subscription_to_net_childs(
    restree: &mut ResourceTree,
    faces: &HashMap<FaceId, Arc<FaceState>>,
    net: &Network,
    childs: &[NodeIndex],
    res: &ResourceTreeIndex,
    src_face: Option<&Arc<FaceState>>,
    routing_context: Option<RoutingContext>,
) {
    for child in childs {
        if net.graph.contains_node(*child) {
            match faces
                .values()
                .find(|face| face.pid == net.graph[*child].pid)
                .cloned()
            {
                Some(someface) => {
                    if src_face.is_none() || someface.id != src_face.unwrap().id {
                        let key_expr = Tables::decl_key(restree, res, &someface);

                        log::debug!(
                            "Send forget subscription {} on {}",
                            restree.expr(res),
                            someface
                        );

                        someface
                            .primitives
                            .forget_subscriber(&key_expr, routing_context);
                    }
                }
                None => log::trace!("Unable to find face for pid {}", net.graph[*child].pid),
            }
        }
    }
}

fn propagate_forget_simple_subscription(tables: &mut Tables, res: &ResourceTreeIndex) {
    for face in tables.faces.values_mut() {
        if face.local_subs.contains(res) {
            let key_expr = Tables::get_best_key(&tables.restree, res, "", face.id);
            face.primitives.forget_subscriber(&key_expr, None);

            get_mut_unchecked(face).local_subs.remove(res);
        }
    }
}

fn propagate_forget_sourced_subscription(
    tables: &mut Tables,
    res: &ResourceTreeIndex,
    src_face: Option<&Arc<FaceState>>,
    source: &PeerId,
    net_type: WhatAmI,
) {
    let net = net!(tables, net_type).unwrap();
    let restree = &mut tables.restree;
    match net.get_idx(source) {
        Some(tree_sid) => {
            if net.trees.len() > tree_sid.index() {
                send_forget_sourced_subscription_to_net_childs(
                    restree,
                    &tables.faces,
                    net,
                    &net.trees[tree_sid.index()].childs,
                    res,
                    src_face,
                    Some(RoutingContext::new(tree_sid.index() as ZInt)),
                );
            } else {
                log::trace!(
                    "Propagating forget sub {}: tree for node {} sid:{} not yet ready",
                    tables.restree.expr(res),
                    tree_sid.index(),
                    source
                );
            }
        }
        None => log::error!(
            "Error propagating forget sub {}: cannot get index of {}!",
            tables.restree.expr(res),
            source
        ),
    }
}

fn unregister_router_subscription(tables: &mut Tables, res: &ResourceTreeIndex, router: &PeerId) {
    log::debug!(
        "Unregister router subscription {} (router: {})",
        tables.restree.expr(res),
        router
    );

    tables
        .restree
        .weight_mut(res)
        .router_subs
        .retain(|sub| sub != router);

    if tables.restree.weight(res).router_subs.is_empty() {
        tables.router_subs.retain(|sub| !Arc::ptr_eq(sub, res));

        undeclare_peer_subscription(tables, None, res, &tables.pid.clone());
        propagate_forget_simple_subscription(tables, res);
    }
}

fn undeclare_router_subscription(
    tables: &mut Tables,
    face: Option<&Arc<FaceState>>,
    res: &ResourceTreeIndex,
    router: &PeerId,
) {
    if tables.restree.weight(res).router_subs.contains(router) {
        unregister_router_subscription(tables, res, router);
        propagate_forget_sourced_subscription(tables, res, face, router, WhatAmI::Router);
    }
}

pub fn forget_router_subscription(
    tables: &mut Tables,
    face: &Arc<FaceState>,
    expr: &KeyExpr,
    router: &PeerId,
) {
    match tables.get_mapping(face, &expr.scope) {
        Some(prefix) => match tables.restree.get(prefix, expr.suffix.as_ref()) {
            Some(res) => {
                undeclare_router_subscription(tables, Some(face), &res, router);

                compute_matches_data_routes(tables, &res);
                tables.clean_resource(res);
            }
            None => log::error!("Undeclare unknown router subscription!"),
        },
        None => log::error!("Undeclare router subscription with unknown scope!"),
    }
}

fn unregister_peer_subscription(tables: &mut Tables, res: &ResourceTreeIndex, peer: &PeerId) {
    log::debug!(
        "Unregister peer subscription {} (peer: {})",
        tables.restree.expr(res),
        peer
    );
    tables
        .restree
        .weight_mut(res)
        .peer_subs
        .retain(|sub| sub != peer);

    if tables.restree.weight(res).peer_subs.is_empty() {
        tables.peer_subs.retain(|sub| !Arc::ptr_eq(sub, res));
    }
}

fn undeclare_peer_subscription(
    tables: &mut Tables,
    face: Option<&Arc<FaceState>>,
    res: &ResourceTreeIndex,
    peer: &PeerId,
) {
    if tables.restree.weight(res).peer_subs.contains(peer) {
        unregister_peer_subscription(tables, res, peer);
        propagate_forget_sourced_subscription(tables, res, face, peer, WhatAmI::Peer);
    }
}

pub fn forget_peer_subscription(
    tables: &mut Tables,
    face: &Arc<FaceState>,
    expr: &KeyExpr,
    peer: &PeerId,
) {
    match tables.get_mapping(face, &expr.scope) {
        Some(prefix) => match tables.restree.get(prefix, expr.suffix.as_ref()) {
            Some(res) => {
                undeclare_peer_subscription(tables, Some(face), &res, peer);

                if tables.whatami == WhatAmI::Router {
                    let client_subs = tables
                        .restree
                        .weight(&res)
                        .session_ctxs
                        .values()
                        .any(|ctx| ctx.subs.is_some());
                    let peer_subs = remote_peer_subs(tables, &res);
                    if !client_subs && !peer_subs {
                        undeclare_router_subscription(tables, None, &res, &tables.pid.clone());
                    }
                }

                compute_matches_data_routes(tables, &res);
                tables.clean_resource(res);
            }
            None => log::error!("Undeclare unknown peer subscription!"),
        },
        None => log::error!("Undeclare peer subscription with unknown scope!"),
    }
}

pub(crate) fn undeclare_client_subscription(
    tables: &mut Tables,
    face: &Arc<FaceState>,
    res: ResourceTreeIndex,
) {
    log::debug!(
        "Unregister client subscription {} for {}",
        tables.restree.expr(&res),
        face
    );
    if let Some(ctx) = tables
        .restree
        .weight_mut(&res)
        .session_ctxs
        .get_mut(&face.id)
    {
        get_mut_unchecked(ctx).subs = None;
    }
    get_mut_unchecked(face).remote_subs.remove(&res);

    let mut client_subs = client_subs(tables, &res);
    let router_subs = remote_router_subs(tables, &res);
    let peer_subs = remote_peer_subs(tables, &res);
    match tables.whatami {
        WhatAmI::Router => {
            if client_subs.is_empty() && !peer_subs {
                undeclare_router_subscription(tables, None, &res, &tables.pid.clone());
            }
        }
        WhatAmI::Peer => {
            if client_subs.is_empty() {
                undeclare_peer_subscription(tables, None, &res, &tables.pid.clone());
            }
        }
        _ => {
            if client_subs.is_empty() {
                propagate_forget_simple_subscription(tables, &res);
            }
        }
    }
    if client_subs.len() == 1 && !router_subs && !peer_subs {
        let face = &mut client_subs[0];
        if face.local_subs.contains(&res) {
            let key_expr = Tables::get_best_key(&tables.restree, &res, "", face.id);
            face.primitives.forget_subscriber(&key_expr, None);

            get_mut_unchecked(face).local_subs.remove(&res);
        }
    }

    compute_matches_data_routes(tables, &res);
    tables.clean_resource(res);
}

pub fn forget_client_subscription(tables: &mut Tables, face: &Arc<FaceState>, expr: &KeyExpr) {
    match tables.get_mapping(face, &expr.scope) {
        Some(prefix) => match tables.restree.get(prefix, expr.suffix.as_ref()) {
            Some(res) => {
                undeclare_client_subscription(tables, face, res);
            }
            None => log::error!("Undeclare unknown subscription!"),
        },
        None => log::error!("Undeclare subscription with unknown scope!"),
    }
}

pub(crate) fn pubsub_new_face(tables: &mut Tables, face: &Arc<FaceState>) {
    let sub_info = SubInfo {
        reliability: Reliability::Reliable, // @TODO
        mode: SubMode::Push,
        period: None,
    };
    if face.whatami == WhatAmI::Client && tables.whatami != WhatAmI::Client {
        for sub in &tables.router_subs {
            get_mut_unchecked(face).local_subs.insert(sub.clone());
            let key_expr = Tables::decl_key(&mut tables.restree, sub, face);
            face.primitives.decl_subscriber(&key_expr, &sub_info, None);
        }
    }
    if tables.whatami == WhatAmI::Client {
        for face in tables
            .faces
            .values()
            .cloned()
            .collect::<Vec<Arc<FaceState>>>()
        {
            for sub in &face.remote_subs {
                propagate_simple_subscription(tables, sub, &sub_info, &face.clone());
            }
        }
    }
}

pub(crate) fn pubsub_remove_node(tables: &mut Tables, node: &PeerId, net_type: WhatAmI) {
    match net_type {
        WhatAmI::Router => {
            for res in tables
                .router_subs
                .iter()
                .filter(|res| tables.restree.weight(res).router_subs.contains(node))
                .cloned()
                .collect::<Vec<ResourceTreeIndex>>()
            {
                unregister_router_subscription(tables, &res, node);

                compute_matches_data_routes(tables, &res);
                tables.clean_resource(res);
            }
        }
        WhatAmI::Peer => {
            for res in tables
                .peer_subs
                .iter()
                .filter(|res| tables.restree.weight(res).peer_subs.contains(node))
                .cloned()
                .collect::<Vec<ResourceTreeIndex>>()
            {
                unregister_peer_subscription(tables, &res, node);

                if tables.whatami == WhatAmI::Router {
                    let client_subs = tables
                        .restree
                        .weight(&res)
                        .session_ctxs
                        .values()
                        .any(|ctx| ctx.subs.is_some());
                    let peer_subs = remote_peer_subs(tables, &res);
                    if !client_subs && !peer_subs {
                        undeclare_router_subscription(tables, None, &res, &tables.pid.clone());
                    }
                }

                compute_matches_data_routes(tables, &res);
                tables.clean_resource(res);
            }
        }
        _ => (),
    }
}

pub(crate) fn pubsub_tree_change(
    tables: &mut Tables,
    new_childs: &[Vec<NodeIndex>],
    net_type: WhatAmI,
) {
    let net = net!(tables, net_type).unwrap();
    let restree = &mut tables.restree;
    // propagate subs to new childs
    for (tree_sid, tree_childs) in new_childs.iter().enumerate() {
        if !tree_childs.is_empty() {
            let tree_idx = NodeIndex::new(tree_sid);
            if net.graph.contains_node(tree_idx) {
                let tree_id = net.graph[tree_idx].pid;

                let subs_res = match net_type {
                    WhatAmI::Router => &tables.router_subs,
                    _ => &tables.peer_subs,
                };

                for res in subs_res {
                    let mut subs = VecMapWalker::new();
                    while let Some(sub) = subs.walk_next(match net_type {
                        WhatAmI::Router => &restree.weight(res).router_subs,
                        _ => &restree.weight(res).peer_subs,
                    }) {
                        if *sub == tree_id {
                            let sub_info = SubInfo {
                                reliability: Reliability::Reliable, // @TODO
                                mode: SubMode::Push,
                                period: None,
                            };
                            send_sourced_subscription_to_net_childs(
                                restree,
                                &tables.faces,
                                net,
                                tree_childs,
                                res,
                                None,
                                &sub_info,
                                Some(RoutingContext::new(tree_sid as ZInt)),
                            );
                        }
                    }
                }
            }
        }
    }

    // recompute routes
    let mut visit = tables.restree.visit();
    while let Some(res) = visit.walk_next(tables.restree.container()) {
        compute_data_routes(tables, &res);
    }
}

#[inline]
fn insert_faces_for_subs(
    route: &mut Route,
    prefix: &ResourceTreeIndex,
    suffix: &str,
    tables: &Tables,
    net: &Network,
    source: usize,
    subs: &VecSet<PeerId>,
) {
    if net.trees.len() > source {
        for sub in subs {
            if let Some(sub_idx) = net.get_idx(sub) {
                if net.trees[source].directions.len() > sub_idx.index() {
                    if let Some(direction) = net.trees[source].directions[sub_idx.index()] {
                        if net.graph.contains_node(direction) {
                            if let Some(face) = tables.get_face(&net.graph[direction].pid) {
                                route.entry(face.id).or_insert_with(|| {
                                    let key_expr = Tables::get_best_key(
                                        &tables.restree,
                                        prefix,
                                        suffix,
                                        face.id,
                                    );
                                    (
                                        face.clone(),
                                        key_expr.to_owned(),
                                        if source != 0 {
                                            Some(RoutingContext::new(source as ZInt))
                                        } else {
                                            None
                                        },
                                    )
                                });
                            }
                        }
                    }
                }
            }
        }
    } else {
        log::trace!("Tree for node sid:{} not yet ready", source);
    }
}

fn compute_data_route(
    tables: &Tables,
    prefix: &ResourceTreeIndex,
    suffix: &str,
    source: Option<usize>,
    source_type: WhatAmI,
) -> Arc<Route> {
    let mut route = HashMap::new();
    let key_expr = [&tables.restree.expr(prefix), suffix].concat();
    let res = tables.restree.get(prefix, suffix);
    let mut matches = match res.as_ref() {
        Some(res) => Matches::PreComputed(tables.restree.weight(res).matches.iter()),
        None => Matches::Computed(tables.restree.matches(prefix, suffix)),
    };

    let master = tables.whatami != WhatAmI::Router
        || *Tables::elect_router(&key_expr, &tables.shared_nodes) == tables.pid;

    while let Some(mres) = matches.walk_next(tables.restree.container()) {
        if tables.whatami == WhatAmI::Router {
            if master || source_type == WhatAmI::Router {
                let net = tables.routers_net.as_ref().unwrap();
                let router_source = match source_type {
                    WhatAmI::Router => source.unwrap(),
                    _ => net.idx.index(),
                };
                insert_faces_for_subs(
                    &mut route,
                    prefix,
                    suffix,
                    tables,
                    net,
                    router_source,
                    &tables.restree.weight(&mres).router_subs,
                );
            }

            if master || source_type != WhatAmI::Router {
                let net = tables.peers_net.as_ref().unwrap();
                let peer_source = match source_type {
                    WhatAmI::Peer => source.unwrap(),
                    _ => net.idx.index(),
                };
                insert_faces_for_subs(
                    &mut route,
                    prefix,
                    suffix,
                    tables,
                    net,
                    peer_source,
                    &tables.restree.weight(&mres).peer_subs,
                );
            }
        }

        if tables.whatami == WhatAmI::Peer {
            let net = tables.peers_net.as_ref().unwrap();
            let peer_source = match source_type {
                WhatAmI::Router | WhatAmI::Peer => source.unwrap(),
                _ => net.idx.index(),
            };
            insert_faces_for_subs(
                &mut route,
                prefix,
                suffix,
                tables,
                net,
                peer_source,
                &tables.restree.weight(&mres).peer_subs,
            );
        }

        if tables.whatami != WhatAmI::Router || master || source_type == WhatAmI::Router {
            for (sid, context) in &tables.restree.weight(&mres).session_ctxs {
                if let Some(subinfo) = &context.subs {
                    if subinfo.mode == SubMode::Push {
                        route.entry(*sid).or_insert_with(|| {
                            let key_expr =
                                Tables::get_best_key(&tables.restree, prefix, suffix, *sid);
                            (context.face.clone(), key_expr.to_owned(), None)
                        });
                    }
                }
            }
        }
    }
    Arc::new(route)
}

fn compute_matching_pulls(
    tables: &Tables,
    prefix: &ResourceTreeIndex,
    suffix: &str,
) -> Arc<PullCaches> {
    let mut pull_caches = vec![];
    let res = tables.restree.get(prefix, suffix);
    let mut matches = match res.as_ref() {
        Some(res) => Matches::PreComputed(tables.restree.weight(res).matches.iter()),
        None => Matches::Computed(tables.restree.matches(prefix, suffix)),
    };

    while let Some(mres) = matches.walk_next(tables.restree.container()) {
        for context in tables.restree.weight(&mres).session_ctxs.values() {
            if let Some(subinfo) = &context.subs {
                if subinfo.mode == SubMode::Pull {
                    pull_caches.push(context.clone());
                }
            }
        }
    }
    Arc::new(pull_caches)
}

pub(crate) fn compute_data_routes(tables: &mut Tables, res: &ResourceTreeIndex) {
    if tables.whatami == WhatAmI::Router {
        let indexes = tables
            .routers_net
            .as_ref()
            .unwrap()
            .graph
            .node_indices()
            .collect::<Vec<NodeIndex>>();
        let max_idx = indexes.iter().max().unwrap();
        tables.restree.weight_mut(res).routers_data_routes.clear();
        tables
            .restree
            .weight_mut(res)
            .routers_data_routes
            .resize_with(max_idx.index() + 1, || Arc::new(HashMap::new()));

        for idx in &indexes {
            let route = compute_data_route(tables, res, "", Some(idx.index()), WhatAmI::Peer);
            tables.restree.weight_mut(res).routers_data_routes[idx.index()] = route;
        }
    }
    if tables.whatami == WhatAmI::Router || tables.whatami == WhatAmI::Peer {
        let indexes = tables
            .peers_net
            .as_ref()
            .unwrap()
            .graph
            .node_indices()
            .collect::<Vec<NodeIndex>>();
        let max_idx = indexes.iter().max().unwrap();
        tables.restree.weight_mut(res).peers_data_routes.clear();
        tables
            .restree
            .weight_mut(res)
            .peers_data_routes
            .resize_with(max_idx.index() + 1, || Arc::new(HashMap::new()));

        for idx in &indexes {
            let route = compute_data_route(tables, res, "", Some(idx.index()), WhatAmI::Peer);
            tables.restree.weight_mut(res).peers_data_routes[idx.index()] = route;
        }
    }
    if tables.whatami == WhatAmI::Client {
        tables.restree.weight_mut(res).client_data_route =
            Some(compute_data_route(tables, res, "", None, WhatAmI::Client));
    }
    tables.restree.weight_mut(res).matching_pulls = compute_matching_pulls(tables, res, "");
}

pub(crate) fn compute_matches_data_routes(tables: &mut Tables, res: &ResourceTreeIndex) {
    compute_data_routes(tables, res);

    let mut walker = VecWalker::new();
    while let Some(match_) = walker
        .walk_next(&tables.restree.weight(res).matches)
        .cloned()
    {
        if let Ok(match_) = match_.strengthen() {
            if match_ != *res {
                compute_data_routes(tables, &match_);
            }
        }
    }
}

macro_rules! treat_timestamp {
    ($hlc:expr, $info:expr) => {
        // if an HLC was configured (via Config.add_timestamp),
        // check DataInfo and add a timestamp if there isn't
        match $hlc {
            Some(hlc) => {
                if let Some(mut data_info) = $info {
                    if let Some(ref ts) = data_info.timestamp {
                        // Timestamp is present; update HLC with it (possibly raising error if delta exceed)
                        match hlc.update_with_timestamp(ts) {
                            Ok(()) => Some(data_info),
                            Err(e) => {
                                log::error!(
                                    "Error treating timestamp for received Data ({}): drop it!",
                                    e
                                );
                                return;
                            }
                        }
                    } else {
                        // Timestamp not present; add one
                        data_info.timestamp = Some(hlc.new_timestamp());
                        log::trace!("Adding timestamp to DataInfo: {:?}", data_info.timestamp);
                        Some(data_info)
                    }
                } else {
                    // No DataInfo; add one with a Timestamp
                    let mut data_info = DataInfo::new();
                    data_info.timestamp = Some(hlc.new_timestamp());
                    Some(data_info)
                }
            },
            None => $info,
        }
    }
}

#[inline(always)]
fn routers_data_route(
    tables: &Tables,
    res: &ResourceTreeIndex,
    context: usize,
) -> Option<Arc<Route>> {
    let ctx = tables.restree.weight(res);
    (ctx.routers_data_routes.len() > context).then(|| ctx.routers_data_routes[context].clone())
}

#[inline(always)]
fn peers_data_route(
    tables: &Tables,
    res: &ResourceTreeIndex,
    context: usize,
) -> Option<Arc<Route>> {
    let ctx = tables.restree.weight(res);
    (ctx.peers_data_routes.len() > context).then(|| ctx.peers_data_routes[context].clone())
}

#[inline(always)]
pub(super) fn client_data_route(tables: &Tables, res: &ResourceTreeIndex) -> Option<Arc<Route>> {
    tables.restree.weight(res).client_data_route.clone()
}

#[inline]
fn get_data_route(
    tables: &Tables,
    face: &Arc<FaceState>,
    res: &Option<ResourceTreeIndex>,
    prefix: &ResourceTreeIndex,
    suffix: &str,
    routing_context: Option<RoutingContext>,
) -> Arc<Route> {
    match tables.whatami {
        WhatAmI::Router => match face.whatami {
            WhatAmI::Router => {
                let routers_net = tables.routers_net.as_ref().unwrap();
                let local_context = routers_net
                    .get_local_context(routing_context.map(|rc| rc.tree_id), face.link_id);
                res.as_ref()
                    .and_then(|res| routers_data_route(tables, res, local_context))
                    .unwrap_or_else(|| {
                        compute_data_route(
                            tables,
                            prefix,
                            suffix,
                            Some(local_context),
                            WhatAmI::Router,
                        )
                    })
            }
            WhatAmI::Peer => {
                let peers_net = tables.peers_net.as_ref().unwrap();
                let local_context =
                    peers_net.get_local_context(routing_context.map(|rc| rc.tree_id), face.link_id);
                res.as_ref()
                    .and_then(|res| peers_data_route(tables, res, local_context))
                    .unwrap_or_else(|| {
                        compute_data_route(
                            tables,
                            prefix,
                            suffix,
                            Some(local_context),
                            WhatAmI::Peer,
                        )
                    })
            }
            _ => res
                .as_ref()
                .and_then(|res| routers_data_route(tables, res, 0))
                .unwrap_or_else(|| {
                    compute_data_route(tables, prefix, suffix, None, WhatAmI::Client)
                }),
        },
        WhatAmI::Peer => match face.whatami {
            WhatAmI::Router | WhatAmI::Peer => {
                let peers_net = tables.peers_net.as_ref().unwrap();
                let local_context =
                    peers_net.get_local_context(routing_context.map(|rc| rc.tree_id), face.link_id);
                res.as_ref()
                    .and_then(|res| peers_data_route(tables, res, local_context))
                    .unwrap_or_else(|| {
                        compute_data_route(
                            tables,
                            prefix,
                            suffix,
                            Some(local_context),
                            WhatAmI::Peer,
                        )
                    })
            }
            _ => res
                .as_ref()
                .and_then(|res| peers_data_route(tables, res, 0))
                .unwrap_or_else(|| {
                    compute_data_route(tables, prefix, suffix, None, WhatAmI::Client)
                }),
        },
        _ => res
            .as_ref()
            .and_then(|res| client_data_route(tables, res))
            .unwrap_or_else(|| compute_data_route(tables, prefix, suffix, None, WhatAmI::Client)),
    }
}

#[inline]
fn get_matching_pulls(
    tables: &Tables,
    res: &Option<ResourceTreeIndex>,
    prefix: &ResourceTreeIndex,
    suffix: &str,
) -> Arc<PullCaches> {
    res.as_ref()
        .map(|res| tables.restree.weight(res).matching_pulls.clone())
        .unwrap_or_else(|| compute_matching_pulls(tables, prefix, suffix))
}

fn send_to_first(
    route: &Route,
    srcface: &FaceState,
    payload: ZBuf,
    channel: Channel,
    cong_ctrl: CongestionControl,
    data_info: Option<DataInfo>,
) {
    let (outface, key_expr, context) = route.values().next().unwrap();
    if srcface.id != outface.id {
        outface.primitives.send_data(
            &key_expr, payload,
            channel, // @TODO: Need to check the active subscriptions to determine the right reliability value
            cong_ctrl, data_info, *context,
        )
    }
}

fn send_to_all(
    route: &Route,
    srcface: &FaceState,
    payload: ZBuf,
    channel: Channel,
    cong_ctrl: CongestionControl,
    data_info: Option<DataInfo>,
) {
    for (outface, key_expr, context) in route.values() {
        if srcface.id != outface.id {
            outface.primitives.send_data(
                key_expr,
                payload.clone(),
                channel, // @TODO: Need to check the active subscriptions to determine the right reliability value
                cong_ctrl,
                data_info.clone(),
                *context,
            )
        }
    }
}

fn cache_data(
    tables: &Tables,
    matching_pulls: Arc<PullCaches>,
    prefix: ResourceTreeIndex,
    suffix: &str,
    payload: &ZBuf,
    info: Option<&DataInfo>,
) {
    for context in matching_pulls.iter() {
        get_mut_unchecked(context).last_values.insert(
            [&tables.restree.expr(&prefix), suffix].concat(),
            (info.cloned(), payload.clone()),
        );
    }
}

#[inline]
#[allow(clippy::too_many_arguments)]
pub fn route_data(
    tables: &Tables,
    face: &Arc<FaceState>,
    expr: &KeyExpr,
    channel: Channel,
    congestion_control: CongestionControl,
    info: Option<DataInfo>,
    payload: ZBuf,
    routing_context: Option<RoutingContext>,
) {
    match tables.get_mapping(face, &expr.scope).cloned() {
        Some(prefix) => {
            log::trace!(
                "Route data for res {}{}",
                tables.restree.expr(&prefix),
                expr.suffix.as_ref()
            );

            let res = tables.restree.get(&prefix, expr.suffix.as_ref());
            let route = get_data_route(
                tables,
                face,
                &res,
                &prefix,
                expr.suffix.as_ref(),
                routing_context,
            );
            let matching_pulls = get_matching_pulls(tables, &res, &prefix, expr.suffix.as_ref());

            if !(route.is_empty() && matching_pulls.is_empty()) {
                let data_info = treat_timestamp!(&tables.hlc, info);

                if route.len() == 1 && matching_pulls.len() == 0 {
                    send_to_first(
                        &*route,
                        face,
                        payload,
                        channel,
                        congestion_control,
                        data_info,
                    );
                } else {
                    if !matching_pulls.is_empty() {
                        let lock = zlock!(tables.pull_caches_lock);
                        cache_data(
                            &*tables,
                            matching_pulls,
                            prefix,
                            expr.suffix.as_ref(),
                            &payload,
                            data_info.as_ref(),
                        );
                        drop(lock);
                    }
                    send_to_all(
                        &*route,
                        face,
                        payload,
                        channel,
                        congestion_control,
                        data_info,
                    );
                }
            }
        }
        None => {
            log::error!("Route data with unknown scope {}!", expr.scope);
        }
    }
}

#[inline]
#[allow(clippy::too_many_arguments)]
pub fn full_reentrant_route_data(
    tables_ref: &Arc<RwLock<Tables>>,
    face: &Arc<FaceState>,
    expr: &KeyExpr,
    channel: Channel,
    congestion_control: CongestionControl,
    info: Option<DataInfo>,
    payload: ZBuf,
    routing_context: Option<RoutingContext>,
) {
    let tables = zread!(tables_ref);
    match tables.get_mapping(face, &expr.scope).cloned() {
        Some(prefix) => {
            log::trace!(
                "Route data for res {}{}",
                tables.restree.expr(&prefix),
                expr.suffix.as_ref()
            );

            let res = tables.restree.get(&prefix, expr.suffix.as_ref());
            let route = get_data_route(
                &tables,
                face,
                &res,
                &prefix,
                expr.suffix.as_ref(),
                routing_context,
            );
            let matching_pulls = get_matching_pulls(&tables, &res, &prefix, expr.suffix.as_ref());

            if !(route.is_empty() && matching_pulls.is_empty()) {
                let data_info = treat_timestamp!(&tables.hlc, info);

                if route.len() == 1 && matching_pulls.len() == 0 {
                    drop(tables);
                    send_to_first(
                        &*route,
                        face,
                        payload,
                        channel,
                        congestion_control,
                        data_info,
                    );
                } else {
                    if !matching_pulls.is_empty() {
                        let lock = zlock!(tables.pull_caches_lock);
                        cache_data(
                            &*tables,
                            matching_pulls,
                            prefix,
                            expr.suffix.as_ref(),
                            &payload,
                            data_info.as_ref(),
                        );
                        drop(lock);
                    }
                    drop(tables);
                    send_to_all(
                        &*route,
                        face,
                        payload,
                        channel,
                        congestion_control,
                        data_info,
                    );
                }
            }
        }
        None => {
            log::error!("Route data with unknown scope {}!", expr.scope);
        }
    }
}

pub fn pull_data(
    tables: &mut Tables,
    face: &Arc<FaceState>,
    _is_final: bool,
    expr: &KeyExpr,
    _pull_id: ZInt,
    _max_samples: &Option<ZInt>,
) {
    match &tables.get_mapping(face, &expr.scope).cloned() {
        Some(prefix) => match tables.restree.get(prefix, expr.suffix.as_ref()) {
            Some(res) => {
                match tables.restree.weight(&res).session_ctxs.get(&face.id) {
                    Some(ctx) => match &ctx.subs {
                        Some(subinfo) => {
                            let lock = zlock!(tables.pull_caches_lock);
                            for (name, (info, data)) in &ctx.last_values {
                                let key_expr = Tables::get_best_key(
                                    &tables.restree,
                                    tables.restree.root(),
                                    name,
                                    face.id,
                                );
                                face.primitives.send_data(
                                    &key_expr,
                                    data.clone(),
                                    Channel {
                                        priority: Priority::default(), // @TODO: Default value for the time being
                                        reliability: subinfo.reliability,
                                    },
                                    CongestionControl::default(), // @TODO: Default value for the time being
                                    info.clone(),
                                    None,
                                );
                            }
                            get_mut_unchecked(ctx).last_values.clear();
                            drop(lock);
                        }
                        None => {
                            log::error!(
                                "Pull data for unknown subscription {} (no info)!",
                                [tables.restree.expr(prefix).as_ref(), expr.suffix.as_ref()]
                                    .concat()
                            );
                        }
                    },
                    None => {
                        log::error!(
                            "Pull data for unknown subscription {} (no context)!",
                            [tables.restree.expr(prefix).as_ref(), expr.suffix.as_ref()].concat()
                        );
                    }
                }
            }
            None => {
                log::error!(
                    "Pull data for unknown subscription {} (no resource)!",
                    [tables.restree.expr(prefix).as_ref(), expr.suffix.as_ref()].concat()
                );
            }
        },
        None => {
            log::error!("Pull data with unknown scope {}!", expr.scope);
        }
    };
}
