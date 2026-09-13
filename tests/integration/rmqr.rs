use qrcode_generator::{
    EncodeError, Segment, SymbolErrorCorrection, SymbolVersion,
    rmqr::{ApplicationIndicator, EciAssignment, Encoder, ErrorCorrection, Fnc1, Version},
};
use rxing::BarcodeFormat;

use super::{
    common::{decode_rmqr_detected, decode_rmqr_pure, matrix_rows, render_rmqr},
    vectors::{ANNEX_RMQR_R11X27_H, RMQR_CAPACITIES},
};

const VERSIONS: [Version; 32] = [
    Version::R7x43,
    Version::R7x59,
    Version::R7x77,
    Version::R7x99,
    Version::R7x139,
    Version::R9x43,
    Version::R9x59,
    Version::R9x77,
    Version::R9x99,
    Version::R9x139,
    Version::R11x27,
    Version::R11x43,
    Version::R11x59,
    Version::R11x77,
    Version::R11x99,
    Version::R11x139,
    Version::R13x27,
    Version::R13x43,
    Version::R13x59,
    Version::R13x77,
    Version::R13x99,
    Version::R13x139,
    Version::R15x43,
    Version::R15x59,
    Version::R15x77,
    Version::R15x99,
    Version::R15x139,
    Version::R17x43,
    Version::R17x59,
    Version::R17x77,
    Version::R17x99,
    Version::R17x139,
];

// Builds a single-mode segment of `count` characters for the capacity boundary test.
fn capacity_segment(mode: usize, count: usize) -> Option<Segment> {
    match mode {
        0 => Some(Segment::numeric("1".repeat(count)).unwrap()),
        1 => Some(Segment::alphanumeric("A".repeat(count)).unwrap()),
        2 => Some(Segment::bytes(vec![b'a'; count])),
        #[cfg(feature = "kanji")]
        3 => Some(Segment::kanji("点".repeat(count)).unwrap()),
        _ => None,
    }
}

// Every version and error correction level encodes and decodes back to the original text.
#[test]
fn every_version_and_error_correction_level_round_trips() {
    for version in VERSIONS {
        for error_correction in [ErrorCorrection::Medium, ErrorCorrection::High] {
            let symbol = Encoder::new(error_correction)
                .version(version)
                .boost_error_correction(false)
                .encode_text("1")
                .unwrap();
            let (image, width, height) = render_rmqr(&symbol, 8);
            let context = format!("{version:?} {error_correction:?}");
            let decoded = decode_rmqr_pure(image, width, height, &context);

            assert_eq!(SymbolVersion::Rmqr(version), symbol.version());
            assert_eq!(SymbolErrorCorrection::from(error_correction), symbol.error_correction());
            assert_eq!("1", decoded.getText(), "{context}");
            assert_eq!(&BarcodeFormat::RECTANGULAR_MICRO_QR_CODE, decoded.getBarcodeFormat());
        }
    }
}

// The ISO capacity is the largest input that fits: one more character overflows every mode and level.
#[test]
fn iso_capacity_boundaries_hold() {
    for (index, version) in VERSIONS.into_iter().enumerate() {
        for (level, error_correction) in
            [ErrorCorrection::Medium, ErrorCorrection::High].into_iter().enumerate()
        {
            let encoder =
                Encoder::new(error_correction).version(version).boost_error_correction(false);

            for (mode, &capacity) in RMQR_CAPACITIES[index][level].iter().enumerate() {
                let capacity = usize::from(capacity);

                let Some(fitting) = capacity_segment(mode, capacity) else {
                    continue;
                };
                let overflowing = capacity_segment(mode, capacity + 1).unwrap();

                assert!(
                    encoder.encode_segments(&[fitting]).is_ok(),
                    "{version:?} {error_correction:?} mode {mode} should fit {capacity}"
                );
                assert!(
                    matches!(
                        encoder.encode_segments(&[overflowing]),
                        Err(EncodeError::DataTooLong { .. })
                    ),
                    "{version:?} {error_correction:?} mode {mode} should reject {}",
                    capacity + 1
                );
            }
        }
    }
}

// The R11x27-H Annex I reference symbol is reproduced module for module.
#[test]
fn annex_i_r11x27_h_matrix_matches() {
    let symbol = Encoder::new(ErrorCorrection::High)
        .version(Version::R11x27)
        .boost_error_correction(false)
        .encode_segments(&[Segment::numeric("0123456").unwrap()])
        .unwrap();

    assert_eq!(ANNEX_RMQR_R11X27_H.as_slice(), matrix_rows(&symbol));
}

// A representative symbol is found by the general reader without pure-barcode hints.
#[test]
fn representative_symbol_is_detected_without_pure_barcode_hints() {
    let symbol = Encoder::new(ErrorCorrection::High)
        .version(Version::R11x27)
        .boost_error_correction(false)
        .encode_text("0123456")
        .unwrap();
    let (image, width, height) = render_rmqr(&symbol, 10);
    let decoded = decode_rmqr_detected(image, width, height);

    assert_eq!("0123456", decoded.getText());
    assert_eq!(&BarcodeFormat::RECTANGULAR_MICRO_QR_CODE, decoded.getBarcodeFormat());
}

