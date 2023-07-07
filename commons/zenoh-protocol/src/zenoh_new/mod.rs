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
pub mod ack;
pub mod del;
pub mod err;
pub mod pull;
pub mod put;
pub mod query;
pub mod reply;

pub use ack::*;
pub use del::*;
pub use err::*;
pub use pull::*;
pub use put::*;
pub use query::*;
pub use reply::*;

pub mod id {
    pub const OAM: u8 = 0x00;
    pub const PUT: u8 = 0x01;
    pub const DEL: u8 = 0x02;
    pub const QUERY: u8 = 0x03;
    pub const REPLY: u8 = 0x04;
    pub const ERR: u8 = 0x05;
    pub const ACK: u8 = 0x06;
    pub const PULL: u8 = 0x07;
}

// Push
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushBody {
    Put(Put),
    Del(Del),
}

impl PushBody {
    #[cfg(feature = "test")]
    pub fn rand() -> Self {
        use rand::Rng;

        let mut rng = rand::thread_rng();

        match rng.gen_range(0..2) {
            0 => PushBody::Put(Put::rand()),
            1 => PushBody::Del(Del::rand()),
            _ => unreachable!(),
        }
    }
}

impl From<Put> for PushBody {
    fn from(p: Put) -> PushBody {
        PushBody::Put(p)
    }
}

impl From<Del> for PushBody {
    fn from(d: Del) -> PushBody {
        PushBody::Del(d)
    }
}

// Request
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestBody {
    Query(Query),
    Put(Put),
    Del(Del),
    Pull(Pull),
}

impl RequestBody {
    #[cfg(feature = "test")]
    pub fn rand() -> Self {
        use rand::Rng;

        let mut rng = rand::thread_rng();

        match rng.gen_range(0..3) {
            0 => RequestBody::Query(Query::rand()),
            1 => RequestBody::Put(Put::rand()),
            2 => RequestBody::Del(Del::rand()),
            _ => unreachable!(),
        }
    }
}

impl From<Query> for RequestBody {
    fn from(q: Query) -> RequestBody {
        RequestBody::Query(q)
    }
}

impl From<Put> for RequestBody {
    fn from(p: Put) -> RequestBody {
        RequestBody::Put(p)
    }
}

impl From<Del> for RequestBody {
    fn from(d: Del) -> RequestBody {
        RequestBody::Del(d)
    }
}

// Response
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseBody {
    Reply(Reply),
    Err(Err),
    Ack(Ack),
    Put(Put),
}

impl ResponseBody {
    #[cfg(feature = "test")]
    pub fn rand() -> Self {
        use rand::Rng;

        let mut rng = rand::thread_rng();

        match rng.gen_range(0..4) {
            0 => ResponseBody::Reply(Reply::rand()),
            1 => ResponseBody::Err(Err::rand()),
            2 => ResponseBody::Ack(Ack::rand()),
            3 => ResponseBody::Put(Put::rand()),
            _ => unreachable!(),
        }
    }
}

impl From<Reply> for ResponseBody {
    fn from(r: Reply) -> ResponseBody {
        ResponseBody::Reply(r)
    }
}

impl From<Err> for ResponseBody {
    fn from(r: Err) -> ResponseBody {
        ResponseBody::Err(r)
    }
}

impl From<Ack> for ResponseBody {
    fn from(r: Ack) -> ResponseBody {
        ResponseBody::Ack(r)
    }
}

pub mod ext {
    use crate::core::ZenohId;

    ///  7 6 5 4 3 2 1 0
    /// +-+-+-+-+-+-+-+-+
    /// |zid_len|X|X|X|X|
    /// +-------+-+-+---+
    /// ~      zid      ~
    /// +---------------+
    /// %      eid      %  -- Counter decided by the Zenoh Node
    /// +---------------+
    /// %      sn       %
    /// +---------------+
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct SourceInfoType<const ID: u8> {
        pub zid: ZenohId,
        pub eid: u32,
        pub sn: u32,
    }

    impl<const ID: u8> SourceInfoType<{ ID }> {
        #[cfg(feature = "test")]
        pub fn rand() -> Self {
            use rand::Rng;
            let mut rng = rand::thread_rng();

            let zid = ZenohId::rand();
            let eid: u32 = rng.gen();
            let sn: u32 = rng.gen();
            Self { zid, eid, sn }
        }
    }
}
