#[cfg(all(feature = "qr", feature = "async-write"))]
use std::{
    future::Future,
    task::{Context, Poll, Waker},
};

#[cfg(all(feature = "qr", feature = "micro-qr"))]
use qrcode_generator::AutoEncoder;
#[cfg(feature = "rmqr")]
use qrcode_generator::Segment;
#[cfg(feature = "qr")]
use qrcode_generator::ToQRText;
#[cfg(feature = "micro-qr")]
use qrcode_generator::micro::{
    Encoder as MicroEncoder, ErrorCorrection as MicroErrorCorrection, Version as MicroVersion,
};
#[cfg(feature = "qr")]
use qrcode_generator::qr::{
    ApplicationIndicator, Encoder, ErrorCorrection, Fnc1, Version as QrVersion,
};
#[cfg(feature = "rmqr")]
use qrcode_generator::rmqr::{
    ApplicationIndicator as RmqrApplicationIndicator, EciAssignment as RmqrEciAssignment,
    Encoder as RmqrEncoder, ErrorCorrection as RmqrErrorCorrection, Fnc1 as RmqrFnc1,
    Version as RmqrVersion,
};
#[cfg(any(feature = "qr", feature = "micro-qr", feature = "rmqr"))]
use qrcode_generator::{Renderer, Symbol, SymbolVersion};
#[cfg(any(feature = "qr", feature = "micro-qr", feature = "rmqr"))]
use rxing::MultiFormatReader;
#[cfg(feature = "qr")]
use rxing::qrcode::QRCodeReader;
#[cfg(any(feature = "qr", feature = "micro-qr", feature = "rmqr"))]
use rxing::{BinaryBitmap, Luma8LuminanceSource, Reader, common::HybridBinarizer};

#[cfg(any(feature = "qr", feature = "micro-qr"))]
fn render(symbol: &Symbol, scale: usize) -> (Vec<u8>, usize) {
    let quiet_zone = match symbol.version() {
        #[cfg(feature = "qr")]
        SymbolVersion::Qr(_) => 4,
        #[cfg(feature = "micro-qr")]
        SymbolVersion::Micro(_) => 2,
        #[cfg(feature = "rmqr")]
        SymbolVersion::Rmqr(_) => 2,
        _ => panic!("the test renderer needs a quiet zone for the new symbol family"),
    };
    let size = (symbol.size() + quiet_zone * 2) * scale;

    (Renderer::new(symbol, size).to_luma8().unwrap(), size)
}

#[cfg(any(feature = "qr", feature = "micro-qr"))]
fn decode_rxing(image: Vec<u8>, size: usize) -> String {
    decode_rxing_result(image, size).getText().to_owned()
}

#[cfg(any(feature = "qr", feature = "micro-qr"))]
fn decode_rxing_result(image: Vec<u8>, size: usize) -> rxing::RXingResult {
    let source = Luma8LuminanceSource::new(image, size as u32, size as u32);
    let mut bitmap = BinaryBitmap::new(HybridBinarizer::new(source));

    MultiFormatReader::default().decode(&mut bitmap).unwrap()
}

#[cfg(feature = "rmqr")]
fn render_rmqr(symbol: &Symbol, scale: usize) -> (Vec<u8>, usize, usize) {
    let width = (symbol.width() + 8) * scale;
    let height = (symbol.height() + 8) * scale;
    let image =
        Renderer::new_with_dimensions(symbol, width, height).quiet_zone(4).to_luma8().unwrap();

    (image, width, height)
}

#[cfg(feature = "rmqr")]
fn decode_rxing_rmqr(
    image: Vec<u8>,
    width: usize,
    height: usize,
    context: &str,
) -> rxing::RXingResult {
    use std::collections::HashSet;

    use rxing::{BarcodeFormat, DecodeHints, qrcode::cpp_port::QrReader};

    let source = Luma8LuminanceSource::new(image, width as u32, height as u32);
    let mut bitmap = BinaryBitmap::new(HybridBinarizer::new(source));
    let hints = DecodeHints {
        PureBarcode: Some(true),
        PossibleFormats: Some(HashSet::from([BarcodeFormat::RECTANGULAR_MICRO_QR_CODE])),
        ..DecodeHints::default()
    };

    QrReader
        .decode_with_hints(&mut bitmap, &hints)
        .unwrap_or_else(|error| panic!("failed to decode {context}: {error:?}"))
}

