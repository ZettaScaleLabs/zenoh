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
use super::transport::TransportUnicastUniversal;
#[cfg(feature = "stats")]
use crate::common::stats::TransportStats;
use crate::{
    common::{
        batch::{Finalize, RBatch},
        pipeline::TransmissionPipelineConsumer,
        priority::TransportPriorityTx,
    },
    unicast::link::{TransportLinkUnicast, TransportLinkUnicastRx, TransportLinkUnicastTx},
};
use async_std::prelude::FutureExt;
use async_std::{
    sync::RwLock as AsyncRwLock,
    task::{self, JoinHandle},
};
use std::{
    sync::{Arc, RwLock},
    time::Duration,
};
use zenoh_buffers::ZSliceBuffer;
use zenoh_core::{zasyncread, zasyncwrite, zwrite};
use zenoh_protocol::transport::{KeepAlive, TransportMessage};
use zenoh_result::{zerror, ZResult};
use zenoh_sync::{RecyclingObject, RecyclingObjectPool, Signal};

pub(super) struct Tasks {
    // The handlers to stop TX/RX tasks
    handle_tx: RwLock<Option<async_executor::Task<()>>>,
    signal_rx: Signal,
    handle_rx: RwLock<Option<JoinHandle<()>>>,
}

#[derive(Clone)]
pub(super) struct TransportLinkUnicastUniversal {
    // The underlying link
    pub(super) link: TransportLinkUnicast,
    // The task handling substruct
    tasks: Arc<Tasks>,
}

impl TransportLinkUnicastUniversal {
    pub(super) fn new(link: TransportLinkUnicast, priority_tx: &[TransportPriorityTx]) -> Self {
        assert!(!priority_tx.is_empty());

        let tasks = Arc::new(Tasks {
            handle_tx: RwLock::new(None),
            signal_rx: Signal::new(),
            handle_rx: RwLock::new(None),
        });

        Self { link, tasks }
    }
}

impl TransportLinkUnicastUniversal {
    pub(super) fn start_rx(&mut self, transport: TransportUnicastUniversal, lease: Duration) {
        let mut guard = zwrite!(self.tasks.handle_rx);
        if guard.is_none() {
            // Spawn the RX task
            let mut rx = self.link.rx();
            let c_signal = self.tasks.signal_rx.clone();

            let handle = task::spawn(async move {
                // Start the consume task
                let res = rx_task(
                    &mut rx,
                    transport.clone(),
                    lease,
                    c_signal.clone(),
                    transport.manager.config.link_rx_buffer_size,
                )
                .await;
                c_signal.trigger();
                if let Err(e) = res {
                    log::debug!("{}", e);
                    // Spawn a task to avoid a deadlock waiting for this same task
                    // to finish in the close() joining its handle
                    task::spawn(async move { transport.del_link((&rx.link).into()).await });
                }
            });
            *guard = Some(handle);
        }
    }

    pub(super) fn stop_rx(&mut self) {
        self.tasks.signal_rx.trigger();
    }

    pub(super) async fn close(mut self) -> ZResult<()> {
        log::trace!("{}: closing", self.link);
        self.stop_rx();

        let handle_tx = zwrite!(self.tasks.handle_tx).take();
        if let Some(handle) = handle_tx {
            handle.await;
        }

        let handle_rx = zwrite!(self.tasks.handle_rx).take();
        if let Some(handle) = handle_rx {
            handle.await;
        }

        self.link.close(None).await
    }
}

/*************************************/
/*              TASKS                */
/*************************************/

