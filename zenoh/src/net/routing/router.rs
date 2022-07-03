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
use super::face::{Face, FaceId, FaceState};
use super::network::{shared_nodes, Network};
pub use super::pubsub::*;
pub use super::queries::*;
use super::restree::ResourceTreeContainer;
use super::restree::{
    arctree::ArcTree, arctree::Index, arctree::WeakIndex, ResourceTree as ResTree, Strengthen,
    Weaken,
};
use super::runtime::Runtime;
use async_std::sync::{Arc, Weak};
use async_std::task::JoinHandle;
use std::any::Any;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::Hasher;
use std::sync::{Mutex, RwLock};
use std::time::Duration;
use uhlc::HLC;
use zenoh_buffers::ZBuf;
use zenoh_collections::{VecWalker, Walker};
use zenoh_link::Link;
use zenoh_protocol::proto::{DataInfo, RoutingContext, ZenohBody, ZenohMessage};
use zenoh_protocol_core::key_expr;
use zenoh_protocol_core::{KeyExpr, PeerId, QueryableInfo, SubInfo, WhatAmI, ZInt};
use zenoh_transport::{DeMux, Mux, Primitives, TransportPeerEventHandler, TransportUnicast};
// use zenoh_collections::Timer;
use zenoh_collections::{vector_map::set::VecSet, vector_map::VecMap};
use zenoh_core::zconfigurable;
use zenoh_core::Result as ZResult;
use zenoh_sync::get_mut_unchecked;

zconfigurable! {
    static ref TREES_COMPUTATION_DELAY: u64 = 100;
}

pub(crate) enum Matches<'a> {
    Computed(<ArcTree<ResourceContext> as ResourceTreeContainer<'a, ResourceContext>>::Matches),
    PreComputed(std::slice::Iter<'a, ResourceTreeWeakIndex>),
}

impl Walker<&ArcTree<ResourceContext>> for Matches<'_> {
    type Item = ResourceTreeIndex;

    fn walk_next(&mut self, _context: &ArcTree<ResourceContext>) -> Option<Self::Item> {
        match self {
            Matches::Computed(walker) => walker.walk_next(_context),
            Matches::PreComputed(iter) => iter.next().map(|w| w.upgrade().unwrap()),
        }
    }
}

pub(crate) type ResourceTree =
    ResTree<ArcTree<ResourceContext>, ResourceTreeIndex, ResourceContext>;
