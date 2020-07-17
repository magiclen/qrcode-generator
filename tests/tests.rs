extern crate qrcode_generator;

#[macro_use]
extern crate slash_formatter;

use std::fs;
use std::path::Path;

use qrcode_generator::QrCodeEcc;

const FOLDER: &str = concat_with_file_separator!("tests", "data");

#[test]
fn text_to_matrix() {
    let result = qrcode_generator::to_matrix("Hello world!", QrCodeEcc::Low).unwrap();

    assert_eq!(
        vec![
            vec![
                true, true, true, true, true, true, true, false, true, true, true, true, true,
                false, true, true, true, true, true, true, true
            ],
            vec![
                true, false, false, false, false, false, true, false, true, false, true, false,
                true, false, true, false, false, false, false, false, true
            ],
            vec![
                true, false, true, true, true, false, true, false, false, false, true, true, false,
                false, true, false, true, true, true, false, true
            ],
            vec![
                true, false, true, true, true, false, true, false, true, false, true, true, true,
                false, true, false, true, true, true, false, true
            ],
            vec![
                true, false, true, true, true, false, true, false, false, false, true, false,
                false, false, true, false, true, true, true, false, true
            ],
            vec![
                true, false, false, false, false, false, true, false, false, true, true, false,
                false, false, true, false, false, false, false, false, true
            ],
            vec![
                true, true, true, true, true, true, true, false, true, false, true, false, true,
                false, true, true, true, true, true, true, true
            ],
            vec![
                false, false, false, false, false, false, false, false, true, true, false, false,
                false, false, false, false, false, false, false, false, false
            ],
            vec![
                true, false, true, true, false, true, true, true, false, false, true, false, false,
                false, true, false, false, true, false, true, true
            ],
            vec![
                false, false, true, true, false, false, false, true, false, true, true, false,
                true, true, true, true, true, true, true, false, true
            ],
            vec![
                true, true, true, false, true, true, true, false, true, false, false, false, true,
                true, false, true, false, false, false, true, true
            ],
            vec![
                false, true, true, true, true, true, false, true, false, false, true, false, true,
                false, false, true, false, true, false, true, false
            ],
            vec![
                false, false, true, true, false, false, true, true, false, false, false, true,
                false, true, true, false, false, false, false, false, true
            ],
            vec![
                false, false, false, false, false, false, false, false, true, true, true, true,
                false, false, true, true, true, false, true, false, true
            ],
            vec![
                true, true, true, true, true, true, true, false, true, false, true, false, false,
                true, true, true, true, false, false, false, false
            ],
            vec![
                true, false, false, false, false, false, true, false, true, true, true, true, true,
                true, false, true, false, true, true, false, false
            ],
            vec![
                true, false, true, true, true, false, true, false, false, true, false, true, false,
                false, false, false, false, true, true, true, false
            ],
            vec![
                true, false, true, true, true, false, true, false, true, false, true, false, true,
                false, true, false, false, true, true, true, false
            ],
            vec![
                true, false, true, true, true, false, true, false, true, false, false, false, true,
                false, false, true, false, false, true, false, false
            ],
            vec![
                true, false, false, false, false, false, true, false, false, true, false, true,
                false, true, true, true, true, false, false, false, true
            ],
            vec![
                true, true, true, true, true, true, true, false, true, false, true, true, false,
                true, true, true, false, false, true, false, false
            ]
        ],
        result
    );
}

#[test]
fn text_to_svg_to_string() {
    let result =
        qrcode_generator::to_svg_to_string("Hello world!", QrCodeEcc::Low, 256, Some("")).unwrap();

    assert_eq!(fs::read_to_string(Path::new(FOLDER).join("hello.svg")).unwrap(), result);
}

#[test]
fn text_to_svg_to_file() {
    qrcode_generator::to_svg_to_file(
        "Hello world!",
        QrCodeEcc::Low,
        256,
        Some(""),
        Path::new(FOLDER).join("hello_output.svg"),
    )
    .unwrap();

    assert_eq!(
        fs::read(Path::new(FOLDER).join("hello.svg")).unwrap(),
        fs::read(Path::new(FOLDER).join("hello_output.svg")).unwrap()
    );
}

#[cfg(feature = "image")]
#[test]
fn text_to_png_to_vec() {
    let result = qrcode_generator::to_png_to_vec("Hello world!", QrCodeEcc::Low, 256).unwrap();

    assert_eq!(fs::read(Path::new(FOLDER).join("hello.png")).unwrap(), result);
}

#[cfg(feature = "image")]
#[test]
fn text_to_png_to_file() {
    qrcode_generator::to_png_to_file(
        "Hello world!",
        QrCodeEcc::Low,
        256,
        Path::new(FOLDER).join("hello_output.png"),
    )
    .unwrap();

    assert_eq!(
        fs::read(Path::new(FOLDER).join("hello.png"),).unwrap(),
        fs::read(Path::new(FOLDER).join("hello_output.png"),).unwrap()
    );
}
