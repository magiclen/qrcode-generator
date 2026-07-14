QR Code Generator
=================

[![CI](https://github.com/magiclen/qrcode-generator/actions/workflows/ci.yml/badge.svg)](https://github.com/magiclen/qrcode-generator/actions/workflows/ci.yml)

This crate generates ISO/IEC 18004 QR Code and Micro QR Code symbols and ISO/IEC 23941 rMQR symbols in pure Rust, then renders them as grayscale, PNG and SVG images.

You give it data and get back a ready-to-scan image. The text encoders automatically choose the shortest Numeric, Alphanumeric, Byte and optional Kanji segmentation for each candidate version, and the Model 2 and rMQR encoders also optimize ECI transitions.

## Quick start

Encode text and read the raw module grid:

```rust
use qrcode_generator::qr::{Encoder, ErrorCorrection};

let symbol = Encoder::new(ErrorCorrection::Medium).encode_text("Hello, world!").unwrap();

// `true` is a dark module and `false` is a light module.
let matrix: Vec<Vec<bool>> = symbol.to_matrix();

println!("{} modules per side", symbol.size());
```

Or render straight to image files:

```rust
use qrcode_generator::{Renderer, qr::{Encoder, ErrorCorrection}};

let symbol = Encoder::new(ErrorCorrection::Medium).encode_text("Hello, world!").unwrap();

Renderer::new(&symbol, 512).save_svg("hello.svg", None::<&str>).unwrap();
Renderer::new(&symbol, 512).save_png("hello.png").unwrap();
```

## Encode, then render

Every workflow has two steps. An `Encoder` (from the `qr`, `micro` or `rmqr` module) turns your data into a `Symbol`, the abstract grid of modules. A `Renderer` then turns that `Symbol` into a concrete output such as an SVG string, a PNG file or a grayscale buffer. The same `Symbol` can be rendered many times at different sizes.

## Key terms

A few QR Code words appear throughout this documentation:

- **Module** — the smallest square of a QR Code, the equivalent of one pixel. A dark module is `true` in the matrix.
- **Matrix** — the full grid of modules. `Symbol::to_matrix` returns it as `Vec<Vec<bool>>`.
- **Symbol** — one complete QR Code, Micro QR Code or rMQR symbol.
- **Version** — the symbol size. QR Code versions run from 1 (21×21 modules) to 40 (177×177), Micro QR Code has versions M1 to M4 (11×11 to 17×17), and rMQR has 32 rectangular sizes.
- **Error correction level** — how much redundancy is added so a dirty or partly hidden symbol still scans. The Low, Medium, Quartile and High levels recover roughly 7%, 15%, 25% and 30% of the codewords, and a higher level is more robust but leaves less room for your own data.
- **Mask** — a regular pattern applied over the data to avoid layouts that confuse scanners. The best mask is chosen automatically, so you rarely set it yourself.
- **Quiet zone** — the plain margin around the symbol that scanners need. It defaults to 4 modules for QR Code and 2 for Micro QR Code and rMQR.
- **Mode / segment** — how characters are packed. Numeric is the most compact, then Alphanumeric, then Byte, plus optional Kanji, and the encoder mixes them automatically to save space.
- **ECI** — an Extended Channel Interpretation header that declares the character set, such as UTF-8, when it is not the default ISO-8859-1.

## Encoding

`encode_text` interprets the input as ISO-8859-1 when possible and adds a UTF-8 ECI header only when it is needed:

```rust
use qrcode_generator::qr::{Encoder, ErrorCorrection};

let symbol = Encoder::new(ErrorCorrection::Medium).encode_text("café ☕").unwrap();
```

`encode_bytes` stores an exact binary payload and never guesses a character set:

```rust
use qrcode_generator::qr::{Encoder, ErrorCorrection};

let symbol = Encoder::new(ErrorCorrection::Quartile).encode_bytes(b"raw\0bytes").unwrap();
```

The encoder is configured with a builder. You can restrict the versions it may use, or force a specific mask:

```rust
use qrcode_generator::qr::{Encoder, ErrorCorrection, Version};

let symbol = Encoder::new(ErrorCorrection::Low)
    .version_range(Version::new(1).unwrap()..=Version::new(10).unwrap())
    .encode_text("HELLO")
    .unwrap();
```

By default the encoder upgrades the error correction level for free when the chosen version has spare room. Call `boost_error_correction(false)` to keep the exact level you requested.

### Explicit segments

Automatic segmentation is optimal for most inputs, but you can also build `Segment` values yourself and keep their boundaries:

```rust
use qrcode_generator::{Segment, qr::{Encoder, ErrorCorrection}};

let segments = [
    Segment::numeric("1234567").unwrap(),
    Segment::alphanumeric("ABCDEFG").unwrap(),
];

let symbol = Encoder::new(ErrorCorrection::Low).encode_segments(&segments).unwrap();
```

### Custom text

`ToQRText` lets a structured value choose a shorter equivalent spelling before the optimizer runs:

```rust
use qrcode_generator::{ToQRText, qr::{Encoder, ErrorCorrection}};

struct ProductCode(&'static str);

impl ToQRText for ProductCode {
    fn to_qr_text(&self) -> String {
        self.0.to_ascii_uppercase()
    }
}

let symbol = Encoder::new(ErrorCorrection::Medium)
    .encode_to_qr_text(&ProductCode("item-123"))
    .unwrap();
```

The value must keep its meaning. With the optional `url` feature, `url::Url` implements `ToQRText` by normalizing case-insensitive URL parts and percent escapes:

```rust
use qrcode_generator::qr::{Encoder, ErrorCorrection};
use url::Url;

let url = Url::parse("https://magiclen.org").unwrap();
let symbol = Encoder::new(ErrorCorrection::Medium).encode_to_qr_text(&url).unwrap();
```

## Rendering

A `Renderer` draws a `Symbol` at exact pixel dimensions. `Renderer::new` keeps the square interface, while `Renderer::new_with_dimensions` accepts a separate width and height for rectangular symbols. In-memory grayscale and SVG output work with `no_std + alloc` and need no extra feature:

```rust
use qrcode_generator::{Renderer, qr::{Encoder, ErrorCorrection}};

let symbol = Encoder::new(ErrorCorrection::Low).encode_text("Hello").unwrap();

let svg: String = Renderer::new(&symbol, 512).to_svg_string(None::<&str>).unwrap();
let pixels: Vec<u8> = Renderer::new(&symbol, 512).to_luma8().unwrap();
```

PNG and `ImageBuffer` output need the default `image` feature:

```rust
use qrcode_generator::{Renderer, qr::{Encoder, ErrorCorrection}};

let symbol = Encoder::new(ErrorCorrection::Low).encode_text("Hello").unwrap();

let png: Vec<u8> = Renderer::new(&symbol, 512).to_png_vec().unwrap();

Renderer::new(&symbol, 512).save_png("hello.png").unwrap();
```

The default `std` feature adds synchronous writer and file output. File output through `save_svg` and `save_png` is written atomically, so an existing file is left untouched if rendering fails. The requested size is exact: modules are scaled by the largest whole number that fits, and any pixels left over widen the quiet zone evenly. Use `quiet_zone` to change the margin, down to zero if you want.

## Async writing

The optional `async-write` feature adds `write_svg_async` and, with the `image` feature, `write_png_async`. Rendering stays synchronous; only writing and flushing are asynchronous:

```rust
use qrcode_generator::{AsyncWrite, RenderError, Renderer, qr::{Encoder, ErrorCorrection}};

async fn write<W: AsyncWrite + Unpin>(writer: W) -> Result<(), RenderError> {
    let symbol = Encoder::new(ErrorCorrection::Low).encode_text("Hello").unwrap();

    Renderer::new(&symbol, 512).write_svg_async(writer, None::<&str>).await
}
```

## Micro QR Code

The optional `micro-qr` feature adds an independent encoder for the four Micro QR Code versions, which are smaller than QR Code and suit short payloads:

```rust
use qrcode_generator::micro::{Encoder, ErrorCorrection, Version};

let symbol = Encoder::new(ErrorCorrection::Low)
    .version(Version::M2)
    .encode_text("12345")
    .unwrap();
```

Micro QR Code does not support ECI, FNC1 or Structured Append. Its text API accepts ISO-8859-1 and, with the `kanji` feature, eligible Shift JIS Kanji characters.

## Rectangular Micro QR Code

The optional `rmqr` feature adds all 32 ISO/IEC 23941 rMQR versions. The encoder automatically selects the smallest-area version that fits and performs exact segment and ECI optimization for each character-count profile:

```rust
use qrcode_generator::{Renderer, rmqr::{Encoder, ErrorCorrection}};

let symbol = Encoder::new(ErrorCorrection::Medium)
    .encode_text("https://magiclen.org")
    .unwrap();

let width = (symbol.width() + 4) * 8;
let height = (symbol.height() + 4) * 8;
let svg = Renderer::new_with_dimensions(&symbol, width, height)
    .to_svg_string(None::<&str>)
    .unwrap();
```

rMQR supports Medium and High error correction, ECI, FNC1 and optional Kanji mode. It has one fixed data mask and does not support Structured Append.

## Automatic family selection

With both `qr` and `micro-qr` enabled, `AutoEncoder` tries the Micro QR Code encoder first and falls back to the Model 2 encoder when the data does not fit, giving you the smallest symbol that works:

```rust
use qrcode_generator::{AutoEncoder, micro, qr};

let encoder = AutoEncoder::new(
    qr::Encoder::new(qr::ErrorCorrection::Medium),
    micro::Encoder::new(micro::ErrorCorrection::Medium),
);

let symbol = encoder.encode_text("HELLO 123").unwrap();
```

## FNC1 and Structured Append

These two QR Code features serve data-exchange standards rather than plain text.

*FNC1* marks a symbol as following a formatted-data standard. `qr::Encoder::fnc1` selects GS1 semantics or an industry application indicator.

*Structured Append* spreads one message across up to 16 symbols that a reader stitches back together. You can supply the parts yourself, or let the encoder split a message automatically:

```rust
use qrcode_generator::qr::{Encoder, ErrorCorrection};

let symbols = Encoder::new(ErrorCorrection::Medium)
    .encode_text_with_structured_append("a very long message …")
    .unwrap();

println!("{} symbols", symbols.len());
```

Automatic splitting first minimizes the number of symbols, then their largest version, and finally the total symbol area.

## Kanji

The optional `kanji` feature adds Kanji mode to automatic text segmentation and exposes `Segment::kanji`. It is off by default for the widest scanner compatibility.

## Cargo features

- `std` (default) enables synchronous writer and file output.
- `qr` (default) enables the Model 2 QR Code encoder.
- `image` (default, requires `std`) enables PNG and `ImageBuffer` output.
- `micro-qr` enables the Micro QR Code encoder with all four versions.
- `rmqr` enables the Rectangular Micro QR Code encoder with all 32 versions.
- `kanji` enables Shift JIS Kanji segments and automatic Kanji mode selection.
- `url` implements `ToQRText` for `url::Url` and works with `no_std + alloc`.
- `async-write` (requires `std`) enables the runtime-independent asynchronous writer methods.

## Crates.io

https://crates.io/crates/qrcode-generator

## Documentation

https://docs.rs/qrcode-generator

## License

[MIT](LICENSE)
