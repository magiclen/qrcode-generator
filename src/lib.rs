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
//! ### Optimized URL segments
//!
//! URL is a common type of data used in QR code. The protocol and the host of a URL is case-insensitive, so they can be converted to a upper-case segment and encoded by **alphanumeric** instead of **binary** to reduce the size.
//!
//! You can use the `optimize_url_segments` function to create URL segments.
//!
//! ```
//! extern crate qrcode_generator;
//!
//! use qrcode_generator::QrCodeEcc;
//!
//!
//! let url = "https://magiclen.org/path/to/12345";
//!
//! let matrix_1 = qrcode_generator::to_matrix(url, QrCodeEcc::Low).unwrap();
//! let matrix_2 = qrcode_generator::to_matrix_by_segments(&qrcode_generator::optimize_url_segments(url), QrCodeEcc::Low).unwrap();
//!
//! assert!(matrix_2.len() < matrix_1.len());
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
//! And the `optimize_validated_http_url_segments` function is available.
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
//! let matrix_2 = qrcode_generator::to_matrix_by_segments(&qrcode_generator::optimize_validated_http_url_segments(&validated_http_url), QrCodeEcc::Low).unwrap();
//!
//! assert!(matrix_2.len() < matrix_1.len());
//! ```

pub extern crate qrcodegen;
extern crate htmlescape;
extern crate image;
extern crate rc_writer;

#[cfg(feature = "validator")]
pub extern crate validators;

use std::io::{self, Write, ErrorKind};

use std::rc::Rc;
use std::cell::RefCell;
use std::fs::{self, File};
use std::path::Path;

use qrcodegen::{QrCode, QrSegment};

pub use qrcodegen::QrCodeEcc;

use image::{ImageBuffer, Luma, png::PNGEncoder, ColorType};

use rc_writer::RcOptionWriter;

#[cfg(feature = "validator")]
use validators::http_url::HttpUrl;

// TODO -----START-----
// Temporary implement.
// Refer to: https://github.com/nayuki/QR-Code-generator/blob/master/java/io/nayuki/qrcodegen/QrSegmentAdvanced.java

static SORTED_ALPHANUMERIC_CHARSET: [char; 45] = [' ', '.', '$', '%', '*', '+', '-', '/',
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
    ':',
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
];

static ECC_CODEWORDS_PER_BLOCK: [[i8; 41]; 4] = [
    [-1, 7, 10, 15, 20, 26, 18, 20, 24, 30, 18, 20, 24, 26, 30, 22, 24, 28, 30, 28, 28, 28, 28, 30, 30, 26, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30],  // Low
    [-1, 10, 16, 26, 18, 24, 16, 18, 22, 22, 26, 30, 22, 22, 24, 24, 28, 28, 26, 26, 26, 26, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28],  // Medium
    [-1, 13, 22, 18, 26, 18, 24, 18, 22, 20, 24, 28, 26, 24, 20, 30, 24, 28, 28, 26, 30, 28, 30, 30, 30, 30, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30],  // Quartile
    [-1, 17, 28, 22, 16, 22, 28, 26, 26, 24, 28, 24, 28, 22, 24, 24, 30, 28, 28, 26, 28, 30, 24, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30],  // High
];

static NUM_ERROR_CORRECTION_BLOCKS: [[i8; 41]; 4] = [
    [-1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 4, 4, 4, 4, 4, 6, 6, 6, 6, 7, 8, 8, 9, 9, 10, 12, 12, 12, 13, 14, 15, 16, 17, 18, 19, 19, 20, 21, 22, 24, 25],  // Low
    [-1, 1, 1, 1, 2, 2, 4, 4, 4, 5, 5, 5, 8, 9, 9, 10, 10, 11, 13, 14, 16, 17, 17, 18, 20, 21, 23, 25, 26, 28, 29, 31, 33, 35, 37, 38, 40, 43, 45, 47, 49],  // Medium
    [-1, 1, 1, 2, 2, 4, 4, 6, 6, 8, 8, 8, 10, 12, 16, 12, 17, 16, 18, 21, 20, 23, 23, 25, 27, 29, 34, 34, 35, 38, 40, 43, 45, 48, 51, 53, 56, 59, 62, 65, 68],  // Quartile
    [-1, 1, 1, 2, 4, 4, 4, 5, 6, 8, 8, 11, 11, 16, 16, 18, 16, 19, 21, 25, 25, 25, 34, 30, 32, 35, 37, 40, 42, 45, 48, 51, 54, 57, 60, 63, 66, 70, 74, 77, 81],  // High
];

#[inline]
fn is_numeric(c: char) -> bool {
    '0' <= c && c <= '9'
}

#[inline]
fn is_alphanumeric(c: char) -> bool {
    SORTED_ALPHANUMERIC_CHARSET.binary_search(&c).is_ok()
}

