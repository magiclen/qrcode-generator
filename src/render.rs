#[cfg(feature = "async-write")]
use std::{
    future::poll_fn,
    io::{self, ErrorKind},
    pin::Pin,
};
use std::{io::Write, path::Path};

use atomic_write_file::AtomicWriteFile;
#[cfg(feature = "image")]
use image::{
    ColorType, ImageBuffer, ImageEncoder, Luma,
    codecs::png::{CompressionType, FilterType, PngEncoder},
};

#[cfg(feature = "async-write")]
use crate::AsyncWrite;
use crate::{RenderError, Symbol, SymbolVersion};

/// Renders an encoded symbol at exact output dimensions.
#[derive(Clone, Copy, Debug)]
pub struct Renderer<'a> {
    symbol:     &'a Symbol,
    width:      usize,
    height:     usize,
    quiet_zone: usize,
}

impl<'a> Renderer<'a> {
    /// Creates a renderer with an exact square output size and the standard quiet zone for the symbol family.
    #[inline]
    pub const fn new(symbol: &'a Symbol, size: usize) -> Self {
        Self::new_with_dimensions(symbol, size, size)
    }

    /// Creates a renderer with exact output dimensions and the standard quiet zone for the symbol family.
    #[inline]
    pub const fn new_with_dimensions(symbol: &'a Symbol, width: usize, height: usize) -> Self {
        let quiet_zone = match symbol.version() {
            #[cfg(feature = "qr")]
            SymbolVersion::Qr(_) => 4,
            #[cfg(feature = "micro-qr")]
            SymbolVersion::Micro(_) => 2,
            #[cfg(feature = "rmqr")]
            SymbolVersion::Rmqr(_) => 2,
        };

        Self {
            symbol,
            width,
            height,
            quiet_zone,
        }
    }

    /// Sets the minimum quiet zone in modules before any extra centering pixels.
    #[inline]
    pub const fn quiet_zone(mut self, modules: usize) -> Self {
        self.quiet_zone = modules;

        self
    }

    /// Renders an 8-bit grayscale image in row-major order.
    pub fn to_luma8(self) -> Result<Vec<u8>, RenderError> {
        let layout = self.layout()?;
        let length = self.width.checked_mul(self.height).ok_or(RenderError::ImageSizeTooLarge)?;
        let mut image = vec![255; length];

        for y in 0..self.symbol.height() {
            let output_y = layout.margin_y + y * layout.scale;

            for x in 0..self.symbol.width() {
                if self.symbol.module(x, y) == Some(true) {
                    let output_x = layout.margin_x + x * layout.scale;

                    for row in output_y..output_y + layout.scale {
                        image[row * self.width + output_x
                            ..row * self.width + output_x + layout.scale]
                            .fill(0);
                    }
                }
            }
        }

        Ok(image)
    }

    /// Writes an SVG document to a writer.
    ///
    /// The description must contain only characters allowed by XML 1.0; markup characters are escaped, but callers must remove or replace disallowed XML characters before rendering.
    pub fn write_svg<W: Write>(
        self,
        mut writer: W,
        description: Option<&str>,
    ) -> Result<(), RenderError> {
        let layout = self.layout()?;

        write!(
            writer,
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg width=\"{}\" height=\"{}\" shape-rendering=\"crispEdges\" version=\"1.1\" xmlns=\"http://www.w3.org/2000/svg\">\n",
            self.width, self.height
        )?;

        if let Some(description) = description {
            if !description.is_empty() {
                writer.write_all(b"\t<desc>")?;
                html_escape::encode_safe_to_writer(description, &mut writer)?;
                writer.write_all(b"</desc>\n")?;
            }
        } else {
            writeln!(
                writer,
                "\t<desc>{} {} by magiclen.org</desc>",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            )?;
        }

        write!(
            writer,
            "\t<rect width=\"{}\" height=\"{}\" fill=\"#FFF\"/>\n\t<path d=\"",
            self.width, self.height
        )?;

        for y in 0..self.symbol.height() {
            // One SVG rectangle command represents each horizontal run of dark modules.
            let mut x = 0;

            while x < self.symbol.width() {
                if self.symbol.module(x, y) != Some(true) {
                    x += 1;
                    continue;
                }

                let start = x;

                while x < self.symbol.width() && self.symbol.module(x, y) == Some(true) {
                    x += 1;
                }

                let output_x = layout.margin_x + start * layout.scale;
                let output_y = layout.margin_y + y * layout.scale;
                let width = (x - start) * layout.scale;

                write!(
                    writer,
                    "M{output_x} {output_y}h{width}v{}H{output_x}V{output_y}",
                    layout.scale
                )?;
            }
        }
        writer.write_all(b"\"/>\n</svg>")?;
        writer.flush()?;
        Ok(())
    }

    /// Renders an SVG document as a UTF-8 string.
    ///
    /// The description must contain only characters allowed by XML 1.0; markup characters are escaped, but callers must remove or replace disallowed XML characters before rendering.
    pub fn to_svg_string(self, description: Option<&str>) -> Result<String, RenderError> {
        let mut bytes = Vec::with_capacity(8192);

        self.write_svg(&mut bytes, description)?;

        // SAFETY: The SVG writer emits UTF-8 literals and escapes the only caller-provided text.
        Ok(unsafe { String::from_utf8_unchecked(bytes) })
    }

