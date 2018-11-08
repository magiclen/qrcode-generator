//! This crate provides functions to generate QR Code matrices and images in RAW, PNG and SVG formats.
//!
//! ## Examples
//!
//! ### Encode any data to a QR Code matrix which is `Vec<Vec<bool>>`.
//!
//! ```
//! extern crate qrcode_generator;
//!
//! use qrcode_generator::QrCodeEcc;
//!
//! let result: Vec<Vec<bool>> = qrcode_generator::to_matrix("Hello world!", QrCodeEcc::Low).unwrap();
//!
//! println!("{:?}", result);
//! ```
//!
//! ### Encode any data to a PNG image stored in a Vec instance.
//!
//! ```
//! extern crate qrcode_generator;
//!
//! use qrcode_generator::QrCodeEcc;
//!
//! let result: Vec<u8> = qrcode_generator::to_png_to_vec("Hello world!", QrCodeEcc::Low, 1024).unwrap();
//!
//! println!("{:?}", result);
//! ```
//!
//! ### Encode any data to a PNG image stored in a file.
//!
//! ```ignore
//! extern crate qrcode_generator;
//!
//! use qrcode_generator::QrCodeEcc;
//!
//! qrcode_generator::to_png_to_file("Hello world!", QrCodeEcc::Low, 1024, "path/to/file.png").unwrap();
//! ```
//!
//! ### Encode any data to a SVG image stored in a String instance.
//!
//! ```
//! extern crate qrcode_generator;
//!
//! use qrcode_generator::QrCodeEcc;
//!
//! let result: String = qrcode_generator::to_svg_to_string("Hello world!", QrCodeEcc::Low, 1024, None).unwrap();
//!
//! println!("{:?}", result);
//! ```
//!
//! ### Encode any data to a SVG image stored in a file.
//!
//! ```ignore
//! extern crate qrcode_generator;
//!
//! use qrcode_generator::QrCodeEcc;
//!
//! qrcode_generator::to_svg_to_file("Hello world!", QrCodeEcc::Low, 1024, None, "path/to/file.svg").unwrap();
//! ```
//!
//! ## Low-level Usage
//!
//! ### Raw Image Data
//!
//! The `to_image` and `to_image_buffer` functions can be used, if you want to modify your image.
//!
//! ### Segments
//!
//! Every **generate** and **to** function has its own **by_segments** function. You can concatenate segments by using different encoding methods, such as **numeric**, **alphanumeric** or **binary** to reduce the size (level) of your QR code matrix/image.
//!
//! ```
//! extern crate qrcode_generator;
//!
//! use qrcode_generator::QrCodeEcc;
//! use qrcode_generator::qrcodegen::QrSegment;
//!
//! let first = "1234567";
//!
//! let second = "ABCDEFG";
//!
//! let first_chars: Vec<char> = first.chars().collect();
//! let second_chars: Vec<char> = second.chars().collect();
//!
//! let segments = vec![QrSegment::make_numeric(&first_chars), QrSegment::make_alphanumeric(&second_chars)];
//!
//! let result: Vec<Vec<bool>> = qrcode_generator::to_matrix_by_segments(&segments, QrCodeEcc::Low).unwrap();
//!
//! println!("{:?}", result);
//! ```
//!
//! ### Validators Support
//!
//! `Validators` is a crate which can help you validate user input.
//!
//! To use with Validators support, you have to enable the **validator** feature for this crate.
//!
//! ```ignore
//! [dependencies.qrcode-generator]
//! version = "*"
//! features = ["validator"]
//! ```
//!
//! And the `optimize_validated_http_url_segments` and `optimize_validated_http_ftp_url_segments` functions are available. They can be used for generating a safe and optimized (as small as possible) URL QR Code.
//!
//! ```ignore
//! extern crate qrcode_generator;
//! extern crate validators;
//!
//! use qrcode_generator::QrCodeEcc;
//! use validators::{ValidatorOption, http_url::HttpUrlValidator};
//!
//! let validator = HttpUrlValidator {
//!     protocol: ValidatorOption::Allow,
//!     local: ValidatorOption::Allow,
//! };
//!
//! let url = "https://magiclen.org/path/to/12345";
//!
//! let validated_http_url = validator.parse_str(url).unwrap();
//!
//! let matrix_1 = qrcode_generator::to_matrix(url, QrCodeEcc::Low).unwrap();
//! let matrix_2 = qrcode_generator::to_matrix_by_segments(&qrcode_generator::optimize_validated_http_url_segments(&validated_http_url, QrCodeEcc::Low).unwrap(), QrCodeEcc::Low).unwrap();
//!
//! assert!(matrix_2.len() < matrix_1.len());
//! ```

