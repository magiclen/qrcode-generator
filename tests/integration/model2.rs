use std::cell::Cell;

use qrcode_generator::{
    EncodeError, Segment, SymbolVersion, ToQRText,
    qr::{ApplicationIndicator, Encoder, ErrorCorrection, Fnc1, Mask, Version},
};

use super::{
    common::{decode_model2, decode_quircs, decode_square, matrix_rows, render_square},
    vectors::{ANNEX_QR_V1_M, QR_CAPACITIES},
};

const LEVELS: [ErrorCorrection; 4] = [
    ErrorCorrection::Low,
    ErrorCorrection::Medium,
    ErrorCorrection::Quartile,
    ErrorCorrection::High,
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
    for value in 1..=40 {
        let version = Version::new(value).unwrap();

        for error_correction in LEVELS {
            let symbol = Encoder::new(error_correction)
                .version(version)
                .boost_error_correction(false)
                .encode_text("1")
                .unwrap();
            let (image, size) = render_square(&symbol, 4);

            assert_eq!(symbol.version(), SymbolVersion::Qr(version));
            assert_eq!(symbol.error_correction(), error_correction.into());
            assert_eq!(decode_square(image, size), "1", "version {value} {error_correction:?}");
        }
    }
}

// The ISO capacity is the largest input that fits: one more character overflows every mode and level.
#[test]
fn iso_capacity_boundaries_hold() {
    for &(value, ecl, capacities) in QR_CAPACITIES {
        let encoder = Encoder::new(LEVELS[usize::from(ecl)])
            .version(Version::new(value).unwrap())
            .mask(Mask::new(0).unwrap())
            .boost_error_correction(false);

        for (mode, &capacity) in capacities.iter().enumerate() {
            let capacity = usize::from(capacity);

            let Some(fitting) = capacity_segment(mode, capacity) else {
                continue;
            };
            let overflowing = capacity_segment(mode, capacity + 1).unwrap();

            assert!(
                encoder.encode_segments(&[fitting]).is_ok(),
                "v{value} ecl {ecl} mode {mode} should fit {capacity}"
            );
            assert!(
                matches!(
                    encoder.encode_segments(&[overflowing]),
                    Err(EncodeError::DataTooLong { .. })
                ),
                "v{value} ecl {ecl} mode {mode} should reject {}",
                capacity + 1
            );
        }
    }
}

// The version 1-M Annex I reference symbol is reproduced module for module.
#[test]
fn annex_i_v1_m_matrix_matches() {
    let symbol = Encoder::new(ErrorCorrection::Medium)
        .version(Version::new(1).unwrap())
        .mask(Mask::new(2).unwrap())
        .boost_error_correction(false)
        .encode_segments(&[Segment::numeric("01234567").unwrap()])
        .unwrap();

    assert_eq!(matrix_rows(&symbol), ANNEX_QR_V1_M);
}

// Every one of the eight data masks can be forced and still decodes.
#[test]
fn every_forced_mask_round_trips() {
    for mask in 0..8 {
        let symbol = Encoder::new(ErrorCorrection::Medium)
            .mask(Mask::new(mask).unwrap())
            .encode_text("MASK TEST 123")
            .unwrap();
        assert_eq!(symbol.mask(), mask);

        let (image, size) = render_square(&symbol, 6);
        assert_eq!(decode_square(image, size), "MASK TEST 123", "mask {mask}");
    }
}

// The optimizer picks compact modes, so the symbol stays smaller than a plain byte encoding would allow.
#[test]
fn optimizer_chooses_compact_modes() {
    let encoder = Encoder::new(ErrorCorrection::Low).boost_error_correction(false);

    // 100 digits fit version 3 in Numeric mode; as bytes they would need version 5.
    let numeric = encoder.encode_text("1".repeat(100)).unwrap();
    assert_eq!(numeric.version(), SymbolVersion::Qr(Version::new(3).unwrap()));

    // 60 letters fit version 3 in Alphanumeric mode; as bytes they would need version 4.
    let alphanumeric = encoder.encode_text("A".repeat(60)).unwrap();
    assert_eq!(alphanumeric.version(), SymbolVersion::Qr(Version::new(3).unwrap()));
}

