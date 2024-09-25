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

//! ZBytes primitives.
use std::{borrow::Cow, fmt::Debug, str::Utf8Error};

use zenoh_buffers::{
    buffer::{Buffer, SplitBuffer},
    reader::{HasReader, Reader},
    writer::HasWriter,
    ZBuf, ZBufReader, ZBufWriter, ZSlice, ZSliceBuffer,
};
use zenoh_codec::{RCodec, WCodec, Zenoh080};
use zenoh_protocol::zenoh::ext::AttachmentType;

/// Wrapper type for API ergonomicity to allow any type `T` to be converted into `Option<ZBytes>` where `T` implements `Into<ZBytes>`.
#[repr(transparent)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OptionZBytes(Option<ZBytes>);

impl<T> From<T> for OptionZBytes
where
    T: Into<ZBytes>,
{
    fn from(value: T) -> Self {
        Self(Some(value.into()))
    }
}

impl<T> From<Option<T>> for OptionZBytes
where
    T: Into<ZBytes>,
{
    fn from(mut value: Option<T>) -> Self {
        match value.take() {
            Some(v) => Self(Some(v.into())),
            None => Self(None),
        }
    }
}

impl<T> From<&Option<T>> for OptionZBytes
where
    for<'a> &'a T: Into<ZBytes>,
{
    fn from(value: &Option<T>) -> Self {
        match value.as_ref() {
            Some(v) => Self(Some(v.into())),
            None => Self(None),
        }
    }
}

impl From<OptionZBytes> for Option<ZBytes> {
    fn from(value: OptionZBytes) -> Self {
        value.0
    }
}

/// ZBytes contains the serialized bytes of user data.
///
/// `ZBytes` can be converted from/to raw bytes:
/// ```rust
/// use std::borrow::Cow;
/// use zenoh::bytes::ZBytes;
///
/// let buf = b"some raw bytes";
/// let payload = ZBytes::from(buf);
/// assert_eq!(payload.to_bytes(), buf.as_slice());
/// ```
///
/// `ZBytes` may store data in non-contiguous regions of memory.
/// The typical case for `ZBytes` to store data in different memory regions is when data is received fragmented from the network.
///
/// To directly access raw data as contiguous slice it is preferred to convert `ZBytes` into a [`std::borrow::Cow<[u8]>`] using [`to_bytes`](Self::to_bytes).
/// If `ZBytes` contains all the data in a single memory location, this is guaranteed to be zero-copy. This is the common case for small messages.
/// If `ZBytes` contains data scattered in different memory regions, this operation will do an allocation and a copy. This is the common case for large messages.
///
/// It is also possible to iterate over the raw data that may be scattered on different memory regions using [`slices`](Self::slices).
/// Please note that no guarantee is provided on the internal memory layout of [`ZBytes`] nor on how many slices a given [`ZBytes`] will be composed of.
/// The only provided guarantee is on the bytes order that is preserved.
/// ```
#[repr(transparent)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ZBytes(ZBuf);

impl ZBytes {
    /// Create an empty ZBytes.
    pub const fn new() -> Self {
        Self(ZBuf::empty())
    }

    /// Returns whether the [`ZBytes`] is empty or not.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the total number of bytes in the [`ZBytes`].
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn to_bytes(&self) -> Cow<[u8]> {
        self.0.contiguous()
    }

    pub fn try_to_string(&self) -> Result<Cow<str>, Utf8Error> {
        Ok(match self.to_bytes() {
            Cow::Borrowed(s) => std::str::from_utf8(s)?.into(),
            Cow::Owned(v) => String::from_utf8(v).map_err(|err| err.utf8_error())?.into(),
        })
    }

