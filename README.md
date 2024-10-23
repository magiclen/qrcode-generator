QR Code Generator
====================

[![Docs](https://docs.rs/qrcode-generator/badge.svg)](https://docs.rs/qrcode-generator)
[![CI](https://github.com/magiclen/qrcode-generator/actions/workflows/ci.yml/badge.svg)](https://github.com/magiclen/qrcode-generator/actions/workflows/ci.yml)

This crate provides functions to generate QR Code matrices and images in RAW, PNG and SVG formats.

## Examples

#### Encode any data to a QR Code matrix which is `Vec<Vec<bool>>`.

```rust
use qrcode_generator::QrCodeEcc;

let result: Vec<Vec<bool>> = qrcode_generator::to_matrix("Hello world!", QrCodeEcc::Low).unwrap();

println!("{:?}", result);
```

#### Encode any data to a PNG image stored in a Vec instance.

```rust
use qrcode_generator::QrCodeEcc;

let result: Vec<u8> = qrcode_generator::to_png_to_vec("Hello world!", QrCodeEcc::Low, 1024).unwrap();

println!("{:?}", result);
```

#### Encode any data to a PNG image stored in a file.

```rust
use qrcode_generator::QrCodeEcc;

qrcode_generator::to_png_to_file("Hello world!", QrCodeEcc::Low, 1024, "tests/data/file_output.png").unwrap();
```

#### Encode any data to a SVG image stored in a String instance.

```rust
use qrcode_generator::QrCodeEcc;

let result: String = qrcode_generator::to_svg_to_string("Hello world!", QrCodeEcc::Low, 1024, None::<&str>).unwrap();

println!("{:?}", result);
```

#### Encode any data to a SVG image stored in a file.

```rust
use qrcode_generator::QrCodeEcc;

qrcode_generator::to_svg_to_file("Hello world!", QrCodeEcc::Low, 1024, None::<&str>, "tests/data/file_output.svg").unwrap();
```

## Low-level Usage

### Raw Image Data

The `to_image` and `to_image_buffer` functions can be used, if you want to modify your image.

### Segments

Every `to_*` function has a corresponding `_from_segments` function. You can concatenate segments by using different encoding methods, such as **numeric**, **alphanumeric** or **binary** to reduce the size (level) of your QR code matrix/image.

```rust
use qrcode_generator::{QrCodeEcc, QrSegment};

let first = "1234567";

let second = "ABCDEFG";

let segments = [QrSegment::make_numeric(&first), QrSegment::make_alphanumeric(&second)];

let result: Vec<Vec<bool>> = qrcode_generator::to_matrix_from_segments(&segments, QrCodeEcc::Low).unwrap();

println!("{:?}", result);
```

More segments optimization apporaches: [magiclen/qrcode-segments-optimizer](https://github.com/magiclen/qrcode-segments-optimizer)

## Crates.io

https://crates.io/crates/qrcode-generator

## Documentation

https://docs.rs/qrcode-generator

## License

[MIT](LICENSE)
