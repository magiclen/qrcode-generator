use qrcode_generator::{Renderer, Symbol};
use rxing::{BinaryBitmap, Luma8LuminanceSource, Reader, common::HybridBinarizer};

// Renders the module matrix as one string per row, using '1' for dark and '0' for light modules.
pub(crate) fn matrix_rows(symbol: &Symbol) -> Vec<String> {
    symbol
        .to_matrix()
        .iter()
        .map(|row| row.iter().map(|&dark| if dark { '1' } else { '0' }).collect())
        .collect()
}

#[cfg(any(feature = "qr", feature = "micro-qr"))]
pub(crate) fn render_square(symbol: &Symbol, scale: usize) -> (Vec<u8>, usize) {
    let quiet_zone = match symbol.version() {
        #[cfg(feature = "qr")]
        qrcode_generator::SymbolVersion::Qr(_) => 4,
        #[cfg(feature = "micro-qr")]
        qrcode_generator::SymbolVersion::Micro(_) => 2,
        #[cfg(feature = "rmqr")]
        qrcode_generator::SymbolVersion::Rmqr(_) => 2,
        _ => panic!("the test renderer needs a quiet zone for the new symbol family"),
    };
    let size = (symbol.size() + quiet_zone * 2) * scale;

    (Renderer::new(symbol, size).to_luma8().unwrap(), size)
}

#[cfg(any(feature = "qr", feature = "micro-qr"))]
pub(crate) fn decode_square(image: Vec<u8>, size: usize) -> String {
    let source = Luma8LuminanceSource::new(image, size as u32, size as u32);
    let mut bitmap = BinaryBitmap::new(HybridBinarizer::new(source));

    rxing::MultiFormatReader::default().decode(&mut bitmap).unwrap().getText().to_owned()
}

#[cfg(feature = "qr")]
pub(crate) fn decode_model2(image: Vec<u8>, size: usize) -> rxing::RXingResult {
    let source = Luma8LuminanceSource::new(image, size as u32, size as u32);
    let mut bitmap = BinaryBitmap::new(HybridBinarizer::new(source));

    rxing::qrcode::QRCodeReader {}.decode(&mut bitmap).unwrap()
}

#[cfg(feature = "qr")]
pub(crate) fn decode_quircs(image: &[u8], size: usize) -> Vec<u8> {
    let mut decoder = quircs::Quirc::default();
    let mut codes = decoder.identify(size, size, image);
    let code = codes.next().expect("a QR Code should be detected").unwrap();

    assert!(codes.next().is_none());
    code.decode().unwrap().payload
}

#[cfg(feature = "rmqr")]
pub(crate) fn render_rmqr(symbol: &Symbol, scale: usize) -> (Vec<u8>, usize, usize) {
    let width = (symbol.width() + 4) * scale;
    let height = (symbol.height() + 4) * scale;
    let image = Renderer::new_with_dimensions(symbol, width, height).to_luma8().unwrap();

    (image, width, height)
}

#[cfg(feature = "rmqr")]
pub(crate) fn decode_rmqr_pure(
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

#[cfg(feature = "rmqr")]
pub(crate) fn decode_rmqr_detected(
    image: Vec<u8>,
    width: usize,
    height: usize,
) -> rxing::RXingResult {
    let source = Luma8LuminanceSource::new(image, width as u32, height as u32);
    let mut bitmap = BinaryBitmap::new(HybridBinarizer::new(source));

    rxing::MultiFormatReader::default().decode(&mut bitmap).unwrap()
}
