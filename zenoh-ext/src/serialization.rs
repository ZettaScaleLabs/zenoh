use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fmt,
    hash::Hash,
    io::{Read, Write},
    marker::PhantomData,
    ptr,
};

use zenoh::bytes::{ZBytes, ZBytesReader, ZBytesWriter};

#[derive(Debug)]
pub struct ZDeserializeError;
impl fmt::Display for ZDeserializeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "deserialization error")
    }
}
impl std::error::Error for ZDeserializeError {}

pub trait Serialize {
    fn serialize(&self, serializer: &mut ZSerializer);
    #[doc(hidden)]
    fn serialize_slice(slice: &[Self], serializer: &mut ZSerializer)
    where
        Self: Sized,
    {
        serializer.serialize_iter(slice);
    }
}
impl<T: Serialize> Serialize for &T {
    fn serialize(&self, serializer: &mut ZSerializer) {
        T::serialize(*self, serializer)
    }
}

pub trait Deserialize: Sized {
    fn deserialize(deserializer: &mut ZDeserializer) -> Result<Self, ZDeserializeError>;
    #[doc(hidden)]
    fn deserialize_slice(
        deserializer: &mut ZDeserializer,
    ) -> Result<Box<[Self]>, ZDeserializeError> {
        deserializer.deserialize_iter()?.collect()
    }
}

pub fn z_serialize<T: Serialize>(t: &T) -> ZBytes {
    let mut zbytes = ZBytes::new();
    ZSerializer::new(&mut zbytes).serialize(t);
    zbytes
}

pub fn z_deserialize<T: Deserialize>(zbytes: &ZBytes) -> Result<T, ZDeserializeError> {
    let mut deserializer = ZDeserializer::new(zbytes);
    let t = T::deserialize(&mut deserializer)?;
    if !deserializer.done() {
        return Err(ZDeserializeError);
    }
    Ok(t)
}

#[derive(Debug)]
pub struct ZSerializer<'a>(ZBytesWriter<'a>);

impl<'a> ZSerializer<'a> {
    pub fn new(zbytes: &'a mut ZBytes) -> Self {
        Self(zbytes.writer())
    }

    pub fn serialize<T: Serialize>(&mut self, t: T) {
        t.serialize(self)
    }

    pub fn serialize_iter<T: Serialize, I: IntoIterator<Item = T>>(&mut self, iter: I)
    where
        I::IntoIter: ExactSizeIterator,
    {
        let iter = iter.into_iter();
        self.0.write_vle(iter.len() as u64);
        for t in iter {
            t.serialize(self);
        }
    }
}

impl<'a> From<ZBytesWriter<'a>> for ZSerializer<'a> {
    fn from(value: ZBytesWriter<'a>) -> Self {
        Self(value)
    }
}

#[derive(Debug)]
pub struct ZDeserializer<'a>(ZBytesReader<'a>);

impl<'a> ZDeserializer<'a> {
    pub fn new(zbytes: &'a ZBytes) -> Self {
        Self(zbytes.reader())
    }

    pub fn done(&self) -> bool {
        self.0.is_empty()
    }

    pub fn deserialize<T: Deserialize>(&mut self) -> Result<T, ZDeserializeError> {
        T::deserialize(self)
    }

    fn read_vle(&mut self) -> Result<usize, ZDeserializeError> {
        Ok(self.0.read_vle().ok_or(ZDeserializeError)? as usize)
    }

    pub fn deserialize_iter<'b, T: Deserialize>(
        &'b mut self,
    ) -> Result<ZReadIter<'a, 'b, T>, ZDeserializeError> {
        let len = self.read_vle()?;
        Ok(ZReadIter {
            deserializer: self,
            len,
            _phantom: PhantomData,
        })
    }
}

impl<'a> From<ZBytesReader<'a>> for ZDeserializer<'a> {
    fn from(value: ZBytesReader<'a>) -> Self {
        Self(value)
    }
}

pub struct ZReadIter<'a, 'b, T: Deserialize> {
    deserializer: &'b mut ZDeserializer<'a>,
    len: usize,
    _phantom: PhantomData<T>,
}

