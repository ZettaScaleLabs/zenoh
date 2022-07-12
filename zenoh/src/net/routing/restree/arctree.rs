use std::{
    borrow::Cow,
    hash::{Hash, Hasher},
};

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
use super::*;
use async_std::sync::{Arc, Weak};
use zenoh_collections::{vector_map::VecMap, VecMapWalker};
use zenoh_protocol_core::key_expr;
use zenoh_sync::get_mut_unchecked;

pub struct Node<Weight> {
    parent: Option<Arc<Node<Weight>>>,
    suffix: String,
    childs: VecMap<String, Arc<Node<Weight>>>,
    weight: Option<Weight>,
}

impl<Weight> Node<Weight> {
    fn new(parent: Option<Arc<Node<Weight>>>, suffix: &str, weight: Option<Weight>) -> Self {
        Self {
            parent,
            suffix: String::from(suffix),
            childs: VecMap::new(),
            weight,
        }
    }

    fn expr(&self) -> Cow<'static, str> {
        match &self.parent {
            Some(parent) => [&parent.expr() as &str, &self.suffix].concat().into(),
            None => "".into(),
        }
    }
}

impl<Weight> PartialEq for Node<Weight> {
    fn eq(&self, other: &Self) -> bool {
        self.expr() == other.expr()
    }
}
impl<Weight> Eq for Node<Weight> {}

impl<Weight> Hash for Node<Weight> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.expr().hash(state);
    }
}

pub type Index<Weight> = Arc<Node<Weight>>;
pub type WeakIndex<Weight> = Weak<Node<Weight>>;

impl<Weight> Weaken for Index<Weight> {
    type Weak = WeakIndex<Weight>;

    fn weaken(&self) -> Self::Weak {
        Arc::downgrade(self)
    }
}

impl<Weight> Strengthen for WeakIndex<Weight> {
    type Strong = Index<Weight>;

    type Err = ();

    fn strengthen(&self) -> Result<Self::Strong, Self::Err> {
        self.upgrade().ok_or(())
    }
}

pub struct Matches<Weight>(std::vec::IntoIter<Weak<Node<Weight>>>);

impl<Weight> Walker<&ArcTree<Weight>> for Matches<Weight> {
    type Item = Index<Weight>;

    fn walk_next(&mut self, _context: &ArcTree<Weight>) -> Option<Self::Item> {
        self.0.next().map(|w| w.upgrade().unwrap())
    }
}
#[allow(clippy::type_complexity)]
pub struct Visit<Weight>(
    Vec<(
        Index<Weight>,
        Option<VecMapWalker<String, Arc<Node<Weight>>>>,
    )>,
);

impl<Weight> Walker<&ArcTree<Weight>> for Visit<Weight> {
    type Item = Index<Weight>;

    fn walk_next(&mut self, _context: &ArcTree<Weight>) -> Option<Self::Item> {
        if let Some(last) = self.0.last_mut() {
            let (last_node, childs_walker) = last;
            if let Some(childs_walker) = childs_walker {
                if let Some(child) = childs_walker
                    .walk_next(&last_node.childs)
                    .map(|(_, child)| child.clone())
                {
                    self.0.push((child, None));
                } else {
                    self.0.pop();
                }
                self.walk_next(_context)
            } else {
                *childs_walker = Some(VecMapWalker::new());
                if last_node.weight.is_some() {
                    Some(last_node.clone())
                } else {
                    self.walk_next(_context)
                }
            }
        } else {
            None
        }
    }
}

pub struct ReversePath<Weight> {
    stack: Vec<Index<Weight>>,
}

impl<Weight> Walker<&ArcTree<Weight>> for ReversePath<Weight> {
    type Item = Index<Weight>;

    fn walk_next(&mut self, _context: &ArcTree<Weight>) -> Option<Self::Item> {
        self.stack.pop()
    }
}

pub struct ArcTree<Weight>(std::marker::PhantomData<Weight>);