    /// Get a [`ZBytesReader`] implementing [`std::io::Read`] trait.
    ///
    /// See [`ZBytesWriter`] on how to chain the deserialization of different types from a single [`ZBytes`].
    pub fn reader(&self) -> ZBytesReader<'_> {
        ZBytesReader(self.0.reader())
    }

    /// Build a [`ZBytes`] from a generic reader implementing [`std::io::Read`]. This operation copies data from the reader.
    pub fn from_reader<R>(mut reader: R) -> Result<Self, std::io::Error>
    where
        R: std::io::Read,
    {
        let mut buf: Vec<u8> = vec![];
        reader.read_to_end(&mut buf)?;
        Ok(buf.into())
    }

    /// Get a [`ZBytesWriter`] implementing [`std::io::Write`] trait.
    ///
    /// See [`ZBytesWriter`] on how to chain the serialization of different types into a single [`ZBytes`].
    pub fn writer(&mut self) -> ZBytesWriter<'_> {
        ZBytesWriter(self.0.writer())
    }

    /// Return an iterator on raw bytes slices contained in the [`ZBytes`].
    ///
    /// [`ZBytes`] may store data in non-contiguous regions of memory, this iterator
    /// then allows to access raw data directly without any attempt of deserializing it.
    /// Please note that no guarantee is provided on the internal memory layout of [`ZBytes`].
    /// The only provided guarantee is on the bytes order that is preserved.
    ///
    /// Please note that [`ZBytes::iter`] will perform deserialization while iterating while [`ZBytes::slices`] will not.
    ///
    /// ```rust
    /// use std::io::Write;
    /// use zenoh::bytes::ZBytes;
    ///
    /// let buf1: Vec<u8> = vec![1, 2, 3];
    /// let buf2: Vec<u8> = vec![4, 5, 6, 7, 8];
    /// let mut zbs = ZBytes::new();
    /// let mut writer = zbs.writer();
    /// writer.write(&buf1);
    /// writer.write(&buf2);
    ///
    /// // Access the raw content
    /// for slice in zbs.slices() {
    ///     println!("{:02x?}", slice);
    /// }
    ///
    /// // Concatenate input in a single vector
    /// let buf: Vec<u8> = buf1.into_iter().chain(buf2.into_iter()).collect();
    /// // Concatenate raw bytes in a single vector
    /// let out: Vec<u8> = zbs.slices().fold(Vec::new(), |mut b, x| { b.extend_from_slice(x); b });
    /// // The previous line is the equivalent of
    /// // let out: Vec<u8> = zbs.into();
    /// assert_eq!(buf, out);    
    /// ```
    ///
    /// The example below shows how the [`ZBytesWriter::append`] simply appends the slices of one [`ZBytes`]
    /// to another and how those slices can be iterated over to access the raw data.
    /// ```rust
    /// use std::io::Write;
    /// use zenoh::bytes::ZBytes;
    ///
    /// let buf1: Vec<u8> = vec![1, 2, 3];
    /// let buf2: Vec<u8> = vec![4, 5, 6, 7, 8];
    ///
    /// let mut zbs = ZBytes::new();
    /// let mut writer = zbs.writer();
    /// writer.append(ZBytes::from(buf1.clone()));
    /// writer.append(ZBytes::from(buf2.clone()));
    ///
    /// let mut iter = zbs.slices();
    /// assert_eq!(buf1.as_slice(), iter.next().unwrap());
    /// assert_eq!(buf2.as_slice(), iter.next().unwrap());
    /// ```
    pub fn slices(&self) -> ZBytesSliceIterator<'_> {
        ZBytesSliceIterator(self.0.slices())
    }
}
#[cfg(all(feature = "unstable", feature = "shared-memory"))]
const _: () = {
    use zenoh_shm::{api::buffer::zshm::zshm, ShmBufInner};
    impl ZBytes {
        pub fn as_shm(&self) -> Option<&zshm> {
            let mut zslices = self.0.zslices();
            if zslices.by_ref().count() != 1 {
                return None;
            }
            let zslice = zslices.next().unwrap();
            zslice.downcast_ref::<ShmBufInner>().map(Into::into)
        }

        pub fn as_shm_mut(&mut self) -> Option<&mut zshm> {
            let mut zslices = self.0.zslices_mut();
            if zslices.by_ref().count() != 1 {
                return None;
            }
            let zslice = zslices.next().unwrap();
            // SAFETY: ShmBufInner cannot change the size of the slice
            unsafe { zslice.downcast_mut::<ShmBufInner>() }.map(Into::into)
        }
    }
};