const NUMERIC: [usize; 4] = [0x1, 10, 12, 14];
const ALPHANUMERIC: [usize; 4] = [0x2, 9, 11, 13];
const BYTE: [usize; 4] = [0x4, 8, 16, 16];

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Numeric,
    Alphanumeric,
    Byte,
}

impl Mode {
    fn get_mode_bits(&self) -> usize {
        match self {
            Numeric => {
                NUMERIC[0]
            }
            Alphanumeric => {
                ALPHANUMERIC[0]
            }
            Byte => {
                BYTE[0]
            }
        }
    }

    fn get_num_char_count_bits(&self, version: usize) -> usize {
        let index = (version + 7) / 17;
        match self {
            Numeric => {
                NUMERIC[index]
            }
            Alphanumeric => {
                ALPHANUMERIC[index]
            }
            Byte => {
                BYTE[index]
            }
        }
    }
}

const NUM_MODES: usize = 3;

#[inline]
fn table_get(table: &'static [[i8; 41]; 4], ver: usize, ecc: QrCodeEcc) -> usize {
    let ordinal = match ecc {
        QrCodeEcc::Low => 0,
        QrCodeEcc::Medium => 1,
        QrCodeEcc::Quartile => 2,
        QrCodeEcc::High => 3,
    };

    table[ordinal][ver] as usize
}

#[inline]
fn get_num_raw_data_modules(ver: usize) -> usize {
    let mut result: usize = (16 * ver + 128) * ver + 64;
    if ver >= 2 {
        let numalign: usize = ver / 7 + 2;
        result -= (25 * numalign - 10) * numalign - 55;
        if ver >= 7 {
            result -= 36;
        }
    }
    result
}


#[inline]
fn get_num_data_codewords(ver: usize, ecl: QrCodeEcc) -> usize {
    get_num_raw_data_modules(ver) / 8 - table_get(&ECC_CODEWORDS_PER_BLOCK, ver, ecl) * table_get(&NUM_ERROR_CORRECTION_BLOCKS, ver, ecl)
}

#[inline]
fn compute_character_modes(code_points: &[char], version: usize) -> Vec<Mode> {
    let mode_types = [Mode::Byte, Mode::Alphanumeric, Mode::Numeric];

    let mut head_costs = [0usize; NUM_MODES];

    for i in 0..NUM_MODES {
        head_costs[i] = (4 + mode_types[i].get_num_char_count_bits(version)) * 6;
    }

    let mut char_modes = {
        let mut out = Vec::new();

        for _ in 0..code_points.len() {
            out.push([None::<Mode>; NUM_MODES]);
        }

        out
    };

    let mut prev_costs = head_costs.clone();

    for i in 0..code_points.len() {
        let c = code_points[i];
        let mut cur_costs = [0usize; NUM_MODES];
        {
            cur_costs[0] = prev_costs[0] + c.len_utf8() * 8 * 6;
            char_modes[i][0] = Some(mode_types[0]);
        }
        if is_alphanumeric(c) {
            cur_costs[1] = prev_costs[1] + 33;
            char_modes[i][1] = Some(mode_types[1]);
        }
        if is_numeric(c) {
            cur_costs[2] = prev_costs[2] + 20;
            char_modes[i][2] = Some(mode_types[2]);
        }

        for j in 0..NUM_MODES {
            for k in 0..NUM_MODES {
                let new_cost = (cur_costs[k] + 5) / 6 * 6 + head_costs[j];
                if char_modes[i][k].is_some() && (char_modes[i][j].is_none() || new_cost < cur_costs[j]) {
                    cur_costs[j] = new_cost;
                    char_modes[i][j] = Some(mode_types[k]);
                }
            }
        }

        prev_costs = cur_costs;
    }

    let mut cur_mode = None::<Mode>;

    let mut min_cost = 0;

    for i in 0..NUM_MODES {
        if cur_mode.is_none() || prev_costs[i] < min_cost {
            min_cost = prev_costs[i];
            cur_mode = Some(mode_types[i]);
        }
    }

    let mut cur_mode = cur_mode.unwrap();

    let mut result = Vec::with_capacity(char_modes.len());

    for i in (0..char_modes.len()).rev() {
        for j in 0..NUM_MODES {
            if mode_types[j] == cur_mode {
                cur_mode = char_modes[i][j].unwrap();
                result.push(cur_mode);
                break;
            }
        }
    }

    result
}

fn split_into_segments(code_points: &[char], char_modes: &[Mode]) -> Vec<QrSegment> {
    let mut result = Vec::new();

    let mut cur_mode = char_modes[0];

    let mut start = 0;

    let mut i = 0;
    loop {
        i += 1;
        if i < code_points.len() && char_modes[i] == cur_mode {
            continue;
        }

        let s = &code_points[start..(i - start)];

        match cur_mode {
            Mode::Byte => {
                let s: String = s.iter().collect();
                let v = s.into_bytes();
                result.push(QrSegment::make_bytes(&v));
            }
            Mode::Numeric => {
                result.push(QrSegment::make_numeric(s));
            }
            Mode::Alphanumeric => {
                result.push(QrSegment::make_alphanumeric(s));
            }
        }

        if i >= code_points.len() {
            return result;
        }

        cur_mode = char_modes[i];
        start = i;
    }

    result
}

