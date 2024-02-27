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

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

#[cfg(not(feature = "test"))]
use log::error;
use log::warn;
use thread_priority::{
    set_current_thread_priority, RealtimeThreadSchedulePolicy, ThreadBuilder, ThreadPriority, ThreadPriorityValue, ThreadSchedulePolicy::Realtime
};

pub struct PeriodicTask {
    running: Arc<AtomicBool>,
}

impl Drop for PeriodicTask {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed)
    }
}

impl PeriodicTask {
    pub fn new<F>(name: String, interval: Duration, mut f: F) -> Self
    where
        F: FnMut() + Send + 'static,
    {
        let running = Arc::new(AtomicBool::new(true));

        let c_running = running.clone();
        let _ = ThreadBuilder::default()
            .name(name)
            .policy(Realtime(RealtimeThreadSchedulePolicy::Fifo))
            .priority(ThreadPriority::Min)
            .spawn(move |result| {
                if let Err(e) = result {
                    warn!("{:?}: error setting realtime FIFO scheduling policy for thread: {:?}, will run with the default one...", std::thread::current().name(), e);
                
                    for priotity in (ThreadPriorityValue::MIN..ThreadPriorityValue::MAX).rev() {
                        if let Ok(p) = priotity.try_into() {
                            if set_current_thread_priority(ThreadPriority::Crossplatform(p)).is_ok() {
                                warn!("{:?}: will use priority {}", std::thread::current().name(), priotity);
                                break;
                            }
                        }
                    }
                }

                //todo: need mlock here!

                while c_running.load(Ordering::Relaxed) {
                    let cycle_start = std::time::Instant::now();

                    f();
                    
                    // sleep for next iteration
                    let elapsed = cycle_start.elapsed();
                    if elapsed < interval {
                        let sleep_interval = interval - elapsed;
                        std::thread::sleep(sleep_interval);
                    } else {
                        let err = format!("{:?}: timer overrun", std::thread::current().name());
                        #[cfg(not(feature = "test"))]
                        error!("{err}");
                        #[cfg(feature = "test")]
                        panic!("{err}");
                    }
                }
            });

        Self { running }
    }
}