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
#[macro_use]
pub mod fifo_queue;
pub use fifo_queue::*;

pub mod lifo_queue;
pub use lifo_queue::*;

pub mod object_pool;
pub use object_pool::*;

pub(crate) mod ring_buffer;
pub(crate) use ring_buffer::*;

pub(crate) mod stack_buffer;
pub(crate) use stack_buffer::*;

pub mod timer;
pub use timer::*;

pub use petgraph::visit::Walker;
pub use petgraph::visit::WalkerIter;

pub use vector_map;

pub struct VecWalker<T> {
    idx: usize,
    phantom: std::marker::PhantomData<T>,
}

impl<T> VecWalker<T> {
    #[inline]
    pub fn new() -> Self {
        Self {
            idx: 0,
            phantom: std::marker::PhantomData,
        }
    }
}

impl<T> Default for VecWalker<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, T> Walker<&'a Vec<T>> for VecWalker<T> {
    type Item = &'a T;

    #[inline]
    fn walk_next(&mut self, context: &'a Vec<T>) -> Option<Self::Item> {
        if self.idx >= context.len() {
            None
        } else {
            let result = Some(&context[self.idx]);
            self.idx += 1;
            result
        }
    }
}

pub struct VecMapWalker<K, V> {
    index: usize,
    _marker: std::marker::PhantomData<(K, V)>,
}

impl<K, V> VecMapWalker<K, V> {
    #[inline]
    pub fn new() -> Self {
        Self {
            index: 0,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<K, V> Default for VecMapWalker<K, V> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

use vector_map::VecMap;
impl<'a, K, V> Walker<&'a VecMap<K, V>> for VecMapWalker<K, V> {
    type Item = (&'a K, &'a V);

    #[inline]
    fn walk_next(&mut self, context: &'a VecMap<K, V>) -> Option<Self::Item> {
        let res = context.iter().nth(self.index);
        self.index += 1;
        res
    }
}

impl<'a, K, V> Walker<&'a mut VecMap<K, V>> for VecMapWalker<K, V> {
    type Item = (&'a K, &'a mut V);

    #[inline]
    fn walk_next(&mut self, context: &'a mut VecMap<K, V>) -> Option<Self::Item> {
        let res = context.iter_mut().nth(self.index);
        self.index += 1;
        res
    }
}

use vector_map::set::VecSet;
impl<'a, K> Walker<&'a VecSet<K>> for VecMapWalker<K, ()> {
    type Item = &'a K;

    #[inline]
    fn walk_next(&mut self, context: &'a VecSet<K>) -> Option<Self::Item> {
        let res = context.iter().nth(self.index);
        self.index += 1;
        res
    }
}
