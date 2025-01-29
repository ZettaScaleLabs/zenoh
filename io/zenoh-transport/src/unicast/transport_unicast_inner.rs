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

use std::{fmt::DebugStruct, sync::Arc, time::Duration};

use async_trait::async_trait;
use tokio::sync::MutexGuard as AsyncMutexGuard;
use zenoh_link::Link;
use zenoh_protocol::{
    core::{WhatAmI, ZenohIdProto},
    network::NetworkMessage,
    transport::TransportSn,
};
use zenoh_result::ZResult;

use super::link::{LinkUnicastWithOpenAck, MaybeOpenAck};
use crate::{
    unicast::{link::TransportLinkUnicast, TransportConfigUnicast},
    TransportPeerEventHandler,
};

pub(crate) type LinkError = (zenoh_result::Error, TransportLinkUnicast, u8);
pub(crate) type TransportError = (zenoh_result::Error, Arc<dyn TransportUnicastTrait>, u8);
pub(crate) enum InitTransportError {
    Link(LinkError),
    Transport(TransportError),
}

pub(crate) type AddLinkResult<'a> = Result<
    (
        Box<dyn FnOnce() + Send + Sync + 'a>,
        Box<dyn FnOnce() + Send + Sync + 'a>,
        MaybeOpenAck,
        Option<AsyncMutexGuard<'a, ()>>,
    ),
    LinkError,
>;
pub(crate) type InitTransportResult = Result<Arc<dyn TransportUnicastTrait>, InitTransportError>;

/*************************************/
/*      UNICAST TRANSPORT TRAIT      */
/*************************************/
#[async_trait]
pub(crate) trait TransportUnicastTrait: Send + Sync {
    /*************************************/
    /*            ACCESSORS              */
    /*************************************/
    fn set_callback(&self, callback: Arc<dyn TransportPeerEventHandler>);

    async fn get_alive(&self) -> AsyncMutexGuard<'_, bool>;
    fn get_zid(&self) -> ZenohIdProto;
    fn get_whatami(&self) -> WhatAmI;
    fn get_callback(&self) -> Option<Arc<dyn TransportPeerEventHandler>>;
    fn get_links(&self) -> Vec<Link>;
    fn get_auth_ids(&self) -> Vec<super::authentication::AuthId>;
    #[cfg(feature = "shared-memory")]
    fn is_shm(&self) -> bool;
    fn is_qos(&self) -> bool;
    fn get_config(&self) -> &TransportConfigUnicast;
    #[cfg(feature = "stats")]
    fn stats(&self) -> Arc<crate::stats::TransportStats>;

    /*************************************/
    /*               LINK                */
    /*************************************/
    async fn add_link(
        &self,
        link: LinkUnicastWithOpenAck,
        other_initial_sn: TransportSn,
        other_lease: Duration,
    ) -> AddLinkResult;

    /*************************************/
    /*                TX                 */
    /*************************************/
    fn schedule(&self, msg: NetworkMessage) -> ZResult<()>;

    fn schedule_batch(&self, batch: &mut dyn Iterator<Item = NetworkMessage>) -> ZResult<()> {
        for msg in batch {
            self.schedule(msg)?;
        }
        Ok(())
    }

    /*************************************/
    /*            TERMINATION            */
    /*************************************/
    async fn close(&self, reason: u8) -> ZResult<()>;

    fn add_debug_fields<'a, 'b: 'a, 'c>(
        &self,
        s: &'c mut DebugStruct<'a, 'b>,
    ) -> &'c mut DebugStruct<'a, 'b> {
        s
    }
}
