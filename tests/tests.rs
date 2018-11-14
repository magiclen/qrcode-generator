extern crate qrcode_generator;
#[cfg(feature = "validator")]
extern crate validators;

use std::path::Path;
use std::fs;

use qrcode_generator::QrCodeEcc;

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

    let matrix_1 = qrcode_generator::to_matrix(url, QrCodeEcc::Low).unwrap();
    let matrix_2 = qrcode_generator::to_matrix_by_segments(&qrcode_generator::optimize_validated_http_url_segments(&validated_http_url, QrCodeEcc::Low).unwrap(), QrCodeEcc::Low).unwrap();

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

    let matrix_1 = qrcode_generator::to_matrix(url, QrCodeEcc::Low).unwrap();
    let matrix_2 = qrcode_generator::to_matrix_by_segments(&qrcode_generator::optimize_validated_http_ftp_url_segments(&validated_http_ftp_url, QrCodeEcc::Low).unwrap(), QrCodeEcc::Low).unwrap();

    assert!(matrix_2.len() < matrix_1.len());
}

#[test]
fn text_to_matrix() {
    let result = qrcode_generator::to_matrix("Hello world!", QrCodeEcc::Low).unwrap();

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
    let result = qrcode_generator::to_svg_to_string("Hello world!", QrCodeEcc::Low, 256, Some("")).unwrap();

    assert_eq!(fs::read_to_string(Path::join(Path::new(FOLDER), "hello.svg")).unwrap(), result);
}

#[test]
fn text_to_svg_to_file() {
    qrcode_generator::to_svg_to_file("Hello world!", QrCodeEcc::Low, 256, Some(""), Path::join(Path::new(FOLDER), "hello_output.svg")).unwrap();

    assert_eq!(fs::read(Path::join(Path::new(FOLDER), "hello.svg")).unwrap(), fs::read(Path::join(Path::new(FOLDER), "hello_output.svg")).unwrap());
}

#[test]
fn text_to_png_to_vec() {
    let result = qrcode_generator::to_png_to_vec("Hello world!", QrCodeEcc::Low, 256).unwrap();

    assert_eq!(fs::read(Path::join(Path::new(FOLDER), "hello.png")).unwrap(), result);
}

#[test]
fn text_to_png_to_file() {
    qrcode_generator::to_png_to_file("Hello world!", QrCodeEcc::Low, 256, Path::join(Path::new(FOLDER), "hello_output.png")).unwrap();

    assert_eq!(fs::read(Path::join(Path::new(FOLDER), "hello.png")).unwrap(), fs::read(Path::join(Path::new(FOLDER), "hello_output.png")).unwrap());
}