#[inline]
fn make_segments_optimally_version(code_points: &[char], version: usize) -> Vec<QrSegment> {
    let char_modes = compute_character_modes(code_points, version);
    split_into_segments(code_points, &char_modes)
}


#[inline]
fn make_segments_optimally(code_points: &[char], ecc: QrCodeEcc) -> Vec<QrSegment> {
    let mut segs = Vec::new();

    for version in 1..=40 {
        if version == 1 || version == 10 || version == 27 {
            segs = make_segments_optimally_version(code_points, version);
        }

        let data_capacity_bits = get_num_data_codewords(version, ecc) * 8;
        // TODO
    }

    vec![]
}


#[inline]
fn make_segments(text: &[char]) -> Vec<QrSegment> {
    QrSegment::make_segments(&text)
}

// TODO -----END-----

#[cfg(feature = "validator")]
/// Optimize URL text for generating QR code.
pub fn optimize_validated_http_url_segments(http_url: &HttpUrl) -> Vec<QrSegment> {
    let host = http_url.get_host().get_full_host();
    let url = http_url.get_full_http_url();

    let first = if http_url.get_path().is_some() {
        url[..(host.len() + 1)].to_uppercase()
    } else {
        url[..host.len()].to_uppercase()
    };

    let first_chars: Vec<char> = first.chars().collect();

    let mut out = make_segments(&first_chars);

    let second = &url[first.len()..];

    let second_chars: Vec<char> = second.chars().collect();

    out.extend_from_slice(&make_segments(&second_chars));

    out
}

/// Optimize URL text for generating QR code.
pub fn optimize_url_segments<S: AsRef<str>>(url: S) -> Vec<QrSegment> {
    let url: &str = url.as_ref();

    let protocol_sep_index = url.find("://");

    match protocol_sep_index {
        Some(protocol_sep_index) => {
            let next_slash_index = &url[protocol_sep_index + 3..].find("/");

            match next_slash_index {
                Some(next_slash_index) => {
                    let next_slash_index = next_slash_index + protocol_sep_index + 4;

                    let first = url[..next_slash_index].to_uppercase();

                    let first_chars: Vec<char> = first.chars().collect();

                    let mut out = make_segments(&first_chars);

                    let second = &url[next_slash_index..];

                    let second_chars: Vec<char> = second.chars().collect();

                    out.extend_from_slice(&make_segments(&second_chars));

                    out
                }
                None => {
                    let chars: Vec<char> = url.to_uppercase().chars().collect();

                    make_segments(&chars)
                }
            }
        }
        None => {
            let chars: Vec<char> = url.chars().collect();

            make_segments(&chars)
        }
    }
}

#[inline]
fn generate_qrcode<D: AsRef<[u8]>>(data: D, ecc: QrCodeEcc) -> Result<QrCode, io::Error> {
    let data = data.as_ref();

    let tried_utf8 = String::from_utf8(data.to_vec());

    match tried_utf8 {
        Ok(text) => {
            let qr = match QrCode::encode_text(text.as_str(), ecc) {
                Some(qr) => qr,
                None => return Err(io::Error::new(ErrorKind::Other, "the data is too long"))
            };

            Ok(qr)
        }
        Err(_) => {
            let qr = match QrCode::encode_binary(data, ecc) {
                Some(qr) => qr,
                None => return Err(io::Error::new(ErrorKind::Other, "the data is too long"))
            };

            Ok(qr)
        }
    }
}

#[inline]
fn generate_qrcode_by_segments(segments: &[QrSegment], ecc: QrCodeEcc) -> Result<QrCode, io::Error> {
    match QrCode::encode_segments(segments, ecc) {
        Some(qr) => Ok(qr),
        None => Err(io::Error::new(ErrorKind::Other, "the data is too long"))
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
    use validators::{ValidatorOption, http_url::HttpUrlValidator};

    #[cfg(not(windows))]
    const FOLDER: &str = "tests/data";

    #[cfg(windows)]
    const FOLDER: &str = r"tests\data";

    #[test]
    fn text_optimize_url() {
        let url = "https://magiclen.org/path/to/12345";

        let matrix_1 = to_matrix(url, QrCodeEcc::Low).unwrap();
        let matrix_2 = to_matrix_by_segments(&optimize_url_segments(url), QrCodeEcc::Low).unwrap();

        assert!(matrix_2.len() < matrix_1.len());
    }

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
        let matrix_2 = to_matrix_by_segments(&optimize_validated_http_url_segments(&validated_http_url), QrCodeEcc::Low).unwrap();

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
