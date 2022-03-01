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

use petgraph::visit::Walker;

pub mod arctree;

pub trait ResourceTreeContainer<'a, Weight>: 'a {
    type Index: Clone + 'a;
    type Matches: Walker<&'a Self, Item = Self::Index>;
    type Visit: Walker<&'a Self, Item = Self::Index>;
    type ReversePath: Walker<&'a Self, Item = Self::Index>;

    fn new() -> Self;
    fn insert(&'a mut self, expr: &str) -> Self::Index
    where
        Weight: Default,
    {
        self.insert_with(expr, Default::default)
    }
    fn insert_with<F: FnOnce() -> Weight>(&'a mut self, expr: &str, f: F) -> Self::Index;
    fn get(&self, prefix: &Self::Index, suffix: &str) -> Option<Self::Index>;
    fn get_or_insert(&'a mut self, prefix: &'a Self::Index, suffix: &str) -> Self::Index
    where
        Weight: Default,
    {
        self.get_or_insert_with(prefix, suffix, Default::default)
    }
    fn get_or_insert_with<F: FnOnce() -> Weight>(
        &'a mut self,
        prefix: &'a Self::Index,
        suffix: &str,
        f: F,
    ) -> Self::Index;
    fn clean(&mut self, index: Self::Index) -> Option<Weight>;

    fn visit_from(&'a self, from: &'a Self::Index) -> Self::Visit;
    fn reverse_path(&'a self, prefix: &'a Self::Index, suffix: &str) -> Self::ReversePath;

    fn matches_from(
        &'a self,
        prefix: &'a Self::Index,
        suffix: &str,
        from: &Self::Index,
    ) -> Self::Matches;

    fn expr<'b>(&'b self, index: &'b Self::Index) -> std::borrow::Cow<'b, str>;
    fn weight<'b>(&'b self, index: &'b Self::Index) -> &'b Weight;
    fn weight_mut<'b>(&'b mut self, index: &'b Self::Index) -> &'b mut Weight;
}

pub trait Weaken {
    type Weak: Strengthen<Strong = Self>;
    fn weaken(&self) -> Self::Weak;
}
pub trait Strengthen {
    type Strong;
    /// `()` if fallible, [`std::convert::Infallible`] otherwise
    type Err;
    fn strengthen(&self) -> Result<Self::Strong, Self::Err>;
}

pub struct ResourceTree<T: for<'a> ResourceTreeContainer<'a, Weight, Index = Index>, Index, Weight>
{
    container: T,
    root: Index,
    weight: std::marker::PhantomData<Weight>,
}
impl<T: for<'a> ResourceTreeContainer<'a, Weight, Index = Index>, Index: Clone, Weight>
    ResourceTree<T, Index, Weight>
{
    pub fn new() -> Self
    where
        Weight: Default,
    {
        let mut container = T::new();
        let root = container.insert("");
        Self {
            container,
            root,
            weight: std::marker::PhantomData,
        }
    }

    #[inline]
    pub fn container(&self) -> &T {
        &self.container
    }

    #[inline]
    pub fn root(&self) -> &Index {
        &self.root
    }

    #[inline]
    pub fn insert(&mut self, expr: &str) -> Index
    where
        Weight: Default,
    {
        self.container.insert(expr)
    }

    #[inline]
    pub fn insert_with<F: FnOnce() -> Weight>(&mut self, expr: &str, f: F) -> Index {
        self.container.insert_with(expr, f)
    }

    #[inline]
    pub fn get(&self, prefix: &Index, suffix: &str) -> Option<Index> {
        self.container.get(prefix, suffix)
    }

    #[inline]
    pub fn get_or_insert<'a>(&'a mut self, prefix: &'a Index, suffix: &str) -> Index
    where
        Weight: Default,
    {
        self.container.get_or_insert(prefix, suffix)
    }

    #[inline]
    pub fn get_or_insert_with<'a, F: FnOnce() -> Weight>(
        &'a mut self,
        prefix: &'a Index,
        suffix: &str,
        f: F,
    ) -> Index {
        self.container.get_or_insert_with(prefix, suffix, f)
    }

    #[inline]
    pub fn clean(&mut self, index: Index) -> Option<Weight> {
        self.container.clean(index)
    }

    #[inline]
    pub fn visit(&self) -> <T as ResourceTreeContainer<'_, Weight>>::Visit {
        self.container.visit_from(&self.root)
    }

    #[inline]
    pub fn visit_from<'a>(
        &'a self,
        from: &'a Index,
    ) -> <T as ResourceTreeContainer<'a, Weight>>::Visit {
        self.container.visit_from(from)
    }

    #[inline]
    pub fn reverse_path<'a>(
        &'a self,
        from: &'a Index,
        path: &str,
    ) -> <T as ResourceTreeContainer<'a, Weight>>::ReversePath {
        self.container.reverse_path(from, path)
    }

    #[inline]
    pub fn matches<'a>(
        &'a self,
        prefix: &'a Index,
        suffix: &str,
    ) -> <T as ResourceTreeContainer<'a, Weight>>::Matches {
        self.container.matches_from(prefix, suffix, &self.root)
    }

    #[inline]
    pub fn matches_from<'a>(
        &'a self,
        prefix: &'a Index,
        suffix: &str,
        from: &Index,
    ) -> <T as ResourceTreeContainer<'a, Weight>>::Matches {
        self.container.matches_from(prefix, suffix, from)
    }

    #[inline]
    pub fn expr<'b>(&'b self, index: &'b Index) -> std::borrow::Cow<'b, str> {
        self.container.expr(index)
    }

    #[inline]
    pub fn weight<'b>(&'b self, index: &'b Index) -> &'b Weight {
        self.container.weight(index)
    }

    #[inline]
    pub fn weight_mut<'b>(&'b mut self, index: &'b Index) -> &'b mut Weight {
        self.container.weight_mut(index)
    }
}

impl<T: for<'a> ResourceTreeContainer<'a, Weight, Index = Index>, Index: Clone, Weight> Default
    for ResourceTree<T, Index, Weight>
where
    Weight: Default,
{
    fn default() -> Self {
        Self::new()
    }
}
