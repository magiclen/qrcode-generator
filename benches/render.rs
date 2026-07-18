//! Rendering benchmarks for the grayscale, SVG and PNG output paths.
//!
//! Run with: `cargo bench --bench render` (the default features already include `image`).

use bencher::{Bencher, benchmark_group, benchmark_main, black_box};
use qrcode_generator::{Renderer, Symbol, qr};

fn sample_symbol() -> Symbol {
    qr::Encoder::new(qr::ErrorCorrection::Medium)
        .version(qr::Version::new(10).unwrap())
        .encode_text("RENDER BENCHMARK 0123456789 ABCDEFGHIJ")
        .unwrap()
}

// Grayscale rendering fills the module rectangles into a raw pixel buffer.
fn render_luma8(bencher: &mut Bencher) {
    let symbol = sample_symbol();

    bencher.iter(|| Renderer::new(black_box(&symbol), 512).to_luma8().unwrap());
}

// SVG rendering run-length encodes each row into path commands.
fn render_svg(bencher: &mut Bencher) {
    let symbol = sample_symbol();

    bencher.iter(|| Renderer::new(black_box(&symbol), 512).to_svg_string(None::<&str>).unwrap());
}

// PNG rendering compresses the grayscale buffer.
fn render_png(bencher: &mut Bencher) {
    let symbol = sample_symbol();

    bencher.iter(|| Renderer::new(black_box(&symbol), 512).to_png_vec().unwrap());
}

benchmark_group!(render, render_luma8, render_svg, render_png);
benchmark_main!(render);