#[cfg(feature = "qr")]
fn decode_rxing_model2_result(image: Vec<u8>, size: usize) -> rxing::RXingResult {
    let source = Luma8LuminanceSource::new(image, size as u32, size as u32);
    let mut bitmap = BinaryBitmap::new(HybridBinarizer::new(source));

    QRCodeReader {}.decode(&mut bitmap).unwrap()
}

#[cfg(feature = "qr")]
fn decode_quircs(image: &[u8], size: usize) -> Vec<u8> {
    let mut decoder = quircs::Quirc::default();
    let mut codes = decoder.identify(size, size, image);
    let code = codes.next().expect("a QR Code should be detected").unwrap();

    assert!(codes.next().is_none());

    code.decode().unwrap().payload
}

#[cfg(feature = "qr")]
#[test]
fn model2_round_trips_through_independent_decoders() {
    for error_correction in [
        ErrorCorrection::Low,
        ErrorCorrection::Medium,
        ErrorCorrection::Quartile,
        ErrorCorrection::High,
    ] {
        let text = "HELLO 1234567890 mixed-mode";
        let symbol =
            Encoder::new(error_correction).boost_error_correction(false).encode_text(text).unwrap();

        assert_eq!(symbol.error_correction(), error_correction.into());

        let (image, size) = render(&symbol, 8);

        assert_eq!(decode_rxing(image.clone(), size), text);
        assert_eq!(decode_quircs(&image, size), text.as_bytes());
    }
}

#[cfg(feature = "qr")]
#[test]
fn custom_qr_text_round_trips_through_independent_decoders() {
    use std::cell::Cell;

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
    let expected = "CUSTOM QR TEXT 123";
    let symbol = Encoder::new(ErrorCorrection::Medium)
        .boost_error_correction(false)
        .encode_to_qr_text(&value)
        .unwrap();

    assert_eq!(value.calls.get(), 1);

    let (image, size) = render(&symbol, 8);

    assert_eq!(decode_rxing(image.clone(), size), expected);
    assert_eq!(decode_quircs(&image, size), expected.as_bytes());
}

#[cfg(feature = "qr")]
#[test]
fn version_information_round_trips_through_independent_decoders() {
    for version in [7, 40] {
        let version = QrVersion::new(version).unwrap();
        let text = format!("FORCED VERSION {}", version.value());

        let symbol = Encoder::new(ErrorCorrection::Medium)
            .version(version)
            .boost_error_correction(false)
            .encode_text(&text)
            .unwrap();

        let (image, size) = render(&symbol, 4);

        assert_eq!(decode_rxing(image.clone(), size), text);
        assert_eq!(decode_quircs(&image, size), text.as_bytes());
    }
}

#[cfg(feature = "qr")]
#[test]
fn utf8_eci_round_trips() {
    let text = "QR Code 😀 café";
    let symbol = Encoder::new(ErrorCorrection::Medium).encode_text(text).unwrap();
    let (image, size) = render(&symbol, 8);

    assert_eq!(decode_rxing(image, size), text);
}

#[cfg(all(feature = "qr", feature = "kanji"))]
#[test]
fn kanji_mode_round_trips_through_rxing() {
    let text = "点茗日本語";
    let symbol = Encoder::new(ErrorCorrection::Medium).encode_text(text).unwrap();
    let (image, size) = render(&symbol, 8);

    assert_eq!(decode_rxing_model2_result(image, size).getText(), text);
}

