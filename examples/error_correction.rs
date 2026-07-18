//! Shows how the error correction level changes the symbol size.
//!
//! A higher level survives more damage but leaves less room for data, so the same text may need a larger version.
//!
//! Run with: `cargo run --example error_correction`

use qrcode_generator::{
    Renderer,
    qr::{Encoder, ErrorCorrection},
};

fn main() {
    let text = "The quick brown fox jumps over the lazy dog.";

    for level in [
        ErrorCorrection::Low,
        ErrorCorrection::Medium,
        ErrorCorrection::Quartile,
        ErrorCorrection::High,
    ] {
        // boost_error_correction is disabled so each symbol keeps the requested level.
        let symbol = Encoder::new(level).boost_error_correction(false).encode_text(text).unwrap();

        let level = match level {
            ErrorCorrection::Low => "L",
            ErrorCorrection::Medium => "M",
            ErrorCorrection::Quartile => "Q",
            ErrorCorrection::High => "H",
        };

        let file_name = format!("error_correction_{level}_output.svg");

        Renderer::new(&symbol, 512).save_svg(&file_name, None::<&str>).unwrap();

        println!("wrote {file_name} ({level:?}: {size} modules per side)", size = symbol.size());
    }
}
