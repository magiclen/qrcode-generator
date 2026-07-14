use qrcode_generator::{
    AutoEncoder, SymbolVersion,
    micro::{Encoder as MicroEncoder, ErrorCorrection as MicroErrorCorrection},
    qr::{Encoder as QrEncoder, ErrorCorrection as QrErrorCorrection},
};

use super::common::{decode_quircs, decode_square, render_square};

// Short data uses a Micro QR symbol, while data too long for Micro falls back to Model 2.
#[test]
fn selects_micro_then_falls_back_to_model2() {
    let encoder = AutoEncoder::new(
        QrEncoder::new(QrErrorCorrection::Medium).boost_error_correction(false),
        MicroEncoder::new(MicroErrorCorrection::Medium).boost_error_correction(false),
    );

    let micro_text = "HELLO 123";
    let micro_symbol = encoder.encode_text(micro_text).unwrap();
    assert!(matches!(micro_symbol.version(), SymbolVersion::Micro(_)));
    let (image, size) = render_square(&micro_symbol, 10);
    assert_eq!(decode_square(image, size), micro_text);

    let qr_text = "This payload is intentionally too long for every Micro QR Code version.";
    let qr_symbol = encoder.encode_text(qr_text).unwrap();
    assert!(matches!(qr_symbol.version(), SymbolVersion::Qr(_)));
    let (image, size) = render_square(&qr_symbol, 8);
    assert_eq!(decode_quircs(&image, size), qr_text.as_bytes());
}

// Owned and borrowed inputs produce identical symbols through the automatic encoder.
#[test]
fn accepts_owned_text_and_bytes() {
    let encoder = AutoEncoder::new(
        QrEncoder::new(QrErrorCorrection::Low),
        MicroEncoder::new(MicroErrorCorrection::Low),
    );

    assert_eq!(
        encoder.encode_text(String::from("12345")).unwrap(),
        encoder.encode_text("12345").unwrap()
    );
    let borrowed: &[u8] = &[1, 2, 3];
    assert_eq!(
        encoder.encode_bytes(vec![1, 2, 3]).unwrap(),
        encoder.encode_bytes(borrowed).unwrap()
    );
}