/// A reader that implements [`std::io::Read`] trait to deserialize from a [`ZBytes`].
#[repr(transparent)]
#[derive(Debug)]
pub struct ZBytesReader<'a>(ZBufReader<'a>);

impl ZBytesReader<'_> {
    /// Returns the number of bytes that can still be read
    pub fn remaining(&self) -> usize {
        self.0.remaining()
    }

    /// Returns true if no more bytes can be read
    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    #[zenoh_macros::internal]
    pub fn read_vle(&mut self) -> Option<u64> {
        let codec = Zenoh080::new();
        codec.read(&mut self.0).ok()
    }
}

impl std::io::Read for ZBytesReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        std::io::Read::read(&mut self.0, buf)
    }
}

impl std::io::Seek for ZBytesReader<'_> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        std::io::Seek::seek(&mut self.0, pos)
    }
}

/// A writer that implements [`std::io::Write`] trait to serialize into a [`ZBytes`].
#[repr(transparent)]
#[derive(Debug)]
pub struct ZBytesWriter<'a>(ZBufWriter<'a>);

impl ZBytesWriter<'_> {
    /// Append a [`ZBytes`] to this [`ZBytes`] by taking ownership.
    /// This allows to compose a [`ZBytes`] out of multiple [`ZBytes`] that may point to different memory regions.
    /// Said in other terms, it allows to create a linear view on different memory regions without copy.
    ///
    /// Example:
    /// ```
    /// use zenoh::bytes::ZBytes;
    ///
    /// let one = ZBytes::from(vec![0, 1]);
    /// let two = ZBytes::from(vec![2, 3, 4, 5]);
    /// let three = ZBytes::from(vec![6, 7]);
    ///
    /// let mut bytes = ZBytes::new();
    /// let mut writer = bytes.writer();
    /// // Append data without copying by passing ownership
    /// writer.append(one);
    /// writer.append(two);
    /// writer.append(three);
    ///
    /// // deserialization
    /// let mut out: Vec<u8> = bytes.into();
    /// assert_eq!(out, vec![0u8, 1, 2, 3, 4, 5, 6, 7]);
    /// ```
    pub fn append(&mut self, b: ZBytes) {
        use zenoh_buffers::writer::Writer;
        for s in b.0.zslices() {
            // SAFETY: we are writing a ZSlice on a ZBuf, this is infallible because we are just pushing a ZSlice to
            //         the list of available ZSlices.
            unsafe { self.0.write_zslice(s).unwrap_unchecked() }
        }
    }

    #[zenoh_macros::internal]
    pub fn write_vle(&mut self, vle: u64) {
        let codec = Zenoh080::new();
        codec.write(&mut self.0, vle).unwrap();
    }
}

impl std::io::Write for ZBytesWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        std::io::Write::write(&mut self.0, buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// An iterator to iterate on raw bytes slices contained in a [`ZBytes`].
///
/// Example:
/// ```rust
/// use std::io::Write;
/// use zenoh::bytes::ZBytes;
///
/// let buf1: Vec<u8> = vec![1, 2, 3];
/// let buf2: Vec<u8> = vec![4, 5, 6, 7, 8];
/// let mut zbs = ZBytes::new();
/// let mut writer = zbs.writer();
/// writer.write(&buf1);
/// writer.write(&buf2);
///
/// // Access the raw content
/// for slice in zbs.slices() {
///     println!("{:02x?}", slice);
/// }
///
/// // Concatenate input in a single vector
/// let buf: Vec<u8> = buf1.into_iter().chain(buf2.into_iter()).collect();
/// // Concatenate raw bytes in a single vector
/// let out: Vec<u8> = zbs.slices().fold(Vec::new(), |mut b, x| { b.extend_from_slice(x); b });
/// // The previous line is the equivalent of
/// // let out: Vec<u8> = zbs.into();
/// assert_eq!(buf, out);    
/// ```
#[repr(transparent)]
#[derive(Debug)]
pub struct ZBytesSliceIterator<'a>(ZBytesSliceIteratorInner<'a>);

// Typedef to make clippy happy about complex type. Encapsulate inner `ZBufSliceOperator`.
type ZBytesSliceIteratorInner<'a> =
    std::iter::Map<core::slice::Iter<'a, ZSlice>, fn(&'a ZSlice) -> &'a [u8]>;

