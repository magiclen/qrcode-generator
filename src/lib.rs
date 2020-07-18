/*!
# QR Code Generator

This crate provides functions to generate QR Code matrices and images in RAW, PNG and SVG formats.

## Examples

#### Encode any data to a QR Code matrix which is `Vec<Vec<bool>>`.

```rust
extern crate qrcode_generator;

use qrcode_generator::QrCodeEcc;

let result: Vec<Vec<bool>> = qrcode_generator::to_matrix("Hello world!", QrCodeEcc::Low).unwrap();

println!("{:?}", result);
```

#### Encode any data to a PNG image stored in a Vec instance.

```rust
extern crate qrcode_generator;

use qrcode_generator::QrCodeEcc;

# #[cfg(feature = "image")] {
let result: Vec<u8> = qrcode_generator::to_png_to_vec("Hello world!", QrCodeEcc::Low, 1024).unwrap();

println!("{:?}", result);
# }
```

#### Encode any data to a PNG image stored in a file.

```rust
extern crate qrcode_generator;

use qrcode_generator::QrCodeEcc;

# #[cfg(feature = "image")] {
qrcode_generator::to_png_to_file("Hello world!", QrCodeEcc::Low, 1024, "tests/data/file_output.png").unwrap();
# }
```

#### Encode any data to a SVG image stored in a String instance.

```rust
extern crate qrcode_generator;

use qrcode_generator::QrCodeEcc;

let result: String = qrcode_generator::to_svg_to_string("Hello world!", QrCodeEcc::Low, 1024, None::<&str>).unwrap();

println!("{:?}", result);
```

#### Encode any data to a SVG image stored in a file.

```rust
extern crate qrcode_generator;

use qrcode_generator::QrCodeEcc;

# #[cfg(feature = "std")] {
qrcode_generator::to_svg_to_file("Hello world!", QrCodeEcc::Low, 1024, None::<&str>, "tests/data/file_output.png").unwrap();
# }
```

## Low-level Usage

### Raw Image Data

The `to_image` and `to_image_buffer` functions can be used, if you want to modify your image.

### Segments

Every `to_*` function has a corresponding `_from_segments` function. You can concatenate segments by using different encoding methods, such as **numeric**, **alphanumeric** or **binary** to reduce the size (level) of your QR code matrix/image.

```rust
extern crate qrcode_generator;

use qrcode_generator::{QrCodeEcc, QrSegment};

let first = "1234567";

let second = "ABCDEFG";

let first_chars: Vec<char> = first.chars().collect();
let second_chars: Vec<char> = second.chars().collect();

let segments = [QrSegment::make_numeric(&first_chars), QrSegment::make_alphanumeric(&second_chars)];

let result: Vec<Vec<bool>> = qrcode_generator::to_matrix_from_segments(&segments, QrCodeEcc::Low).unwrap();

println!("{:?}", result);
```

## No Std

Disable the default features to compile this crate without std.

```toml
[dependencies.qrcode-generator]
version = "*"
default-features = false
```
*/

extern crate html_escape;
pub extern crate qrcodegen;

#[cfg(feature = "image")]
extern crate image_dep as image;

mod qr_code_error;

use core::str::from_utf8;

#[cfg(feature = "std")]
use std::fs::{self, File};
#[cfg(feature = "std")]
use std::io::Write;
#[cfg(feature = "std")]
use std::path::Path;

pub use qr_code_error::*;

use qrcodegen::QrCode;

pub use qrcodegen::{QrCodeEcc, QrSegment};

#[cfg(feature = "image")]
use image::png::{CompressionType, FilterType, PNGEncoder};

#[cfg(feature = "image")]
use image::{ColorType, ImageBuffer, Luma};

#[inline]
fn generate_qrcode<D: AsRef<[u8]>>(data: D, ecc: QrCodeEcc) -> Result<QrCode, QRCodeError> {
    match from_utf8(data.as_ref()) {
        Ok(text) => generate_qrcode_from_str(text, ecc),
        Err(_) => {
            let qr = match QrCode::encode_binary(data.as_ref(), ecc) {
                Ok(qr) => qr,
                Err(_) => return Err(QRCodeError::DataTooLong),
            };

            Ok(qr)
        }
    }
}