pub(crate) type ResourceTreeIndex = Index<ResourceContext>;
pub(crate) type ResourceTreeWeakIndex = WeakIndex<ResourceContext>;
pub(super) type Direction = (Arc<FaceState>, KeyExpr<'static>, Option<RoutingContext>);
pub(super) type Route = HashMap<FaceId, Direction>;
#[cfg(feature = "complete_n")]
pub(super) type QueryRoute = HashMap<FaceId, (Direction, zenoh_protocol_core::Target)>;
#[cfg(not(feature = "complete_n"))]
pub(super) type QueryRoute = Route;
pub(super) struct TargetQabl {
    pub(super) direction: Direction,
    pub(super) kind: ZInt,
    pub(super) complete: ZInt,
    pub(super) distance: f64,
}

pub(super) type TargetQablSet = Vec<TargetQabl>;
pub(super) type PullCaches = Vec<Arc<SessionContext>>;

pub(crate) struct SessionContext {
    pub(super) face: Arc<FaceState>,
    pub(super) local_expr_id: Option<ZInt>,
    pub(super) remote_expr_id: Option<ZInt>,
    pub(super) subs: Option<SubInfo>,
    pub(super) qabl: HashMap<ZInt, QueryableInfo>,
    pub(super) last_values: HashMap<String, (Option<DataInfo>, ZBuf)>,
}
pub(crate) struct ResourceContext {
    pub(super) matches: Vec<ResourceTreeWeakIndex>,
    pub(super) session_ctxs: VecMap<FaceId, Arc<SessionContext>>,
    pub(super) router_subs: VecSet<PeerId>,
    pub(super) peer_subs: VecSet<PeerId>,
    pub(super) router_qabls: VecMap<(PeerId, ZInt), QueryableInfo>,
    pub(super) peer_qabls: VecMap<(PeerId, ZInt), QueryableInfo>,
    pub(super) matching_pulls: Arc<PullCaches>,
    pub(super) routers_data_routes: Vec<Arc<Route>>,
    pub(super) peers_data_routes: Vec<Arc<Route>>,
    pub(super) client_data_route: Option<Arc<Route>>,
    pub(super) routers_query_routes: Vec<Arc<TargetQablSet>>,
    pub(super) peers_query_routes: Vec<Arc<TargetQablSet>>,
    pub(super) client_query_route: Option<Arc<TargetQablSet>>,
}

impl Default for ResourceContext {
    fn default() -> Self {
        Self {
            matches: vec![],
            session_ctxs: VecMap::new(),
            router_subs: VecSet::new(),
            peer_subs: VecSet::new(),
            router_qabls: VecMap::new(),
            peer_qabls: VecMap::new(),
            matching_pulls: Arc::new(vec![]),
            routers_data_routes: vec![],
            peers_data_routes: vec![],
            client_data_route: None,
            routers_query_routes: vec![],
            peers_query_routes: vec![],
            client_query_route: None,
        }
    }
}

pub struct Tables {
    pub(crate) pid: PeerId,
    pub(crate) whatami: WhatAmI,
    face_counter: usize,
    #[allow(dead_code)]
    pub(crate) hlc: Option<Arc<HLC>>,
    // pub(crate) timer: Timer,
    // pub(crate) queries_default_timeout: Duration,
    pub(crate) restree: ResourceTree,
    pub(crate) faces: HashMap<FaceId, Arc<FaceState>>,
    pub(crate) pull_caches_lock: Mutex<()>,
    pub(crate) router_subs: HashSet<ResourceTreeIndex>,
    pub(crate) peer_subs: HashSet<ResourceTreeIndex>,
    pub(crate) router_qabls: HashSet<ResourceTreeIndex>,
    pub(crate) peer_qabls: HashSet<ResourceTreeIndex>,
    pub(crate) routers_net: Option<Network>,
    pub(crate) peers_net: Option<Network>,
    pub(crate) shared_nodes: Vec<PeerId>,
    pub(crate) routers_trees_task: Option<JoinHandle<()>>,
    pub(crate) peers_trees_task: Option<JoinHandle<()>>,
}

impl Tables {
    pub fn new(
        pid: PeerId,
        whatami: WhatAmI,
        hlc: Option<Arc<HLC>>,
        _queries_default_timeout: Duration,
    ) -> Self {
        Tables {
            pid,
            whatami,
            face_counter: 0,
            hlc,
            // timer: Timer::new(true),
            // queries_default_timeout,
            restree: ResourceTree::new(),
            faces: HashMap::new(),
            pull_caches_lock: Mutex::new(()),
            router_subs: HashSet::new(),
            peer_subs: HashSet::new(),
            router_qabls: HashSet::new(),
            peer_qabls: HashSet::new(),
            routers_net: None,
            peers_net: None,
            shared_nodes: vec![],
            routers_trees_task: None,
            peers_trees_task: None,
        }
    }

    #[doc(hidden)]
    pub fn _contains_res(&self, expr: &str) -> bool {
        self.restree.get(self.restree.root(), expr).is_some()
    }

    #[doc(hidden)]
    pub fn _matches(&self, expr: &str) -> Vec<String> {
        self.restree
            .matches(self.restree.root(), expr)
            .iter(self.restree.container())
            .map(|m| self.restree.expr(&m).to_string())
            .collect()
    }

    #[doc(hidden)]
    pub fn _print_tree(&self) {
        let mut visit = self.restree.visit();
        while let Some(res) = visit.walk_next(self.restree.container()) {
            for ctx in self.restree.weight(&res).session_ctxs.values() {
                println!(
                    "{}: {} {:?} {:?}",
                    self.restree.expr(&res),
                    ctx.face.id,
                    ctx.local_expr_id,
                    ctx.remote_expr_id
                );
            }
        }
    }

    #[inline]
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub(crate) fn get_mapping<'a>(
        &'a self,
        face: &'a FaceState,
        expr_id: &ZInt,
    ) -> Option<&'a ResourceTreeIndex> {
        match expr_id {
            0 => Some(self.restree.root()),
            expr_id => face.get_mapping(expr_id),
        }
    }

    #[inline]
    pub(crate) fn get_face(&self, pid: &PeerId) -> Option<&Arc<FaceState>> {
        self.faces.values().find(|face| face.pid == *pid)
    }

    fn open_net_face(
        &mut self,
        pid: PeerId,
        whatami: WhatAmI,
        primitives: Arc<dyn Primitives + Send + Sync>,
        link_id: usize,
    ) -> Weak<FaceState> {
        let fid = FaceId::new(self.face_counter);
        self.face_counter += 1;
        let newface = self
            .faces
            .entry(fid)
            .or_insert_with(|| FaceState::new(fid, pid, whatami, primitives.clone(), link_id))
            .clone();
        log::debug!("New {}", newface);

        pubsub_new_face(self, &newface);
        queries_new_face(self, &newface);

        Arc::downgrade(&newface)
    }

    pub fn open_face(
        &mut self,
        pid: PeerId,
        whatami: WhatAmI,
        primitives: Arc<dyn Primitives + Send + Sync>,
    ) -> Weak<FaceState> {
        self.open_net_face(pid, whatami, primitives, 0)
    }

    pub fn close_face(&mut self, face: &Weak<FaceState>) {
        match face.upgrade() {
            Some(face) => {
                log::debug!("Close {}", face);
                finalize_pending_queries(self, &face);

                let face_clone = face.clone();
                let face = get_mut_unchecked(&face);
                for (_, res) in face.remote_mappings.drain() {
                    self.restree.weight_mut(&res).session_ctxs.remove(&face.id);
                    self.restree.clean(res);
                }
                for (_, res) in face.local_mappings.drain() {
                    self.restree.weight_mut(&res).session_ctxs.remove(&face.id);
                    self.restree.clean(res);
                }
                for res in face.remote_subs.drain() {
                    self.restree.weight_mut(&res).session_ctxs.remove(&face.id);
                    undeclare_client_subscription(self, &face_clone, res);
                    // Resource::clean(&mut res);
                }
                for (res, kind) in face.remote_qabls.drain() {
                    self.restree.weight_mut(&res).session_ctxs.remove(&face.id);
                    undeclare_client_queryable(self, &face_clone, res, kind);
                    // Resource::clean(&mut res);
                }
                self.faces.remove(&face.id);
            }
            None => log::error!("Face already closed!"),
        }
    }

    pub fn register_expr(&mut self, face: &Arc<FaceState>, expr_id: ZInt, expr: &KeyExpr) {
        match self.get_mapping(face, &expr.scope).cloned() {
            Some(prefix) => match face.remote_mappings.get(&expr_id) {
                Some(res) => {
                    if self.restree.expr(res)
                        != format!("{}{}", self.restree.expr(&prefix), expr.suffix)
                    {
                        log::error!("Resource {} remapped. Remapping unsupported!", expr_id);
                    }
                }
                None => {
                    let res = self.restree.get_or_insert(&prefix, expr.suffix.as_ref());
                    self.match_resource(&res);

                    let session_ctxs = &mut self.restree.weight_mut(&res).session_ctxs;
                    if !session_ctxs.contains_key(&face.id) {
                        session_ctxs.insert(
                            face.id,
                            Arc::new(SessionContext {
                                face: face.clone(),
                                local_expr_id: None,
                                remote_expr_id: Some(expr_id),
                                subs: None,
                                qabl: HashMap::new(),
                                last_values: HashMap::new(),
                            }),
                        );
                    }
                    let ctx = session_ctxs.get(&face.id).unwrap();

                    if face.local_mappings.get(&expr_id).is_some() && ctx.local_expr_id == None {
                        let local_expr_id = get_mut_unchecked(face).get_next_local_id();
                        get_mut_unchecked(ctx).local_expr_id = Some(local_expr_id);

                        get_mut_unchecked(face)
                            .local_mappings
                            .insert(local_expr_id, res.clone());

                        face.primitives.decl_resource(
                            local_expr_id,
                            &self.restree.expr(&res).into_owned().into(),
                        );
                    }

                    get_mut_unchecked(face)
                        .remote_mappings
                        .insert(expr_id, res.clone());
                    self.compute_matches_routes(&res);
                }
            },
            None => log::error!("Declare resource with unknown scope {}!", expr.scope),
        }
    }

    pub fn unregister_expr(&mut self, face: &Arc<FaceState>, expr_id: ZInt) {
        match get_mut_unchecked(face).remote_mappings.remove(&expr_id) {
            Some(res) => {
                self.restree.clean(res);
            }
            None => log::error!("Undeclare unknown resource!"),
        }
    }

    pub(super) fn match_resource(&mut self, res: &ResourceTreeIndex) {
        let mut matches = self
            .restree
            .matches(res, "")
            .iter(self.restree.container())
            .map(|i| i.weaken())
            .collect::<Vec<ResourceTreeWeakIndex>>();

        fn matches_contain(matches: &[ResourceTreeWeakIndex], res: &ResourceTreeIndex) -> bool {
            for match_ in matches {
                if Arc::ptr_eq(&match_.upgrade().unwrap(), res) {
                    return true;
                }
            }
            false
        }

        for match_ in &mut matches {
            let match_ = match_.strengthen().unwrap();
            if !matches_contain(&self.restree.weight(&match_).matches, res) {
                self.restree
                    .weight_mut(&match_)
                    .matches
                    .push(Arc::downgrade(res));
            }
        }
        self.restree.weight_mut(res).matches = matches;
    }

    pub(crate) fn clean_resource(&mut self, res: ResourceTreeIndex) {
        let expr = if log::log_enabled!(log::Level::Debug) {
            self.restree.expr(&res).to_string()
        } else {
            "".into()
        };
        if let Some(context) = self.restree.clean(res) {
            log::debug!("Unregister resource {}", expr);
            for match_ in context.matches {
                if let Ok(match_) = match_.strengthen() {
                    self.restree
                        .weight_mut(&match_)
                        .matches
                        .retain(|m| m.strengthen().is_ok());
                }
            }
        }
    }

    #[inline]
    pub(super) fn decl_key(
        restree: &mut ResourceTree,
        res: &ResourceTreeIndex,
        face: &Arc<FaceState>,
    ) -> KeyExpr<'static> {
        let expr = restree.expr(res).to_string();
        let (nonwild_prefix, wildsuffix) = key_expr::nonwild_prefix(expr.as_ref());
        if nonwild_prefix.is_empty() {
            wildsuffix.to_string().into()
        } else {
            let prefix = restree.get_or_insert(&restree.root().clone(), nonwild_prefix);
            let session_ctxs = &mut restree.weight_mut(&prefix).session_ctxs;
            if !session_ctxs.contains_key(&face.id) {
                session_ctxs.insert(
                    face.id,
                    Arc::new(SessionContext {
                        face: face.clone(),
                        local_expr_id: None,
                        remote_expr_id: None,
                        subs: None,
                        qabl: HashMap::new(),
                        last_values: HashMap::new(),
                    }),
                );
            }
            let ctx = session_ctxs.get(&face.id).unwrap();

            let expr_id = match ctx.local_expr_id.or(ctx.remote_expr_id) {
                Some(expr_id) => expr_id,
                None => {
                    let expr_id = face.get_next_local_id();
                    get_mut_unchecked(ctx).local_expr_id = Some(expr_id);
                    get_mut_unchecked(face)
                        .local_mappings
                        .insert(expr_id, prefix);
                    face.primitives
                        .decl_resource(expr_id, &nonwild_prefix.into());
                    expr_id
                }
            };
            KeyExpr {
                scope: expr_id,
                suffix: wildsuffix.to_string().into(),
            }
        }
    }

    #[inline]
    pub(super) fn get_best_key<'a>(
        restree: &ResourceTree,
        prefix: &ResourceTreeIndex,
        suffix: &'a str,
        sid: FaceId,
    ) -> KeyExpr<'a> {
        let mut path = restree.reverse_path(prefix, suffix);
        while let Some(res) = path.walk_next(restree.container()) {
            if let Some(ctx) = restree.weight(&res).session_ctxs.get(&sid) {
                if let Some(scope) = ctx.local_expr_id {
                    return KeyExpr {
                        scope,
                        suffix: suffix[(restree.expr(&res).len() - restree.expr(prefix).len())..]
                            .into(),
                    };
                }
            }
        }
        let prefix_key = restree.expr(prefix);
        if !prefix_key.is_empty() {
            let key = Tables::get_best_key(restree, restree.root(), &prefix_key, sid);
            KeyExpr {
                scope: key.scope,
                suffix: [&key.suffix, suffix].concat().into(),
            }
        } else {
            KeyExpr {
                scope: 0,
                suffix: suffix.into(),
            }
        }
    }

    #[inline]
    pub(super) fn elect_router<'a>(key_expr: &str, routers: &'a [PeerId]) -> &'a PeerId {
        if routers.len() == 1 {
            &routers[0]
        } else {
            routers
                .iter()
                .map(|router| {
                    let mut hasher = DefaultHasher::new();
                    for b in key_expr.as_bytes() {
                        hasher.write_u8(*b);
                    }
                    for b in router.as_slice() {
                        hasher.write_u8(*b);
                    }
                    (router, hasher.finish())
                })
                .max_by(|(_, s1), (_, s2)| s1.partial_cmp(s2).unwrap())
                .unwrap()
                .0
        }
    }

    fn compute_routes(&mut self, res: &ResourceTreeIndex) {
        compute_data_routes(self, res);
        compute_query_routes(self, res);
    }

    pub(crate) fn compute_matches_routes(&mut self, res: &ResourceTreeIndex) {
        self.compute_routes(res);

        let mut walker = VecWalker::new();
        while let Some(match_) = walker.walk_next(&self.restree.weight(res).matches).cloned() {
            if let Ok(match_) = match_.strengthen() {
                if !Arc::ptr_eq(&match_, res) {
                    self.compute_routes(&match_);
                }
            }
        }
    }

    pub(crate) fn schedule_compute_trees(
        &mut self,
        tables_ref: Arc<RwLock<Tables>>,
        net_type: WhatAmI,
    ) {
        log::trace!("Schedule computations");
        if (net_type == WhatAmI::Router && self.routers_trees_task.is_none())
            || (net_type == WhatAmI::Peer && self.peers_trees_task.is_none())
        {
            let task = Some(async_std::task::spawn(async move {
                async_std::task::sleep(std::time::Duration::from_millis(*TREES_COMPUTATION_DELAY))
                    .await;
                let mut tables = zwrite!(tables_ref);

                log::trace!("Compute trees");
                let new_childs = match net_type {
                    WhatAmI::Router => tables.routers_net.as_mut().unwrap().compute_trees(),
                    _ => tables.peers_net.as_mut().unwrap().compute_trees(),
                };

                log::trace!("Compute routes");
                pubsub_tree_change(&mut tables, &new_childs, net_type);
                queries_tree_change(&mut tables, &new_childs, net_type);

                log::trace!("Computations completed");
                match net_type {
                    WhatAmI::Router => tables.routers_trees_task = None,
                    _ => tables.peers_trees_task = None,
                };
            }));
            match net_type {
                WhatAmI::Router => self.routers_trees_task = task,
                _ => self.peers_trees_task = task,
            };
        }
    }
}