pub extern crate qrcodegen;
extern crate htmlescape;
extern crate image;
extern crate rc_writer;

#[cfg(feature = "validator")]
extern crate percent_encoding;
#[cfg(feature = "validator")]
extern crate idna;

#[cfg(feature = "validator")]
pub extern crate validators;

use std::io::{self, Write, ErrorKind};

use std::rc::Rc;
use std::cell::RefCell;
use std::fs::{self, File};
use std::path::Path;

use qrcodegen::{QrCode, QrSegment, qr_segment_advanced};

pub use qrcodegen::QrCodeEcc;

use image::{ImageBuffer, Luma, png::PNGEncoder, ColorType};

use rc_writer::RcOptionWriter;

#[cfg(feature = "validator")]
use validators::http_url::HttpUrl;

#[cfg(feature = "validator")]
use validators::http_ftp_url::HttpFtpUrl;

#[cfg(feature = "validator")]
use validators::host::Host;

#[inline]
/// Optimize any text for generating QR code.
pub fn make_text_segments(text: &[char], ecc: QrCodeEcc) -> Result<Vec<QrSegment>, io::Error> {
    match qr_segment_advanced::make_segments_optimally(&text, ecc, qrcodegen::QrCode_MIN_VERSION, qrcodegen::QrCode_MAX_VERSION) {
        Some(segments) => Ok(segments),
        None => return Err(io::Error::new(ErrorKind::Other, "the data is too long"))
    }
}

#[cfg(feature = "validator")]
/// Optimize URL text for generating QR code.
pub fn optimize_validated_http_url_segments(http_url: &HttpUrl, ecc: QrCodeEcc) -> Result<Vec<QrSegment>, io::Error> {
    let mut url = String::new();

    if let Some(protocol) = http_url.get_protocol() {
        url.push_str(&protocol.to_uppercase());
    }

    if http_url.is_absolute() {
        url.push_str("://");
    } else {
        url.push_str(":");
    }

    let host = http_url.get_host();

    if let Host::Domain(domain) = host {
        match idna::domain_to_ascii(domain.get_full_domain_without_port()) {
            Ok(domain_without_port) => {
                url.push_str(&domain_without_port);
            }
            Err(_) => {
                return Err(io::Error::new(ErrorKind::Other, "the url may not be correct"));
            }
        }

        if let Some(port) = domain.get_port() {
            url.push_str(":");
            url.push_str(&format!("{}", port));
        }
    } else {
        url.push_str(host.get_full_host());
    }

    if let Some(path) = http_url.get_path() {
        url.push_str(&percent_encoding::utf8_percent_encode(path, percent_encoding::DEFAULT_ENCODE_SET).to_string());
    }

    if let Some(query) = http_url.get_query() {
        url.push_str("?");
        url.push_str(&percent_encoding::utf8_percent_encode(query, percent_encoding::QUERY_ENCODE_SET).to_string());
    }

    if let Some(fragment) = http_url.get_fragment() {
        url.push_str("#");
        url.push_str(&percent_encoding::utf8_percent_encode(fragment, percent_encoding::QUERY_ENCODE_SET).to_string());
    }

    let chars: Vec<char> = url.chars().collect();

    make_text_segments(&chars, ecc)
}