impl<T: Deserialize> Iterator for ZReadIter<'_, '_, T> {
    type Item = Result<T, ZDeserializeError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        Some(T::deserialize(self.deserializer))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<T: Deserialize> ExactSizeIterator for ZReadIter<'_, '_, T> {}

impl<T: Deserialize> Drop for ZReadIter<'_, '_, T> {
    fn drop(&mut self) {
        self.by_ref().for_each(drop);
    }
}

impl Serialize for ZBytes {
    fn serialize(&self, serializer: &mut ZSerializer) {
        serializer.0.write_vle(self.len() as u64);
        serializer.0.append(self.clone());
    }
}

macro_rules! impl_num {
    ($($ty:ty),* $(,)?) => {$(
        impl Serialize for $ty {
            fn serialize(&self, serializer: &mut ZSerializer) {
                serializer.0.write_all(&(*self).to_le_bytes()).unwrap();
            }
            fn serialize_slice(slice: &[Self], serializer: &mut ZSerializer) where Self: Sized {
                if cfg!(target_endian = "little") || std::mem::size_of::<Self>() == 1 {
                    serializer.0.write_vle(slice.len() as u64);
                    // SAFETY: transmuting numeric types to their little endian bytes is safe
                    serializer.0.write_all(unsafe { slice.align_to().1 }).unwrap();
                } else {
                    serializer.serialize_iter(slice);
                }
            }
        }
        impl Deserialize for $ty {
            fn deserialize(deserializer: &mut ZDeserializer) -> Result<Self, ZDeserializeError> {
                let mut buf = [0; { std::mem::size_of::<Self>() }];
                deserializer.0.read_exact(&mut buf).or(Err(ZDeserializeError))?;
                Ok(<$ty>::from_le_bytes(buf))
            }
            fn deserialize_slice(deserializer: &mut ZDeserializer) -> Result<Box<[Self]>, ZDeserializeError> {
                let size = std::mem::size_of::<Self>();
                if cfg!(target_endian = "little") || size == 1 {
                    let len = deserializer.read_vle()? as usize;
                    let total_size = len * size;
                    let mut buf = std::mem::ManuallyDrop::new(vec![0; total_size].into_boxed_slice());
                    deserializer.0.read_exact(&mut buf).or(Err(ZDeserializeError))?;
                    // SAFETY: transmuting numeric types from their little endian bytes is safe
                    Ok(unsafe { Box::from_raw(ptr::slice_from_raw_parts_mut(buf.as_mut_ptr().cast(), len)) })
                } else {
                    deserializer.deserialize_iter()?.collect()
                }
            }
        }
    )*};
}
impl_num!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, f32, f64);

impl Serialize for bool {
    fn serialize(&self, serializer: &mut ZSerializer) {
        (*self as u8).serialize(serializer);
    }
}
impl Deserialize for bool {
    fn deserialize(deserializer: &mut ZDeserializer) -> Result<Self, ZDeserializeError> {
        match u8::deserialize(deserializer)? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(ZDeserializeError),
        }
    }
}

