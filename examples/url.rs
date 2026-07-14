//! Normalizes a URL before encoding it, using the ToQRText trait.
//!
//! Case-insensitive URL parts are upper-cased so the QR Code can use the more compact alphanumeric mode, without changing what the URL means.
//!
//! Run with: `cargo run --example url --features url`

use qrcode_generator::{
    Renderer,
    qr::{Encoder, ErrorCorrection},
};
use url::Url;

fn main() {
    let url_str = "https://magiclen.org";

    {
        let symbol = Encoder::new(ErrorCorrection::Medium).encode_text(url_str).unwrap();

        Renderer::new(&symbol, 1024).save_svg("url_original_output.svg", None::<&str>).unwrap();

        println!("wrote url_original_output.svg ({} modules per side)", symbol.size());
    }

    {
        let url = Url::parse("https://magiclen.org").unwrap();

        let symbol = Encoder::new(ErrorCorrection::Medium).encode_to_qr_text(&url).unwrap();

        Renderer::new(&symbol, 1024).save_svg("url_qrtext_output.svg", None::<&str>).unwrap();

        println!("wrote url_qrtext_output.svg ({} modules per side)", symbol.size());
    }
}