    #[cfg(feature = "async-write")]
    #[cfg_attr(docsrs, doc(cfg(feature = "async-write")))]
    /// Renders an SVG document in memory, then writes and flushes it asynchronously.
    ///
    /// The description must contain only characters allowed by XML 1.0; markup characters are escaped, but callers must remove or replace disallowed XML characters before rendering.
    pub async fn write_svg_async<W: AsyncWrite + Unpin>(
        self,
        mut writer: W,
        description: Option<&str>,
    ) -> Result<(), RenderError> {
        let svg = self.to_svg_string(description)?;

        write_all_async(&mut writer, svg.as_bytes()).await?;

        Ok(())
    }

    /// Atomically saves an SVG document after rendering succeeds.
    ///
    /// The description must contain only characters allowed by XML 1.0; markup characters are escaped, but callers must remove or replace disallowed XML characters before rendering.
    pub fn save_svg(
        self,
        path: impl AsRef<Path>,
        description: Option<&str>,
    ) -> Result<(), RenderError> {
        let mut file = AtomicWriteFile::open(path)?;

        self.write_svg(&mut file, description)?;

        file.commit()?;

        Ok(())
    }

    #[cfg(feature = "image")]
    /// Writes a grayscale PNG image to a writer.
    pub fn write_png<W: Write>(self, writer: W) -> Result<(), RenderError> {
        let image = self.to_luma8()?;

        let width = u32::try_from(self.width).map_err(|_| RenderError::ImageSizeTooLarge)?;
        let height = u32::try_from(self.height).map_err(|_| RenderError::ImageSizeTooLarge)?;

        PngEncoder::new_with_quality(writer, CompressionType::Best, FilterType::NoFilter)
            .write_image(&image, width, height, ColorType::L8.into())?;

        Ok(())
    }

    #[cfg(feature = "image")]
    /// Renders a grayscale PNG image into a byte vector.
    pub fn to_png_vec(self) -> Result<Vec<u8>, RenderError> {
        let mut bytes = Vec::with_capacity(4096);

        self.write_png(&mut bytes)?;

        Ok(bytes)
    }

    #[cfg(all(feature = "async-write", feature = "image"))]
    #[cfg_attr(docsrs, doc(cfg(all(feature = "async-write", feature = "image"))))]
    /// Renders a PNG image in memory, then writes and flushes it asynchronously.
    pub async fn write_png_async<W: AsyncWrite + Unpin>(
        self,
        mut writer: W,
    ) -> Result<(), RenderError> {
        let png = self.to_png_vec()?;

        write_all_async(&mut writer, &png).await?;

        Ok(())
    }

    #[cfg(feature = "image")]
    /// Atomically saves a PNG image after rendering succeeds.
    pub fn save_png(self, path: impl AsRef<Path>) -> Result<(), RenderError> {
        let mut file = AtomicWriteFile::open(path)?;

        self.write_png(&mut file)?;

        file.commit()?;

        Ok(())
    }

    #[cfg(feature = "image")]
    /// Renders a grayscale image buffer.
    pub fn to_image_buffer(self) -> Result<ImageBuffer<Luma<u8>, Vec<u8>>, RenderError> {
        let image = self.to_luma8()?;

        let width = u32::try_from(self.width).map_err(|_| RenderError::ImageSizeTooLarge)?;
        let height = u32::try_from(self.height).map_err(|_| RenderError::ImageSizeTooLarge)?;

        ImageBuffer::from_vec(width, height, image).ok_or(RenderError::ImageSizeTooLarge)
    }

    fn layout(self) -> Result<Layout, RenderError> {
        let quiet_zone_modules =
            self.quiet_zone.checked_mul(2).ok_or(RenderError::ImageSizeTooLarge)?;
        let modules_width = self
            .symbol
            .width()
            .checked_add(quiet_zone_modules)
            .ok_or(RenderError::ImageSizeTooLarge)?;
        let modules_height = self
            .symbol
            .height()
            .checked_add(quiet_zone_modules)
            .ok_or(RenderError::ImageSizeTooLarge)?;

        // Integer scaling keeps every module edge aligned to an output pixel boundary.
        let scale = (self.width / modules_width).min(self.height / modules_height);

        if scale == 0 {
            return Err(RenderError::ImageSizeTooSmall);
        }

        let symbol_width =
            self.symbol.width().checked_mul(scale).ok_or(RenderError::ImageSizeTooLarge)?;
        let symbol_height =
            self.symbol.height().checked_mul(scale).ok_or(RenderError::ImageSizeTooLarge)?;

        // Centering splits pixels left over after fitting the requested quiet zone and integer scale.
        Ok(Layout {
            scale,
            margin_x: (self.width - symbol_width) / 2,
            margin_y: (self.height - symbol_height) / 2,
        })
    }
}

#[cfg(feature = "async-write")]
async fn write_all_async<W: AsyncWrite + Unpin>(
    writer: &mut W,
    mut bytes: &[u8],
) -> io::Result<()> {
    while !bytes.is_empty() {
        let written = poll_fn(|context| Pin::new(&mut *writer).poll_write(context, bytes)).await?;

        if written == 0 {
            return Err(io::Error::new(ErrorKind::WriteZero, "failed to write rendered output"));
        }

        bytes = &bytes[written..];
    }

    poll_fn(|context| Pin::new(&mut *writer).poll_flush(context)).await
}

struct Layout {
    scale:    usize,
    margin_x: usize,
    margin_y: usize,
}
