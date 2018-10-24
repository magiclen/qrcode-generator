pub extern crate qrcodegen;
pub extern crate htmlescape;
pub extern crate image;
pub extern crate rc_writer;

#[cfg(feature = "validator")]
pub extern crate validators;

use std::io::{self, Write, ErrorKind};

use std::rc::Rc;
use std::cell::RefCell;
use std::fs::{self, File};
use std::path::Path;

use qrcodegen::{QrCode, QrCodeEcc};

use image::{ImageBuffer, Luma, png::PNGEncoder, ColorType};

use rc_writer::RcOptionWriter;

#[cfg(feature = "validator")]
use validators::http_url::HttpUrl;

#[cfg(feature = "validator")]
/// Optimize URL text for generating QR code.
pub fn optimize_with_validated_http_url(url: &HttpUrl) -> String {
    let mut text = String::with_capacity(url.get_full_http_url().len());

    if let Some(path) = url.get_path() {
        let a = url.get_full_http_url_without_query_and_fragment();
        text.push_str(&a[..(a.len() - path.len())].to_uppercase());
        text.push_str(path);
    } else {
        text.push_str(&url.get_full_http_url_without_query_and_fragment().to_uppercase());
    }

    if let Some(query) = url.get_query() {
        text.push('?');
        text.push_str(query);
    }

    if let Some(fragment) = url.get_fragment() {
        text.push('#');
        text.push_str(fragment);
    }

    text
}

/// Optimize URL text for generating QR code.
pub fn optimize_url(url: &str) -> String {
    let protocol_sep_index = url.find("://");

    match protocol_sep_index {
        Some(protocol_sep_index) => {
            let next_slash_index = &url[protocol_sep_index + 3..].find("/");

            match next_slash_index {
                Some(next_slash_index) => {
                    let next_slash_index = next_slash_index + protocol_sep_index + 3;
                    format!("{}{}", &url[..next_slash_index].to_uppercase(), &url[next_slash_index..])
                }
                None => {
                    url.to_uppercase()
                }
            }
        }
        None => {
            url.to_string()
        }
    }
}

/// Optimize URL text for generating QR code.
pub fn optimize_url_owned(url: String) -> String {
    let protocol_sep_index = url.find("://");

    match protocol_sep_index {
        Some(protocol_sep_index) => {
            let next_slash_index = &url[protocol_sep_index + 3..].find("/");

            match next_slash_index {
                Some(next_slash_index) => {
                    let next_slash_index = next_slash_index + protocol_sep_index + 3;
                    format!("{}{}", &url[..next_slash_index].to_uppercase(), &url[next_slash_index..])
                }
                None => {
                    url.to_uppercase()
                }
            }
        }
        None => {
            url
        }
    }
}

/// Encode data to a QR code matrix.
pub fn to_matrix<D: AsRef<[u8]>>(data: D, ecc: QrCodeEcc) -> Result<Vec<Vec<bool>>, &'static str> {
    let qr = match QrCode::encode_binary(data.as_ref(), ecc) {
        Some(qr) => qr,
        None => return Err("The data is too long!")
    };

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

    Ok(rows)
}