macro_rules! net {
    ($tables:expr, $net_type:expr) => {
        match $net_type {
            WhatAmI::Router => $tables.routers_net.as_ref(),
            WhatAmI::Peer => $tables.peers_net.as_ref(),
            _ => None,
        }
    };
}

pub(crate) use net;

pub struct Router {
    whatami: WhatAmI,
    pub tables: Arc<RwLock<Tables>>,
}

impl Router {
    pub fn new(
        pid: PeerId,
        whatami: WhatAmI,
        hlc: Option<Arc<HLC>>,
        queries_default_timeout: Duration,
    ) -> Self {
        Router {
            whatami,
            tables: Arc::new(RwLock::new(Tables::new(
                pid,
                whatami,
                hlc,
                queries_default_timeout,
            ))),
        }
    }

    pub fn init_link_state(
        &mut self,
        runtime: Runtime,
        peers_autoconnect: bool,
        routers_autoconnect_gossip: bool,
    ) {
        let mut tables = zwrite!(self.tables);
        tables.peers_net = Some(Network::new(
            "[Peers network]".to_string(),
            tables.pid,
            runtime.clone(),
            peers_autoconnect,
            routers_autoconnect_gossip,
        ));
        if runtime.whatami == WhatAmI::Router {
            tables.routers_net = Some(Network::new(
                "[Routers network]".to_string(),
                tables.pid,
                runtime,
                peers_autoconnect,
                routers_autoconnect_gossip,
            ));
            tables.shared_nodes = shared_nodes(
                tables.routers_net.as_ref().unwrap(),
                tables.peers_net.as_ref().unwrap(),
            );
        }
    }

