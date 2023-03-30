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
use alloc::string::String;

/// The kind of consolidation.
#[repr(u8)]
#[derive(Debug, Default, Clone, PartialEq, Eq, Copy)]
pub enum ConsolidationMode {
    /// No consolidation applied: multiple samples may be received for the same key-timestamp.
    None,
    /// Monotonic consolidation immediately forwards samples, except if one with an equal or more recent timestamp
    /// has already been sent with the same key.
    ///
    /// This optimizes latency while potentially reducing bandwidth.
    ///
    /// Note that this doesn't cause re-ordering, but drops the samples for which a more recent timestamp has already
    /// been observed with the same key.
    Monotonic,
    /// Holds back samples to only send the set of samples that had the highest timestamp for their key.
    #[default]
    Latest,
}

impl ConsolidationMode {
    #[cfg(feature = "test")]
    pub fn rand() -> Self {
        use rand::prelude::SliceRandom;
        let mut rng = rand::thread_rng();

        *[Self::None, Self::Monotonic, Self::Latest]
            .choose(&mut rng)
            .unwrap()
    }
}

/// # Query message
///
/// ```text
/// Flags:
/// - P: Parameters     If P==1 then the parameters are present
/// - C: Consolidation  If C==1 then the consolidation is present
/// - Z: Extension      If Z==1 then at least one extension is present
///
///   7 6 5 4 3 2 1 0
///  +-+-+-+-+-+-+-+-+
///  |Z|C|P|  QUERY  |
///  +-+-+-+---------+
///  ~ ps: <u8;z16>  ~  if P==1
///  +---------------+
///  ~ consolidation ~  if C==1
///  +---------------+
///  ~  [qry_exts]   ~  if Z==1
///  +---------------+
/// ```
pub mod flag {
    pub const P: u8 = 1 << 5; // 0x20 Parameters    if P==1 then the parameters are present
    pub const C: u8 = 1 << 6; // 0x40 Consolidation if C==1 then the consolidation is present
    pub const Z: u8 = 1 << 7; // 0x80 Extensions    if Z==1 then an extension will follow
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    pub parameters: String,
    pub consolidation: ConsolidationMode,
    pub ext_sinfo: Option<ext::SourceInfoType>,
    pub ext_body: Option<ext::QueryBodyType>,
}

pub mod ext {
    use crate::{common::ZExtZBuf, core::Encoding, zextzbuf};
    use zenoh_buffers::ZBuf;

    /// # SourceInfo extension
    /// Used to carry additional information about the source of data
    pub type SourceInfo = crate::zenoh_new::put::ext::SourceInfo;
    pub type SourceInfoType = crate::zenoh_new::put::ext::SourceInfoType;

    /// # QueryBody extension
    /// Used to carry a body attached to the query
    pub type QueryBody = zextzbuf!(0x02, false);

    ///   7 6 5 4 3 2 1 0
    ///  +-+-+-+-+-+-+-+-+
    ///  ~   encoding    ~
    ///  +---------------+
    ///  ~ pl: [u8;z32]  ~  -- Payload
    ///  +---------------+
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct QueryBodyType {
        pub encoding: Encoding,
        pub payload: ZBuf,
    }

    impl QueryBodyType {
        #[cfg(feature = "test")]
        pub fn rand() -> Self {
            use rand::Rng;
            let mut rng = rand::thread_rng();

            let encoding = Encoding::rand();
            let payload = ZBuf::rand(rng.gen_range(1..=64));

            Self { encoding, payload }
        }
    }
}

impl Query {
    #[cfg(feature = "test")]
    pub fn rand() -> Self {
        use rand::{
            distributions::{Alphanumeric, DistString},
            Rng,
        };
        let mut rng = rand::thread_rng();

        const MIN: usize = 2;
        const MAX: usize = 16;

        let parameters: String = if rng.gen_bool(0.5) {
            let len = rng.gen_range(MIN..MAX);
            Alphanumeric.sample_string(&mut rng, len)
        } else {
            String::new()
        };
        let consolidation = ConsolidationMode::rand();
        let ext_sinfo = rng.gen_bool(0.5).then_some(ext::SourceInfoType::rand());
        let ext_body = rng.gen_bool(0.5).then_some(ext::QueryBodyType::rand());

        Self {
            parameters,
            consolidation,
            ext_sinfo,
            ext_body,
        }
    }
}