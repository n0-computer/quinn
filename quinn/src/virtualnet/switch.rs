use std::{
    collections::BTreeMap,
    net::{IpAddr, SocketAddr},
};

use futures_buffered::FuturesUnordered;
use smol::future::FutureExt;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tokio_util::task::AbortOnDropHandle;

use super::{
    socket::{Plug, VirtualSocket},
    wire::Wire,
    OwnedTransmit,
};

pub struct Switch {
    _task: AbortOnDropHandle<()>,
    inbox: mpsc::Sender<SwitchMessage>,
}

enum SwitchMessage {
    PlugIn(IpAddr, Plug),
}

impl Switch {
    pub fn new() -> Self {
        let (inbox, receiver) = mpsc::channel(64);
        Self {
            _task: AbortOnDropHandle::new(tokio::spawn(async move { Self::run(receiver).await })),
            inbox,
        }
    }

    pub async fn connect_socket(&self, addr: impl Into<SocketAddr>) -> VirtualSocket {
        let addr = addr.into();
        let wire = Wire::new(64);
        let socket = VirtualSocket::new(addr, wire.start);
        self.plug_in(addr.ip(), wire.end).await;
        socket
    }

    pub async fn plug_in(&self, addr: IpAddr, plug: Plug) {
        self.inbox
            .send(SwitchMessage::PlugIn(addr, plug))
            .await
            .expect("couldn't send: switch actor dead")
    }

    async fn run(mut switch_messages: mpsc::Receiver<SwitchMessage>) {
        let mut all_incoming = FuturesUnordered::new();
        let mut all_outgoing = FuturesUnordered::new();
        let mut senders: BTreeMap<IpAddr, mpsc::Sender<OwnedTransmit>> = BTreeMap::new();
        loop {
            tokio::select! {
                biased;
                message = switch_messages.recv() => {
                    match message {
                        None => {
                            tracing::debug!("switch: switch messages dropped, stop the actor");
                            return; // stop the actor
                        },
                        Some(SwitchMessage::PlugIn(addr, mut plug)) => {
                            if senders.contains_key(&addr) {
                                tracing::debug!("switch: can't plug in at {addr}, already used.");
                            } else {
                                tracing::debug!("switch: new device plugged in");
                                all_incoming.push(async move {
                                    let recieved = plug.receiver.recv().await;
                                    (addr, recieved, plug.receiver)
                                }.boxed());
                                senders.insert(addr, plug.sender);
                            }
                        }
                    }
                },
                res = all_outgoing.next(), if !all_outgoing.is_empty() => {
                    tracing::debug!("switch: finished sending");
                    if let Some((dst_ip, Err(_e))) = res {
                        tracing::debug!("switch: sending to {dst_ip} failed, unplugging");
                        senders.remove(&dst_ip);
                    }
                },
                incoming = all_incoming.next(), if !all_incoming.is_empty() => {
                    tracing::debug!("switch: received");
                    let (addr, transmit, mut receiver) = incoming.expect("impossible panic: checked is_empty");

                    let Some(transmit) = transmit else {
                        tracing::debug!("switch: receiver for {addr} dropped, removing sender, too");
                        senders.remove(&addr);
                        continue;
                    };
                    // Make sure to poll the plug for further recieves again
                    all_incoming.push(async move {
                        let recieved = receiver.recv().await;
                        (addr, recieved, receiver)
                    }.boxed());
                    if addr != transmit.src_ip.ip() {
                        tracing::debug!("switch: sender lies about its address, possibly a bug?");
                    }
                    let dst = transmit.destination.ip();
                    let Some(sender) = senders.get(&dst).cloned() else {
                        tracing::debug!("switch: no receiver for transmit to {addr}");
                        continue;
                    };
                    all_outgoing.push(async move {
                        (dst, sender.send(transmit).await)
                    });
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use crate::virtualnet::TestAddr;

    use super::Switch;

    #[tokio::test]
    async fn test_switch_like_wire() -> anyhow::Result<()> {
        let switch = Switch::new();

        let socket0 = switch.connect_socket(TestAddr(11)).await;
        let mut socket1 = switch.connect_socket(TestAddr(99)).await;

        let contents = Bytes::copy_from_slice(b"Hello, world!");
        socket0
            .send_datagram(socket1.addr, contents.clone())
            .await?;

        let (source_addr, received) = socket1.receive_datagram().await?;

        assert_eq!(source_addr, socket0.addr);
        assert_eq!(received, contents);

        Ok(())
    }

    #[tokio::test]
    async fn test_switch_chooses_destination() -> anyhow::Result<()> {
        let switch = Switch::new();

        let socket0 = switch.connect_socket(TestAddr(11)).await;
        let mut socket1 = switch.connect_socket(TestAddr(99)).await;
        let mut socket2 = switch.connect_socket(TestAddr(44)).await;

        let content99 = Bytes::copy_from_slice(b"Hello, world!");
        socket0
            .send_datagram(socket1.addr, content99.clone())
            .await?;

        let content44 = Bytes::copy_from_slice(b"Hello, 44!");
        socket0
            .send_datagram(socket2.addr, content44.clone())
            .await?;

        let (source_addr, received) = socket1.receive_datagram().await?;

        assert_eq!(source_addr, socket0.addr);
        assert_eq!(received, content99);

        let (source_addr, received) = socket2.receive_datagram().await?;
        assert_eq!(source_addr, socket0.addr);
        assert_eq!(received, content44);

        Ok(())
    }
}
