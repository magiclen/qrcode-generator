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

// rxing 0.9.2 still cannot read a doubled percent: it collapses the pair but keeps the original index range, so the segment ends in an out-of-range read and the symbol fails to decode.
// This helper therefore reads and corrects the codewords itself before parsing the FNC1 data.
// It reaches into rxing internals to do that, so recheck it whenever the dev dependency moves.
#[cfg(feature = "qr")]
pub(crate) fn decode_fnc1(symbol: &Symbol) -> String {
    use rxing::{
        common::{BitMatrix, BitSource},
        qrcode::{
            common::Mode,
            cpp_port::{Type, decoder::CorrectErrors},
            decoder::{BitMatrixParser, DataBlock},
        },
    };

    let matrix = BitMatrix::parse_bools(&symbol.to_matrix());
    let mut parser = BitMatrixParser::new(matrix).unwrap();
    let error_correction = parser.readFormatInformation().unwrap().getErrorCorrectionLevel();
    let version = parser.readVersion().unwrap();
    let codewords = parser.readCodewords().unwrap();
    let blocks = DataBlock::getDataBlocks(&codewords, version, error_correction).unwrap();
    let mut data = Vec::new();
    for block in blocks {
        let count = block.getNumDataCodewords();
        let mut words = block.getCodewords().to_vec();
        assert!(CorrectErrors(&mut words, count).unwrap());
        data.extend_from_slice(&words[..count as usize]);
    }

    let mut bits = BitSource::new(&data);
    let mode_width = Mode::get_codec_mode_bits_length(version) as usize;
    let header =
        Mode::CodecModeForBits(bits.readBits(mode_width).unwrap(), Some(Type::Model2)).unwrap();
    assert_eq!(Mode::FNC1_FIRST_POSITION, header);
    let mut result = String::new();
    while bits.available() >= mode_width {
        let mode =
            Mode::CodecModeForBits(bits.readBits(mode_width).unwrap(), Some(Type::Model2)).unwrap();
        if mode == Mode::TERMINATOR {
            break;
        }
        let mut count = bits.readBits(mode.CharacterCountBits(version) as usize).unwrap() as usize;
        match mode {
            Mode::NUMERIC => {
                while count > 0 {
                    let digits = count.min(3);
                    let value = bits.readBits([0, 4, 7, 10][digits]).unwrap();
                    result.push_str(&format!("{value:0digits$}"));
                    count -= digits;
                }
            },
            Mode::BYTE => {
                for _ in 0..count {
                    result.push(char::from(bits.readBits(8).unwrap() as u8));
                }
            },
            Mode::ALPHANUMERIC => {
                let alphabet = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";
                let mut encoded = Vec::new();
                while count >= 2 {
                    let value = bits.readBits(11).unwrap() as usize;
                    encoded.extend_from_slice(&[alphabet[value / 45], alphabet[value % 45]]);
                    count -= 2;
                }
                if count != 0 {
                    encoded.push(alphabet[bits.readBits(6).unwrap() as usize]);
                }
                let mut chars = encoded.into_iter().peekable();
                while let Some(byte) = chars.next() {
                    result.push(if byte != b'%' {
                        char::from(byte)
                    } else if chars.peek() == Some(&b'%') {
                        chars.next();
                        '%'
                    } else {
                        '\u{1D}'
                    });
                }
            },
            _ => panic!("unexpected mode in the FNC1 test: {mode:?}"),
        }
    }
    result
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
    let source = Luma8LuminanceSource::new(image, size as u32, size as u32).unwrap();
    let mut bitmap = BinaryBitmap::new(HybridBinarizer::new(source));

    rxing::MultiFormatReader::default().decode(&mut bitmap).unwrap().getText().to_owned()
}

#[cfg(feature = "qr")]
pub(crate) fn decode_model2(image: Vec<u8>, size: usize) -> rxing::RXingResult {
    let source = Luma8LuminanceSource::new(image, size as u32, size as u32).unwrap();
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

    let source = Luma8LuminanceSource::new(image, width as u32, height as u32).unwrap();
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
    let source = Luma8LuminanceSource::new(image, width as u32, height as u32).unwrap();
    let mut bitmap = BinaryBitmap::new(HybridBinarizer::new(source));

    rxing::MultiFormatReader::default().decode(&mut bitmap).unwrap()
}