    pub fn new_primitives(&self, primitives: Arc<dyn Primitives + Send + Sync>) -> Arc<Face> {
        Arc::new(Face {
            tables: self.tables.clone(),
            state: {
                let mut tables = zwrite!(self.tables);
                let pid = tables.pid;
                tables
                    .open_face(pid, WhatAmI::Client, primitives)
                    .upgrade()
                    .unwrap()
            },
        })
    }

    pub fn new_transport_unicast(
        &self,
        transport: TransportUnicast,
    ) -> ZResult<Arc<LinkStateInterceptor>> {
        let mut tables = zwrite!(self.tables);
        let whatami = transport.get_whatami()?;

        let link_id = match (self.whatami, whatami) {
            (WhatAmI::Router, WhatAmI::Router) => tables
                .routers_net
                .as_mut()
                .unwrap()
                .add_link(transport.clone()),
            (WhatAmI::Router, WhatAmI::Peer)
            | (WhatAmI::Peer, WhatAmI::Router)
            | (WhatAmI::Peer, WhatAmI::Peer) => tables
                .peers_net
                .as_mut()
                .unwrap()
                .add_link(transport.clone()),
            _ => 0,
        };

        if tables.whatami == WhatAmI::Router {
            tables.shared_nodes = shared_nodes(
                tables.routers_net.as_ref().unwrap(),
                tables.peers_net.as_ref().unwrap(),
            );
        }

        let handler = Arc::new(LinkStateInterceptor::new(
            transport.clone(),
            self.tables.clone(),
            Face {
                tables: self.tables.clone(),
                state: tables
                    .open_net_face(
                        transport.get_pid().unwrap(),
                        whatami,
                        Arc::new(Mux::new(transport)),
                        link_id,
                    )
                    .upgrade()
                    .unwrap(),
            },
        ));

        match (self.whatami, whatami) {
            (WhatAmI::Router, WhatAmI::Router) => {
                tables.schedule_compute_trees(self.tables.clone(), WhatAmI::Router);
            }
            (WhatAmI::Router, WhatAmI::Peer)
            | (WhatAmI::Peer, WhatAmI::Router)
            | (WhatAmI::Peer, WhatAmI::Peer) => {
                tables.schedule_compute_trees(self.tables.clone(), WhatAmI::Peer);
            }
            _ => (),
        }
        Ok(handler)
    }
}

