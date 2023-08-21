use std::sync::Arc;

//
// Copyright (c) 2022 ZettaScale Technology
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
use async_std::sync::RwLock;
use rand::{Rng, SeedableRng};
use zenoh_core::zerror;
use zenoh_crypto::PseudoRng;
use zenoh_result::ZResult;
use zenoh_shm::{SharedMemoryBuf, SharedMemoryManager, SharedMemoryReader};

pub(crate) type Challenge = u64;

/*************************************/
/*          Authenticator            */
/*************************************/
pub(crate) struct SharedMemoryUnicast {
    // Rust guarantees that fields are dropped in the order of declaration.
    // Buffer needs to be dropped before the manager.
    pub(crate) challenge: SharedMemoryBuf,
    pub(crate) _manager: Arc<std::sync::Mutex<SharedMemoryManager>>,
    pub(crate) reader: RwLock<SharedMemoryReader>,
}

unsafe impl Sync for SharedMemoryUnicast {}

impl SharedMemoryUnicast {
    pub fn make(
        manager: Arc<std::sync::Mutex<SharedMemoryManager>>,
    ) -> ZResult<SharedMemoryUnicast> {
        // Create a challenge for session establishment
        let mut prng = PseudoRng::from_entropy();
        let nonce = prng.gen::<Challenge>();
        let size = std::mem::size_of::<Challenge>();

        let mut guard = manager.lock().map_err(|e| zerror!("{e}"))?;
        let mut challenge = guard.alloc(size).map_err(|e| zerror!("{e}"))?;
        drop(guard);

        let slice = unsafe { challenge.as_mut_slice() };
        slice[0..size].copy_from_slice(&nonce.to_le_bytes());

        let shmauth = SharedMemoryUnicast {
            challenge,
            _manager: manager,
            reader: RwLock::new(SharedMemoryReader::new()),
        };
        Ok(shmauth)
    }
}
