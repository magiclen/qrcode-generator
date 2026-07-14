//! Encodes text into a QR Code and saves it as both SVG and PNG.
//!
//! Run with: `cargo run --example basic`

use qrcode_generator::{
    Renderer,
    qr::{Encoder, ErrorCorrection},
};

fn main() {
    // Encoding turns the input into a Symbol, the abstract grid of modules.
    let symbol = Encoder::new(ErrorCorrection::Medium).encode_text("Hello, world!").unwrap();

    // Rendering turns that Symbol into a concrete image at an exact pixel size.
    let renderer = Renderer::new(&symbol, 512);

    renderer.save_svg("basic.svg", None).unwrap();
    renderer.save_png("basic.png").unwrap();

    println!("wrote basic.svg and basic.png ({} modules per side)", symbol.size());
}