pub(super) async fn tx_task(
    mut pipeline: TransmissionPipelineConsumer,
    links: Arc<AsyncRwLock<Vec<TransportLinkUnicastTx>>>,
    keep_alive: Duration,
) -> ZResult<()> {
    #[derive(Default, Debug)]
    enum Disc {
        #[default]
        Sequential,
        Parallel,
        Spawn,
    }

    let env = std::env::var("ZENOH_TX_DISC");
    log::debug!("ZENOH_TX_DISC: {env:?}");
    let disc = match env {
        Ok(d) => match d.as_str() {
            "sequential" => Disc::Sequential,
            "parallel" => Disc::Parallel,
            "spawn" => Disc::Spawn,
            _ => Disc::default(),
        },
        Err(_) => Disc::default(),
    };
    log::debug!("Tx disc: {disc:?}");

    loop {
        let res = pipeline.pull().timeout(keep_alive).await;
        let mut ls = zasyncread!(links);
        match res {
            Ok(res) => match res {
                Some((mut batch, priority)) => {
                    while ls.is_empty() {
                        log::trace!("No links available for TX");
                        drop(ls);
                        task::sleep(Duration::from_millis(100)).await;
                        ls = zasyncread!(links);
                    }

                    #[cfg(feature = "transport_compression")]
                    {
                        batch.config.is_compression = false;
                    }

                    let res = batch
                        .finalize(None)
                        .map_err(|_| zerror!("Batch finalization error"))?;

                    if let Finalize::Buffer = res {
                        panic!("Compression is not supported");
                    };

                    // Send the message on the links
                    match disc {
                        Disc::Sequential => {
                            // Sequential
                            for l in ls.as_slice() {
                                let _ = l.inner.link.write_all(batch.as_slice()).await;
                            }
                        }
                        Disc::Parallel => {
                            // Parallel but blocking
                            use futures::stream::StreamExt;
                            let _ = futures::stream::iter(0..ls.len())
                                .map(|i| ls[i].inner.link.write_all(batch.as_slice()))
                                .buffer_unordered(ls.len())
                                .collect::<Vec<_>>()
                                .await; // Ignore errors
                        }
                        Disc::Spawn => {
                            // Parallel spawn
                            for l in ls.iter() {
                                let c_b = batch.clone();
                                let c_l = l.inner.link.clone();
                                task::spawn(async move {
                                    if let Err(e) = c_l.write_all(c_b.as_slice()).await {
                                        log::debug!("Write failed on {:?}: {}", c_l, e);
                                    }
                                });
                            }
                        }
                    }

                    // Reinsert the batch into the queue
                    pipeline.refill(batch, priority);
                }
                None => break,
            },
            Err(_) => {
                let message: TransportMessage = KeepAlive.into();
                drop(ls);
                let mut ls = zasyncwrite!(links);
                for l in ls.as_mut_slice() {
                    let _ = l.send(&message).await?;
                }
            }
        }
    }

    // Drain the transmission pipeline and write remaining bytes on the wire
    let mut batches = pipeline.drain();
    for (mut b, _) in batches.drain(..) {
        let mut ls = zasyncwrite!(links);
        for l in ls.as_mut_slice() {
            l.send_batch(&mut b)
                .timeout(keep_alive)
                .await
                .map_err(|_| {
                    zerror!("{}: flush failed after {} ms", l, keep_alive.as_millis())
                })??;
        }
    }

    Ok(())
}

async fn rx_task(
    link: &mut TransportLinkUnicastRx,
    transport: TransportUnicastUniversal,
    lease: Duration,
    signal: Signal,
    rx_buffer_size: usize,
) -> ZResult<()> {
    enum Action {
        Read(RBatch),
        Stop,
    }

    async fn read<T, F>(
        link: &mut TransportLinkUnicastRx,
        pool: &RecyclingObjectPool<T, F>,
    ) -> ZResult<Action>
    where
        T: ZSliceBuffer + 'static,
        F: Fn() -> T,
        RecyclingObject<T>: ZSliceBuffer,
    {
        let batch = link
            .recv_batch(|| pool.try_take().unwrap_or_else(|| pool.alloc()))
            .await?;
        Ok(Action::Read(batch))
    }

    async fn stop(signal: Signal) -> ZResult<Action> {
        signal.wait().await;
        Ok(Action::Stop)
    }

    // The pool of buffers
    let mtu = link.batch.max_buffer_size();
    let mut n = rx_buffer_size / mtu;
    if rx_buffer_size % mtu != 0 {
        n += 1;
    }

    let pool = RecyclingObjectPool::new(n, || vec![0_u8; mtu].into_boxed_slice());
    let l = (&link.link).into();
    while !signal.is_triggered() {
        // Async read from the underlying link
        let action = read(link, &pool)
            .race(stop(signal.clone()))
            .timeout(lease)
            .await
            .map_err(|_| zerror!("{}: expired after {} milliseconds", link, lease.as_millis()))??;
        match action {
            Action::Read(batch) => {
                #[cfg(feature = "stats")]
                {
                    transport.stats.inc_rx_bytes(2 + n); // Account for the batch len encoding (16 bits)
                }
                transport.read_messages(batch, &l)?;
            }
            Action::Stop => break,
        }
    }

    Ok(())
}
