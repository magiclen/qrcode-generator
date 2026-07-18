//! Encodes a Rectangular Micro QR Code and saves it as SVG.
//!
//! Run with: `cargo run --example rmqr --features rmqr`

use qrcode_generator::{
    Renderer,
    rmqr::{Encoder, ErrorCorrection},
};

fn main() {
    let symbol = Encoder::new(ErrorCorrection::Medium).encode_text("https://magiclen.org").unwrap();

    // Add the two-module quiet zone on each side and render every module as an 8 by 8 pixel square.
    let width = (symbol.width() + 4) * 8;
    let height = (symbol.height() + 4) * 8;

    Renderer::new_with_dimensions(&symbol, width, height)
        .save_svg("rmqr_output.svg", None::<&str>)
        .unwrap();

    println!("wrote rmqr_output.svg ({} x {} modules)", symbol.width(), symbol.height());
}
