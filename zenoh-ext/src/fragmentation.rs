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
use zenoh::{bytes::ZBytes, sample::Sample};

/// Per-source DoS cap on the number of fragments a single sample may carry.
///
/// 4096 fragments times each fragment's payload size (typically at most the
/// link MTU) bounds worst-case per-sequence-number memory while still allowing
/// very large payloads.  Users can raise this limit via
/// [`AdvancedSubscriberBuilder::max_fragments`](crate::AdvancedSubscriberBuilder::max_fragments).
pub(crate) const MAX_FRAGMENTS_DEFAULT: u32 = 4096;

#[derive(Debug, Clone)]
pub(crate) enum FragmentedSample {
    Single(Sample),
    Partial {
        frag_count: u32,
        frags: Vec<Option<Sample>>,
    },
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) enum FragInsertError {
    InvalidFragNum { frag_count: u32, frag_num: u32 },
    InvalidFragCount { frag_count: u32 },
    CountMismatch { expected: u32, got: u32 },
    ExceedsMax { frag_count: u32, max: u32 },
}

impl FragmentedSample {
    #[inline]
    pub(crate) fn single(sample: Sample) -> Self {
        Self::Single(sample)
    }

    /// Build a `FragmentedSample` from a vector of already-complete fragments.
    /// The caller guarantees all fragments are present and in-order.
    pub(crate) fn from_complete_vec(fragments: Vec<Sample>) -> Self {
        match fragments.len() {
            0 => Self::Partial {
                frag_count: 0,
                frags: Vec::new(),
            },
            1 => Self::Single(fragments.into_iter().next().unwrap()),
            n => Self::Partial {
                frag_count: n as u32,
                frags: fragments.into_iter().map(Some).collect(),
            },
        }
    }

    /// Create the first slot for a fragmented sample.
    pub(crate) fn from_first_fragment(
        sample: Sample,
        frag_num: u32,
        frag_count: u32,
        max_fragments: u32,
    ) -> Result<Self, FragInsertError> {
        if frag_count == 0 {
            return Err(FragInsertError::InvalidFragCount { frag_count });
        }
        if frag_num >= frag_count {
            return Err(FragInsertError::InvalidFragNum {
                frag_count,
                frag_num,
            });
        }
        if frag_count > max_fragments {
            return Err(FragInsertError::ExceedsMax {
                frag_count,
                max: max_fragments,
            });
        }
        if frag_count == 1 {
            return Ok(Self::Single(sample));
        }
        let mut frags = vec![None; frag_count as usize];
        frags[frag_num as usize] = Some(sample);
        Ok(Self::Partial { frag_count, frags })
    }

    #[inline]
    pub(crate) fn is_complete(&self) -> bool {
        match self {
            Self::Single(_) => true,
            Self::Partial { frags, .. } => !frags.is_empty() && frags.iter().all(Option::is_some),
        }
    }

