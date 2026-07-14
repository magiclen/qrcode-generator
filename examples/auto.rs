//! Lets AutoEncoder pick the smallest symbol family for the data.
//!
//! It tries Micro QR Code first and falls back to a full QR Code when the data does not fit.
//!
//! Run with: `cargo run --example auto --features qr,micro-qr`

use qrcode_generator::{AutoEncoder, Renderer, SymbolVersion, micro, qr};

fn main() {
    let encoder = AutoEncoder::new(
        qr::Encoder::new(qr::ErrorCorrection::Medium),
        micro::Encoder::new(micro::ErrorCorrection::Medium),
    );

    for text in ["HELLO 123", "This text is far too long for any Micro QR Code version."] {
        let symbol = encoder.encode_text(text).unwrap();

        let family = match symbol.version() {
            SymbolVersion::Qr(_) => "QR Code",
            SymbolVersion::Micro(_) => "Micro QR Code",
            #[cfg(feature = "rmqr")]
            SymbolVersion::Rmqr(_) => "rMQR Code",
            _ => unreachable!("AutoEncoder returns only Model 2 or Micro QR Code symbols"),
        };

        let file_name = format!("auto_{}_output.svg", family.replace(' ', "_").to_lowercase());

        Renderer::new(&symbol, 300).save_svg(&file_name, None::<&str>).unwrap();

        println!("wrote {file_name} ({family:?}: {size} modules per side)", size = symbol.size());
    }
}
