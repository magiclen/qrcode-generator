use qrcode_generator::{Renderer, Symbol};

fn symbol() -> Symbol {
    #[cfg(feature = "qr")]
    {
        qrcode_generator::qr::Encoder::new(qrcode_generator::qr::ErrorCorrection::Low)
            .encode_text("HELLO")
            .unwrap()
    }

    #[cfg(all(not(feature = "qr"), feature = "micro-qr"))]
    {
        qrcode_generator::micro::Encoder::new(qrcode_generator::micro::ErrorCorrection::Low)
            .encode_text("12345")
            .unwrap()
    }

    #[cfg(all(not(feature = "qr"), not(feature = "micro-qr"), feature = "rmqr"))]
    {
        qrcode_generator::rmqr::Encoder::new(qrcode_generator::rmqr::ErrorCorrection::Medium)
            .encode_text("12345")
            .unwrap()
    }
}

fn default_quiet_zone(symbol: &Symbol) -> usize {
    match symbol.version() {
        #[cfg(feature = "qr")]
        qrcode_generator::SymbolVersion::Qr(_) => 4,
        #[cfg(feature = "micro-qr")]
        qrcode_generator::SymbolVersion::Micro(_) => 2,
        #[cfg(feature = "rmqr")]
        qrcode_generator::SymbolVersion::Rmqr(_) => 2,
        _ => panic!("the renderer test needs a quiet zone for the new symbol family"),
    }
}

// Grayscale output scales modules by an integer factor and centres the leftover pixels in the quiet zone.
#[test]
fn luma_layout_uses_integer_scaling_quiet_zone_and_centering() {
    let symbol = symbol();
    let quiet_zone = default_quiet_zone(&symbol);
    let scale = 3;
    let width = (symbol.width() + quiet_zone * 2) * scale + 3;
    let height = (symbol.height() + quiet_zone * 2) * scale + 5;
    let image = Renderer::new_with_dimensions(&symbol, width, height).to_luma8().unwrap();

    assert_eq!(width * height, image.len());
    assert!(image[..width].iter().all(|&pixel| pixel == 255));

    let margin_x = (width - symbol.width() * scale) / 2;
    let margin_y = (height - symbol.height() * scale) / 2;
    assert_eq!(0, image[margin_y * width + margin_x]);
    assert_eq!(0, image[(margin_y + scale - 1) * width + margin_x + scale - 1]);
}

// Each family renders with its standard quiet zone, so the first dark run starts at the expected offset.
#[test]
fn standard_quiet_zones_are_used_for_every_enabled_family() {
    #[cfg(feature = "qr")]
    {
        let symbol = qrcode_generator::qr::Encoder::new(qrcode_generator::qr::ErrorCorrection::Low)
            .encode_text("HELLO")
            .unwrap();
        let size = (symbol.size() + 8) * 2;
        assert!(Renderer::new(&symbol, size).to_svg_string(None::<&str>).unwrap().contains("M8 8"));
    }

    #[cfg(feature = "micro-qr")]
    {
        let symbol =
            qrcode_generator::micro::Encoder::new(qrcode_generator::micro::ErrorCorrection::Low)
                .encode_text("12345")
                .unwrap();
        let size = (symbol.size() + 4) * 2;
        assert!(Renderer::new(&symbol, size).to_svg_string(None::<&str>).unwrap().contains("M4 4"));
    }

    #[cfg(feature = "rmqr")]
    {
        let symbol =
            qrcode_generator::rmqr::Encoder::new(qrcode_generator::rmqr::ErrorCorrection::Medium)
                .encode_text("12345")
                .unwrap();
        let width = (symbol.width() + 4) * 2;
        let height = (symbol.height() + 4) * 2;
        let svg = Renderer::new_with_dimensions(&symbol, width, height)
            .to_svg_string(None::<&str>)
            .unwrap();
        assert!(svg.contains("M4 4"));
    }
}

// Text output packs two module rows per line with half block characters, includes the quiet zone, and the alternate form flips visibility.
#[test]
fn unicode_text_output_matches_modules_in_both_forms() {
    let symbol = symbol();
    let quiet_zone = default_quiet_zone(&symbol);
    let renderer = Renderer::new(&symbol, 0);
    let columns = symbol.width() + quiet_zone * 2;
    let rows = symbol.height() + quiet_zone * 2;

    for (visible_dark, output) in [(true, format!("{renderer}")), (false, format!("{renderer:#}"))]
    {
        let lines: Vec<&str> = output.lines().collect();

        assert_eq!(rows.div_ceil(2), lines.len(), "line count for visible_dark={visible_dark}");

        for (pair, line) in lines.iter().enumerate() {
            let cells: Vec<char> = line.chars().collect();

            assert_eq!(columns, cells.len());

            for (x, &cell) in cells.iter().enumerate() {
                let visible = |y: usize| {
                    y < rows
                        && (x
                            .checked_sub(quiet_zone)
                            .zip(y.checked_sub(quiet_zone))
                            .and_then(|(x, y)| symbol.module(x, y))
                            == Some(true))
                            == visible_dark
                };
                let expected = match (visible(pair * 2), visible(pair * 2 + 1)) {
                    (true, true) => '█',
                    (true, false) => '▀',
                    (false, true) => '▄',
                    (false, false) => ' ',
                };

                assert_eq!(expected, cell, "cell ({x}, {pair}) visible_dark={visible_dark}");
            }
        }
    }
}

