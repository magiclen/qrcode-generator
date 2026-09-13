//! Encoding benchmarks covering every symbol family and both optimizer-heavy input paths.
//!
//! Run with: `cargo bench --bench encode --features micro-qr,rmqr` (or `--all-features`).

use bencher::{Bencher, benchmark_group, benchmark_main, black_box};
use qrcode_generator::{AutoEncoder, Segment, micro, qr, rmqr};

// A short URL is the most common real-world input and stays a single Model 2 symbol.
fn qr_text_url(bencher: &mut Bencher) {
    let encoder = qr::Encoder::new(qr::ErrorCorrection::Medium);

    bencher
        .iter(|| encoder.encode_text(black_box("https://magiclen.org/qrcode-generator")).unwrap());
}

// A large mixed string exercises the text optimizer that segments modes and interpretations.
fn qr_text_large(bencher: &mut Bencher) {
    let text = "QR Code 123 example payload, ".repeat(80);
    let encoder = qr::Encoder::new(qr::ErrorCorrection::Low);

    bencher.iter(|| encoder.encode_text(black_box(text.as_str())).unwrap());
}

// A large binary payload exercises the byte optimizer over its full capacity.
fn qr_bytes_large(bencher: &mut Bencher) {
    let data: Vec<u8> =
        (0..2000u32).map(|value| (value.wrapping_mul(31).wrapping_add(7)) as u8).collect();
    let encoder = qr::Encoder::new(qr::ErrorCorrection::Low);

    bencher.iter(|| encoder.encode_bytes(black_box(&data)).unwrap());
}

// Explicit segments skip the optimizer and measure bitstream, Reed-Solomon and masking only.
fn qr_segments(bencher: &mut Bencher) {
    let segments =
        [Segment::numeric("0123456789012345").unwrap(), Segment::bytes(b"benchmark payload")];
    let encoder = qr::Encoder::new(qr::ErrorCorrection::Quartile);

    bencher.iter(|| encoder.encode_segments(black_box(&segments)).unwrap());
}

// Micro QR selects a small symbol and a mask by score.
fn micro_text(bencher: &mut Bencher) {
    let encoder = micro::Encoder::new(micro::ErrorCorrection::Low);

    bencher.iter(|| encoder.encode_text(black_box("MICRO 1234")).unwrap());
}

// Text that no Micro QR version can hold measures the rejection path, which must not segment the input first.
fn micro_unicode_rejection(bencher: &mut Bencher) {
    let encoder = micro::Encoder::new(micro::ErrorCorrection::Low);
    let text = "日本語".repeat(100);

    bencher.iter(|| encoder.encode_text(black_box(&text)).unwrap_err());
}

// Text the Micro QR encoder rejects falls through to Model 2, the common automatic path for non-Latin input.
fn auto_unicode_fallback(bencher: &mut Bencher) {
    let encoder = AutoEncoder::new(
        qr::Encoder::new(qr::ErrorCorrection::Low),
        micro::Encoder::new(micro::ErrorCorrection::Low),
    );
    let text = "😀日本語".repeat(30);

    bencher.iter(|| encoder.encode_text(black_box(&text)).unwrap());
}

// rMQR searches its rectangular versions and interleaves multiple blocks.
fn rmqr_text(bencher: &mut Bencher) {
    let encoder = rmqr::Encoder::new(rmqr::ErrorCorrection::Medium);

    bencher.iter(|| {
        encoder.encode_text(black_box("rMQR benchmark payload 0123456789 ABCDEF")).unwrap()
    });
}

benchmark_group!(
    encode,
    qr_text_url,
    qr_text_large,
    qr_bytes_large,
    qr_segments,
    micro_text,
    micro_unicode_rejection,
    auto_unicode_fallback,
    rmqr_text
);
benchmark_main!(encode);