#[cfg(feature = "validator")]
/// Optimize URL text for generating QR code.
pub fn optimize_validated_http_ftp_url_segments(http_ftp_url: &HttpFtpUrl, ecc: QrCodeEcc) -> Result<Vec<QrSegment>, io::Error> {
    let mut url = String::new();

    if let Some(protocol) = http_ftp_url.get_protocol() {
        url.push_str(&protocol.to_uppercase());
    }

    if http_ftp_url.is_absolute() {
        url.push_str("://");
    } else {
        url.push_str(":");
    }

    let host = http_ftp_url.get_host();

    if let Host::Domain(domain) = host {
        match idna::domain_to_ascii(domain.get_full_domain_without_port()) {
            Ok(domain_without_port) => {
                url.push_str(&domain_without_port);
            }
            Err(_) => {
                return Err(io::Error::new(ErrorKind::Other, "the url may not be correct"));
            }
        }

        if let Some(port) = domain.get_port() {
            url.push_str(":");
            url.push_str(&format!("{}", port));
        }
    } else {
        url.push_str(host.get_full_host());
    }

    if let Some(path) = http_ftp_url.get_path() {
        url.push_str(&percent_encoding::utf8_percent_encode(path, percent_encoding::DEFAULT_ENCODE_SET).to_string());
    }

    if let Some(query) = http_ftp_url.get_query() {
        url.push_str("?");
        url.push_str(&percent_encoding::utf8_percent_encode(query, percent_encoding::QUERY_ENCODE_SET).to_string());
    }

    if let Some(fragment) = http_ftp_url.get_fragment() {
        url.push_str("#");
        url.push_str(&percent_encoding::utf8_percent_encode(fragment, percent_encoding::QUERY_ENCODE_SET).to_string());
    }

    let chars: Vec<char> = url.chars().collect();

    make_text_segments(&chars, ecc)
}

#[inline]
fn generate_qrcode<D: AsRef<[u8]>>(data: D, ecc: QrCodeEcc) -> Result<QrCode, io::Error> {
    let data = data.as_ref();

    let tried_utf8 = String::from_utf8(data.to_vec());

    match tried_utf8 {
        Ok(text) => {
            let segments = make_text_segments(&text.chars().collect::<Vec<char>>(), ecc)?;

            let qr = match QrCode::encode_segments(&segments, ecc) {
                Ok(qr) => qr,
                Err(_) => return Err(io::Error::new(ErrorKind::Other, "the data is too long"))
            };

            Ok(qr)
        }
        Err(_) => {
            let qr = match QrCode::encode_binary(data, ecc) {
                Ok(qr) => qr,
                Err(_) => return Err(io::Error::new(ErrorKind::Other, "the data is too long"))
            };

            Ok(qr)
        }
    }
}

#[inline]
fn generate_qrcode_by_segments(segments: &[QrSegment], ecc: QrCodeEcc) -> Result<QrCode, io::Error> {
    match QrCode::encode_segments(segments, ecc) {
        Ok(qr) => Ok(qr),
        Err(_) => Err(io::Error::new(ErrorKind::Other, "the data is too long"))
    }
}

#[inline]
fn to_matrix_inner(qr: QrCode) -> Vec<Vec<bool>> {
    let size = qr.size();

    let size_u = size as usize;

    let mut rows = Vec::with_capacity(size_u);

    for y in 0..size {
        let mut row = Vec::with_capacity(size_u);

        for x in 0..size {
            row.push(qr.get_module(x, y));
        }

        rows.push(row);
    }

    rows
}

/// Encode data to a QR code matrix.
pub fn to_matrix<D: AsRef<[u8]>>(data: D, ecc: QrCodeEcc) -> Result<Vec<Vec<bool>>, io::Error> {
    Ok(to_matrix_inner(generate_qrcode(data, ecc)?))
}

/// Encode data to a QR code matrix.
pub fn to_matrix_by_segments(segments: &[QrSegment], ecc: QrCodeEcc) -> Result<Vec<Vec<bool>>, io::Error> {
    Ok(to_matrix_inner(generate_qrcode_by_segments(segments, ecc)?))
}