pub struct LinkStateInterceptor {
    pub(crate) transport: TransportUnicast,
    pub(crate) tables: Arc<RwLock<Tables>>,
    pub(crate) face: Face,
    pub(crate) demux: DeMux<Face>,
}

impl LinkStateInterceptor {
    fn new(transport: TransportUnicast, tables: Arc<RwLock<Tables>>, face: Face) -> Self {
        LinkStateInterceptor {
            transport,
            tables,
            face: face.clone(),
            demux: DeMux::new(face),
        }
    }
}

impl TransportPeerEventHandler for LinkStateInterceptor {
    fn handle_message(&self, msg: ZenohMessage) -> ZResult<()> {
        log::trace!("Recv {:?}", msg);
        match msg.body {
            ZenohBody::LinkStateList(list) => {
                if let Ok(pid) = self.transport.get_pid() {
                    let mut tables = zwrite!(self.tables);
                    let whatami = self.transport.get_whatami()?;
                    match (tables.whatami, whatami) {
                        (WhatAmI::Router, WhatAmI::Router) => {
                            for (_, removed_node) in tables
                                .routers_net
                                .as_mut()
                                .unwrap()
                                .link_states(list.link_states, pid)
                            {
                                pubsub_remove_node(&mut tables, &removed_node.pid, WhatAmI::Router);
                                queries_remove_node(
                                    &mut tables,
                                    &removed_node.pid,
                                    WhatAmI::Router,
                                );
                            }

                            tables.shared_nodes = shared_nodes(
                                tables.routers_net.as_ref().unwrap(),
                                tables.peers_net.as_ref().unwrap(),
                            );

                            tables.schedule_compute_trees(self.tables.clone(), WhatAmI::Router);
                        }
                        (WhatAmI::Router, WhatAmI::Peer)
                        | (WhatAmI::Peer, WhatAmI::Router)
                        | (WhatAmI::Peer, WhatAmI::Peer) => {
                            for (_, removed_node) in tables
                                .peers_net
                                .as_mut()
                                .unwrap()
                                .link_states(list.link_states, pid)
                            {
                                pubsub_remove_node(&mut tables, &removed_node.pid, WhatAmI::Peer);
                                queries_remove_node(&mut tables, &removed_node.pid, WhatAmI::Peer);
                            }

                            if tables.whatami == WhatAmI::Router {
                                tables.shared_nodes = shared_nodes(
                                    tables.routers_net.as_ref().unwrap(),
                                    tables.peers_net.as_ref().unwrap(),
                                );
                            }

                            tables.schedule_compute_trees(self.tables.clone(), WhatAmI::Peer);
                        }
                        _ => (),
                    };
                }

                Ok(())
            }
            _ => self.demux.handle_message(msg),
        }
    }

