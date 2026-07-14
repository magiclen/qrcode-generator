//! Encodes explicit segments to control exactly how the data is split.
//!
//! encode_text and encode_bytes choose segments automatically; encode_segments keeps the boundaries you build.
//!
//! Run with: `cargo run --example segments`

use qrcode_generator::{
    Renderer, Segment,
    qr::{Encoder, ErrorCorrection},
};

fn main() {
    let segments =
        [Segment::numeric("1234567").unwrap(), Segment::alphanumeric("ABCDEFG").unwrap()];

    let symbol = Encoder::new(ErrorCorrection::Low).encode_segments(&segments).unwrap();

    Renderer::new(&symbol, 512).save_svg("segments.svg", None).unwrap();

    println!("wrote segments.svg ({} modules per side)", symbol.size());
}
