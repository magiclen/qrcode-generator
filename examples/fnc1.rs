//! Encodes GS1 and industry data by turning on FNC1.
//!
//! FNC1 marks a symbol as following a formatted-data standard instead of plain text.
//! First position selects GS1; second position selects an industry application indicator.
//!
//! Run with: `cargo run --example fnc1`

use qrcode_generator::{
    Renderer,
    qr::{ApplicationIndicator, Encoder, ErrorCorrection, Fnc1},
};

fn main() {
    // GS1 uses FNC1 in the first position and separates variable-length element strings with a 0x1D (GS) byte.
    // Here AI 01 is a GTIN, then a separator, then AI 10 is a batch number.
    let gs1 = b"0101234567890128\x1D10ABC";

    let symbol =
        Encoder::new(ErrorCorrection::Medium).fnc1(Some(Fnc1::Gs1)).encode_bytes(gs1).unwrap();

    Renderer::new(&symbol, 512).save_svg("fnc1_gs1_output.svg", None::<&str>).unwrap();

    // Industry data uses FNC1 in the second position with an application indicator.
    let indicator = ApplicationIndicator::numeric(12).unwrap();

    let symbol = Encoder::new(ErrorCorrection::Medium)
        .fnc1(Some(Fnc1::Industry(indicator)))
        .encode_bytes(b"1234ABC")
        .unwrap();

    Renderer::new(&symbol, 512).save_svg("fnc1_industry_output.svg", None::<&str>).unwrap();

    println!("wrote fnc1_gs1_output.svg and fnc1_industry_output.svg");
}
