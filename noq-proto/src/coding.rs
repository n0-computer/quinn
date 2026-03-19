//! Coding related traits.

use std::net::{Ipv4Addr, Ipv6Addr};

use bytes::{Buf, BufMut};
use thiserror::Error;

use crate::VarInt;

/// Error indicating that the provided buffer was too small
#[derive(Error, Debug, Copy, Clone, Eq, PartialEq)]
#[error("unexpected end of buffer")]
pub struct UnexpectedEnd;

/// Coding result type
pub type Result<T> = ::std::result::Result<T, UnexpectedEnd>;

/// Infallible encoding of QUIC primitives.
pub trait Encodable {
    /// Append the encoding of `self` to the provided buffer.
    fn encode<B: BufMut>(&self, buf: &mut B);
}

/// Decoding of QUIC primitives.
pub trait Decodable: Sized {
    /// Decode a `Self` from the provided buffer, if the buffer is large enough
    fn decode<B: Buf>(buf: &mut B) -> Result<Self>;
}

impl Decodable for u8 {
    fn decode<B: Buf>(buf: &mut B) -> Result<Self> {
        if buf.remaining() < 1 {
            return Err(UnexpectedEnd);
        }
        Ok(buf.get_u8())
    }
}

impl Encodable for u8 {
    fn encode<B: BufMut>(&self, buf: &mut B) {
        buf.put_u8(*self);
    }
}

impl Decodable for u16 {
    fn decode<B: Buf>(buf: &mut B) -> Result<Self> {
        if buf.remaining() < 2 {
            return Err(UnexpectedEnd);
        }
        Ok(buf.get_u16())
    }
}

impl Encodable for u16 {
    fn encode<B: BufMut>(&self, buf: &mut B) {
        buf.put_u16(*self);
    }
}

impl Decodable for u32 {
    fn decode<B: Buf>(buf: &mut B) -> Result<Self> {
        if buf.remaining() < 4 {
            return Err(UnexpectedEnd);
        }
        Ok(buf.get_u32())
    }
}

impl Encodable for u32 {
    fn encode<B: BufMut>(&self, buf: &mut B) {
        buf.put_u32(*self);
    }
}

impl Decodable for u64 {
    fn decode<B: Buf>(buf: &mut B) -> Result<Self> {
        if buf.remaining() < 8 {
            return Err(UnexpectedEnd);
        }
        Ok(buf.get_u64())
    }
}

impl Encodable for u64 {
    fn encode<B: BufMut>(&self, buf: &mut B) {
        buf.put_u64(*self);
    }
}

impl Decodable for Ipv4Addr {
    fn decode<B: Buf>(buf: &mut B) -> Result<Self> {
        if buf.remaining() < 4 {
            return Err(UnexpectedEnd);
        }
        let mut octets = [0; 4];
        buf.copy_to_slice(&mut octets);
        Ok(octets.into())
    }
}

impl Encodable for Ipv4Addr {
    fn encode<B: BufMut>(&self, buf: &mut B) {
        buf.put_slice(&self.octets());
    }
}

impl Decodable for Ipv6Addr {
    fn decode<B: Buf>(buf: &mut B) -> Result<Self> {
        if buf.remaining() < 16 {
            return Err(UnexpectedEnd);
        }
        let mut octets = [0; 16];
        buf.copy_to_slice(&mut octets);
        Ok(octets.into())
    }
}

impl Encodable for Ipv6Addr {
    fn encode<B: BufMut>(&self, buf: &mut B) {
        buf.put_slice(&self.octets());
    }
}

pub(crate) trait BufExt {
    fn get<T: Decodable>(&mut self) -> Result<T>;
    fn get_var(&mut self) -> Result<u64>;
}

impl<T: Buf> BufExt for T {
    fn get<U: Decodable>(&mut self) -> Result<U> {
        U::decode(self)
    }

    fn get_var(&mut self) -> Result<u64> {
        Ok(VarInt::decode(self)?.into_inner())
    }
}

pub(crate) trait BufMutExt {
    fn write<T: Encodable>(&mut self, x: T);
    fn write_var(&mut self, x: u64);
}

impl<T: BufMut> BufMutExt for T {
    fn write<U: Encodable>(&mut self, x: U) {
        x.encode(self);
    }

    fn write_var(&mut self, x: u64) {
        VarInt::from_u64(x).unwrap().encode(self);
    }
}