impl<'a, Weight: 'a> ArcTree<Weight> {
    fn get_ref(&'a self, prefix: &'a Index<Weight>, suffix: &str) -> Option<&'a Index<Weight>> {
        if suffix.is_empty() {
            Some(prefix)
        } else if let Some(stripped_suffix) = suffix.strip_prefix('/') {
            let (chunk, rest) = match stripped_suffix.find('/') {
                Some(idx) => (&suffix[0..(idx + 1)], &suffix[(idx + 1)..]),
                None => (suffix, ""),
            };

            match prefix.childs.get(&chunk) {
                Some(res) => self.get_ref(res, rest),
                None => None,
            }
        } else {
            match &prefix.parent {
                Some(parent) => self.get_ref(parent, &[&prefix.suffix, suffix].concat()),
                None => {
                    let (chunk, rest) = match suffix[1..].find('/') {
                        Some(idx) => (&suffix[0..(idx + 1)], &suffix[(idx + 1)..]),
                        None => (suffix, ""),
                    };

                    match prefix.childs.get(&chunk) {
                        Some(res) => self.get_ref(res, rest),
                        None => None,
                    }
                }
            }
        }
    }

    fn compute_matches(
        &'a self,
        expr: &str,
        is_admin: bool,
        from: &Index<Weight>,
    ) -> Vec<Weak<Node<Weight>>> {
        let mut matches = Vec::new();
        if from.parent.is_none() {
            for child in from.childs.values() {
                matches.append(&mut self.compute_matches(expr, is_admin, child));
            }
            return matches;
        }
        if expr.is_empty() {
            if from.suffix == "/**" || from.suffix == "/" {
                if from.weight.is_some()
                    && is_admin == from.expr().starts_with(key_expr::ADMIN_PREFIX)
                {
                    matches.push(Arc::downgrade(from));
                }
                for child in from.childs.values() {
                    matches.append(&mut self.compute_matches(expr, is_admin, child));
                }
            }
            return matches;
        }
        let (chunk, rest) = key_expr::fst_chunk(expr);
        if key_expr::intersect(chunk, &from.suffix) {
            if rest.is_empty() || rest == "/" || rest == "/**" {
                if from.weight.is_some()
                    && is_admin == from.expr().starts_with(key_expr::ADMIN_PREFIX)
                {
                    matches.push(Arc::downgrade(from));
                }
            } else if chunk == "/**" || from.suffix == "/**" {
                matches.append(&mut self.compute_matches(rest, is_admin, from));
            }
            for child in from.childs.values() {
                matches.append(&mut self.compute_matches(rest, is_admin, child));
                if chunk == "/**" || from.suffix == "/**" {
                    matches.append(&mut self.compute_matches(expr, is_admin, child));
                }
            }
        }
        matches
    }
}

