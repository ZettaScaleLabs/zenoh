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
use crate::{RCodec, WCodec, Zenoh080, Zenoh080Header};
use alloc::vec::Vec;
use zenoh_buffers::{
    reader::{DidntRead, Reader},
    writer::{DidntWrite, Writer},
    ZBuf,
};
use zenoh_protocol::common::{
    iext, imsg::has_flag, ZExtUnit, ZExtUnknown, ZExtZ64, ZExtZBuf, ZExtZBufHeader, ZExtensionBody,
};

// ZExtUnit
impl<const ID: u8, W> WCodec<(&ZExtUnit<{ ID }>, bool), &mut W> for Zenoh080
where
    W: Writer,
{
    type Output = Result<(), DidntWrite>;

    fn write(self, writer: &mut W, x: (&ZExtUnit<{ ID }>, bool)) -> Self::Output {
        let (_x, more) = x;
        let mut header: u8 = ID | iext::ENC_UNIT;
        if more {
            header |= iext::FLAG_Z;
        }
        self.write(&mut *writer, header)?;
        Ok(())
    }
}

impl<const ID: u8, R> RCodec<(ZExtUnit<{ ID }>, bool), &mut R> for Zenoh080
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<(ZExtUnit<{ ID }>, bool), Self::Error> {
        let header: u8 = self.read(&mut *reader)?;
        let codec = Zenoh080Header::new(header);
        codec.read(&mut *reader)
    }
}

impl<const ID: u8, R> RCodec<(ZExtUnit<{ ID }>, bool), &mut R> for Zenoh080Header
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, _reader: &mut R) -> Result<(ZExtUnit<{ ID }>, bool), Self::Error> {
        if (self.header & iext::ID_MASK != ID) || (self.header & iext::ENC_MASK != iext::ENC_UNIT) {
            return Err(DidntRead);
        }

        Ok((ZExtUnit::new(), has_flag(self.header, iext::FLAG_Z)))
    }
}

// ZExtZ64
impl<const ID: u8, W> WCodec<(&ZExtZ64<{ ID }>, bool), &mut W> for Zenoh080
where
    W: Writer,
{
    type Output = Result<(), DidntWrite>;

    fn write(self, writer: &mut W, x: (&ZExtZ64<{ ID }>, bool)) -> Self::Output {
        let (x, more) = x;
        let mut header: u8 = ID | iext::ENC_Z64;
        if more {
            header |= iext::FLAG_Z;
        }
        self.write(&mut *writer, header)?;
        self.write(&mut *writer, x.value)?;
        Ok(())
    }
}

impl<const ID: u8, R> RCodec<(ZExtZ64<{ ID }>, bool), &mut R> for Zenoh080
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<(ZExtZ64<{ ID }>, bool), Self::Error> {
        let header: u8 = self.read(&mut *reader)?;
        let codec = Zenoh080Header::new(header);
        codec.read(&mut *reader)
    }
}

impl<const ID: u8, R> RCodec<(ZExtZ64<{ ID }>, bool), &mut R> for Zenoh080Header
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<(ZExtZ64<{ ID }>, bool), Self::Error> {
        if (self.header & iext::ID_MASK != ID) || (self.header & iext::ENC_MASK != iext::ENC_Z64) {
            return Err(DidntRead);
        }

        let value: u64 = self.codec.read(&mut *reader)?;

        Ok((ZExtZ64::new(value), has_flag(self.header, iext::FLAG_Z)))
    }
}

// ZExtZBuf
impl<const ID: u8, W> WCodec<(&ZExtZBuf<{ ID }>, bool), &mut W> for Zenoh080
where
    W: Writer,
{
    type Output = Result<(), DidntWrite>;

    fn write(self, writer: &mut W, x: (&ZExtZBuf<{ ID }>, bool)) -> Self::Output {
        let (x, more) = x;
        let mut header: u8 = ID | iext::ENC_ZBUF;
        if more {
            header |= iext::FLAG_Z;
        }
        self.write(&mut *writer, header)?;
        self.write(&mut *writer, &x.value)?;
        Ok(())
    }
}

impl<const ID: u8, R> RCodec<(ZExtZBuf<{ ID }>, bool), &mut R> for Zenoh080
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<(ZExtZBuf<{ ID }>, bool), Self::Error> {
        let header: u8 = self.read(&mut *reader)?;
        let codec = Zenoh080Header::new(header);
        codec.read(&mut *reader)
    }
}

impl<const ID: u8, R> RCodec<(ZExtZBuf<{ ID }>, bool), &mut R> for Zenoh080Header
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<(ZExtZBuf<{ ID }>, bool), Self::Error> {
        if (self.header & iext::ID_MASK != ID) || (self.header & iext::ENC_MASK != iext::ENC_ZBUF) {
            return Err(DidntRead);
        }

        let value: ZBuf = self.codec.read(&mut *reader)?;

        Ok((ZExtZBuf::new(value), has_flag(self.header, iext::FLAG_Z)))
    }
}

// ZExtZBufHeader
impl<const ID: u8, W> WCodec<(&ZExtZBufHeader<{ ID }>, bool), &mut W> for Zenoh080
where
    W: Writer,
{
    type Output = Result<(), DidntWrite>;

    fn write(self, writer: &mut W, x: (&ZExtZBufHeader<{ ID }>, bool)) -> Self::Output {
        let (x, more) = x;
        let mut header: u8 = ID | iext::ENC_ZBUF;
        if more {
            header |= iext::FLAG_Z;
        }
        self.write(&mut *writer, header)?;
        self.write(&mut *writer, x.len)?;
        Ok(())
    }
}

