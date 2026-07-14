//! Shows how the error correction level changes the symbol size.
//!
//! A higher level survives more damage but leaves less room for data, so the same text may need a larger version.
//!
//! Run with: `cargo run --example error_correction`

use qrcode_generator::qr::{Encoder, ErrorCorrection};

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

        println!("{level:?}: {size} x {size} modules", size = symbol.size());
    }
}