    fn new_link(&self, _link: Link) {}

    fn del_link(&self, _link: Link) {}

    fn closing(&self) {
        self.demux.closing();
        let tables_ref = self.tables.clone();
        match (self.transport.get_pid(), self.transport.get_whatami()) {
            (Ok(pid), Ok(whatami)) => {
                let mut tables = zwrite!(tables_ref);
                match (tables.whatami, whatami) {
                    (WhatAmI::Router, WhatAmI::Router) => {
                        for (_, removed_node) in
                            tables.routers_net.as_mut().unwrap().remove_link(&pid)
                        {
                            pubsub_remove_node(&mut tables, &removed_node.pid, WhatAmI::Router);
                            queries_remove_node(&mut tables, &removed_node.pid, WhatAmI::Router);
                        }

                        tables.shared_nodes = shared_nodes(
                            tables.routers_net.as_ref().unwrap(),
                            tables.peers_net.as_ref().unwrap(),
                        );

                        tables.schedule_compute_trees(tables_ref.clone(), WhatAmI::Router);
                    }
                    (WhatAmI::Router, WhatAmI::Peer)
                    | (WhatAmI::Peer, WhatAmI::Router)
                    | (WhatAmI::Peer, WhatAmI::Peer) => {
                        for (_, removed_node) in
                            tables.peers_net.as_mut().unwrap().remove_link(&pid)
                        {
                            pubsub_remove_node(&mut tables, &removed_node.pid, WhatAmI::Peer);
                            queries_remove_node(&mut tables, &removed_node.pid, WhatAmI::Peer);
                        }

                        if tables.whatami == WhatAmI::Router {
                            tables.shared_nodes = shared_nodes(
                                tables.routers_net.as_ref().unwrap(),
                                tables.peers_net.as_ref().unwrap(),
                            );
                        }

                        tables.schedule_compute_trees(tables_ref.clone(), WhatAmI::Peer);
                    }
                    _ => (),
                };
            }
            (_, _) => log::error!("Closed transport in session closing!"),
        }
    }

    fn closed(&self) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}
