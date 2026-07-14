//! Encodes a Micro QR Code, a smaller symbol family for short payloads.
//!
//! Run with: `cargo run --example micro_qr --features micro-qr`

use qrcode_generator::{
    Renderer,
    micro::{Encoder, ErrorCorrection, Version},
};

fn main() {
    let symbol =
        Encoder::new(ErrorCorrection::Low).version(Version::M2).encode_text("12345").unwrap();

    Renderer::new(&symbol, 300).save_svg("micro_qr_output.svg", None::<&str>).unwrap();

    println!("wrote micro_qr_output.svg ({} modules per side)", symbol.size());
}
