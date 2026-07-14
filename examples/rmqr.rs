//! Encodes a Rectangular Micro QR Code and saves it as SVG.
//!
//! Run with: `cargo run --example rmqr --features rmqr`

use qrcode_generator::{
    Renderer,
    rmqr::{Encoder, ErrorCorrection},
};

fn main() {
    let symbol = Encoder::new(ErrorCorrection::Medium).encode_text("https://example.com").unwrap();
    let width = (symbol.width() + 4) * 8;
    let height = (symbol.height() + 4) * 8;

    Renderer::new_with_dimensions(&symbol, width, height).save_svg("rmqr.svg", None).unwrap();

    println!("wrote rmqr.svg ({} by {} modules)", symbol.width(), symbol.height());
}
