//! Encodes Japanese text using Kanji mode.
//!
//! Kanji mode packs eligible Shift JIS characters more tightly than byte mode, and is behind the `kanji` feature.
//!
//! Run with: `cargo run --example kanji --features kanji`
//! Or Run with: `cargo run --example kanji`

use qrcode_generator::{
    Renderer,
    qr::{Encoder, ErrorCorrection},
};

fn main() {
    // The text encoder selects Kanji mode automatically when the `kanji` feature is on.
    let symbol = Encoder::new(ErrorCorrection::Medium).encode_text("日本語　あいうえお").unwrap();

    let file_name = if cfg!(feature = "kanji") { "kanji_output.svg" } else { "utf8_output.svg" };

    Renderer::new(&symbol, 400).save_svg(file_name, None::<&str>).unwrap();

    println!("wrote {file_name} ({} modules per side)", symbol.size());
}