// A near-capacity payload that spans several error correction blocks still decodes.
#[test]
fn near_capacity_multiblock_payload_round_trips() {
    let data = vec![b'a'; 74];
    let symbol = Encoder::new(ErrorCorrection::High)
        .version(Version::R17x139)
        .boost_error_correction(false)
        .encode_bytes(&data)
        .unwrap();
    let (image, width, height) = render_rmqr(&symbol, 6);

    assert_eq!(
        data,
        decode_rmqr_pure(image, width, height, "near-capacity R17x139-H").getText().as_bytes()
    );
}

// Automatic version selection, UTF-8 ECI, FNC1 and explicit segments each round trip through a decoder.
#[test]
fn automatic_selection_eci_fnc1_and_explicit_segments_round_trip() {
    let symbol = Encoder::new(ErrorCorrection::Medium).encode_text("1").unwrap();
    assert_eq!(SymbolVersion::Rmqr(Version::R11x27), symbol.version());

    let text = "rMQR 😀";
    let symbol = Encoder::new(ErrorCorrection::Medium).encode_text(text).unwrap();
    let (image, width, height) = render_rmqr(&symbol, 8);
    assert_eq!(text, decode_rmqr_pure(image, width, height, "UTF-8 ECI").getText());

    let data = b"0101234567890128\x1D10ABC";
    let symbol =
        Encoder::new(ErrorCorrection::Medium).fnc1(Some(Fnc1::Gs1)).encode_bytes(data).unwrap();
    let (image, width, height) = render_rmqr(&symbol, 8);
    assert_eq!(
        data.as_slice(),
        decode_rmqr_pure(image, width, height, "FNC1").getText().as_bytes()
    );

    Encoder::new(ErrorCorrection::Medium)
        .fnc1(Some(Fnc1::Industry(ApplicationIndicator::numeric(12).unwrap())))
        .encode_bytes(data)
        .unwrap();

    let segments = [Segment::numeric("12345").unwrap(), Segment::bytes(b"abc")];
    let symbol = Encoder::new(ErrorCorrection::Medium).encode_segments(&segments).unwrap();
    let (image, width, height) = render_rmqr(&symbol, 8);
    assert_eq!("12345abc", decode_rmqr_pure(image, width, height, "explicit segments").getText());

    let segments = [Segment::eci(EciAssignment::UTF_8), Segment::bytes("é".as_bytes())];
    let symbol = Encoder::new(ErrorCorrection::Medium).encode_segments(&segments).unwrap();
    let (image, width, height) = render_rmqr(&symbol, 8);
    assert_eq!("é", decode_rmqr_pure(image, width, height, "explicit ECI").getText());
}

// Kanji text is stored in Kanji mode and read back unchanged.
#[cfg(feature = "kanji")]
#[test]
fn kanji_mode_round_trips() {
    let text = "点茗日本語";
    let symbol = Encoder::new(ErrorCorrection::Medium).encode_text(text).unwrap();
    let (image, width, height) = render_rmqr(&symbol, 8);

    assert_eq!(text, decode_rmqr_pure(image, width, height, "Kanji").getText());

    for text in ["−", "－", "−日本語", "−ﾃｽﾄ"] {
        let symbol = Encoder::new(ErrorCorrection::Medium).encode_text(text).unwrap();
        let (image, width, height) = render_rmqr(&symbol, 8);
        assert_eq!(text, decode_rmqr_pure(image, width, height, text).getText());
    }
}

// Text mixing UTF-8 and Shift JIS data declares each interpretation explicitly and still decodes.
#[cfg(feature = "kanji")]
#[test]
fn mixed_utf8_and_kanji_text_round_trips() {
    let text = "😀日本語テスト";
    let symbol = Encoder::new(ErrorCorrection::Medium).encode_text(text).unwrap();
    let (image, width, height) = render_rmqr(&symbol, 8);

    assert_eq!(text, decode_rmqr_pure(image, width, height, "mixed UTF-8 and Kanji").getText());
}

// Owned and borrowed inputs produce identical symbols, confirming the generic argument bounds.
#[test]
fn public_inputs_accept_owned_and_borrowed_forms_without_changing_symbols() {
    let encoder = Encoder::new(ErrorCorrection::Medium).boost_error_correction(false);
    let text = String::from("HELLO 123");
    assert_eq!(encoder.encode_text(text.clone()).unwrap(), encoder.encode_text(&text).unwrap());

    let bytes = vec![1, 2, 3];
    assert_eq!(encoder.encode_bytes(bytes.clone()).unwrap(), encoder.encode_bytes(&bytes).unwrap());
}
