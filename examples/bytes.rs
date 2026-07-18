//! Encodes an exact binary payload into a QR Code and saves an SVG.
//!
//! Unlike the text API, the byte API never guesses a character set and never adds an ECI header.
//!
//! Run with: `cargo run --example bytes`

use qrcode_generator::{
    Renderer,
    qr::{Encoder, ErrorCorrection},
};

fn main() {
    let payload = b"\x00\x01\x02 raw bytes \xFF";

    let symbol = Encoder::new(ErrorCorrection::Quartile).encode_bytes(payload).unwrap();

    Renderer::new(&symbol, 512).save_svg("bytes_output.svg", None::<&str>).unwrap();

    println!("wrote bytes_output.svg ({} modules per side)", symbol.size());
}
