use qrcode_generator::ToQRText;
#[cfg(feature = "qr")]
use qrcode_generator::qr::{Encoder, ErrorCorrection};
use url::Url;

#[cfg(feature = "qr")]
use super::common::{decode_square, render_square};

// URL normalization upper-cases only the case-insensitive scheme, host and percent escapes.
#[test]
fn normalizes_only_equivalent_url_parts() {
    for (input, expected) in [
        (
            "https://User:Pass@magiclen.org/%7eCase?x=%ab#Part",
            "HTTPS://User:Pass@MAGICLEN.ORG/%7ECase?x=%AB#Part",
        ),
        ("https://magiclen.org", "HTTPS://MAGICLEN.ORG"),
        ("mailto:User%2f@example.com", "MAILTO:User%2F@example.com"),
    ] {
        let url = Url::parse(input).unwrap();
        assert_eq!(url.to_qr_text(), expected);
    }
}

// A url::Url is encoded directly through ToQRText and decodes to its normalized form.
#[cfg(feature = "qr")]
#[test]
fn url_can_be_encoded_directly() {
    let url = Url::parse("https://magiclen.org").unwrap();
    let symbol = Encoder::new(ErrorCorrection::Medium).encode_to_qr_text(&url).unwrap();
    let (image, size) = render_square(&symbol, 8);

    assert_eq!(decode_square(image, size), "HTTPS://MAGICLEN.ORG");
}
