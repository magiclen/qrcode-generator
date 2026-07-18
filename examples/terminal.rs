//! Prints a QR Code as Unicode text in the terminal.
//!
//! The renderer implements `Display` with half block characters that pack two module rows into every line, including the quiet zone.
//! The default form draws dark modules as visible blocks for light backgrounds, and the alternate form draws light modules instead for dark backgrounds.
//!
//! Run with: `cargo run --example terminal`

use qrcode_generator::{
    Renderer,
    qr::{Encoder, ErrorCorrection},
};

fn main() {
    let symbol = Encoder::new(ErrorCorrection::Low).encode_text("https://magiclen.org").unwrap();

    // Text output works in module units, so the pixel size passed to the renderer does not affect it.
    let renderer = Renderer::new(&symbol, 0);

    println!("For light terminal backgrounds:");
    print!("{renderer}");

    println!("For dark terminal backgrounds:");
    print!("{renderer:#}");
}