#[cfg(feature = "qr")]
#[test]
fn binary_payload_round_trips_through_quircs() {
    let data = b"binary\0payload\xFF";
    let symbol = Encoder::new(ErrorCorrection::Quartile).encode_bytes(data).unwrap();
    let (image, size) = render(&symbol, 8);

    assert_eq!(decode_quircs(&image, size), data);
}

#[cfg(feature = "qr")]
#[test]
fn structured_append_has_consistent_metadata() {
    let symbols = Encoder::new(ErrorCorrection::Medium)
        .encode_structured_append_bytes(&[b"FIRST", b"SECOND"])
        .unwrap();

    assert_eq!(symbols.len(), 2);

    let first = symbols[0].structured_append().unwrap();
    let second = symbols[1].structured_append().unwrap();

    assert_eq!((first.index(), first.total()), (0, 2));
    assert_eq!((second.index(), second.total()), (1, 2));
    assert_eq!(first.parity(), second.parity());

    for (index, symbol) in symbols.iter().enumerate() {
        let (image, size) = render(symbol, 8);
        let decoded = decode_rxing_model2_result(image, size);

        use rxing::{RXingResultMetadataType, RXingResultMetadataValue};

        assert_eq!(
            decoded
                .getRXingResultMetadata()
                .get(&RXingResultMetadataType::STRUCTURED_APPEND_SEQUENCE),
            Some(&RXingResultMetadataValue::StructuredAppendSequence(((index as i32) << 4) | 1))
        );

        assert_eq!(
            decoded
                .getRXingResultMetadata()
                .get(&RXingResultMetadataType::STRUCTURED_APPEND_PARITY),
            Some(&RXingResultMetadataValue::StructuredAppendParity(i32::from(first.parity())))
        );
    }
}

#[cfg(feature = "qr")]
#[test]
fn structured_append_text_redeclares_eci_in_each_symbol() {
    let parts = ["😀", "café"];
    let symbols =
        Encoder::new(ErrorCorrection::Medium).encode_structured_append_text(&parts).unwrap();

    for (symbol, expected) in symbols.iter().zip(parts) {
        let (image, size) = render(symbol, 8);

        assert_eq!(decode_rxing_model2_result(image, size).getText(), expected);
    }
}

#[cfg(feature = "qr")]
#[test]
fn structured_append_default_eci_uses_full_capacity() {
    let parts = ["ABCDEFGHIJKLMNOPQRST", "QRSTUVWXYZ0123456789"];
    let version = QrVersion::new(1).unwrap();
    let symbols = Encoder::new(ErrorCorrection::Low)
        .version(version)
        .boost_error_correction(false)
        .encode_structured_append_text(&parts)
        .unwrap();

    for (symbol, expected) in symbols.iter().zip(parts) {
        assert_eq!(symbol.version(), SymbolVersion::Qr(version));

        let (image, size) = render(symbol, 8);

        assert_eq!(decode_rxing_model2_result(image, size).getText(), expected);
    }
}

#[cfg(feature = "qr")]
#[test]
fn fnc1_round_trips_through_rxing() {
    let data = b"0101234567890128\x1D10ABC";
    let symbol =
        Encoder::new(ErrorCorrection::Medium).fnc1(Some(Fnc1::Gs1)).encode_bytes(data).unwrap();
    let (image, size) = render(&symbol, 8);

    assert_eq!(decode_rxing_model2_result(image, size).getText().as_bytes(), data);

    Encoder::new(ErrorCorrection::Medium)
        .fnc1(Some(Fnc1::Industry(ApplicationIndicator::numeric(12).unwrap())))
        .encode_bytes(data)
        .unwrap();
}

#[cfg(feature = "qr")]
#[test]
fn svg_uses_default_and_custom_quiet_zones() {
    let symbol = Encoder::new(ErrorCorrection::Low).encode_text("HELLO").unwrap();
    let size = (symbol.size() + 8) * 4;
    let svg = Renderer::new(&symbol, size).to_svg_string(Some("test")).unwrap();

    assert!(svg.contains("<desc>test</desc>"));
    assert!(svg.contains("M16 16"));

    let size = symbol.size() * 4;
    let svg = Renderer::new(&symbol, size).quiet_zone(0).to_svg_string(None).unwrap();

    assert!(svg.contains("M0 0"));
}

