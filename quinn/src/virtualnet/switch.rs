use std::{collections::BTreeMap, net::SocketAddr};

use tokio_util::task::AbortOnDropHandle;

use super::socket::Plug;

pub struct Switch {
    task: AbortOnDropHandle<()>,
}

impl Switch {
    pub fn new(plugs: BTreeMap<SocketAddr, Plug>) -> Self {
        Self {
            task: AbortOnDropHandle::new(tokio::spawn(async move { Self::run(plugs).await })),
        }
    }

    async fn run(mut plugs: BTreeMap<SocketAddr, Plug>) {
        // let mut pending_transmits = Vec::<OwnedTransmit>::new();
        // loop {
        //     for (_, plug) in plugs.iter_mut() {
        //        plug.receiver.poll_recv_many(cx, buffer, limit)
        //     }
        // }
    }
}