    /// Iterate over all fragments. For a `Single` sample yields one element.
    ///
    /// For a [`Self::Partial`] sample with out-of-order fragments this yields
    /// only the fragments that have arrived so far, in fragment-number order.
    #[inline]
    pub(crate) fn iter_frags(&self) -> FragsIter<'_> {
        match self {
            Self::Single(s) => FragsIter {
                inner: FragsIterInner::Single(Some(s)),
            },
            Self::Partial { frags, .. } => FragsIter {
                inner: FragsIterInner::Partial(frags.iter()),
            },
        }
    }

    /// Return the ranges of missing fragment numbers.
    /// For a complete or single-fragment sample the result is empty.
    pub(crate) fn missing_ranges(&self) -> Vec<(Option<u32>, Option<u32>)> {
        match self {
            Self::Single(_) => Vec::new(),
            Self::Partial { frags, .. } => missing_ranges_impl(frags),
        }
    }

    /// Insert a fragment into this slot.
    ///
    /// # Errors
    /// * `InvalidFragNum` if `frag_num >= frag_count`.
    /// * `InvalidFragCount` if `frag_count == 0`.
    /// * `CountMismatch` if the incoming `frag_count` disagrees with the slot.
    pub(crate) fn insert(
        &mut self,
        sample: Sample,
        frag_num: u32,
        frag_count: u32,
    ) -> Result<(), FragInsertError> {
        if frag_count == 0 {
            return Err(FragInsertError::InvalidFragCount { frag_count });
        }
        if frag_num >= frag_count {
            return Err(FragInsertError::InvalidFragNum {
                frag_count,
                frag_num,
            });
        }
        match self {
            Self::Single(_) => {
                if frag_count == 1 {
                    // duplicate of a single-fragment sample; keep existing
                    return Ok(());
                }
                // A Single only exists when the sample was advertised as a
                // single fragment (or assembled from a complete vector).  A
                // later fragment claiming a different count is inconsistent.
                Err(FragInsertError::CountMismatch {
                    expected: 1,
                    got: frag_count,
                })
            }
            Self::Partial {
                frag_count: existing,
                frags,
            } => {
                if *existing != frag_count {
                    return Err(FragInsertError::CountMismatch {
                        expected: *existing,
                        got: frag_count,
                    });
                }
                let slot = &mut frags[frag_num as usize];
                if slot.is_some() {
                    tracing::trace!(
                        "AdvancedSubscriber: duplicate fragment {frag_num}/{frag_count} ignored"
                    );
                    return Ok(());
                }
                *slot = Some(sample);
                Ok(())
            }
        }
    }

    /// Consume this slot and return the reassembled [`Sample`] if complete.
    pub(crate) fn into_sample(self) -> Option<Sample> {
        match self {
            Self::Single(s) => Some(s),
            Self::Partial { frags, .. } => {
                if !frags.iter().all(Option::is_some) {
                    return None;
                }
                let mut iter = frags.into_iter();
                let first = iter.next()??;
                let mut payload = ZBytes::writer();
                payload.append(first.payload().clone());
                for frag in iter {
                    let frag = frag?;
                    payload.append(frag.payload().clone());
                }
                Some(first.with_payload(payload.finish()))
            }
        }
    }
}

pub(crate) struct FragsIter<'a> {
    inner: FragsIterInner<'a>,
}

enum FragsIterInner<'a> {
    Single(Option<&'a Sample>),
    Partial(::std::slice::Iter<'a, Option<Sample>>),
}

impl<'a> Iterator for FragsIter<'a> {
    type Item = &'a Sample;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            FragsIterInner::Single(s) => s.take(),
            FragsIterInner::Partial(iter) => iter.find_map(|f| f.as_ref()),
        }
    }
}

fn missing_ranges_impl(frags: &[Option<Sample>]) -> Vec<(Option<u32>, Option<u32>)> {
    let mut missing_ranges: Vec<(Option<u32>, Option<u32>)> = vec![];
    for (i, frag) in frags.iter().enumerate() {
        if missing_ranges.is_empty() {
            if frag.is_none() {
                missing_ranges.push((Some(i as u32), None));
            }
        } else {
            let last_index = missing_ranges.len() - 1;
            if frag.is_none() {
                if missing_ranges[last_index].1.is_some() {
                    missing_ranges.push((Some(i as u32), None));
                }
            } else if missing_ranges[last_index].0.is_some()
                && missing_ranges[last_index].1.is_none()
            {
                missing_ranges[last_index].1 = Some((i - 1) as u32);
            }
        }
    }
    missing_ranges
}

#[cfg(test)]
mod tests {
    use zenoh::{
        key_expr::KeyExpr,
        sample::{FragInfo, Sample, SampleBuilder},
    };

    use super::{FragInsertError, FragmentedSample, MAX_FRAGMENTS_DEFAULT};

    fn make_sample(payload: &str, frag_num: u32, frag_count: u32) -> Sample {
        SampleBuilder::put(KeyExpr::try_from("test/key").unwrap(), payload)
            .frag_info(FragInfo::new(frag_count, frag_num))
            .into()
    }

    #[test]
    fn out_of_order_insert_completes() {
        let s0 = make_sample("A", 0, 3);
        let s1 = make_sample("B", 1, 3);
        let s2 = make_sample("C", 2, 3);

        let mut fs =
            FragmentedSample::from_first_fragment(s2, 2, 3, MAX_FRAGMENTS_DEFAULT).unwrap();
        assert!(!fs.is_complete());
        fs.insert(s1, 1, 3).unwrap();
        assert!(!fs.is_complete());
        fs.insert(s0, 0, 3).unwrap();
        assert!(fs.is_complete());

        let sample = fs.into_sample().unwrap();
        assert_eq!(sample.payload().try_to_string().unwrap().as_ref(), "ABC");
    }

