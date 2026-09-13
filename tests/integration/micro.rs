use qrcode_generator::{
    EncodeError, Segment, SymbolErrorCorrection, SymbolVersion,
    micro::{Encoder, ErrorCorrection, Mask, Version},
};

use super::{
    common::{decode_square, matrix_rows, render_square},
    vectors::{ANNEX_MICRO_M2_L, MICRO_CAPACITIES},
};

const VERSIONS: [Version; 4] = [Version::M1, Version::M2, Version::M3, Version::M4];
const LEVELS: [ErrorCorrection; 4] = [
    ErrorCorrection::DetectionOnly,
    ErrorCorrection::Low,
    ErrorCorrection::Medium,
    ErrorCorrection::Quartile,
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

// Every legal version and error correction pair encodes and decodes back to the original text.
#[test]
fn every_legal_version_and_error_correction_pair_round_trips() {
    for (version, error_correction, text) in [
        (Version::M1, ErrorCorrection::DetectionOnly, "12345"),
        (Version::M2, ErrorCorrection::Low, "01234567"),
        (Version::M2, ErrorCorrection::Medium, "01234567"),
        (Version::M3, ErrorCorrection::Low, "HELLO 123"),
        (Version::M3, ErrorCorrection::Medium, "HELLO 123"),
        (Version::M4, ErrorCorrection::Low, "HELLO 123"),
        (Version::M4, ErrorCorrection::Medium, "HELLO 123"),
        (Version::M4, ErrorCorrection::Quartile, "HELLO 123"),
    ] {
        let symbol = Encoder::new(error_correction)
            .version(version)
            .boost_error_correction(false)
            .encode_text(text)
            .unwrap();
        let (image, size) = render_square(&symbol, 10);

        assert_eq!(SymbolVersion::Micro(version), symbol.version());
        assert_eq!(SymbolErrorCorrection::from(error_correction), symbol.error_correction());
        assert_eq!(text, decode_square(image, size), "{version:?} {error_correction:?}");
    }
}

// The ISO capacity is the largest input that fits: one more character overflows every supported mode.
#[test]
fn iso_capacity_boundaries_hold() {
    for &(value, ecl, capacities) in MICRO_CAPACITIES {
        let version = VERSIONS[usize::from(value) - 1];
        let encoder = Encoder::new(LEVELS[usize::from(ecl)])
            .version(version)
            .mask(Mask::new(0).unwrap())
            .boost_error_correction(false);

        for (mode, &capacity) in capacities.iter().enumerate() {
            if capacity == 0 {
                continue;
            }
            let capacity = usize::from(capacity);

            let Some(fitting) = capacity_segment(mode, capacity) else {
                continue;
            };
            let overflowing = capacity_segment(mode, capacity + 1).unwrap();

            assert!(
                encoder.encode_segments(&[fitting]).is_ok(),
                "{version:?} ecl {ecl} mode {mode} should fit {capacity}"
            );
            assert!(
                matches!(
                    encoder.encode_segments(&[overflowing]),
                    Err(EncodeError::DataTooLong { .. })
                ),
                "{version:?} ecl {ecl} mode {mode} should reject {}",
                capacity + 1
            );
        }
    }
}

// The M2-L Annex I reference symbol is reproduced module for module.
#[test]
fn annex_i_m2_l_matrix_matches() {
    let symbol = Encoder::new(ErrorCorrection::Low)
        .version(Version::M2)
        .mask(Mask::new(1).unwrap())
        .boost_error_correction(false)
        .encode_segments(&[Segment::numeric("01234567").unwrap()])
        .unwrap();

    assert_eq!(ANNEX_MICRO_M2_L.as_slice(), matrix_rows(&symbol));
}

// Every one of the four data masks can be forced and still decodes.
#[test]
fn every_forced_mask_round_trips() {
    for mask in 0..4 {
        let symbol = Encoder::new(ErrorCorrection::Low)
            .version(Version::M4)
            .mask(Mask::new(mask).unwrap())
            .encode_text("MICRO 12")
            .unwrap();
        assert_eq!(mask, symbol.mask());

        let (image, size) = render_square(&symbol, 10);
        assert_eq!("MICRO 12", decode_square(image, size), "mask {mask}");
    }
}

// A near-capacity payload for the largest version still decodes.
#[test]
fn near_capacity_payload_round_trips() {
    let text = "1".repeat(21);
    let symbol = Encoder::new(ErrorCorrection::Quartile)
        .version(Version::M4)
        .boost_error_correction(false)
        .encode_text(&text)
        .unwrap();
    let (image, size) = render_square(&symbol, 10);

    assert_eq!(text, decode_square(image, size));
}

// Kanji text is stored in Kanji mode and read back unchanged.
#[cfg(feature = "kanji")]
#[test]
fn kanji_mode_round_trips() {
    let text = "点茗";
    let symbol = Encoder::new(ErrorCorrection::Low).version(Version::M3).encode_text(text).unwrap();
    let (image, size) = render_square(&symbol, 10);

    assert_eq!(text, decode_square(image, size));

    let encoder = Encoder::new(ErrorCorrection::Low).version(Version::M3);
    let symbol = encoder.encode_text("－").unwrap();
    let (image, size) = render_square(&symbol, 10);
    assert_eq!("－", decode_square(image, size));
    assert!(matches!(
        encoder.encode_text("−"),
        Err(EncodeError::TextNotRepresentable {
            byte_offset: 0,
            ..
        })
    ));
    assert!(matches!(
        Segment::kanji("−"),
        Err(EncodeError::InvalidData {
            byte_offset: 0,
            ..
        })
    ));
}

// Owned and borrowed inputs produce identical symbols, confirming the generic argument bounds.
#[test]
fn public_inputs_accept_owned_and_borrowed_forms_without_changing_symbols() {
    let encoder = Encoder::new(ErrorCorrection::Low).version(Version::M4);
    let text = String::from("HELLO 123");
    assert_eq!(encoder.encode_text(text.clone()).unwrap(), encoder.encode_text(&text).unwrap());

    let bytes = vec![1, 2, 3];
    assert_eq!(encoder.encode_bytes(bytes.clone()).unwrap(), encoder.encode_bytes(&bytes).unwrap());
    assert_eq!(
        Segment::alphanumeric(String::from("HELLO")).unwrap(),
        Segment::alphanumeric("HELLO").unwrap()
    );
}
