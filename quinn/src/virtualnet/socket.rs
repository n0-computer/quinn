use std::{
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
};

use crate::AsyncUdpSocket;
use bytes::Bytes;
use tokio::sync::mpsc;
use tokio_util::sync::PollSender;
use udp::Transmit;

use super::OwnedTransmit;

#[derive(Debug)]
pub struct VirtualSocket {
    pub addr: SocketAddr,
    sender: mpsc::Sender<OwnedTransmit>,
    receiver: mpsc::Receiver<OwnedTransmit>,
}

#[derive(Debug)]
pub struct VirtualSocketSender {
    addr: SocketAddr,
    sender: PollSender<OwnedTransmit>,
}

#[derive(Debug)]
pub struct Plug {
    pub sender: mpsc::Sender<OwnedTransmit>,
    pub receiver: mpsc::Receiver<OwnedTransmit>,
}

impl Plug {
    pub fn testometer(
        capacity: usize,
    ) -> (
        Self,
        mpsc::Sender<OwnedTransmit>,
        mpsc::Receiver<OwnedTransmit>,
    ) {
        let (sender, test_receiver) = mpsc::channel(capacity);
        let (test_sender, receiver) = mpsc::channel(capacity);
        (Self { sender, receiver }, test_sender, test_receiver)
    }
}

impl VirtualSocket {
    pub fn new(addr: impl Into<SocketAddr>, plug: Plug) -> Self {
        Self {
            addr: addr.into(),
            sender: plug.sender,
            receiver: plug.receiver,
        }
    }

    pub async fn receive_datagram(&mut self) -> std::io::Result<(SocketAddr, Bytes)> {
        use std::io::IoSliceMut;

        let mut buf = [0u8; 1200];
        let mut bufs = [IoSliceMut::new(&mut buf)];
        let mut meta = [udp::RecvMeta::default()];

        let num_datagrams =
            std::future::poll_fn(|cx| self.poll_recv(cx, &mut bufs, &mut meta)).await?;

        debug_assert_eq!(num_datagrams, 1); // we don't support GSO/GRO(?)

        Ok((meta[0].addr, Bytes::copy_from_slice(&buf[..meta[0].len])))
    }

    pub async fn send_datagram(
        &self,
        destination: SocketAddr,
        contents: Bytes,
    ) -> std::io::Result<()> {
        let transmit = OwnedTransmit {
            contents,
            destination,
            ecn: None,
            segment_size: None,
            src_ip: self.addr,
        };

        let mut socket_sender = self.create_sender();
        std::future::poll_fn(|cx| {
            socket_sender
                .as_mut()
                .poll_send(&transmit.as_quinn_transmit(), cx)
        })
        .await?;

        Ok(())
    }
}

impl crate::UdpSender for VirtualSocketSender {
    fn max_transmit_segments(&self) -> usize {
        1
    }

    fn poll_send(
        mut self: Pin<&mut Self>,
        transmit: &Transmit,
        cx: &mut Context,
    ) -> Poll<std::io::Result<()>> {
        if let Err(_closed) = std::task::ready!(self.as_mut().sender.poll_reserve(cx)) {
            return Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "virtual socket closed when sending",
            )));
        }

        self.try_send(transmit)?;

        Poll::Ready(Ok(()))
    }

    fn try_send(mut self: Pin<&mut Self>, transmit: &Transmit) -> std::io::Result<()> {
        let addr = self.addr;
        if let Err(_closed) = self
            .as_mut()
            .sender
            .send_item(OwnedTransmit::new(addr, transmit))
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "virtual socket closed when sending",
            ));
        }

        Ok(())
    }
}

impl AsyncUdpSocket for VirtualSocket {
    fn create_sender(&self) -> Pin<Box<dyn crate::UdpSender>> {
        Box::pin(VirtualSocketSender {
            addr: self.addr,
            sender: PollSender::new(self.sender.clone()),
        })
    }

    fn poll_recv(
        &mut self,
        cx: &mut Context,
        bufs: &mut [std::io::IoSliceMut<'_>],
        meta: &mut [udp::RecvMeta],
    ) -> Poll<std::io::Result<usize>> {
        let mut transmits = Vec::new();
        let recvd = ready!(self.receiver.poll_recv_many(cx, &mut transmits, bufs.len()));
        if recvd == 0 {
            return Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "virtual socket closed when receiving",
            )));
        }

        for ((t, buf), meta) in transmits
            .into_iter()
            .zip(bufs.iter_mut())
            .zip(meta.iter_mut())
        {
            if buf.len() >= t.contents.len() {
                tracing::debug!(
                    "Received from {:?} to {:?}: {} bytes",
                    t.src_ip,
                    t.destination,
                    t.contents.len()
                );
                t.receive_into(buf, meta)?;
            }
        }

        Poll::Ready(Ok(recvd))
    }

    fn local_addr(&self) -> std::io::Result<std::net::SocketAddr> {
        Ok(self.addr)
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use crate::virtualnet::{OwnedTransmit, TestAddr};

    use super::{Plug, VirtualSocket};

    #[tokio::test]
    async fn test_recv() -> anyhow::Result<()> {
        let (plug, sender, _receiver) = Plug::testometer(64);
        let mut socket = VirtualSocket::new(TestAddr(11), plug);

        let other_addr = TestAddr(99).into();
        let contents = Bytes::copy_from_slice(b"Hello, world!");
        let transmit = OwnedTransmit {
            contents: contents.clone(),
            destination: socket.addr,
            ecn: None,
            segment_size: None,
            src_ip: other_addr,
        };

        sender.send(transmit).await?;

        let (source_addr, received) = socket.receive_datagram().await?;

        assert_eq!(received, contents);
        assert_eq!(source_addr, other_addr);

        Ok(())
    }

    #[tokio::test]
    async fn test_send() -> anyhow::Result<()> {
        let (plug, _sender, mut receiver) = Plug::testometer(64);
        let socket = VirtualSocket::new(TestAddr(11), plug);

        let other_addr = TestAddr(99).into();
        let contents = Bytes::copy_from_slice(b"Hello, world!");
        socket.send_datagram(other_addr, contents.clone()).await?;

        assert!(!receiver.is_empty());
        let received = receiver.recv().await.unwrap();
        assert_eq!(received.src_ip, socket.addr);
        assert_eq!(received.destination, other_addr);
        assert_eq!(received.contents, contents);

        Ok(())
    }
}
