use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use wasmtimer::tokio::{sleep_until, Sleep};

use crate::{AsyncTimer, Runtime};
use proto::Instant;

#[cfg(feature = "wasm")]
fn instant_to_wasmtimer_instant(input: Instant) -> wasmtimer::std::Instant {
    let now = Instant::now();
    let remaining = input.checked_duration_since(now).unwrap_or_default();
    let target = wasmtimer::std::Instant::now() + remaining;
    target
}

/// A runtime for WASM browser.
#[derive(Debug)]
pub struct WasmRuntime;

impl Runtime for WasmRuntime {
    fn new_timer(&self, t: Instant) -> Pin<Box<dyn AsyncTimer>> {
        let t = instant_to_wasmtimer_instant(t);
        Box::pin(MySleep(Box::pin(sleep_until(t.into()))))
    }

    fn spawn(&self, future: Pin<Box<dyn Future<Output = ()> + Send>>) {
        wasm_bindgen_futures::spawn_local(future);
    }
}

#[derive(Debug)]
struct MySleep(Pin<Box<Sleep>>);

impl AsyncTimer for MySleep {
    fn reset(mut self: Pin<&mut Self>, t: Instant) {
        let t = instant_to_wasmtimer_instant(t);
        Sleep::reset(self.0.as_mut(), t.into())
    }
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
        Future::poll(self.0.as_mut(), cx)
    }
}