#[inline]
fn to_svg_inner<W: Write>(qr: QrCode, size: usize, description: Option<&str>, mut writer: W) -> Result<(), io::Error> {
    let margin_size = 1;

    let s = qr.size();

    let data_length = s as usize;

    let data_length_with_margin = data_length + 2 * margin_size;

    let point_size = size / data_length_with_margin;

    if point_size == 0 {
        return Err(io::Error::new(ErrorKind::Other, "the size is too small"));
    }

    let margin = (size - (point_size * data_length)) / 2;

    let size = size.to_string();

    writer.write(b"<?xml version=\"1.0\" encoding=\"utf-8\"?>")?;

    writer.write(b"<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"")?;

    writer.write(size.as_bytes())?;

    writer.write(b"\" height=\"")?;

    writer.write(size.as_bytes())?;

    writer.write(b"\">")?;

    match description {
        Some(description) => {
            if !description.is_empty() {
                writer.write(b"<desc>")?;
                writer.write(htmlescape::encode_minimal(description).as_bytes())?;
                writer.write(b"</desc>")?;
            }
        }
        None => {
            writer.write(b"<desc>")?;
            writer.write(env!("CARGO_PKG_NAME").as_bytes())?;
            writer.write(b" ")?;
            writer.write(env!("CARGO_PKG_VERSION").as_bytes())?;
            writer.write(b" by magiclen.org")?;
            writer.write(b"</desc>")?;
        }
    }

    writer.write(b"<rect width=\"")?;

    writer.write(size.as_bytes())?;

    writer.write(b"\" height=\"")?;

    writer.write(size.as_bytes())?;

    writer.write(b"\" fill=\"#FFFFFF\" cx=\"0\" cy=\"0\" />")?;

    let point_size_string = point_size.to_string();

    for i in 0..s {
        for j in 0..s {
            if qr.get_module(j, i) {
                let x = j as usize * point_size + margin;
                let y = i as usize * point_size + margin;

                writer.write(b"<rect x=\"")?;
                writer.write(x.to_string().as_bytes())?;

                writer.write(b"\" y=\"")?;
                writer.write(y.to_string().as_bytes())?;

                writer.write(b"\" width=\"")?;
                writer.write(point_size_string.as_bytes())?;

                writer.write(b"\" height=\"")?;
                writer.write(point_size_string.as_bytes())?;

                writer.write(b"\" fill=\"#000000\" shape-rendering=\"crispEdges\" />")?;
            }
        }
    }

    writer.write(b"</svg>")?;

    writer.flush()?;

    Ok(())
}

/// Encode data to a SVG image via any writer.
pub fn to_svg<D: AsRef<[u8]>, W: Write>(data: D, ecc: QrCodeEcc, size: usize, description: Option<&str>, writer: W) -> Result<(), io::Error> {
    to_svg_inner(generate_qrcode(data, ecc)?, size, description, writer)
}

/// Encode data to a SVG image via any writer.
pub fn to_svg_by_segments<W: Write>(segments: &[QrSegment], ecc: QrCodeEcc, size: usize, description: Option<&str>, writer: W) -> Result<(), io::Error> {
    to_svg_inner(generate_qrcode_by_segments(segments, ecc)?, size, description, writer)
}

#[inline]
fn to_svg_to_string_inner(qr: QrCode, size: usize, description: Option<&str>) -> Result<String, io::Error> {
    let temp = RefCell::new(Some(Vec::new()));

    let temp_rc = Rc::new(temp);

    to_svg_inner(qr, size, description, RcOptionWriter::new(temp_rc.clone()))?;

    let svg = temp_rc.borrow_mut().take().unwrap();

    Ok(unsafe { String::from_utf8_unchecked(svg) })
}

/// Encode data to a SVG image in memory.
pub fn to_svg_to_string<D: AsRef<[u8]>>(data: D, ecc: QrCodeEcc, size: usize, description: Option<&str>) -> Result<String, io::Error> {
    to_svg_to_string_inner(generate_qrcode(data, ecc)?, size, description)
}

/// Encode data to a SVG image in memory.
pub fn to_svg_to_string_by_segments(segments: &[QrSegment], ecc: QrCodeEcc, size: usize, description: Option<&str>) -> Result<String, io::Error> {
    to_svg_to_string_inner(generate_qrcode_by_segments(segments, ecc)?, size, description)
}

#[inline]
fn to_svg_to_file_inner<P: AsRef<Path>>(qr: QrCode, size: usize, description: Option<&str>, path: P) -> Result<(), io::Error> {
    let path = path.as_ref();

    let file = File::create(path)?;

    to_svg_inner(qr, size, description, file).map_err(|err| {
        if let Err(_) = fs::remove_file(path) {}
        err
    })
}

/// Encode data to a SVG image via a file path.
pub fn to_svg_to_file<D: AsRef<[u8]>, P: AsRef<Path>>(data: D, ecc: QrCodeEcc, size: usize, description: Option<&str>, path: P) -> Result<(), io::Error> {
    to_svg_to_file_inner(generate_qrcode(data, ecc)?, size, description, path)
}

/// Encode data to a SVG image via a file path.
pub fn to_svg_to_file_by_segments<P: AsRef<Path>>(segments: &[QrSegment], ecc: QrCodeEcc, size: usize, description: Option<&str>, path: P) -> Result<(), io::Error> {
    to_svg_to_file_inner(generate_qrcode_by_segments(segments, ecc)?, size, description, path)
}

#[inline]
fn to_image_inner(qr: QrCode, size: usize) -> Result<Vec<u8>, io::Error> {
    let margin_size = 1;

    let s = qr.size();

    let data_length = s as usize;

    let data_length_with_margin = data_length + 2 * margin_size;

    let point_size = size / data_length_with_margin;

    if point_size == 0 {
        return Err(io::Error::new(ErrorKind::Other, "the size is too small"));
    }

    let margin = (size - (point_size * data_length)) / 2;

    let length = size * size;

    let mut img_raw: Vec<u8> = vec![255u8; length];

    for i in 0..s {
        for j in 0..s {
            if qr.get_module(i, j) {
                let x = i as usize * point_size + margin;
                let y = j as usize * point_size + margin;

                for j in y..(y + point_size) {
                    let offset = j * size;
                    for i in x..(x + point_size) {
                        img_raw[offset + i] = 0;
                    }
                }
            }
        }
    }

    Ok(img_raw)
}

/// Encode data to image data stored in a Vec instance.
pub fn to_image<D: AsRef<[u8]>>(data: D, ecc: QrCodeEcc, size: usize) -> Result<Vec<u8>, io::Error> {
    to_image_inner(generate_qrcode(data, ecc)?, size)
}

/// Encode data to image data stored in a Vec instance.
pub fn to_image_by_segments(segments: &[QrSegment], ecc: QrCodeEcc, size: usize) -> Result<Vec<u8>, io::Error> {
    to_image_inner(generate_qrcode_by_segments(segments, ecc)?, size)
}

#[inline]
fn to_image_buffer_inner(qr: QrCode, size: usize) -> Result<ImageBuffer<Luma<u8>, Vec<u8>>, io::Error> {
    let img_raw = to_image_inner(qr, size)?;

    let img: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::from_vec(size as u32, size as u32, img_raw).unwrap();

    Ok(img)
}

/// Encode data to a image buffer.
pub fn to_image_buffer<D: AsRef<[u8]>>(data: D, ecc: QrCodeEcc, size: usize) -> Result<ImageBuffer<Luma<u8>, Vec<u8>>, io::Error> {
    to_image_buffer_inner(generate_qrcode(data, ecc)?, size)
}

/// Encode data to a image buffer.
pub fn to_image_buffer_by_segments(segments: &[QrSegment], ecc: QrCodeEcc, size: usize) -> Result<ImageBuffer<Luma<u8>, Vec<u8>>, io::Error> {
    to_image_buffer_inner(generate_qrcode_by_segments(segments, ecc)?, size)
}

#[inline]
fn to_png_inner<W: Write>(qr: QrCode, size: usize, writer: W) -> Result<(), io::Error> {
    let img_raw = to_image_inner(qr, size)?;

    let encoder = PNGEncoder::new(writer);

    encoder.encode(&img_raw, size as u32, size as u32, ColorType::Gray(8))
}

/// Encode data to a PNG image via any writer.
pub fn to_png<D: AsRef<[u8]>, W: Write>(data: D, ecc: QrCodeEcc, size: usize, writer: W) -> Result<(), io::Error> {
    to_png_inner(generate_qrcode(data, ecc)?, size, writer)
}

/// Encode data to a PNG image via any writer.
pub fn to_png_by_segments<W: Write>(segments: &[QrSegment], ecc: QrCodeEcc, size: usize, writer: W) -> Result<(), io::Error> {
    to_png_inner(generate_qrcode_by_segments(segments, ecc)?, size, writer)
}

#[inline]
fn to_png_to_vec_inner(qr: QrCode, size: usize) -> Result<Vec<u8>, io::Error> {
    let temp = RefCell::new(Some(Vec::new()));

    let temp_rc = Rc::new(temp);

    to_png_inner(qr, size, RcOptionWriter::new(temp_rc.clone()))?;

    let png = temp_rc.borrow_mut().take().unwrap();

    Ok(png)
}

/// Encode data to a PNG image in memory.
pub fn to_png_to_vec<D: AsRef<[u8]>>(data: D, ecc: QrCodeEcc, size: usize) -> Result<Vec<u8>, io::Error> {
    to_png_to_vec_inner(generate_qrcode(data, ecc)?, size)
}

/// Encode data to a PNG image in memory.
pub fn to_png_to_vec_by_segments(segments: &[QrSegment], ecc: QrCodeEcc, size: usize) -> Result<Vec<u8>, io::Error> {
    to_png_to_vec_inner(generate_qrcode_by_segments(segments, ecc)?, size)
}

#[inline]
fn to_png_to_file_inner<P: AsRef<Path>>(qr: QrCode, size: usize, path: P) -> Result<(), io::Error> {
    let path = path.as_ref();

    let file = File::create(path)?;

    to_png_inner(qr, size, file).map_err(|err| {
        if let Err(_) = fs::remove_file(path) {}
        err
    })
}

/// Encode data to a PNG image via a file path.
pub fn to_png_to_file<D: AsRef<[u8]>, P: AsRef<Path>>(data: D, ecc: QrCodeEcc, size: usize, path: P) -> Result<(), io::Error> {
    to_png_to_file_inner(generate_qrcode(data, ecc)?, size, path)
}

/// Encode data to a PNG image via a file path.
pub fn to_png_to_file_by_segments<P: AsRef<Path>>(segments: &[QrSegment], ecc: QrCodeEcc, size: usize, path: P) -> Result<(), io::Error> {
    to_png_to_file_inner(generate_qrcode_by_segments(segments, ecc)?, size, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;

    #[cfg(feature = "validator")]
    use validators::{ValidatorOption, http_url::HttpUrlValidator, http_ftp_url::HttpFtpUrlValidator};

    #[cfg(not(windows))]
    const FOLDER: &str = "tests/data";

    #[cfg(windows)]
    const FOLDER: &str = r"tests\data";

    #[cfg(feature = "validator")]
    #[test]
    fn text_optimize_with_validated_http_url_segments() {
        let validator = HttpUrlValidator {
            protocol: ValidatorOption::Allow,
            local: ValidatorOption::Allow,
        };

        let url = "https://magiclen.org/path/to/12345";

        let validated_http_url = validator.parse_str(url).unwrap();

        let matrix_1 = to_matrix(url, QrCodeEcc::Low).unwrap();
        let matrix_2 = to_matrix_by_segments(&optimize_validated_http_url_segments(&validated_http_url, QrCodeEcc::Low).unwrap(), QrCodeEcc::Low).unwrap();

        assert!(matrix_2.len() < matrix_1.len());
    }

    #[cfg(feature = "validator")]
    #[test]
    fn text_optimize_with_validated_http_ftp_url_segments() {
        let validator = HttpFtpUrlValidator {
            protocol: ValidatorOption::Allow,
            local: ValidatorOption::Allow,
        };

        let url = "https://magiclen.org/path/to/12345";

        let validated_http_ftp_url = validator.parse_str(url).unwrap();

        let matrix_1 = to_matrix(url, QrCodeEcc::Low).unwrap();
        let matrix_2 = to_matrix_by_segments(&optimize_validated_http_ftp_url_segments(&validated_http_ftp_url, QrCodeEcc::Low).unwrap(), QrCodeEcc::Low).unwrap();

        assert!(matrix_2.len() < matrix_1.len());
    }

    #[test]
    fn text_to_matrix() {
        let result = to_matrix("Hello world!", QrCodeEcc::Low).unwrap();

        assert_eq!(vec![
            vec![true, true, true, true, true, true, true, false, true, true, true, true, true, false, true, true, true, true, true, true, true],
            vec![true, false, false, false, false, false, true, false, true, false, true, false, true, false, true, false, false, false, false, false, true],
            vec![true, false, true, true, true, false, true, false, false, false, true, true, false, false, true, false, true, true, true, false, true],
            vec![true, false, true, true, true, false, true, false, true, false, true, true, true, false, true, false, true, true, true, false, true],
            vec![true, false, true, true, true, false, true, false, false, false, true, false, false, false, true, false, true, true, true, false, true],
            vec![true, false, false, false, false, false, true, false, false, true, true, false, false, false, true, false, false, false, false, false, true],
            vec![true, true, true, true, true, true, true, false, true, false, true, false, true, false, true, true, true, true, true, true, true],
            vec![false, false, false, false, false, false, false, false, true, true, false, false, false, false, false, false, false, false, false, false, false],
            vec![true, false, true, true, false, true, true, true, false, false, true, false, false, false, true, false, false, true, false, true, true],
            vec![false, false, true, true, false, false, false, true, false, true, true, false, true, true, true, true, true, true, true, false, true],
            vec![true, true, true, false, true, true, true, false, true, false, false, false, true, true, false, true, false, false, false, true, true],
            vec![false, true, true, true, true, true, false, true, false, false, true, false, true, false, false, true, false, true, false, true, false],
            vec![false, false, true, true, false, false, true, true, false, false, false, true, false, true, true, false, false, false, false, false, true],
            vec![false, false, false, false, false, false, false, false, true, true, true, true, false, false, true, true, true, false, true, false, true],
            vec![true, true, true, true, true, true, true, false, true, false, true, false, false, true, true, true, true, false, false, false, false],
            vec![true, false, false, false, false, false, true, false, true, true, true, true, true, true, false, true, false, true, true, false, false],
            vec![true, false, true, true, true, false, true, false, false, true, false, true, false, false, false, false, false, true, true, true, false],
            vec![true, false, true, true, true, false, true, false, true, false, true, false, true, false, true, false, false, true, true, true, false],
            vec![true, false, true, true, true, false, true, false, true, false, false, false, true, false, false, true, false, false, true, false, false],
            vec![true, false, false, false, false, false, true, false, false, true, false, true, false, true, true, true, true, false, false, false, true],
            vec![true, true, true, true, true, true, true, false, true, false, true, true, false, true, true, true, false, false, true, false, false]]
                   , result);
    }

    #[test]
    fn text_to_svg_to_string() {
        let result = to_svg_to_string("Hello world!", QrCodeEcc::Low, 256, Some("")).unwrap();

        assert_eq!(fs::read_to_string(Path::join(Path::new(FOLDER), "hello.svg")).unwrap(), result);
    }

    #[test]
    fn text_to_svg_to_file() {
        to_svg_to_file("Hello world!", QrCodeEcc::Low, 256, Some(""), Path::join(Path::new(FOLDER), "hello_output.svg")).unwrap();

        assert_eq!(fs::read(Path::join(Path::new(FOLDER), "hello.svg")).unwrap(), fs::read(Path::join(Path::new(FOLDER), "hello_output.svg")).unwrap());
    }

    #[test]
    fn text_to_png_to_vec() {
        let result = to_png_to_vec("Hello world!", QrCodeEcc::Low, 256).unwrap();

        assert_eq!(fs::read(Path::join(Path::new(FOLDER), "hello.png")).unwrap(), result);
    }

    #[test]
    fn text_to_png_to_file() {
        to_png_to_file("Hello world!", QrCodeEcc::Low, 256, Path::join(Path::new(FOLDER), "hello_output.png")).unwrap();

        assert_eq!(fs::read(Path::join(Path::new(FOLDER), "hello.png")).unwrap(), fs::read(Path::join(Path::new(FOLDER), "hello_output.png")).unwrap());
    }
}