#[cfg(all(feature = "qr", feature = "async-write"))]
#[test]
fn async_writer_matches_synchronous_rendering() {
    let symbol = Encoder::new(ErrorCorrection::Low).encode_text("HELLO").unwrap();
    let renderer = Renderer::new(&symbol, 256);

    let svg = renderer.to_svg_string(Some("async")).unwrap();
    let mut async_svg = Vec::new();

    run_ready(renderer.write_svg_async(&mut async_svg, Some("async"))).unwrap();

    assert_eq!(async_svg, svg.as_bytes());

    #[cfg(feature = "image")]
    {
        let png = renderer.to_png_vec().unwrap();
        let mut async_png = Vec::new();

        run_ready(renderer.write_png_async(&mut async_png)).unwrap();

        assert_eq!(async_png, png);
    }
}

#[cfg(all(feature = "qr", feature = "async-write"))]
fn run_ready<F: Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let mut context = Context::from_waker(Waker::noop());

    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("the in-memory writer should be ready"),
    }
}

#[cfg(feature = "qr")]
#[test]
fn automatic_structured_append_splits_and_round_trips() {
    let data = vec![b'x'; 80];

    let symbols = Encoder::new(ErrorCorrection::Low)
        .version_range(QrVersion::new(1).unwrap()..=QrVersion::new(2).unwrap())
        .encode_bytes_with_structured_append(&data)
        .unwrap();

    assert!(symbols.len() > 1);

    let mut decoded = Vec::new();

    for symbol in symbols {
        let (image, size) = render(&symbol, 8);

        decoded.extend_from_slice(decode_rxing_model2_result(image, size).getText().as_bytes());
    }

    assert_eq!(decoded, data);
}

#[cfg(feature = "qr")]
#[test]
fn atomic_file_output_preserves_an_existing_file_on_render_error() {
    let symbol = Encoder::new(ErrorCorrection::Low).encode_text("HELLO").unwrap();
    let path = std::env::temp_dir().join(format!(
        "qrcode-generator-atomic-{}-{}.svg",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));

    std::fs::write(&path, b"original").unwrap();

    assert!(Renderer::new(&symbol, 1).save_svg(&path, None).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"original");

    Renderer::new(&symbol, 256).save_svg(&path, Some("atomic")).unwrap();

    assert!(std::fs::read_to_string(&path).unwrap().contains("<desc>atomic</desc>"));

    std::fs::remove_file(path).unwrap();
}

#[cfg(feature = "micro-qr")]
#[test]
fn all_micro_versions_round_trip_through_rxing() {
    for (version, error_correction, text) in [
        (MicroVersion::M1, MicroErrorCorrection::DetectionOnly, "12345"),
        (MicroVersion::M2, MicroErrorCorrection::Low, "01234567"),
        (MicroVersion::M3, MicroErrorCorrection::Medium, "HELLO 123"),
        (MicroVersion::M4, MicroErrorCorrection::Quartile, "HELLO 123"),
    ] {
        micro_round_trip(version, error_correction, text);
    }
}

#[cfg(feature = "micro-qr")]
#[test]
fn micro_svg_uses_the_standard_quiet_zone() {
    let symbol = MicroEncoder::new(MicroErrorCorrection::Low)
        .version(MicroVersion::M2)
        .encode_text("12345")
        .unwrap();
    let size = (symbol.size() + 4) * 4;
    let svg = Renderer::new(&symbol, size).to_svg_string(None).unwrap();

    assert!(svg.contains("M8 8"));
}

#[cfg(feature = "micro-qr")]
fn micro_round_trip(version: MicroVersion, error_correction: MicroErrorCorrection, text: &str) {
    let symbol = MicroEncoder::new(error_correction)
        .version(version)
        .boost_error_correction(false)
        .encode_text(text)
        .unwrap();

    assert_eq!(symbol.error_correction(), error_correction.into());

    let (image, size) = render(&symbol, 10);

    assert_eq!(decode_rxing(image, size), text);
}

