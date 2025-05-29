#![allow(missing_docs)]
use std::net::SocketAddr;

use bytes::Bytes;
use udp::{EcnCodepoint, Transmit};

pub mod socket;
#[cfg(feature = "runtime-tokio")]
pub mod switch;
pub mod wire;

pub struct TestAddr(pub u8);

impl From<TestAddr> for SocketAddr {
    fn from(TestAddr(id): TestAddr) -> Self {
        ([1, 1, 1, id], 42u16).into()
    }
}

#[derive(Debug, Clone)]
pub struct OwnedTransmit {
    pub destination: SocketAddr,
    pub ecn: Option<EcnCodepoint>,
    pub contents: Bytes,
    pub segment_size: Option<usize>,
    pub src_ip: SocketAddr,
}

impl OwnedTransmit {
    fn new(src: SocketAddr, t: &udp::Transmit) -> Self {
        Self {
            destination: t.destination,
            ecn: t.ecn.clone(),
            contents: Bytes::copy_from_slice(t.contents),
            segment_size: t.segment_size.clone(),
            src_ip: SocketAddr::new(t.src_ip.unwrap_or(src.ip()), src.port()),
        }
    }

    fn as_quinn_transmit(&self) -> Transmit<'_> {
        Transmit {
            destination: self.destination,
            ecn: self.ecn,
            contents: self.contents.as_ref(),
            segment_size: self.segment_size.clone(),
            src_ip: Some(self.src_ip.ip()),
        }
    }

    fn receive_into(
        &self,
        buf: &mut std::io::IoSliceMut<'_>,
        meta: &mut udp::RecvMeta,
    ) -> std::io::Result<()> {
        buf[..self.contents.len()].copy_from_slice(&self.contents);
        meta.addr = self.src_ip;
        meta.dst_ip = Some(self.destination.ip());
        meta.len = self.contents.len();
        meta.stride = self.contents.len();
        meta.ecn = self.ecn;
        Ok(())
    }
}