// Version 1, 7 and 40 decode through the independent quircs reader as well.
#[test]
fn representative_versions_round_trip_through_quircs() {
    for value in [1, 7, 40] {
        let text = format!("MODEL 2 VERSION {value}");
        let symbol = Encoder::new(ErrorCorrection::Medium)
            .version(Version::new(value).unwrap())
            .boost_error_correction(false)
            .encode_text(&text)
            .unwrap();
        let (image, size) = render_square(&symbol, 5);

        assert_eq!(decode_quircs(&image, size), text.as_bytes());
    }
}

// A near-capacity payload that spans several error correction blocks still decodes.
#[test]
fn near_capacity_multiblock_payload_round_trips() {
    let data = vec![b'a'; 151];
    let symbol = Encoder::new(ErrorCorrection::Quartile)
        .version(Version::new(10).unwrap())
        .boost_error_correction(false)
        .encode_bytes(&data)
        .unwrap();
    let (image, size) = render_square(&symbol, 5);

    assert_eq!(decode_quircs(&image, size), data);
}

// UTF-8 text, raw binary, GS1 FNC1 and explicit segments each round trip through a decoder.
#[test]
fn utf8_binary_fnc1_and_explicit_segments_round_trip() {
    let text = "QR Code 😀 café";
    let symbol = Encoder::new(ErrorCorrection::Medium).encode_text(text).unwrap();
    let (image, size) = render_square(&symbol, 8);
    assert_eq!(decode_square(image, size), text);

    let data = b"binary\0payload\xFF";
    let symbol = Encoder::new(ErrorCorrection::Quartile).encode_bytes(data).unwrap();
    let (image, size) = render_square(&symbol, 8);
    assert_eq!(decode_quircs(&image, size), data);

    let gs1 = b"0101234567890128\x1D10ABC";
    let symbol =
        Encoder::new(ErrorCorrection::Medium).fnc1(Some(Fnc1::Gs1)).encode_bytes(gs1).unwrap();
    let (image, size) = render_square(&symbol, 8);
    assert_eq!(decode_model2(image, size).getText().as_bytes(), gs1);

    Encoder::new(ErrorCorrection::Medium)
        .fnc1(Some(Fnc1::Industry(ApplicationIndicator::numeric(12).unwrap())))
        .encode_bytes(gs1)
        .unwrap();

    let segments = [Segment::numeric("12345").unwrap(), Segment::bytes(b"abc")];
    let symbol = Encoder::new(ErrorCorrection::Low).encode_segments(&segments).unwrap();
    let (image, size) = render_square(&symbol, 8);
    assert_eq!(decode_square(image, size), "12345abc");
}

// Kanji text is stored in Kanji mode and read back unchanged.
#[cfg(feature = "kanji")]
#[test]
fn kanji_mode_round_trips() {
    let text = "点茗日本語";
    let symbol = Encoder::new(ErrorCorrection::Medium).encode_text(text).unwrap();
    let (image, size) = render_square(&symbol, 8);

    assert_eq!(decode_model2(image, size).getText(), text);
}

// Text mixing UTF-8 and Shift JIS data declares each interpretation explicitly and still decodes.
#[cfg(feature = "kanji")]
#[test]
fn mixed_utf8_and_kanji_text_round_trips() {
    let text = "😀日本語のテスト ﾃｽﾄ ¥100";
    let symbol = Encoder::new(ErrorCorrection::Medium).encode_text(text).unwrap();
    let (image, size) = render_square(&symbol, 8);

    assert_eq!(decode_square(image, size), text);
}

