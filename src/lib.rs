pub extern crate qrcodegen;
pub extern crate htmlescape;
pub extern crate image;

#[cfg(feature = "validator")]
pub extern crate validators;

use std::io::{self, Write, ErrorKind};

use qrcodegen::{QrCode, QrCodeEcc};

use image::{ImageBuffer, Luma, png::PNGEncoder, ColorType};

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

/// Encode data to a QR code SVG image.
pub fn to_svg<D: AsRef<[u8]>>(data: D, ecc: QrCodeEcc, size: usize, description: Option<&str>) -> Result<String, io::Error> {
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

    let mut sb = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>");

    sb.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"");

    sb.push_str(&size);

    sb.push_str("\" height=\"");

    sb.push_str(&size);

    sb.push_str("\">");

    match description {
        Some(description) => {
            if !description.is_empty() {
                sb.push_str("<desc>");
                sb.push_str(&htmlescape::encode_minimal(description));
                sb.push_str("</desc>");
            }
        }
        None => {
            sb.push_str("<desc>");
            sb.push_str(env!("CARGO_PKG_NAME"));
            sb.push(' ');
            sb.push_str(env!("CARGO_PKG_VERSION"));
            sb.push_str(" by magiclen.org");
            sb.push_str("</desc>");
        }
    }

    sb.push_str("<rect width=\"");

    sb.push_str(&size);

    sb.push_str("\" height=\"");

    sb.push_str(&size);

    sb.push_str("\" fill=\"#FFFFFF\" cx=\"0\" cy=\"0\" />");

    let point_size_string = point_size.to_string();

    for i in 0..s {
        for j in 0..s {
            if qr.get_module(j, i) {
                let x = j as usize * point_size + margin;
                let y = i as usize * point_size + margin;

                sb.push_str("<rect x=\"");
                sb.push_str(&x.to_string());

                sb.push_str("\" y=\"");
                sb.push_str(&y.to_string());

                sb.push_str("\" width=\"");
                sb.push_str(&point_size_string);

                sb.push_str("\" height=\"");
                sb.push_str(&point_size_string);

                sb.push_str("\" fill=\"#000000\" shape-rendering=\"crispEdges\" />");
            }
        }
    }

    sb.push_str("</svg>");

    Ok(sb)
}

/// Encode data to a Vec instance.
pub fn to_vec<D: AsRef<[u8]>>(data: D, ecc: QrCodeEcc, size: usize) -> Result<Vec<u8>, io::Error> {
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
    let img_raw = to_vec(data, ecc, size)?;

    let img: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::from_vec(size as u32, size as u32, img_raw).unwrap();

    Ok(img)
}

/// Encode data to a PNG image.
pub fn to_png<D: AsRef<[u8]>, W: Write>(data: D, ecc: QrCodeEcc, size: usize, writer: W) -> Result<(), io::Error> {
    let img_raw = to_vec(data, ecc, size)?;

    let encoder = PNGEncoder::new(writer);

    encoder.encode(&img_raw, size as u32, size as u32, ColorType::Gray(8))
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs::{self, File};

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
    fn text_to_svg() {
        let result = to_svg("Hello world!", QrCodeEcc::Low, 256, Some("")).unwrap();

        assert_eq!(fs::read_to_string("tests/data/hello.svg").unwrap(), result);
    }

    #[test]
    fn text_to_png() {
        let file = File::create("tests/data/hello_output.png").unwrap();

        to_png("Hello world!", QrCodeEcc::Low, 256, file).unwrap();

        assert_eq!(fs::read("tests/data/hello.png").unwrap(), fs::read("tests/data/hello_output.png").unwrap());
    }
}