impl<const ID: u8, R> RCodec<(ZExtZBufHeader<{ ID }>, bool), &mut R> for Zenoh080
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<(ZExtZBufHeader<{ ID }>, bool), Self::Error> {
        let header: u8 = self.read(&mut *reader)?;
        let codec = Zenoh080Header::new(header);
        codec.read(&mut *reader)
    }
}

impl<const ID: u8, R> RCodec<(ZExtZBufHeader<{ ID }>, bool), &mut R> for Zenoh080Header
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<(ZExtZBufHeader<{ ID }>, bool), Self::Error> {
        if (self.header & iext::ID_MASK != ID) || (self.header & iext::ENC_MASK != iext::ENC_ZBUF) {
            return Err(DidntRead);
        }

        let len: usize = self.codec.read(&mut *reader)?;

        Ok((
            ZExtZBufHeader::new(len),
            has_flag(self.header, iext::FLAG_Z),
        ))
    }
}

// ZExtUnknown
impl<W> WCodec<(&ZExtUnknown, bool), &mut W> for Zenoh080
where
    W: Writer,
{
    type Output = Result<(), DidntWrite>;

    fn write(self, writer: &mut W, x: (&ZExtUnknown, bool)) -> Self::Output {
        let (x, more) = x;
        let mut header: u8 = x.id;
        if more {
            header |= iext::FLAG_Z;
        }
        match &x.body {
            ZExtensionBody::Unit => {
                header |= iext::ENC_UNIT;
                self.write(&mut *writer, header)?
            }
            ZExtensionBody::Z64(u64) => {
                header |= iext::ENC_Z64;
                self.write(&mut *writer, header)?;
                self.write(&mut *writer, *u64)?
            }
            ZExtensionBody::ZBuf(zbuf) => {
                header |= iext::ENC_ZBUF;
                self.write(&mut *writer, header)?;
                self.write(&mut *writer, zbuf)?
            }
        }
        Ok(())
    }
}

impl<R> RCodec<(ZExtUnknown, bool), &mut R> for Zenoh080
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<(ZExtUnknown, bool), Self::Error> {
        let header: u8 = self.read(&mut *reader)?;
        let codec = Zenoh080Header::new(header);
        codec.read(&mut *reader)
    }
}

impl<R> RCodec<(ZExtUnknown, bool), &mut R> for Zenoh080Header
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<(ZExtUnknown, bool), Self::Error> {
        let body = match self.header & iext::ENC_MASK {
            iext::ENC_UNIT => ZExtensionBody::Unit,
            iext::ENC_Z64 => {
                let u64: u64 = self.codec.read(&mut *reader)?;
                ZExtensionBody::Z64(u64)
            }
            iext::ENC_ZBUF => {
                let zbuf: ZBuf = self.codec.read(&mut *reader)?;
                ZExtensionBody::ZBuf(zbuf)
            }
            _ => return Err(DidntRead),
        };

        Ok((
            ZExtUnknown {
                id: self.header & iext::ID_MASK,
                body,
            },
            has_flag(self.header, iext::FLAG_Z),
        ))
    }
}

impl<W> WCodec<&[ZExtUnknown], &mut W> for Zenoh080
where
    W: Writer,
{
    type Output = Result<(), DidntWrite>;

    fn write(self, writer: &mut W, x: &[ZExtUnknown]) -> Self::Output {
        let len = x.len();
        for (i, e) in x.iter().enumerate() {
            self.write(&mut *writer, (e, i < len - 1))?;
        }
        Ok(())
    }
}

impl<R> RCodec<Vec<ZExtUnknown>, &mut R> for Zenoh080
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<Vec<ZExtUnknown>, Self::Error> {
        let mut exts = Vec::new();
        let mut has_ext = reader.can_read();
        while has_ext {
            let (e, more): (ZExtUnknown, bool) = self.read(&mut *reader)?;
            exts.push(e);
            has_ext = more;
        }
        Ok(exts)
    }
}

// Macros
#[macro_export]
macro_rules! impl_zextz64 {
    ($ext:ty, $id:expr) => {
        impl<W> WCodec<($ext, bool), &mut W> for Zenoh080
        where
            W: Writer,
        {
            type Output = Result<(), DidntWrite>;

            fn write(self, writer: &mut W, x: ($ext, bool)) -> Self::Output {
                let (qos, more) = x;
                let ext: ZExtZ64<{ $id }> = qos.into();
                self.write(&mut *writer, (&ext, more))
            }
        }

        impl<R> RCodec<($ext, bool), &mut R> for Zenoh080
        where
            R: Reader,
        {
            type Error = DidntRead;

            fn read(self, reader: &mut R) -> Result<($ext, bool), Self::Error> {
                let header: u8 = self.read(&mut *reader)?;
                let codec = Zenoh080Header::new(header);
                codec.read(reader)
            }
        }

        impl<R> RCodec<($ext, bool), &mut R> for Zenoh080Header
        where
            R: Reader,
        {
            type Error = DidntRead;

            fn read(self, reader: &mut R) -> Result<($ext, bool), Self::Error> {
                let (ext, more): (ZExtZ64<{ $id }>, bool) = self.read(&mut *reader)?;
                Ok((ext.into(), more))
            }
        }
    };
}