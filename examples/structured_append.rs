//! Splits a long payload into a Structured Append sequence of symbols.
//!
//! A reader that supports Structured Append recombines them into the original message.
//!
//! Run with: `cargo run --example structured_append`

use qrcode_generator::{
    Renderer,
    qr::{Encoder, ErrorCorrection, Version},
};

fn main() {
    let text = "Structured Append lets several small symbols carry one long message that a reader \
                stitches back together again.";

    // A small version range forces the message to span more than one symbol.
    let symbols = Encoder::new(ErrorCorrection::Low)
        .version_range(Version::new(1).unwrap()..=Version::new(2).unwrap())
        .encode_text_with_structured_append(text)
        .unwrap();

    for (index, symbol) in symbols.iter().enumerate() {
        let path = format!("structured_append_{index}_output.svg");

        Renderer::new(symbol, 400).save_svg(&path, None::<&str>).unwrap();

        println!("wrote {path} ({} modules per side)", symbol.size());
    }

    println!("{} symbols in the sequence", symbols.len());
}
