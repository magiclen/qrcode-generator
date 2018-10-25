QR Code Generator
====================

[![Build Status](https://travis-ci.org/magiclen/qrcode-generator.svg?branch=master)](https://travis-ci.org/magiclen/qrcode-generator)

## Examples

### Encode any data to a QR Code matrix which is `Vec<Vec<bool>>`.

```rust
extern crate qrcode_generator;

use qrcode_generator::QrCodeEcc;

let result: Vec<Vec<bool>> = qrcode_generator::to_matrix("Hello world!", QrCodeEcc::Low).unwrap();

println!("{:?}", result);
```

### Encode any data to a PNG image stored in a Vec instance.

```rust
extern crate qrcode_generator;

use qrcode_generator::QrCodeEcc;

let result: Vec<u8> = qrcode_generator::to_png_to_vec("Hello world!", QrCodeEcc::Low, 1024).unwrap();

println!("{:?}", result);
```

### Encode any data to a PNG image stored in a file.

```rust
extern crate qrcode_generator;

use qrcode_generator::QrCodeEcc;

qrcode_generator::to_png_to_file("Hello world!", QrCodeEcc::Low, 1024, "path/to/file.png").unwrap();
```

### Encode any data to a SVG image stored in a String instance.

```rust
extern crate qrcode_generator;

use qrcode_generator::QrCodeEcc;

let result: String = qrcode_generator::to_svg_to_string("Hello world!", QrCodeEcc::Low, 1024, None).unwrap();

println!("{:?}", result);
```

### Encode any data to a SVG image stored in a file.

```rust
extern crate qrcode_generator;

use qrcode_generator::QrCodeEcc;

qrcode_generator::to_svg_to_file("Hello world!", QrCodeEcc::Low, 1024, None, "path/to/file.svg").unwrap();
```

## Low-level Usage

### Raw Image Data

The `to_image` and `to_image_buffer` functions can be used, if you want to modify your image.

### Segments

Every **generate** and **to** function has its own **by_segments** function. You can concatenate segments by using different encoding methods, such as **numeric**, **alphanumeric** or **binary** to reduce the size (level) of your QR code matrix/image.

```rust
extern crate qrcode_generator;

use qrcode_generator::QrCodeEcc;
use qrcode_generator::qrcodegen::QrSegment;

let first = "1234567";

let second = "ABCDEFG";

let first_chars: Vec<char> = first.chars().collect();
let second_chars: Vec<char> = second.chars().collect();

let segments = vec![QrSegment::make_numeric(&first_chars), QrSegment::make_alphanumeric(&second_chars)];

let result: Vec<Vec<bool>> = qrcode_generator::to_matrix_by_segments(&segments, QrCodeEcc::Low).unwrap();

println!("{:?}", result);
```

## Optimized URL segments

URL is a common type of data used in QR code. The protocol and the scheme of a URL is case-insensitive, so they can be converted to a upper-case segment and encoded by **alphanumeric** instead of **binary** to reduce the size.

You can use the `optimize_url_segments` function to create URL segments.

```rust
extern crate qrcode_generator;

use qrcode_generator::QrCodeEcc;


let url = "https://magiclen.org/path/to/12345";

let matrix_1 = qrcode_generator::to_matrix(url, QrCodeEcc::Low).unwrap();
let matrix_2 = qrcode_generator::to_matrix_by_segments(&qrcode_generator::optimize_url_segments(url), QrCodeEcc::Low).unwrap();

assert!(matrix_2.len() < matrix_1.len());
```

## Validators Support

`Validators` is a crate which can help you validate user input.

To use with Validators support, you have to enable the **validator** feature for this crate.

```toml
[dependencies.qrcode-generator]
version = "*"
features = ["validator"]
```

And the `optimize_validated_http_url_segments` function is available.

```rust
extern crate qrcode_generator;
extern crate validators;

use qrcode_generator::QrCodeEcc;
use validators::{ValidatorOption, http_url::HttpUrlValidator};

let validator = HttpUrlValidator {
    protocol: ValidatorOption::Allow,
    local: ValidatorOption::Allow,
};

let url = "https://magiclen.org/path/to/12345";

let validated_http_url = validator.parse_str(url).unwrap();

let matrix_1 = qrcode_generator::to_matrix(url, QrCodeEcc::Low).unwrap();
let matrix_2 = qrcode_generator::to_matrix_by_segments(&qrcode_generator::optimize_validated_http_url_segments(&validated_http_url), QrCodeEcc::Low).unwrap();

assert!(matrix_2.len() < matrix_1.len());
```

## Crates.io

https://crates.io/crates/qrcode-generator

## Documentation

https://docs.rs/qrcode-generator

## License

[MIT](LICENSE)