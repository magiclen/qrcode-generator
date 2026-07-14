//! Renders a QR Code and writes it through the asynchronous writer API.
//!
//! Rendering itself is synchronous; only writing and flushing are asynchronous.
//! This example drives the future with a tiny poll so it needs no async runtime; a real program awaits it on its own runtime instead.
//!
//! Run with: `cargo run --example async_write --features async-write`

use std::{
    future::Future,
    pin::pin,
    task::{Context, Poll, Waker},
};

use qrcode_generator::{
    Renderer,
    qr::{Encoder, ErrorCorrection},
};

fn main() {
    let symbol = Encoder::new(ErrorCorrection::Medium).encode_text("Hello, async world!").unwrap();

    let mut svg = Vec::new();

    block_on(Renderer::new(&symbol, 512).write_svg_async(&mut svg, None)).unwrap();

    println!("wrote {} SVG bytes through the async writer", svg.len());
}

// The SVG is fully rendered before the first write, so the future is ready at once and one poll is enough.
fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());

    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => unreachable!("in-memory writing never suspends"),
    }
}
