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
use async_std::{sync::RwLock, task};
use zenoh_buffers::{reader::HasReader, writer::HasWriter, ZBuf, ZSlice};
use zenoh_codec::{RCodec, WCodec, Zenoh080};
use zenoh_core::{zasyncread, zasyncwrite, zerror};
use zenoh_protocol::{
    network::{NetworkBody, NetworkMessage, Push},
    zenoh_new::{
        put::{ext::ShmType, Put},
        PushBody,
    },
};
use zenoh_result::ZResult;
use zenoh_shm::{SharedMemoryBuf, SharedMemoryBufInfo, SharedMemoryReader};

// ShmBuf -> ShmInfo
pub fn map_zmsg_to_shminfo(msg: &mut NetworkMessage) -> ZResult<bool> {
    let mut res = false;

    if let NetworkBody::Push(Push {
        payload: PushBody::Put(Put {
            payload, ext_shm, ..
        }),
        ..
    }) = &mut msg.body
    {
        res |= map_zbuf_to_shminfo(payload)?;
        if res {
            *ext_shm = Some(ShmType::new());
        }
    }

    Ok(res)
}

pub fn map_zbuf_to_shminfo(zbuf: &mut ZBuf) -> ZResult<bool> {
    let mut res = false;
    for zs in zbuf.zslices_mut() {
        let ZSlice { buf, .. } = zs;
        if let Some(shmb) = buf.as_any().downcast_ref::<SharedMemoryBuf>() {
            *zs = map_zslice_to_shminfo(shmb)?;
            res = true;
        }
    }
    Ok(res)
}

#[cold]
#[inline(never)]
pub fn map_zslice_to_shminfo(shmb: &SharedMemoryBuf) -> ZResult<ZSlice> {
    // Serialize the shmb info
    let codec = Zenoh080::new();
    let mut info = vec![];
    let mut writer = info.writer();
    codec
        .write(&mut writer, &shmb.info)
        .map_err(|e| zerror!("{:?}", e))?;
    // Increase the reference count so to keep the SharedMemoryBuf valid
    shmb.inc_ref_count();
    // Replace the content of the slice
    Ok(info.into())
}

// ShmInfo -> ShmBuf
pub fn map_zmsg_to_shmbuf(
    msg: &mut NetworkMessage,
    shmr: &RwLock<SharedMemoryReader>,
) -> ZResult<bool> {
    let mut res = false;

    if let NetworkBody::Push(Push {
        payload: PushBody::Put(Put {
            payload, ext_shm, ..
        }),
        ..
    }) = &mut msg.body
    {
        if ext_shm.is_some() {
            res |= map_zbuf_to_shmbuf(payload, shmr)?;
            *ext_shm = None;
        }
    }

    Ok(res)
}

pub fn map_zbuf_to_shmbuf(zbuf: &mut ZBuf, shmr: &RwLock<SharedMemoryReader>) -> ZResult<bool> {
    let mut res = false;
    for zs in zbuf.zslices_mut() {
        res |= map_zslice_to_shmbuf(zs, shmr)?;
    }
    Ok(res)
}

#[cold]
#[inline(never)]
pub fn map_zslice_to_shmbuf(
    zslice: &mut ZSlice,
    shmr: &RwLock<SharedMemoryReader>,
) -> ZResult<bool> {
    // Deserialize the shmb info into shm buff
    let codec = Zenoh080::new();
    let mut reader = zslice.reader();

    let shmbinfo: SharedMemoryBufInfo = codec.read(&mut reader).map_err(|e| zerror!("{:?}", e))?;

    // First, try in read mode allowing concurrenct lookups
    let r_guard = task::block_on(async { zasyncread!(shmr) });
    let smb = r_guard.try_read_shmbuf(&shmbinfo).or_else(|_| {
        drop(r_guard);
        let mut w_guard = task::block_on(async { zasyncwrite!(shmr) });
        w_guard.read_shmbuf(&shmbinfo)
    })?;

    // Replace the content of the slice
    let zs: ZSlice = smb.into();
    *zslice = zs;

    Ok(true)
}
