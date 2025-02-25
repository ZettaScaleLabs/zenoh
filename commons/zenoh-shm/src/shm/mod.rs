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


#[cfg(target_os="windows")]
mod windows;
use std::{fmt::Display, num::NonZeroUsize, ops::Deref};

use num_traits::Unsigned;
#[cfg(target_os="windows")]
use windows as platform;
#[cfg(any(target_os="freebsd", target_os="linux", target_os="macos"))]
pub mod unix;
#[cfg(any(target_os="freebsd", target_os="linux", target_os="macos"))]
pub use unix as platform;
#[cfg(all(not(target_os="windows"), not(any(target_os="freebsd", target_os="linux", target_os="macos"))))]
compile_error!("shared_memory isnt implemented for this platform...");

pub enum SegmentCreateError {
    SegmentExists,
    OsError(u32),
}

#[derive(Debug)]
pub enum SegmentOpenError {
    OsError(u32),
    ZeroSizedSegment
}

pub type ShmCreateResult<T> = core::result::Result<T, SegmentCreateError>;
pub type ShmOpenResult<T> = core::result::Result<T, SegmentOpenError>;

pub trait SegmentID: Unsigned + Display + Copy + Send + 'static {}
impl<T: Unsigned + Display+ Copy + Send + 'static> SegmentID for T {}

pub struct Segment<ID: SegmentID> {
    inner: platform::SegmentImpl<ID>
}

impl<ID: SegmentID> Segment<ID> {
    pub fn create(id: ID, len: NonZeroUsize) -> ShmCreateResult<Self> {
        let inner = platform::SegmentImpl::create(id, len)?;
        Ok( Self { inner } )
    }

    pub fn open(id: ID) -> ShmOpenResult<Self> {
        let inner = platform::SegmentImpl::open(id)?;
        Ok( Self { inner } )
    }
}

impl<ID: SegmentID> Deref for Segment<ID> {
    type Target = platform::SegmentImpl<ID>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