#[inline]
fn generate_qrcode_from_str<S: AsRef<str>>(text: S, ecc: QrCodeEcc) -> Result<QrCode, QRCodeError> {
    let qr = match QrCode::encode_text(text.as_ref(), ecc) {
        Ok(qr) => qr,
        Err(_) => return Err(QRCodeError::DataTooLong),
    };

    Ok(qr)
}

#[inline]
fn generate_qrcode_from_segments(
    segments: &[QrSegment],
    ecc: QrCodeEcc,
) -> Result<QrCode, QRCodeError> {
    let qr = match QrCode::encode_segments(segments, ecc) {
        Ok(qr) => qr,
        Err(_) => return Err(QRCodeError::DataTooLong),
    };

    Ok(qr)
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

#[cfg(feature = "std")]
#[inline]
fn to_svg_inner<S: AsRef<str>, W: Write>(
    qr: QrCode,
    size: usize,
    description: Option<S>,
    mut writer: W,
) -> Result<(), QRCodeError> {
    let margin_size = 1;

    let s = qr.size();

    let data_length = s as usize;

    let data_length_with_margin = data_length + 2 * margin_size;

    let point_size = size / data_length_with_margin;

    if point_size == 0 {
        return Err(QRCodeError::ImageSizeTooSmall);
    }

    let margin = (size - (point_size * data_length)) / 2;

    let size = format!("{}", size);

    writer.write_all(b"<?xml version=\"1.0\" encoding=\"utf-8\"?>")?;

    writer.write_all(b"<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"")?;

    writer.write_all(size.as_bytes())?;

    writer.write_all(b"\" height=\"")?;

    writer.write_all(size.as_bytes())?;

    writer.write_all(b"\">")?;

    match description {
        Some(description) => {
            let description = description.as_ref();

            if !description.is_empty() {
                writer.write_all(b"<desc>")?;
                html_escape::encode_safe_to_writer(description, &mut writer)?;
                writer.write_all(b"</desc>")?;
            }
        }
        None => {
            writer.write_all(b"<desc>")?;
            writer.write_all(env!("CARGO_PKG_NAME").as_bytes())?;
            writer.write_all(b" ")?;
            writer.write_all(env!("CARGO_PKG_VERSION").as_bytes())?;
            writer.write_all(b" by magiclen.org")?;
            writer.write_all(b"</desc>")?;
        }
    }

    writer.write_all(b"<rect width=\"")?;

    writer.write_all(size.as_bytes())?;

    writer.write_all(b"\" height=\"")?;

    writer.write_all(size.as_bytes())?;

    writer.write_all(b"\" fill=\"#FFFFFF\" cx=\"0\" cy=\"0\" />")?;

    let point_size_string = format!("{}", point_size);

    for i in 0..s {
        for j in 0..s {
            if qr.get_module(j, i) {
                let x = j as usize * point_size + margin;
                let y = i as usize * point_size + margin;

                writer.write_all(b"<rect x=\"")?;
                writer.write_all(x.to_string().as_bytes())?;

                writer.write_all(b"\" y=\"")?;
                writer.write_all(y.to_string().as_bytes())?;

                writer.write_all(b"\" width=\"")?;
                writer.write_all(point_size_string.as_bytes())?;

                writer.write_all(b"\" height=\"")?;
                writer.write_all(point_size_string.as_bytes())?;

                writer.write_all(b"\" fill=\"#000000\" shape-rendering=\"crispEdges\" />")?;
            }
        }
    }

    writer.write_all(b"</svg>")?;

    writer.flush()?;

    Ok(())
}

#[inline]
fn to_svg_to_vec_inner<S: AsRef<str>>(
    qr: QrCode,
    size: usize,
    description: Option<S>,
) -> Result<Vec<u8>, QRCodeError> {
    let mut svg = Vec::with_capacity(32768);

    let margin_size = 1;

    let s = qr.size();

    let data_length = s as usize;

    let data_length_with_margin = data_length + 2 * margin_size;

    let point_size = size / data_length_with_margin;

    if point_size == 0 {
        return Err(QRCodeError::ImageSizeTooSmall);
    }

    let margin = (size - (point_size * data_length)) / 2;

    let size = format!("{}", size);

    svg.extend_from_slice(b"<?xml version=\"1.0\" encoding=\"utf-8\"?>");

    svg.extend_from_slice(b"<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"");

    svg.extend_from_slice(size.as_bytes());

    svg.extend_from_slice(b"\" height=\"");

    svg.extend_from_slice(size.as_bytes());

    svg.extend_from_slice(b"\">");

    match description {
        Some(description) => {
            let description = description.as_ref();

            if !description.is_empty() {
                svg.extend_from_slice(b"<desc>");
                html_escape::encode_safe_to_writer(description, &mut svg)?;
                svg.extend_from_slice(b"</desc>");
            }
        }
        None => {
            svg.extend_from_slice(b"<desc>");
            svg.extend_from_slice(env!("CARGO_PKG_NAME").as_bytes());
            svg.extend_from_slice(b" ");
            svg.extend_from_slice(env!("CARGO_PKG_VERSION").as_bytes());
            svg.extend_from_slice(b" by magiclen.org");
            svg.extend_from_slice(b"</desc>");
        }
    }

    svg.extend_from_slice(b"<rect width=\"");

    svg.extend_from_slice(size.as_bytes());

    svg.extend_from_slice(b"\" height=\"");

    svg.extend_from_slice(size.as_bytes());

    svg.extend_from_slice(b"\" fill=\"#FFFFFF\" cx=\"0\" cy=\"0\" />");

    let point_size_string = format!("{}", point_size);

    for i in 0..s {
        for j in 0..s {
            if qr.get_module(j, i) {
                let x = j as usize * point_size + margin;
                let y = i as usize * point_size + margin;

                svg.extend_from_slice(b"<rect x=\"");
                svg.extend_from_slice(x.to_string().as_bytes());

                svg.extend_from_slice(b"\" y=\"");
                svg.extend_from_slice(y.to_string().as_bytes());

                svg.extend_from_slice(b"\" width=\"");
                svg.extend_from_slice(point_size_string.as_bytes());

                svg.extend_from_slice(b"\" height=\"");
                svg.extend_from_slice(point_size_string.as_bytes());

                svg.extend_from_slice(b"\" fill=\"#000000\" shape-rendering=\"crispEdges\" />");
            }
        }
    }

    svg.extend_from_slice(b"</svg>");

    Ok(svg)
}

#[inline]
fn to_svg_to_string_inner<S: AsRef<str>>(
    qr: QrCode,
    size: usize,
    description: Option<S>,
) -> Result<String, QRCodeError> {
    let svg = to_svg_to_vec_inner(qr, size, description)?;

    Ok(unsafe { String::from_utf8_unchecked(svg) })
}

#[cfg(feature = "std")]
#[inline]
fn to_svg_to_file_inner<S: AsRef<str>, P: AsRef<Path>>(
    qr: QrCode,
    size: usize,
    description: Option<S>,
    path: P,
) -> Result<(), QRCodeError> {
    let path = path.as_ref();

    let file = File::create(path)?;

    to_svg_inner(qr, size, description, file).map_err(|err| {
        if fs::remove_file(path).is_err() {}
        err
    })
}

#[inline]
fn to_image_inner(qr: QrCode, size: usize) -> Result<Vec<u8>, QRCodeError> {
    let margin_size = 1;

    let s = qr.size();

    let data_length = s as usize;

    let data_length_with_margin = data_length + 2 * margin_size;

    let point_size = size / data_length_with_margin;

    if point_size == 0 {
        return Err(QRCodeError::ImageSizeTooSmall);
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

#[cfg(feature = "image")]
#[inline]
fn to_png_inner<W: Write>(qr: QrCode, size: usize, writer: W) -> Result<(), QRCodeError> {
    let img_raw = to_image_inner(qr, size)?;

    let encoder = PNGEncoder::new_with_quality(writer, CompressionType::Best, FilterType::NoFilter);

    Ok(encoder.encode(&img_raw, size as u32, size as u32, ColorType::L8)?)
}

#[cfg(feature = "image")]
#[inline]
fn to_png_to_vec_inner(qr: QrCode, size: usize) -> Result<Vec<u8>, QRCodeError> {
    let mut png = Vec::with_capacity(4096);

    to_png_inner(qr, size, &mut png)?;

    Ok(png)
}

#[cfg(feature = "image")]
#[inline]
fn to_png_to_file_inner<P: AsRef<Path>>(
    qr: QrCode,
    size: usize,
    path: P,
) -> Result<(), QRCodeError> {
    let path = path.as_ref();

    let file = File::create(path)?;

    to_png_inner(qr, size, file).map_err(|err| {
        if fs::remove_file(path).is_err() {}
        err
    })
}

#[cfg(feature = "image")]
#[inline]
fn to_image_buffer_inner(
    qr: QrCode,
    size: usize,
) -> Result<ImageBuffer<Luma<u8>, Vec<u8>>, QRCodeError> {
    let img_raw = to_image_inner(qr, size)?;

    let img: ImageBuffer<Luma<u8>, Vec<u8>> =
        ImageBuffer::from_vec(size as u32, size as u32, img_raw).unwrap();

    Ok(img)
}

// TODO public functions

/// Encode data to a QR code matrix.
#[inline]
pub fn to_matrix<D: AsRef<[u8]>>(data: D, ecc: QrCodeEcc) -> Result<Vec<Vec<bool>>, QRCodeError> {
    Ok(to_matrix_inner(generate_qrcode(data, ecc)?))
}

/// Encode text to a QR code matrix.
#[inline]
pub fn to_matrix_from_str<S: AsRef<str>>(
    text: S,
    ecc: QrCodeEcc,
) -> Result<Vec<Vec<bool>>, QRCodeError> {
    Ok(to_matrix_inner(generate_qrcode_from_str(text, ecc)?))
}

/// Encode segments to a QR code matrix.
#[inline]
pub fn to_matrix_from_segments(
    segments: &[QrSegment],
    ecc: QrCodeEcc,
) -> Result<Vec<Vec<bool>>, QRCodeError> {
    Ok(to_matrix_inner(generate_qrcode_from_segments(segments, ecc)?))
}

/// Encode data to raw image in memory.
pub fn to_image<D: AsRef<[u8]>>(
    data: D,
    ecc: QrCodeEcc,
    size: usize,
) -> Result<Vec<u8>, QRCodeError> {
    to_image_inner(generate_qrcode(data, ecc)?, size)
}

/// Encode text to raw image in memory.
pub fn to_image_from_str<S: AsRef<str>>(
    text: S,
    ecc: QrCodeEcc,
    size: usize,
) -> Result<Vec<u8>, QRCodeError> {
    to_image_inner(generate_qrcode_from_str(text, ecc)?, size)
}

/// Encode segments to raw image in memory.
pub fn to_image_from_segments(
    segments: &[QrSegment],
    ecc: QrCodeEcc,
    size: usize,
) -> Result<Vec<u8>, QRCodeError> {
    to_image_inner(generate_qrcode_from_segments(segments, ecc)?, size)
}

/// Encode data to a SVG image in memory.
#[inline]
pub fn to_svg_to_string<D: AsRef<[u8]>, DESC: AsRef<str>>(
    data: D,
    ecc: QrCodeEcc,
    size: usize,
    description: Option<DESC>,
) -> Result<String, QRCodeError> {
    to_svg_to_string_inner(generate_qrcode(data, ecc)?, size, description)
}

/// Encode text to a SVG image in memory.
#[inline]
pub fn to_svg_to_string_from_str<S: AsRef<str>, DESC: AsRef<str>>(
    text: S,
    ecc: QrCodeEcc,
    size: usize,
    description: Option<DESC>,
) -> Result<String, QRCodeError> {
    to_svg_to_string_inner(generate_qrcode_from_str(text, ecc)?, size, description)
}

/// Encode segments to a SVG image in memory.
#[inline]
pub fn to_svg_to_string_from_segments<DESC: AsRef<str>>(
    segments: &[QrSegment],
    ecc: QrCodeEcc,
    size: usize,
    description: Option<DESC>,
) -> Result<String, QRCodeError> {
    to_svg_to_string_inner(generate_qrcode_from_segments(segments, ecc)?, size, description)
}

#[cfg(feature = "std")]
/// Encode data to a SVG image via a file path.
#[inline]
pub fn to_svg_to_file<D: AsRef<[u8]>, DESC: AsRef<str>, P: AsRef<Path>>(
    data: D,
    ecc: QrCodeEcc,
    size: usize,
    description: Option<DESC>,
    path: P,
) -> Result<(), QRCodeError> {
    to_svg_to_file_inner(generate_qrcode(data, ecc)?, size, description, path)
}

#[cfg(feature = "std")]
/// Encode text to a SVG image via a file path.
#[inline]
pub fn to_svg_to_file_from_str<S: AsRef<str>, DESC: AsRef<str>, P: AsRef<Path>>(
    text: S,
    ecc: QrCodeEcc,
    size: usize,
    description: Option<DESC>,
    path: P,
) -> Result<(), QRCodeError> {
    to_svg_to_file_inner(generate_qrcode_from_str(text, ecc)?, size, description, path)
}

#[cfg(feature = "std")]
/// Encode segments to a SVG image via a file path.
#[inline]
pub fn to_svg_to_file_from_segments<DESC: AsRef<str>, P: AsRef<Path>>(
    segments: &[QrSegment],
    ecc: QrCodeEcc,
    size: usize,
    description: Option<DESC>,
    path: P,
) -> Result<(), QRCodeError> {
    to_svg_to_file_inner(generate_qrcode_from_segments(segments, ecc)?, size, description, path)
}

#[cfg(feature = "std")]
/// Encode data to a SVG image via a writer.
#[inline]
pub fn to_svg_to_writer<D: AsRef<[u8]>, DESC: AsRef<str>, W: Write>(
    data: D,
    ecc: QrCodeEcc,
    size: usize,
    description: Option<DESC>,
    writer: &mut W,
) -> Result<(), QRCodeError> {
    to_svg_inner(generate_qrcode(data, ecc)?, size, description, writer)
}

#[cfg(feature = "std")]
/// Encode text to a SVG image via a writer.
#[inline]
pub fn to_svg_to_writer_from_str<S: AsRef<str>, DESC: AsRef<str>, W: Write>(
    text: S,
    ecc: QrCodeEcc,
    size: usize,
    description: Option<DESC>,
    writer: &mut W,
) -> Result<(), QRCodeError> {
    to_svg_inner(generate_qrcode_from_str(text, ecc)?, size, description, writer)
}

#[cfg(feature = "std")]
/// Encode segments to a SVG image via a writer.
#[inline]
pub fn to_svg_to_writer_from_segments<DESC: AsRef<str>, W: Write>(
    segments: &[QrSegment],
    ecc: QrCodeEcc,
    size: usize,
    description: Option<DESC>,
    writer: &mut W,
) -> Result<(), QRCodeError> {
    to_svg_inner(generate_qrcode_from_segments(segments, ecc)?, size, description, writer)
}

#[cfg(feature = "image")]
/// Encode data to a PNG image in memory.
#[inline]
pub fn to_png_to_vec<D: AsRef<[u8]>>(
    data: D,
    ecc: QrCodeEcc,
    size: usize,
) -> Result<Vec<u8>, QRCodeError> {
    to_png_to_vec_inner(generate_qrcode(data, ecc)?, size)
}

#[cfg(feature = "image")]
/// Encode text to a PNG image in memory.
#[inline]
pub fn to_png_to_vec_from_str<S: AsRef<str>>(
    text: S,
    ecc: QrCodeEcc,
    size: usize,
) -> Result<Vec<u8>, QRCodeError> {
    to_png_to_vec_inner(generate_qrcode_from_str(text, ecc)?, size)
}

#[cfg(feature = "image")]
/// Encode segments to a PNG image in memory.
#[inline]
pub fn to_png_to_vec_from_segments(
    segments: &[QrSegment],
    ecc: QrCodeEcc,
    size: usize,
) -> Result<Vec<u8>, QRCodeError> {
    to_png_to_vec_inner(generate_qrcode_from_segments(segments, ecc)?, size)
}

#[cfg(feature = "image")]
/// Encode data to a PNG image via a file path.
#[inline]
pub fn to_png_to_file<D: AsRef<[u8]>, P: AsRef<Path>>(
    data: D,
    ecc: QrCodeEcc,
    size: usize,
    path: P,
) -> Result<(), QRCodeError> {
    to_png_to_file_inner(generate_qrcode(data, ecc)?, size, path)
}

#[cfg(feature = "image")]
/// Encode text to a PNG image via a file path.
#[inline]
pub fn to_png_to_file_from_str<S: AsRef<str>, P: AsRef<Path>>(
    text: S,
    ecc: QrCodeEcc,
    size: usize,
    path: P,
) -> Result<(), QRCodeError> {
    to_png_to_file_inner(generate_qrcode_from_str(text, ecc)?, size, path)
}

#[cfg(feature = "image")]
/// Encode text to a PNG image via a file path.
#[inline]
pub fn to_png_to_file_from_segments<P: AsRef<Path>>(
    segments: &[QrSegment],
    ecc: QrCodeEcc,
    size: usize,
    path: P,
) -> Result<(), QRCodeError> {
    to_png_to_file_inner(generate_qrcode_from_segments(segments, ecc)?, size, path)
}

#[cfg(feature = "image")]
/// Encode data to a PNG image via a writer.
#[inline]
pub fn to_png_to_writer<D: AsRef<[u8]>, W: Write>(
    data: D,
    ecc: QrCodeEcc,
    size: usize,
    writer: &mut W,
) -> Result<(), QRCodeError> {
    to_png_inner(generate_qrcode(data, ecc)?, size, writer)
}

#[cfg(feature = "image")]
/// Encode text to a PNG image via a writer.
#[inline]
pub fn to_png_to_writer_from_str<S: AsRef<str>, W: Write>(
    text: S,
    ecc: QrCodeEcc,
    size: usize,
    writer: &mut W,
) -> Result<(), QRCodeError> {
    to_png_inner(generate_qrcode_from_str(text, ecc)?, size, writer)
}

#[cfg(feature = "image")]
/// Encode segments to a PNG image via a writer.
#[inline]
pub fn to_png_to_writer_from_segments<W: Write>(
    segments: &[QrSegment],
    ecc: QrCodeEcc,
    size: usize,
    writer: &mut W,
) -> Result<(), QRCodeError> {
    to_png_inner(generate_qrcode_from_segments(segments, ecc)?, size, writer)
}

#[cfg(feature = "image")]
/// Encode data to a image buffer.
pub fn to_image_buffer<D: AsRef<[u8]>>(
    data: D,
    ecc: QrCodeEcc,
    size: usize,
) -> Result<ImageBuffer<Luma<u8>, Vec<u8>>, QRCodeError> {
    to_image_buffer_inner(generate_qrcode(data, ecc)?, size)
}

#[cfg(feature = "image")]
/// Encode text to a image buffer.
pub fn to_image_buffer_from_str<S: AsRef<str>>(
    text: S,
    ecc: QrCodeEcc,
    size: usize,
) -> Result<ImageBuffer<Luma<u8>, Vec<u8>>, QRCodeError> {
    to_image_buffer_inner(generate_qrcode_from_str(text, ecc)?, size)
}

#[cfg(feature = "image")]
/// Encode segments to a image buffer.
pub fn to_image_buffer_from_segments<S: AsRef<str>>(
    segments: &[QrSegment],
    ecc: QrCodeEcc,
    size: usize,
) -> Result<ImageBuffer<Luma<u8>, Vec<u8>>, QRCodeError> {
    to_image_buffer_inner(generate_qrcode_from_segments(segments, ecc)?, size)
}