// A pure Kanji Structured Append sequence keeps Kanji mode and reassembles to the whole message.
#[cfg(feature = "kanji")]
#[test]
fn structured_append_kanji_parts_round_trip() {
    let parts = ["日本語構造的連接", "試験一二三四五"];
    let symbols =
        Encoder::new(ErrorCorrection::Medium).encode_structured_append_text(&parts).unwrap();
    let parity = symbols[0].structured_append().unwrap().parity();

    for (symbol, expected) in symbols.iter().zip(parts) {
        assert_eq!(symbol.structured_append().unwrap().parity(), parity);

        let (image, size) = render_square(symbol, 8);
        assert_eq!(decode_square(image, size), expected);
    }
}

// A Structured Append sequence shares one parity byte and reassembles to the whole message.
#[test]
fn structured_append_metadata_and_payloads_round_trip() {
    let parts = ["FIRST", "SECOND"];
    let symbols =
        Encoder::new(ErrorCorrection::Medium).encode_structured_append_text(&parts).unwrap();

    for (index, (symbol, expected)) in symbols.iter().zip(parts).enumerate() {
        let info = symbol.structured_append().unwrap();
        assert_eq!((info.index(), info.total()), (index as u8, 2));
        assert_eq!(info.parity(), symbols[0].structured_append().unwrap().parity());

        let (image, size) = render_square(symbol, 8);
        assert_eq!(decode_model2(image, size).getText(), expected);
    }

    let data = vec![b'x'; 80];
    let symbols = Encoder::new(ErrorCorrection::Low)
        .version_range(Version::new(1).unwrap()..=Version::new(2).unwrap())
        .encode_bytes_with_structured_append(&data)
        .unwrap();
    let mut decoded = Vec::new();

    for symbol in symbols {
        let (image, size) = render_square(&symbol, 8);
        decoded.extend_from_slice(decode_model2(image, size).getText().as_bytes());
    }
    assert_eq!(decoded, data);
}

// A Structured Append parity derived from optimized UTF-8 parts stays consistent and decodes.
#[test]
fn structured_append_text_with_eci_round_trips() {
    let parts = ["café ☕", "second 12345"];
    let symbols =
        Encoder::new(ErrorCorrection::Medium).encode_structured_append_text(&parts).unwrap();
    let parity = symbols[0].structured_append().unwrap().parity();

    for (symbol, expected) in symbols.iter().zip(parts) {
        assert_eq!(symbol.structured_append().unwrap().parity(), parity);

        let (image, size) = render_square(symbol, 8);
        assert_eq!(decode_square(image, size), expected);
    }
}

// A custom ToQRText value is converted exactly once and its shorter spelling is encoded.
#[test]
fn qr_text_conversion_is_called_once() {
    struct NormalizedText<'a> {
        calls: Cell<usize>,
        text:  &'a str,
    }

    impl ToQRText for NormalizedText<'_> {
        fn to_qr_text(&self) -> String {
            self.calls.set(self.calls.get() + 1);
            self.text.to_ascii_uppercase()
        }
    }

    let value = NormalizedText {
        calls: Cell::new(0), text: "custom qr text 123"
    };
    let symbol = Encoder::new(ErrorCorrection::Medium).encode_to_qr_text(&value).unwrap();
    let (image, size) = render_square(&symbol, 8);

    assert_eq!(value.calls.get(), 1);
    assert_eq!(decode_square(image, size), "CUSTOM QR TEXT 123");
}

// Owned and borrowed inputs produce identical symbols, confirming the generic argument bounds.
#[test]
fn public_inputs_accept_owned_and_borrowed_forms_without_changing_symbols() {
    let encoder = Encoder::new(ErrorCorrection::Low).boost_error_correction(false);
    let text = String::from("HELLO 123");
    assert_eq!(encoder.encode_text(text.clone()).unwrap(), encoder.encode_text(&text).unwrap());

    let bytes = vec![0x12, 0x34, 0x56];
    assert_eq!(encoder.encode_bytes(bytes.clone()).unwrap(), encoder.encode_bytes(&bytes).unwrap());
    assert_eq!(Segment::numeric(String::from("123")).unwrap(), Segment::numeric("123").unwrap());
    let borrowed: &[u8] = &[1, 2, 3];
    assert_eq!(Segment::bytes([1, 2, 3]), Segment::bytes(borrowed));
}
