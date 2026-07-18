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
    // The quiet zone is the white border around the symbol. It is required for scanners to detect the symbol. Here we disable it to show the symbol's exact size in modules.
    // The output image will be 512 by 512 pixels, so each module will be 512 / symbol.size() pixels wide and tall. There are still 4 pixels of white border around the symbol because the symbol is centered in the image.
    let renderer = Renderer::new(&symbol, 512).quiet_zone(0);

    renderer.save_svg("basic_output.svg", None::<&str>).unwrap();
    renderer.save_png("basic_output.png").unwrap();

    println!("wrote basic_output.svg and basic_output.png ({} modules per side)", symbol.size());
}