#[cfg(all(feature = "qr", feature = "micro-qr"))]
#[test]
fn auto_encoder_selects_micro_then_falls_back_to_model2() {
    let encoder = AutoEncoder::new(
        Encoder::new(ErrorCorrection::Medium).boost_error_correction(false),
        MicroEncoder::new(MicroErrorCorrection::Medium).boost_error_correction(false),
    );

    let micro_text = "HELLO 123";
    let micro_symbol = encoder.encode_text(micro_text).unwrap();
    assert!(matches!(micro_symbol.version(), SymbolVersion::Micro(_)));

    let (micro_image, micro_size) = render(&micro_symbol, 10);
    assert_eq!(decode_rxing(micro_image, micro_size), micro_text);

    let qr_text = "This payload is intentionally too long for every Micro QR Code version.";
    let qr_symbol = encoder.encode_text(qr_text).unwrap();
    assert!(matches!(qr_symbol.version(), SymbolVersion::Qr(_)));

    let (qr_image, qr_size) = render(&qr_symbol, 8);
    assert_eq!(decode_rxing(qr_image.clone(), qr_size), qr_text);
    assert_eq!(decode_quircs(&qr_image, qr_size), qr_text.as_bytes());
}

#[cfg(feature = "rmqr")]
#[test]
fn all_rmqr_versions_and_error_correction_levels_round_trip_through_rxing() {
    let versions = [
        RmqrVersion::R7x43,
        RmqrVersion::R7x59,
        RmqrVersion::R7x77,
        RmqrVersion::R7x99,
        RmqrVersion::R7x139,
        RmqrVersion::R9x43,
        RmqrVersion::R9x59,
        RmqrVersion::R9x77,
        RmqrVersion::R9x99,
        RmqrVersion::R9x139,
        RmqrVersion::R11x27,
        RmqrVersion::R11x43,
        RmqrVersion::R11x59,
        RmqrVersion::R11x77,
        RmqrVersion::R11x99,
        RmqrVersion::R11x139,
        RmqrVersion::R13x27,
        RmqrVersion::R13x43,
        RmqrVersion::R13x59,
        RmqrVersion::R13x77,
        RmqrVersion::R13x99,
        RmqrVersion::R13x139,
        RmqrVersion::R15x43,
        RmqrVersion::R15x59,
        RmqrVersion::R15x77,
        RmqrVersion::R15x99,
        RmqrVersion::R15x139,
        RmqrVersion::R17x43,
        RmqrVersion::R17x59,
        RmqrVersion::R17x77,
        RmqrVersion::R17x99,
        RmqrVersion::R17x139,
    ];

    for version in versions {
        for error_correction in [RmqrErrorCorrection::Medium, RmqrErrorCorrection::High] {
            let symbol = RmqrEncoder::new(error_correction)
                .version(version)
                .boost_error_correction(false)
                .encode_text("1")
                .unwrap();

            assert_eq!(symbol.error_correction(), error_correction.into());

            let (image, width, height) = render_rmqr(&symbol, 8);
            let context = format!("{version:?} {error_correction:?}");
            let decoded = decode_rxing_rmqr(image, width, height, &context);

            assert_eq!(decoded.getText(), "1", "{version:?} {error_correction:?}");
            assert_eq!(
                decoded.getBarcodeFormat(),
                &rxing::BarcodeFormat::RECTANGULAR_MICRO_QR_CODE
            );
        }
    }
}

#[cfg(feature = "rmqr")]
#[test]
fn rmqr_automatic_version_selection_minimizes_area() {
    let symbol = RmqrEncoder::new(RmqrErrorCorrection::Medium).encode_text("1").unwrap();

    assert_eq!(symbol.version(), SymbolVersion::Rmqr(RmqrVersion::R11x27));
}