impl<'a> Iterator for ZBytesSliceIterator<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl From<ZBuf> for ZBytes {
    fn from(value: ZBuf) -> Self {
        Self(value)
    }
}
impl From<ZBytes> for ZBuf {
    fn from(value: ZBytes) -> Self {
        value.0
    }
}
impl<const N: usize> From<[u8; N]> for ZBytes {
    fn from(value: [u8; N]) -> Self {
        Self(value.into())
    }
}
impl<const N: usize> From<&[u8; N]> for ZBytes {
    fn from(value: &[u8; N]) -> Self {
        value.to_vec().into()
    }
}
impl From<Vec<u8>> for ZBytes {
    fn from(value: Vec<u8>) -> Self {
        Self(value.into())
    }
}
impl From<&Vec<u8>> for ZBytes {
    fn from(value: &Vec<u8>) -> Self {
        value.clone().into()
    }
}
impl From<&[u8]> for ZBytes {
    fn from(value: &[u8]) -> Self {
        value.to_vec().into()
    }
}
impl From<Cow<'_, [u8]>> for ZBytes {
    fn from(value: Cow<'_, [u8]>) -> Self {
        value.into_owned().into()
    }
}
impl From<&Cow<'_, [u8]>> for ZBytes {
    fn from(value: &Cow<'_, [u8]>) -> Self {
        value.clone().into()
    }
}
impl From<String> for ZBytes {
    fn from(value: String) -> Self {
        value.into_bytes().into()
    }
}
impl From<&String> for ZBytes {
    fn from(value: &String) -> Self {
        value.clone().into()
    }
}
impl From<&str> for ZBytes {
    fn from(value: &str) -> Self {
        value.as_bytes().into()
    }
}
impl From<Cow<'_, str>> for ZBytes {
    fn from(value: Cow<'_, str>) -> Self {
        value.into_owned().into()
    }
}
impl From<&Cow<'_, str>> for ZBytes {
    fn from(value: &Cow<'_, str>) -> Self {
        value.clone().into()
    }
}

