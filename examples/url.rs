//! Normalizes a URL before encoding it, using the ToQRText trait.
//!
//! Case-insensitive URL parts are upper-cased so the QR Code can use the more compact alphanumeric mode, without changing what the URL means.
//!
//! Run with: `cargo run --example url -- "https://example.com/path"`

use std::{env, error::Error};

use qrcode_generator::{
    Renderer, ToQRText,
    qr::{Encoder, ErrorCorrection},
};
use url::{Position, Url};

struct QRUrl(Url);

impl ToQRText for QRUrl {
    fn to_qr_text(&self) -> String {
        let url = &self.0;
        let serialized = url.as_str();
        let scheme_end = url[..Position::AfterScheme].len();
        let path_start = url[..Position::BeforePath].len();
        let path_end = url[..Position::AfterPath].len();
        let omit_root_path = matches!(url.scheme(), "http" | "https") && url.path() == "/";
        let mut normalized = String::with_capacity(serialized.len());

        normalized.push_str(&serialized[..scheme_end].to_ascii_uppercase());

        // URL hosts are case-insensitive, but user information and path data may not be.
        if url.has_host() {
            let host_start = url[..Position::BeforeHost].len();
            let host_end = url[..Position::AfterHost].len();

            normalized.push_str(&serialized[scheme_end..host_start]);
            normalized.push_str(&serialized[host_start..host_end].to_ascii_uppercase());
            normalized.push_str(&serialized[host_end..path_start]);
        } else {
            normalized.push_str(&serialized[scheme_end..path_start]);
        }

        if !omit_root_path {
            normalized.push_str(&serialized[path_start..path_end]);
        }

        normalized.push_str(&serialized[path_end..]);

        uppercase_percent_escapes(&normalized)
    }
}

fn uppercase_percent_escapes(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut normalized = String::with_capacity(text.len());
    let mut copied = 0;
    let mut index = 0;

    // Only complete percent-encoded octets can be changed without changing URL data.
    while index + 2 < bytes.len() {
        if bytes[index] == b'%'
            && bytes[index + 1].is_ascii_hexdigit()
            && bytes[index + 2].is_ascii_hexdigit()
        {
            normalized.push_str(&text[copied..index]);
            normalized.push('%');
            normalized.push(char::from(bytes[index + 1].to_ascii_uppercase()));
            normalized.push(char::from(bytes[index + 2].to_ascii_uppercase()));

            index += 3;

            copied = index;
        } else {
            index += 1;
        }
    }

    normalized.push_str(&text[copied..]);

    normalized
}

fn main() -> Result<(), Box<dyn Error>> {
    let input = env::args()
        .nth(1)
        .unwrap_or_else(|| "https://example.com/%2fMixed/Path?key=%aaValue".to_owned());

    let url = QRUrl(Url::parse(&input)?);
    let qr_text = url.to_qr_text();
    let symbol = Encoder::new(ErrorCorrection::Medium).encode_to_qr_text(&url)?;

    Renderer::new(&symbol, 1024).save_svg("url.svg", Some(&qr_text))?;

    println!("Encoded {qr_text} as url.svg");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn normalize(input: &str) -> String {
        QRUrl(Url::parse(input).unwrap()).to_qr_text()
    }

    #[test]
    fn normalizes_hierarchical_urls() {
        assert_eq!(
            normalize("https://User:Pass@example.com/%2fMixed/Path?key=%aaValue#Fragment"),
            "HTTPS://User:Pass@EXAMPLE.COM/%2FMixed/Path?key=%AAValue#Fragment"
        );
        assert_eq!(normalize("https://example.com/"), "HTTPS://EXAMPLE.COM");
        assert_eq!(normalize("http://[2001:db8::1]/"), "HTTP://[2001:DB8::1]");
    }

    #[test]
    fn normalizes_opaque_urls() {
        assert_eq!(normalize("mailto:User%2f@example.com"), "MAILTO:User%2F@example.com");
    }
}