#[cfg(feature = "rmqr")]
#[test]
fn rmqr_rendered_image_is_detected_by_rxing() {
    let symbol = RmqrEncoder::new(RmqrErrorCorrection::High)
        .version(RmqrVersion::R11x27)
        .boost_error_correction(false)
        .encode_text("0123456")
        .unwrap();
    let (image, width, height) = render_rmqr(&symbol, 10);
    let source = Luma8LuminanceSource::new(image, width as u32, height as u32);
    let mut bitmap = BinaryBitmap::new(HybridBinarizer::new(source));
    let decoded = MultiFormatReader::default().decode(&mut bitmap).unwrap();

    assert_eq!(decoded.getText(), "0123456");
    assert_eq!(decoded.getBarcodeFormat(), &rxing::BarcodeFormat::RECTANGULAR_MICRO_QR_CODE);
}

#[cfg(feature = "rmqr")]
#[test]
fn rmqr_eci_fnc1_segments_and_renderer_follow_the_public_api() {
    let text = "rMQR 😀";
    let symbol = RmqrEncoder::new(RmqrErrorCorrection::Medium).encode_text(text).unwrap();
    let (image, width, height) = render_rmqr(&symbol, 8);

    assert_eq!(decode_rxing_rmqr(image, width, height, "UTF-8 ECI").getText(), text);

    let data = b"0101234567890128\x1D10ABC";
    let symbol = RmqrEncoder::new(RmqrErrorCorrection::Medium)
        .fnc1(Some(RmqrFnc1::Gs1))
        .encode_bytes(data)
        .unwrap();
    let (image, width, height) = render_rmqr(&symbol, 8);

    assert_eq!(decode_rxing_rmqr(image, width, height, "FNC1").getText().as_bytes(), data);

    RmqrEncoder::new(RmqrErrorCorrection::Medium)
        .fnc1(Some(RmqrFnc1::Industry(RmqrApplicationIndicator::numeric(12).unwrap())))
        .encode_bytes(data)
        .unwrap();

    let segments = [Segment::numeric("12345").unwrap(), Segment::bytes(b"abc")];
    let symbol = RmqrEncoder::new(RmqrErrorCorrection::Medium).encode_segments(&segments).unwrap();
    let (image, width, height) = render_rmqr(&symbol, 8);

    assert_eq!(decode_rxing_rmqr(image, width, height, "explicit segments").getText(), "12345abc");

    let segments = [Segment::eci(RmqrEciAssignment::UTF_8), Segment::bytes("é".as_bytes())];
    let symbol = RmqrEncoder::new(RmqrErrorCorrection::Medium).encode_segments(&segments).unwrap();
    let (image, width, height) = render_rmqr(&symbol, 8);

    assert_eq!(decode_rxing_rmqr(image, width, height, "explicit ECI").getText(), "é");

    let width = (symbol.width() + 4) * 4;
    let height = (symbol.height() + 4) * 4;
    let svg = Renderer::new_with_dimensions(&symbol, width, height).to_svg_string(None).unwrap();

    assert!(svg.contains(&format!("<svg width=\"{width}\" height=\"{height}\"")));
    assert!(svg.contains("M8 8"));
}

#[cfg(all(feature = "rmqr", feature = "kanji"))]
#[test]
fn rmqr_kanji_mode_round_trips_through_rxing() {
    let text = "点茗日本語";
    let symbol = RmqrEncoder::new(RmqrErrorCorrection::Medium).encode_text(text).unwrap();
    let (image, width, height) = render_rmqr(&symbol, 8);

    assert_eq!(decode_rxing_rmqr(image, width, height, "Kanji").getText(), text);
}

#[cfg(all(feature = "micro-qr", feature = "kanji"))]
#[test]
fn micro_kanji_mode_round_trips_through_rxing() {
    let text = "点茗";
    let symbol = MicroEncoder::new(MicroErrorCorrection::Low)
        .version(MicroVersion::M3)
        .encode_text(text)
        .unwrap();
    let (image, size) = render(&symbol, 10);

    assert_eq!(decode_rxing(image, size), text);
}