/// Encode data to a SVG image via any writer.
pub fn to_svg<D: AsRef<[u8]>, W: Write>(data: D, ecc: QrCodeEcc, size: usize, description: Option<&str>, mut writer: W) -> Result<(), io::Error> {
    let margin_size = 1;

    let qr = match QrCode::encode_binary(data.as_ref(), ecc) {
        Some(qr) => qr,
        None => return Err(io::Error::new(ErrorKind::Other, "the data is too long"))
    };

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

/// Encode data to a SVG image in memory.
pub fn to_svg_to_string<D: AsRef<[u8]>>(data: D, ecc: QrCodeEcc, size: usize, description: Option<&str>) -> Result<String, io::Error> {
    let temp = RefCell::new(Some(Vec::new()));

    let temp_rc = Rc::new(temp);

    to_svg(data, ecc, size, description, RcOptionWriter::new(temp_rc.clone()))?;

    let svg = temp_rc.borrow_mut().take().unwrap();

    Ok(unsafe { String::from_utf8_unchecked(svg) })
}

/// Encode data to a SVG image via a file path.
pub fn to_svg_to_file<D: AsRef<[u8]>, P: AsRef<Path>>(data: D, ecc: QrCodeEcc, size: usize, description: Option<&str>, path: P) -> Result<(), io::Error> {
    let path = path.as_ref();

    let file = File::create(path)?;

    to_svg(data, ecc, size, description, file).map_err(|err| {
        if let Err(_) = fs::remove_file(path) {}
        err
    })
}

/// Encode data to image data stored in a Vec instance.
pub fn to_image<D: AsRef<[u8]>>(data: D, ecc: QrCodeEcc, size: usize) -> Result<Vec<u8>, io::Error> {
    let margin_size = 1;

    let qr = match QrCode::encode_binary(data.as_ref(), ecc) {
        Some(qr) => qr,
        None => return Err(io::Error::new(ErrorKind::Other, "the data is too long"))
    };

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

/// Encode data to a image buffer.
pub fn to_image_buffer<D: AsRef<[u8]>>(data: D, ecc: QrCodeEcc, size: usize) -> Result<ImageBuffer<Luma<u8>, Vec<u8>>, io::Error> {
    let img_raw = to_image(data, ecc, size)?;

    let img: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::from_vec(size as u32, size as u32, img_raw).unwrap();

    Ok(img)
}

/// Encode data to a PNG image via any writer.
pub fn to_png<D: AsRef<[u8]>, W: Write>(data: D, ecc: QrCodeEcc, size: usize, writer: W) -> Result<(), io::Error> {
    let img_raw = to_image(data, ecc, size)?;

    let encoder = PNGEncoder::new(writer);

    encoder.encode(&img_raw, size as u32, size as u32, ColorType::Gray(8))
}

/// Encode data to a PNG image in memory.
pub fn to_png_to_vec<D: AsRef<[u8]>>(data: D, ecc: QrCodeEcc, size: usize) -> Result<Vec<u8>, io::Error> {
    let temp = RefCell::new(Some(Vec::new()));

    let temp_rc = Rc::new(temp);

    to_png(data, ecc, size, RcOptionWriter::new(temp_rc.clone()))?;

    let png = temp_rc.borrow_mut().take().unwrap();

    Ok(png)
}

/// Encode data to a PNG image via a file path.
pub fn to_png_to_file<D: AsRef<[u8]>, P: AsRef<Path>>(data: D, ecc: QrCodeEcc, size: usize, path: P) -> Result<(), io::Error> {
    let path = path.as_ref();

    let file = File::create(path)?;

    to_png(data, ecc, size, file).map_err(|err| {
        if let Err(_) = fs::remove_file(path) {}
        err
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;

    #[cfg(feature = "validator")]
    use validators::{ValidatorOption, http_url::HttpUrlValidator};

    #[test]
    fn text_optimize_url_lv1() {
        assert_eq!("HTTPS://MAGICLEN.ORG", optimize_url("https://magiclen.org"));
    }

    #[test]
    fn text_optimize_url_lv2() {
        assert_eq!("HTTPS://MAGICLEN.ORG/path?a=10#hi", optimize_url("https://magiclen.org/path?a=10#hi"));
    }

    #[test]
    fn text_optimize_url_owned_lv1() {
        let url = "https://magiclen.org".to_string();

        assert_eq!("HTTPS://MAGICLEN.ORG", optimize_url_owned(url));
    }

    #[test]
    fn text_optimize_url_owned_lv2() {
        let url = "https://magiclen.org/path?a=10#hi".to_string();

        assert_eq!("HTTPS://MAGICLEN.ORG/path?a=10#hi", optimize_url_owned(url));
    }

    #[cfg(feature = "validator")]
    #[test]
    fn text_optimize_with_validated_http_url_lv1() {
        let validator = HttpUrlValidator {
            protocol: ValidatorOption::Allow,
            local: ValidatorOption::Allow,
        };

        let url = validator.parse_str("https://magiclen.org").unwrap();

        assert_eq!("HTTPS://MAGICLEN.ORG", optimize_with_validated_http_url(&url));
    }

    #[cfg(feature = "validator")]
    #[test]
    fn text_optimize_with_validated_http_url_lv2() {
        let validator = HttpUrlValidator {
            protocol: ValidatorOption::Allow,
            local: ValidatorOption::Allow,
        };

        let url = validator.parse_str("https://magiclen.org/path?a=10#hi").unwrap();

        assert_eq!("HTTPS://MAGICLEN.ORG/path?a=10#hi", optimize_with_validated_http_url(&url));
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

        assert_eq!(fs::read_to_string("tests/data/hello.svg").unwrap(), result);
    }

    #[test]
    fn text_to_svg_to_file() {
        to_svg_to_file("Hello world!", QrCodeEcc::Low, 256, Some(""), "tests/data/hello_output.svg").unwrap();

        assert_eq!(fs::read("tests/data/hello.svg").unwrap(), fs::read("tests/data/hello_output.svg").unwrap());
    }

    #[test]
    fn text_to_png_to_vec() {
        assert_eq!(fs::read("tests/data/hello.png").unwrap(), to_png_to_vec("Hello world!", QrCodeEcc::Low, 256).unwrap());
    }

    #[test]
    fn text_to_png_to_file() {
        to_png_to_file("Hello world!", QrCodeEcc::Low, 256, "tests/data/hello_output.png").unwrap();

        assert_eq!(fs::read("tests/data/hello.png").unwrap(), fs::read("tests/data/hello_output.png").unwrap());
    }
}