impl<T: Serialize> Serialize for [T] {
    fn serialize(&self, serializer: &mut ZSerializer) {
        T::serialize_slice(self, serializer)
    }
}
impl<'a, T: Serialize + 'a> Serialize for Cow<'a, [T]>
where
    [T]: ToOwned,
{
    fn serialize(&self, serializer: &mut ZSerializer) {
        T::serialize_slice(self, serializer)
    }
}
impl<T: Serialize> Serialize for Box<[T]> {
    fn serialize(&self, serializer: &mut ZSerializer) {
        T::serialize_slice(self, serializer)
    }
}
impl<T: Deserialize> Deserialize for Box<[T]> {
    fn deserialize(deserializer: &mut ZDeserializer) -> Result<Self, ZDeserializeError> {
        T::deserialize_slice(deserializer)
    }
}
impl<T: Serialize> Serialize for Vec<T> {
    fn serialize(&self, serializer: &mut ZSerializer) {
        T::serialize_slice(self, serializer)
    }
}
impl<T: Deserialize> Deserialize for Vec<T> {
    fn deserialize(deserializer: &mut ZDeserializer) -> Result<Self, ZDeserializeError> {
        T::deserialize_slice(deserializer).map(Into::into)
    }
}
impl<T: Serialize + Eq + Hash> Serialize for HashSet<T> {
    fn serialize(&self, serializer: &mut ZSerializer) {
        serializer.serialize_iter(self);
    }
}
impl<T: Deserialize + Eq + Hash> Deserialize for HashSet<T> {
    fn deserialize(deserializer: &mut ZDeserializer) -> Result<Self, ZDeserializeError> {
        deserializer.deserialize_iter()?.collect()
    }
}
impl<T: Serialize + Ord> Serialize for BTreeSet<T> {
    fn serialize(&self, serializer: &mut ZSerializer) {
        serializer.serialize_iter(self);
    }
}
impl<T: Deserialize + Ord> Deserialize for BTreeSet<T> {
    fn deserialize(deserializer: &mut ZDeserializer) -> Result<Self, ZDeserializeError> {
        deserializer.deserialize_iter()?.collect()
    }
}
impl<K: Serialize + Eq + Hash, V: Serialize> Serialize for HashMap<K, V> {
    fn serialize(&self, serializer: &mut ZSerializer) {
        serializer.serialize_iter(self);
    }
}
impl<K: Deserialize + Eq + Hash, V: Deserialize> Deserialize for HashMap<K, V> {
    fn deserialize(deserializer: &mut ZDeserializer) -> Result<Self, ZDeserializeError> {
        deserializer.deserialize_iter()?.collect()
    }
}
impl<K: Serialize + Ord, V: Serialize> Serialize for BTreeMap<K, V> {
    fn serialize(&self, serializer: &mut ZSerializer) {
        serializer.serialize_iter(self);
    }
}
impl<K: Deserialize + Ord, V: Deserialize> Deserialize for BTreeMap<K, V> {
    fn deserialize(deserializer: &mut ZDeserializer) -> Result<Self, ZDeserializeError> {
        deserializer.deserialize_iter()?.collect()
    }
}
impl Serialize for str {
    fn serialize(&self, serializer: &mut ZSerializer) {
        self.as_bytes().serialize(serializer);
    }
}
impl Serialize for Cow<'_, str> {
    fn serialize(&self, serializer: &mut ZSerializer) {
        self.as_bytes().serialize(serializer);
    }
}
impl Serialize for String {
    fn serialize(&self, serializer: &mut ZSerializer) {
        self.as_bytes().serialize(serializer);
    }
}
impl Deserialize for String {
    fn deserialize(deserializer: &mut ZDeserializer) -> Result<Self, ZDeserializeError> {
        String::from_utf8(Deserialize::deserialize(deserializer)?).or(Err(ZDeserializeError))
    }
}

macro_rules! impl_tuple {
    ($($ty:ident/$i:tt),* $(,)?) => {
        impl_tuple!(@;$($ty/$i),*);
    };
    (@$($ty:ident/$i:tt),*; $next:ident/$next_i:tt $(, $remain:ident/$remain_i:tt)*) => {
        impl_tuple!(@@$($ty/$i),*);
        impl_tuple!(@$($ty/$i,)* $next/$next_i; $($remain/$remain_i),*);
    };
    (@$($ty:ident/$i:tt),*;) => {
        impl_tuple!(@@$($ty/$i),*);
    };
    (@@$($ty:ident/$i:tt),* $(,)?) => {
        #[allow(unused)]
        impl<$($ty: Serialize),*> Serialize for ($($ty,)*) {
            fn serialize(&self, serializer: &mut ZSerializer) {
                $(self.$i.serialize(serializer);)*
            }
        }
        #[allow(unused)]
        impl<$($ty: Deserialize),*> Deserialize for ($($ty,)*) {
            fn deserialize(deserializer: &mut ZDeserializer) -> Result<Self, ZDeserializeError> {
                Ok(($($ty::deserialize(deserializer)?,)*))
            }
        }
    };
}
impl_tuple!(
    T0 / 0,
    T1 / 1,
    T2 / 2,
    T3 / 3,
    T4 / 4,
    T5 / 5,
    T6 / 6,
    T7 / 7,
    T8 / 8,
    T9 / 9,
    T10 / 10,
    T11 / 11,
    T12 / 12,
    T13 / 13,
    T14 / 14,
    T15 / 15,
);