impl<'a, Weight: 'a> ResourceTreeContainer<'a, Weight> for ArcTree<Weight> {
    type Index = Index<Weight>;
    type Matches = Matches<Weight>;
    type Visit = Visit<Weight>;
    type ReversePath = ReversePath<Weight>;

    fn new() -> Self {
        ArcTree(std::marker::PhantomData)
    }

    fn insert_with<F: FnOnce() -> Weight>(&'a mut self, expr: &str, f: F) -> Self::Index {
        Arc::new(Node::new(None, expr, Some(f())))
    }

    #[inline]
    fn get(&self, prefix: &Self::Index, suffix: &str) -> Option<Self::Index> {
        self.get_ref(prefix, suffix).cloned()
    }

    fn get_or_insert_with<F: FnOnce() -> Weight>(
        &'a mut self,
        prefix: &'a Self::Index,
        suffix: &str,
        f: F,
    ) -> Self::Index {
        if suffix.is_empty() {
            if prefix.weight.is_none() {
                get_mut_unchecked(prefix).weight = Some(f());
            }
            prefix.clone()
        } else if let Some(stripped_suffix) = suffix.strip_prefix('/') {
            let (chunk, rest) = match stripped_suffix.find('/') {
                Some(idx) => (&suffix[0..(idx + 1)], &suffix[(idx + 1)..]),
                None => (suffix, ""),
            };

            if get_mut_unchecked(prefix).childs.get(&chunk).is_none() {
                let new = Arc::new(Node::new(Some(prefix.clone()), chunk, None));
                if log::log_enabled!(log::Level::Debug) && rest.is_empty() {
                    log::debug!("Register resource {}", self.expr(&new));
                }
                get_mut_unchecked(prefix)
                    .childs
                    .insert(String::from(chunk), new);
            }

            self.get_or_insert_with(
                get_mut_unchecked(prefix).childs.get_mut(&chunk).unwrap(),
                rest,
                f,
            )
        } else if get_mut_unchecked(prefix).parent.is_some() {
            let suffix = [&prefix.suffix, suffix].concat();
            self.get_or_insert_with(
                get_mut_unchecked(prefix).parent.as_mut().unwrap(),
                &suffix,
                f,
            )
        } else {
            let (chunk, rest) = match suffix[1..].find('/') {
                Some(idx) => (&suffix[0..(idx + 1)], &suffix[(idx + 1)..]),
                None => (suffix, ""),
            };

            if get_mut_unchecked(prefix).childs.get(&chunk).is_none() {
                let new = Arc::new(Node::new(Some(prefix.clone()), chunk, None));
                if log::log_enabled!(log::Level::Debug) && rest.is_empty() {
                    log::debug!("Register resource {}", self.expr(&new));
                }
                get_mut_unchecked(prefix)
                    .childs
                    .insert(String::from(chunk), new);
            }

            self.get_or_insert_with(
                get_mut_unchecked(prefix).childs.get_mut(&chunk).unwrap(),
                rest,
                f,
            )
        }
    }

    fn clean(&mut self, index: Self::Index) -> Option<Weight> {
        let mutres = get_mut_unchecked(&index);
        if let Some(ref mut parent) = mutres.parent {
            if Arc::strong_count(&index) <= 2 && mutres.childs.is_empty() {
                log::debug!("Unregister resource {}", self.expr(&index));
                {
                    get_mut_unchecked(parent).childs.remove(&mutres.suffix);
                }
                self.clean(mutres.parent.take().unwrap());
                return match Arc::try_unwrap(index) {
                    Ok(node) => node.weight,
                    Err(_) => None,
                };
            }
        }
        None
    }

    fn visit_from(&'a self, from: &Self::Index) -> Self::Visit {
        Visit(vec![(from.clone(), None)])
    }

    #[inline]
    fn reverse_path(&'a self, from: &Self::Index, path: &str) -> Self::ReversePath {
        let mut stack = vec![from.clone()];
        let mut current = from;
        let mut rest = path;
        while !rest.is_empty() {
            let (chunk, remainder) = key_expr::fst_chunk(rest);
            if let Some(child) = current.childs.get(&chunk) {
                if child.weight.is_some() {
                    stack.push(child.clone());
                }
                current = child;
                rest = remainder;
            } else {
                break;
            }
        }
        ReversePath { stack }
    }

    fn matches_from(
        &'a self,
        prefix: &'a Self::Index,
        suffix: &str,
        from: &Self::Index,
    ) -> Self::Matches {
        let expr = [&self.expr(prefix), suffix].concat();
        Matches(
            self.compute_matches(&expr, expr.starts_with(key_expr::ADMIN_PREFIX), from)
                .into_iter(),
        )
    }

    #[inline]
    fn expr<'b>(&'b self, index: &'b Self::Index) -> Cow<'b, str> {
        index.expr()
    }

    #[inline]
    fn weight<'b>(&'b self, index: &'b Self::Index) -> &'b Weight {
        index.weight.as_ref().unwrap()
    }

    #[inline]
    fn weight_mut<'b>(&'b mut self, index: &'b Self::Index) -> &'b mut Weight {
        get_mut_unchecked(index).weight.as_mut().unwrap()
    }
}