// Define a transparent wrapper type to get around Rust's orphan rule.
// This allows to use bytes::Bytes directly as supporting buffer of a
// ZSlice resulting in zero-copy and zero-alloc bytes::Bytes serialization.
#[repr(transparent)]
#[derive(Debug)]
struct BytesWrap(bytes::Bytes);
impl ZSliceBuffer for BytesWrap {
    fn as_slice(&self) -> &[u8] {
        &self.0
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
impl From<bytes::Bytes> for ZBytes {
    fn from(value: bytes::Bytes) -> Self {
        Self(BytesWrap(value).into())
    }
}

#[cfg(all(feature = "unstable", feature = "shared-memory"))]
const _: () = {
    use zenoh_shm::api::buffer::{zshm::ZShm, zshmmut::ZShmMut};
    impl From<ZShm> for ZBytes {
        fn from(value: ZShm) -> Self {
            Self(ZSlice::from(value).into())
        }
    }
    impl From<ZShmMut> for ZBytes {
        fn from(value: ZShmMut) -> Self {
            Self(ZSlice::from(value).into())
        }
    }
};

// Protocol attachment extension
impl<const ID: u8> From<ZBytes> for AttachmentType<ID> {
    fn from(this: ZBytes) -> Self {
        AttachmentType {
            buffer: this.into(),
        }
    }
}

impl<const ID: u8> From<AttachmentType<ID>> for ZBytes {
    fn from(this: AttachmentType<ID>) -> Self {
        this.buffer.into()
    }
}

// mod tests {
//
//     #[test]
//     fn serializer() {
//         use std::borrow::Cow;
//
//         use rand::Rng;
//         use zenoh_buffers::{ZBuf, ZSlice};
//         use zenoh_protocol::core::Parameters;
//         #[cfg(feature = "shared-memory")]
//         use zenoh_shm::api::{
//             buffer::zshm::{zshm, ZShm},
//             protocol_implementations::posix::{
//                 posix_shm_provider_backend::PosixShmProviderBackend, protocol_id::POSIX_PROTOCOL_ID,
//             },
//             provider::shm_provider::ShmProviderBuilder,
//         };
//
//         use super::ZBytes;
//         #[cfg(feature = "shared-memory")]
//         use crate::zenoh_core::Wait;
//
//         const NUM: usize = 1_000;
//
//         macro_rules! serialize_deserialize {
//             ($t:ty, $in:expr) => {
//                 let i = $in;
//                 let t = i.clone();
//                 println!("Serialize:\t{:?}", t);
//                 let v = ZBytes::serialize(t);
//                 println!("Deserialize:\t{:?}", v);
//                 let o: $t = v.deserialize().unwrap();
//                 assert_eq!(i, o);
//                 println!("");
//             };
//         }
//
//         // WARN: test function body produces stack overflow, so I split it into subroutines
//         #[inline(never)]
//         fn numeric() {
//             let mut rng = rand::thread_rng();
//
//             // unsigned integer
//             serialize_deserialize!(u8, u8::MIN);
//             serialize_deserialize!(u16, u16::MIN);
//             serialize_deserialize!(u32, u32::MIN);
//             serialize_deserialize!(u64, u64::MIN);
//             serialize_deserialize!(usize, usize::MIN);
//
//             serialize_deserialize!(u8, u8::MAX);
//             serialize_deserialize!(u16, u16::MAX);
//             serialize_deserialize!(u32, u32::MAX);
//             serialize_deserialize!(u64, u64::MAX);
//             serialize_deserialize!(usize, usize::MAX);
//
//             for _ in 0..NUM {
//                 serialize_deserialize!(u8, rng.gen::<u8>());
//                 serialize_deserialize!(u16, rng.gen::<u16>());
//                 serialize_deserialize!(u32, rng.gen::<u32>());
//                 serialize_deserialize!(u64, rng.gen::<u64>());
//                 serialize_deserialize!(usize, rng.gen::<usize>());
//             }
//
//             // signed integer
//             serialize_deserialize!(i8, i8::MIN);
//             serialize_deserialize!(i16, i16::MIN);
//             serialize_deserialize!(i32, i32::MIN);
//             serialize_deserialize!(i64, i64::MIN);
//             serialize_deserialize!(isize, isize::MIN);
//
//             serialize_deserialize!(i8, i8::MAX);
//             serialize_deserialize!(i16, i16::MAX);
//             serialize_deserialize!(i32, i32::MAX);
//             serialize_deserialize!(i64, i64::MAX);
//             serialize_deserialize!(isize, isize::MAX);
//
//             for _ in 0..NUM {
//                 serialize_deserialize!(i8, rng.gen::<i8>());
//                 serialize_deserialize!(i16, rng.gen::<i16>());
//                 serialize_deserialize!(i32, rng.gen::<i32>());
//                 serialize_deserialize!(i64, rng.gen::<i64>());
//                 serialize_deserialize!(isize, rng.gen::<isize>());
//             }
//
//             // float
//             serialize_deserialize!(f32, f32::MIN);
//             serialize_deserialize!(f64, f64::MIN);
//
//             serialize_deserialize!(f32, f32::MAX);
//             serialize_deserialize!(f64, f64::MAX);
//
//             for _ in 0..NUM {
//                 serialize_deserialize!(f32, rng.gen::<f32>());
//                 serialize_deserialize!(f64, rng.gen::<f64>());
//             }
//         }
//         numeric();
//
//         // WARN: test function body produces stack overflow, so I split it into subroutines
//         #[inline(never)]
//         fn basic() {
//             let mut rng = rand::thread_rng();
//
//             // bool
//             serialize_deserialize!(bool, true);
//             serialize_deserialize!(bool, false);
//
//             // char
//             serialize_deserialize!(char, char::MAX);
//             serialize_deserialize!(char, rng.gen::<char>());
//
//             let a = 'a';
//             let bytes = ZSerde.serialize(a);
//             let s: String = ZSerde.deserialize(&bytes).unwrap();
//             assert_eq!(a.to_string(), s);
//
//             let a = String::from("a");
//             let bytes = ZSerde.serialize(&a);
//             let s: char = ZSerde.deserialize(&bytes).unwrap();
//             assert_eq!(a, s.to_string());
//
//             // String
//             serialize_deserialize!(String, "");
//             serialize_deserialize!(String, String::from("abcdef"));
//
//             // Cow<str>
//             serialize_deserialize!(Cow<str>, Cow::from(""));
//             serialize_deserialize!(Cow<str>, Cow::from(String::from("abcdef")));
//
//             // Vec
//             serialize_deserialize!(Vec<u8>, vec![0u8; 0]);
//             serialize_deserialize!(Vec<u8>, vec![0u8; 64]);
//
//             // Cow<[u8]>
//             serialize_deserialize!(Cow<[u8]>, Cow::from(vec![0u8; 0]));
//             serialize_deserialize!(Cow<[u8]>, Cow::from(vec![0u8; 64]));
//
//             // ZBuf
//             serialize_deserialize!(ZBuf, ZBuf::from(vec![0u8; 0]));
//             serialize_deserialize!(ZBuf, ZBuf::from(vec![0u8; 64]));
//         }
//         basic();
//
//         // WARN: test function body produces stack overflow, so I split it into subroutines
//         #[inline(never)]
//         fn reader_writer() {
//             let mut bytes = ZBytes::new();
//             let mut writer = bytes.writer();
//
//             let i1 = 1_u8;
//             let i2 = String::from("abcdef");
//             let i3 = vec![2u8; 64];
//
//             println!("Write: {:?}", i1);
//             writer.serialize(i1);
//             println!("Write: {:?}", i2);
//             writer.serialize(&i2);
//             println!("Write: {:?}", i3);
//             writer.serialize(&i3);
//
//             let mut reader = bytes.reader();
//             let o1: u8 = reader.deserialize().unwrap();
//             println!("Read: {:?}", o1);
//             let o2: String = reader.deserialize().unwrap();
//             println!("Read: {:?}", o2);
//             let o3: Vec<u8> = reader.deserialize().unwrap();
//             println!("Read: {:?}", o3);
//
//             println!();
//
//             assert_eq!(i1, o1);
//             assert_eq!(i2, o2);
//             assert_eq!(i3, o3);
//         }
//         reader_writer();
//
//         // SHM
//         #[cfg(feature = "shared-memory")]
//         fn shm() {
//             // create an SHM backend...
//             let backend = PosixShmProviderBackend::builder()
//                 .with_size(4096)
//                 .unwrap()
//                 .wait()
//                 .unwrap();
//             // ...and an SHM provider
//             let provider = ShmProviderBuilder::builder()
//                 .protocol_id::<POSIX_PROTOCOL_ID>()
//                 .backend(backend)
//                 .wait();
//
//             // Prepare a layout for allocations
//             let layout = provider.alloc(1024).into_layout().unwrap();
//
//             // allocate an SHM buffer
//             let mutable_shm_buf = layout.alloc().wait().unwrap();
//
//             // convert to immutable SHM buffer
//             let immutable_shm_buf: ZShm = mutable_shm_buf.into();
//
//             serialize_deserialize!(&zshm, immutable_shm_buf);
//         }
//         #[cfg(feature = "shared-memory")]
//         shm();
//
//         // Parameters
//         serialize_deserialize!(Parameters, Parameters::from(""));
//         serialize_deserialize!(Parameters, Parameters::from("a=1;b=2;c3"));
//
//         // Bytes
//         serialize_deserialize!(bytes::Bytes, bytes::Bytes::from(vec![1, 2, 3, 4]));
//         serialize_deserialize!(bytes::Bytes, bytes::Bytes::from("Hello World"));
//
//         // Tuple
//         serialize_deserialize!((usize, usize), (0, 1));
//         serialize_deserialize!((usize, String), (0, String::from("a")));
//         serialize_deserialize!((String, String), (String::from("a"), String::from("b")));
//         serialize_deserialize!(
//             (Cow<'static, [u8]>, Cow<'static, [u8]>),
//             (Cow::from(vec![0u8; 8]), Cow::from(vec![0u8; 8]))
//         );
//         serialize_deserialize!(
//             (Cow<'static, str>, Cow<'static, str>),
//             (Cow::from("a"), Cow::from("b"))
//         );
//     }
// }