    #[test]
    fn invalid_frag_num_ge_count() {
        let s = make_sample("A", 5, 3);
        assert!(matches!(
            FragmentedSample::from_first_fragment(s, 5, 3, MAX_FRAGMENTS_DEFAULT),
            Err(FragInsertError::InvalidFragNum {
                frag_count: 3,
                frag_num: 5,
            })
        ));
    }

    #[test]
    fn invalid_frag_count_zero() {
        let s = make_sample("A", 0, 0);
        assert!(matches!(
            FragmentedSample::from_first_fragment(s, 0, 0, MAX_FRAGMENTS_DEFAULT),
            Err(FragInsertError::InvalidFragCount { frag_count: 0 })
        ));
    }

    #[test]
    fn exceeds_max() {
        let s = make_sample("A", 0, 5000);
        assert!(matches!(
            FragmentedSample::from_first_fragment(s, 0, 5000, MAX_FRAGMENTS_DEFAULT),
            Err(FragInsertError::ExceedsMax {
                frag_count: 5000,
                max: 4096,
            })
        ));
    }

    #[test]
    fn from_complete_vec_empty() {
        let fs = FragmentedSample::from_complete_vec(Vec::new());
        assert!(!fs.is_complete());
        assert!(fs.into_sample().is_none());
    }

    #[test]
    fn from_complete_vec_single() {
        let s = make_sample("A", 0, 1);
        let fs = FragmentedSample::from_complete_vec(vec![s]);
        assert!(fs.is_complete());
        assert_eq!(
            fs.into_sample()
                .unwrap()
                .payload()
                .try_to_string()
                .unwrap()
                .as_ref(),
            "A"
        );
    }

    #[test]
    fn from_complete_vec_multi() {
        let s0 = make_sample("A", 0, 3);
        let s1 = make_sample("B", 1, 3);
        let s2 = make_sample("C", 2, 3);
        let fs = FragmentedSample::from_complete_vec(vec![s0, s1, s2]);
        assert!(fs.is_complete());
        assert_eq!(
            fs.into_sample()
                .unwrap()
                .payload()
                .try_to_string()
                .unwrap()
                .as_ref(),
            "ABC"
        );
    }

    #[test]
    fn missing_ranges() {
        let s0 = make_sample("A", 0, 5);
        let s2 = make_sample("C", 2, 5);
        let s4 = make_sample("E", 4, 5);
        let mut fs =
            FragmentedSample::from_first_fragment(s0, 0, 5, MAX_FRAGMENTS_DEFAULT).unwrap();
        fs.insert(s2, 2, 5).unwrap();
        fs.insert(s4, 4, 5).unwrap();
        let ranges = fs.missing_ranges();
        assert_eq!(ranges, vec![(Some(1), Some(1)), (Some(3), Some(3))]);
    }

    #[test]
    fn single_promotion_count_mismatch() {
        let s = make_sample("A", 0, 1);
        let mut fs = FragmentedSample::single(s);
        let s3 = make_sample("B", 0, 3);
        assert!(matches!(
            fs.insert(s3, 0, 3),
            Err(FragInsertError::CountMismatch {
                expected: 1,
                got: 3
            })
        ));
    }

    #[test]
    fn insert_duplicate_no_overwrite() {
        let s0 = make_sample("A", 0, 3);
        let s0_dup = make_sample("X", 0, 3);
        let s1 = make_sample("B", 1, 3);
        let s2 = make_sample("C", 2, 3);

        let mut fs =
            FragmentedSample::from_first_fragment(s0, 0, 3, MAX_FRAGMENTS_DEFAULT).unwrap();
        fs.insert(s0_dup, 0, 3).unwrap(); // duplicate, should be ignored
        fs.insert(s1, 1, 3).unwrap();
        fs.insert(s2, 2, 3).unwrap();

        let sample = fs.into_sample().unwrap();
        assert_eq!(sample.payload().try_to_string().unwrap().as_ref(), "ABC");
    }
}