// SVG rendering escapes a supplied description and falls back to a default when none is given.
#[test]
fn svg_memory_rendering_accepts_all_description_forms() {
    let symbol = symbol();
    let width = (symbol.width() + default_quiet_zone(&symbol) * 2) * 4;
    let height = (symbol.height() + default_quiet_zone(&symbol) * 2) * 4;
    let renderer = Renderer::new_with_dimensions(&symbol, width, height);

    let owned = renderer.to_svg_string(Some(String::from("<&>"))).unwrap();
    assert!(owned.contains("<desc>&lt;&amp;&gt;</desc>"));
    assert!(renderer.to_svg_string(Some("plain")).unwrap().contains("<desc>plain</desc>"));
    assert!(renderer.to_svg_string(None::<&str>).unwrap().contains("by magiclen.org</desc>"));
}

// The synchronous writer produces the same bytes as in-memory SVG rendering.
#[cfg(feature = "std")]
#[test]
fn synchronous_svg_writer_matches_memory_rendering() {
    let symbol = symbol();
    let renderer = Renderer::new_with_dimensions(&symbol, 256, 128);

    for description in [None::<&str>, Some("plain"), Some("<&>")] {
        let expected = renderer.to_svg_string(description).unwrap();
        let mut actual = Vec::new();
        renderer.write_svg(&mut actual, description).unwrap();
        assert_eq!(expected.as_bytes(), actual);
    }
}

// PNG and image buffer outputs carry the PNG signature and the exact requested dimensions.
#[cfg(feature = "image")]
#[test]
fn png_and_image_buffer_match_requested_dimensions() {
    let symbol = symbol();
    let width = 256;
    let height = 128;
    let renderer = Renderer::new_with_dimensions(&symbol, width, height);
    let png = renderer.to_png_vec().unwrap();
    let image = renderer.to_image_buffer().unwrap();

    assert_eq!(b"\x89PNG\r\n\x1A\n", &png[..8]);
    assert_eq!((width as u32, height as u32), image.dimensions());

    let mut written = Vec::new();
    renderer.write_png(&mut written).unwrap();
    assert_eq!(png, written);
}

#[cfg(all(feature = "image", target_pointer_width = "64"))]
// An output wider than an image dimension is rejected before the pixel buffer is allocated.
#[test]
fn image_dimensions_are_checked_before_allocating_pixels() {
    let symbol = symbol();
    let renderer = Renderer::new_with_dimensions(&symbol, u32::MAX as usize + 1, 128);

    assert!(matches!(renderer.to_png_vec(), Err(qrcode_generator::RenderError::ImageSizeTooLarge)));
    assert!(matches!(
        renderer.to_image_buffer(),
        Err(qrcode_generator::RenderError::ImageSizeTooLarge)
    ));
}

// The save APIs write a real SVG and PNG file to disk.
#[cfg(feature = "std")]
#[test]
fn save_apis_write_to_temporary_paths() {
    let symbol = symbol();
    let renderer = Renderer::new_with_dimensions(&symbol, 256, 128);
    let base = std::env::temp_dir().join(format!(
        "qrcode-generator-render-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let svg_path = base.with_extension("svg");

    renderer.save_svg(&svg_path, Some("saved")).unwrap();
    assert!(std::fs::read_to_string(&svg_path).unwrap().contains("<desc>saved</desc>"));
    std::fs::remove_file(svg_path).unwrap();

    #[cfg(feature = "image")]
    {
        let png_path = base.with_extension("png");
        renderer.save_png(&png_path).unwrap();
        assert_eq!(b"\x89PNG\r\n\x1A\n", &std::fs::read(&png_path).unwrap()[..8]);
        std::fs::remove_file(png_path).unwrap();
    }
}

// The tokio writers and save APIs produce the same bytes as their in-memory and synchronous counterparts.
#[cfg(feature = "tokio")]
#[tokio::test]
async fn tokio_writers_and_savers_match_the_synchronous_apis() {
    let symbol = symbol();
    let renderer = Renderer::new_with_dimensions(&symbol, 256, 128);
    let svg = renderer.to_svg_string(Some("async")).unwrap();

    let mut async_svg = Vec::new();
    renderer.write_svg_async(&mut async_svg, Some(String::from("async"))).await.unwrap();
    assert_eq!(svg.as_bytes(), async_svg);

    let base = std::env::temp_dir().join(format!(
        "qrcode-generator-tokio-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));

    let svg_path = base.with_extension("svg");
    renderer.save_svg_async(&svg_path, Some("async")).await.unwrap();
    assert_eq!(svg.as_bytes(), std::fs::read(&svg_path).unwrap());
    std::fs::remove_file(svg_path).unwrap();

    #[cfg(feature = "image")]
    {
        let png = renderer.to_png_vec().unwrap();

        let mut async_png = Vec::new();
        renderer.write_png_async(&mut async_png).await.unwrap();
        assert_eq!(png, async_png);

        let png_path = base.with_extension("png");
        renderer.save_png_async(&png_path).await.unwrap();
        assert_eq!(png, std::fs::read(&png_path).unwrap());
        std::fs::remove_file(png_path).unwrap();
    }
}
