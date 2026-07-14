//! Encodes Japanese text using Kanji mode.
//!
//! Kanji mode packs eligible Shift JIS characters more tightly than byte mode, and is behind the `kanji` feature.
//!
//! Run with: `cargo run --example kanji --features kanji`

use qrcode_generator::{
    Renderer,
    qr::{Encoder, ErrorCorrection},
};

fn main() {
    // The text encoder selects Kanji mode automatically when the `kanji` feature is on.
    let symbol = Encoder::new(ErrorCorrection::Medium).encode_text("日本語").unwrap();

    Renderer::new(&symbol, 400).save_svg("kanji.svg", None).unwrap();

    println!("wrote kanji.svg ({} modules per side)", symbol.size());
}
