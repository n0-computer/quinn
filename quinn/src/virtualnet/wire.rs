use tokio::sync::mpsc;

use super::socket::Plug;

pub struct Wire {
    start: Plug,
    end: Plug,
}

impl Wire {
    pub fn new(capacity: usize) -> Self {
        // start -> end transmission
        let (start_sender, end_receiver) = mpsc::channel(capacity);
        // end -> start transmission
        let (end_sender, start_receiver) = mpsc::channel(capacity);
        let start = Plug {
            sender: start_sender,
            receiver: start_receiver,
        };
        let end = Plug {
            sender: end_sender,
            receiver: end_receiver,
        };

        Self { start, end }
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use crate::{
        virtualnet::{socket::VirtualSocket, OwnedTransmit, TestAddr},
        AsyncUdpSocket,
    };

    use super::Wire;

    #[tokio::test]
    async fn test_wire_plugging() -> std::io::Result<()> {
        let wire = Wire::new(64);
        let socket0 = VirtualSocket::new(TestAddr(11), wire.start);
        let mut socket1 = VirtualSocket::new(TestAddr(99), wire.end);

        let contents = Bytes::copy_from_slice(b"Hello, world!");
        let transmit = OwnedTransmit {
            contents: contents.clone(),
            destination: socket1.addr,
            ecn: None,
            segment_size: None,
            src_ip: socket0.addr,
        };

        let mut socket_sender = socket0.create_sender();
        std::future::poll_fn(|cx| {
            socket_sender
                .as_mut()
                .poll_send(&transmit.as_quinn_transmit(), cx)
        })
        .await?;

        let (source_addr, received) = socket1.receive_data().await?;

        assert_eq!(source_addr, socket0.addr);
        assert_eq!(received, contents);

        Ok(())
    }
}
