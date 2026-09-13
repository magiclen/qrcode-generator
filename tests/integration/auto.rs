use qrcode_generator::{
    AutoEncoder, Segment, SymbolVersion,
    micro::{Encoder as MicroEncoder, ErrorCorrection as MicroErrorCorrection},
    qr::{ApplicationIndicator, Encoder as QrEncoder, ErrorCorrection as QrErrorCorrection, Fnc1},
};

use super::common::{decode_quircs, decode_square, render_square};

// FNC1 data skips the Micro QR candidate entirely, because Micro QR has no FNC1 header.
#[test]
fn fnc1_uses_the_configured_model2_encoder() {
    for fnc1 in [Fnc1::Gs1, Fnc1::Industry(ApplicationIndicator::numeric(12).unwrap())] {
        let qr = QrEncoder::new(QrErrorCorrection::Medium).fnc1(Some(fnc1));
        let encoder = AutoEncoder::new(qr.clone(), MicroEncoder::new(MicroErrorCorrection::Low));
        let text = "12345";
        let segments = [Segment::numeric(text).unwrap()];

        assert_eq!(qr.encode_text(text).unwrap(), encoder.encode_text(text).unwrap());
        assert_eq!(qr.encode_bytes(text).unwrap(), encoder.encode_bytes(text).unwrap());
        assert_eq!(
            qr.encode_segments(&segments).unwrap(),
            encoder.encode_segments(&segments).unwrap()
        );
    }
}

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
    assert_eq!(micro_text, decode_square(image, size));

    let qr_text = "This payload is intentionally too long for every Micro QR Code version.";
    let qr_symbol = encoder.encode_text(qr_text).unwrap();
    assert!(matches!(qr_symbol.version(), SymbolVersion::Qr(_)));
    let (image, size) = render_square(&qr_symbol, 8);
    assert_eq!(qr_text.as_bytes(), decode_quircs(&image, size));

    for text in ["😀日本語".repeat(30), "日本語".repeat(100)] {
        let symbol = encoder.encode_text(&text).unwrap();
        assert!(matches!(symbol.version(), SymbolVersion::Qr(_)));
        let (image, size) = render_square(&symbol, 8);
        assert_eq!(text, decode_square(image, size));
    }
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
