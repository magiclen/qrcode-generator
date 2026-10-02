use alloc::{string::String, vec, vec::Vec};
use core::fmt;
#[cfg(feature = "std")]
use std::{
    io::{self, Write as IoWrite},
    path::Path,
};

#[cfg(feature = "std")]
use atomic_write_file::AtomicWriteFile;
#[cfg(feature = "image")]
use image::{
    ColorType, ImageBuffer, ImageEncoder, Luma,
    codecs::png::{CompressionType, FilterType, PngEncoder},
};
#[cfg(feature = "tokio")]
use tokio::io::{AsyncWrite as TokioAsyncWrite, AsyncWriteExt};

use crate::{RenderError, Symbol, SymbolVersion};

/// Renders an encoded symbol at exact output dimensions.
#[derive(Clone, Copy, Debug)]
pub struct Renderer<'a> {
    symbol:              &'a Symbol,
    width:               usize,
    height:              usize,
    quiet_zone:          usize,
    svg_xml_declaration: bool,
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
            svg_xml_declaration: true,
        }
    }

    /// Sets the minimum quiet zone in modules before any extra centering pixels.
    #[must_use]
    #[inline]
    pub const fn quiet_zone(mut self, modules: usize) -> Self {
        self.quiet_zone = modules;

        self
    }

    /// Sets whether SVG output starts with an XML declaration; enabled by default.
    /// This setting applies to all SVG string, writer and file output methods.
    #[must_use]
    #[inline]
    pub const fn svg_xml_declaration(mut self, enabled: bool) -> Self {
        self.svg_xml_declaration = enabled;

        self
    }

    /// Renders an 8-bit grayscale image in row-major order.
    pub fn to_luma8(self) -> Result<Vec<u8>, RenderError> {
        let layout = self.layout()?;
        let length = self.width.checked_mul(self.height).ok_or(RenderError::ImageSizeTooLarge)?;
        let mut image = vec![255; length];
        let symbol_width = self.symbol.width();
        let modules = self.symbol.modules();

        for y in 0..self.symbol.height() {
            let first_row = layout.margin_y + y * layout.scale;
            let row_start = first_row * self.width;

            // The row is read directly from the module slice because x and y are always in range here.
            let module_row = &modules[y * symbol_width..][..symbol_width];

            // Each horizontal run of dark modules is drawn once into the first pixel row.
            let mut x = 0;

            while x < symbol_width {
                if !module_row[x] {
                    x += 1;
                    continue;
                }

                let start = x;

                while x < symbol_width && module_row[x] {
                    x += 1;
                }

                let output_x = layout.margin_x + start * layout.scale;

                image[row_start + output_x..row_start + output_x + (x - start) * layout.scale]
                    .fill(0);
            }

            // The finished pixel row is copied to the remaining rows of this module row.
            for row in 1..layout.scale {
                let destination = (first_row + row) * self.width;

                image.copy_within(row_start..row_start + self.width, destination);
            }
        }

        Ok(image)
    }

    /// Writes an SVG document to a writer.
    ///
    /// The description is written as XML text; markup characters are escaped and characters that XML 1.0 does not allow are replaced with U+FFFD.
    /// Pass `None::<&str>` when no description is needed.
    #[cfg(feature = "std")]
    pub fn write_svg<W: IoWrite>(
        self,
        writer: W,
        description: Option<impl AsRef<str>>,
    ) -> Result<(), RenderError> {
        let description = description.as_ref().map(AsRef::as_ref);
        let layout = self.layout()?;
        let mut writer = IoFmtWriter {
            inner: writer, error: None
        };

        if self.write_svg_content(&mut writer, description, layout).is_err() {
            return Err(writer.error.expect("the I/O adapter stores formatting errors").into());
        }

        writer.inner.flush()?;
        Ok(())
    }

    /// Renders an SVG document as a UTF-8 string.
    ///
    /// The description is written as XML text; markup characters are escaped and characters that XML 1.0 does not allow are replaced with U+FFFD.
    /// Pass `None::<&str>` when no description is needed.
    pub fn to_svg_string(
        self,
        description: Option<impl AsRef<str>>,
    ) -> Result<String, RenderError> {
        let description = description.as_ref().map(AsRef::as_ref);
        let layout = self.layout()?;
        let mut svg = String::with_capacity(8192);

        self.write_svg_content(&mut svg, description, layout)
            .expect("writing an SVG to a String cannot fail");

        Ok(svg)
    }

    fn write_svg_content<W: fmt::Write>(
        self,
        writer: &mut W,
        description: Option<&str>,
        layout: Layout,
    ) -> fmt::Result {
        if self.svg_xml_declaration {
            writer.write_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n")?;
        }

        writeln!(
            writer,
            "<svg width=\"{}\" height=\"{}\" shape-rendering=\"crispEdges\" version=\"1.1\" xmlns=\"http://www.w3.org/2000/svg\">",
            self.width, self.height
        )?;

        if let Some(description) = description {
            if !description.is_empty() {
                writer.write_str("\t<desc>")?;
                write_xml_text(writer, description)?;
                writer.write_str("</desc>\n")?;
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

        let symbol_width = self.symbol.width();
        let modules = self.symbol.modules();

        for y in 0..self.symbol.height() {
            // The row is read directly from the module slice because x and y are always in range here.
            let module_row = &modules[y * symbol_width..][..symbol_width];

            // One SVG rectangle command represents each horizontal run of dark modules.
            let mut x = 0;

            while x < symbol_width {
                if !module_row[x] {
                    x += 1;
                    continue;
                }

                let start = x;

                while x < symbol_width && module_row[x] {
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
        writer.write_str("\"/>\n</svg>")
    }

    #[cfg(feature = "tokio")]
    /// Renders an SVG document in memory, then writes and flushes it to a tokio asynchronous writer.
    ///
    /// The description is written as XML text; markup characters are escaped and characters that XML 1.0 does not allow are replaced with U+FFFD.
    /// Pass `None::<&str>` when no description is needed.
    pub async fn write_svg_async<W: TokioAsyncWrite + Unpin>(
        self,
        mut writer: W,
        description: Option<impl AsRef<str>>,
    ) -> Result<(), RenderError> {
        let svg = self.to_svg_string(description)?;

        writer.write_all(svg.as_bytes()).await?;
        writer.flush().await?;

        Ok(())
    }

    /// Atomically saves an SVG document after rendering succeeds.
    ///
    /// The description is written as XML text; markup characters are escaped and characters that XML 1.0 does not allow are replaced with U+FFFD.
    /// Pass `None::<&str>` when no description is needed.
    #[cfg(feature = "std")]
    pub fn save_svg(
        self,
        path: impl AsRef<Path>,
        description: Option<impl AsRef<str>>,
    ) -> Result<(), RenderError> {
        let mut file = AtomicWriteFile::open(path)?;

        // Buffering coalesces the many small SVG path writes into a few large writes to the file.
        self.write_svg(io::BufWriter::new(&mut file), description)?;

        file.commit()?;

        Ok(())
    }

    #[cfg(feature = "tokio")]
    /// Atomically saves an SVG document after rendering succeeds, offloading the write to tokio's blocking pool.
    ///
    /// Like [`save_svg`](Self::save_svg), the write goes through a temporary file, so an existing file is left untouched if it fails.
    /// The description is written as XML text; markup characters are escaped and characters that XML 1.0 does not allow are replaced with U+FFFD.
    /// Pass `None::<&str>` when no description is needed.
    pub async fn save_svg_async(
        self,
        path: impl AsRef<Path>,
        description: Option<impl AsRef<str>>,
    ) -> Result<(), RenderError> {
        let svg = self.to_svg_string(description)?;

        save_atomic_blocking(path.as_ref().to_path_buf(), svg.into_bytes()).await
    }

    #[cfg(feature = "image")]
    /// Writes a grayscale PNG image to a writer, then flushes it.
    pub fn write_png<W: IoWrite>(self, writer: W) -> Result<(), RenderError> {
        let width = u32::try_from(self.width).map_err(|_| RenderError::ImageSizeTooLarge)?;
        let height = u32::try_from(self.height).map_err(|_| RenderError::ImageSizeTooLarge)?;
        let image = self.to_luma8()?;
        let mut writer = ErrorKeepingWriter {
            inner: writer, error: None
        };
        let result =
            PngEncoder::new_with_quality(&mut writer, CompressionType::Best, FilterType::NoFilter)
                .write_image(&image, width, height, ColorType::L8.into());

        // The PNG encoder ignores the write error of its final chunk, so the kept error is checked before its result.
        if let Some(error) = writer.error.take() {
            return Err(error.into());
        }

        result?;
        writer.inner.flush()?;
        Ok(())
    }

    #[cfg(feature = "image")]
    /// Renders a grayscale PNG image into a byte vector.
    pub fn to_png_vec(self) -> Result<Vec<u8>, RenderError> {
        let mut bytes = Vec::with_capacity(4096);

        self.write_png(&mut bytes)?;

        Ok(bytes)
    }

    #[cfg(all(feature = "tokio", feature = "image"))]
    /// Renders a PNG image in memory, then writes and flushes it to a tokio asynchronous writer.
    pub async fn write_png_async<W: TokioAsyncWrite + Unpin>(
        self,
        mut writer: W,
    ) -> Result<(), RenderError> {
        let png = self.to_png_vec()?;

        writer.write_all(&png).await?;
        writer.flush().await?;

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

    #[cfg(all(feature = "tokio", feature = "image"))]
    /// Atomically saves a PNG image after rendering succeeds, offloading the write to tokio's blocking pool.
    ///
    /// Like [`save_png`](Self::save_png), the write goes through a temporary file, so an existing file is left untouched if it fails.
    pub async fn save_png_async(self, path: impl AsRef<Path>) -> Result<(), RenderError> {
        let png = self.to_png_vec()?;

        save_atomic_blocking(path.as_ref().to_path_buf(), png).await
    }

    #[cfg(feature = "image")]
    /// Renders a grayscale image buffer.
    pub fn to_image_buffer(self) -> Result<ImageBuffer<Luma<u8>, Vec<u8>>, RenderError> {
        let width = u32::try_from(self.width).map_err(|_| RenderError::ImageSizeTooLarge)?;
        let height = u32::try_from(self.height).map_err(|_| RenderError::ImageSizeTooLarge)?;
        let image = self.to_luma8()?;

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

/// Draws the symbol as compact Unicode text using half-block characters, with one module per column and two module rows per line.
///
/// The alternate form (`{:#}`) inverts dark and light modules for dark terminal backgrounds.
/// Pixel dimensions are ignored; only the quiet zone setting applies.
/// Each line ends with a newline.
/// Panics if the quiet zone makes the drawing wider or taller than `usize::MAX` modules.
///
/// Note: half-block characters have ambiguous East Asian width and may render as two columns in some CJK terminals, causing misalignment.
impl fmt::Display for Renderer<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let visible_dark = !f.alternate();
        let quiet_zone = self.quiet_zone;
        let quiet_zone_modules = quiet_zone.checked_mul(2);
        let with_quiet_zone = |modules: usize| {
            quiet_zone_modules
                .and_then(|margin| modules.checked_add(margin))
                .expect("the quiet zone is too large")
        };
        let columns = with_quiet_zone(self.symbol.width());
        let rows = with_quiet_zone(self.symbol.height());

        for upper in (0..rows).step_by(2) {
            for x in 0..columns {
                let visible = |y: usize| {
                    // Out-of-range lookups return `None`, so the quiet zone reads as light without a boundary check.
                    let dark = x
                        .checked_sub(quiet_zone)
                        .zip(y.checked_sub(quiet_zone))
                        .and_then(|(x, y)| self.symbol.module(x, y))
                        == Some(true);

                    dark == visible_dark
                };

                // The leftover half of an odd final line stays blank in both forms.
                let lower = upper + 1 < rows && visible(upper + 1);

                f.write_str(match (visible(upper), lower) {
                    (true, true) => "█",
                    (true, false) => "▀",
                    (false, true) => "▄",
                    (false, false) => " ",
                })?;
            }

            f.write_str("\n")?;
        }

        Ok(())
    }
}

// Writes text as XML 1.0 character data.
// Characters that XML 1.0 does not allow, even as character references, are replaced with U+FFFD.
fn write_xml_text<W: fmt::Write>(writer: &mut W, text: &str) -> fmt::Result {
    let mut start = 0;

    for (index, c) in text.char_indices() {
        let replacement = match c {
            '&' => "&amp;",
            '<' => "&lt;",
            // Escaping `>` keeps `]]>` out of the character data.
            '>' => "&gt;",
            // A literal CR would be normalized to LF by XML parsers.
            '\r' => "&#xD;",
            '\t' | '\n' => continue,
            '\u{0}'..='\u{1F}' | '\u{FFFE}' | '\u{FFFF}' => "\u{FFFD}",
            _ => continue,
        };

        writer.write_str(&text[start..index])?;
        writer.write_str(replacement)?;
        start = index + c.len_utf8();
    }

    writer.write_str(&text[start..])
}

#[cfg(feature = "std")]
struct IoFmtWriter<W> {
    inner: W,
    error: Option<io::Error>,
}

#[cfg(feature = "std")]
impl<W: IoWrite> fmt::Write for IoFmtWriter<W> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.inner.write_all(text.as_bytes()).map_err(|error| {
            self.error = Some(error);
            fmt::Error
        })
    }
}

// Keeps the first write error, because an encoder may ignore errors that it cannot return.
#[cfg(feature = "image")]
struct ErrorKeepingWriter<W> {
    inner: W,
    error: Option<io::Error>,
}

#[cfg(feature = "image")]
impl<W: IoWrite> ErrorKeepingWriter<W> {
    fn keep<T>(&mut self, result: io::Result<T>) -> io::Result<T> {
        result.map_err(|error| {
            let kind = error.kind();

            self.error.get_or_insert(error);
            kind.into()
        })
    }
}

#[cfg(feature = "image")]
impl<W: IoWrite> IoWrite for ErrorKeepingWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        match self.inner.write(bytes) {
            // An interrupted write is retried by `write_all`, so it is not a failure yet.
            Err(error) if error.kind() == io::ErrorKind::Interrupted => Err(error),
            result => self.keep(result),
        }
    }

    fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
        let result = self.inner.write_all(bytes);

        self.keep(result)
    }

    fn flush(&mut self) -> io::Result<()> {
        let result = self.inner.flush();

        self.keep(result)
    }
}

// atomic-write-file has no async backend, so the atomic save runs on tokio's blocking pool.
#[cfg(feature = "tokio")]
async fn save_atomic_blocking(path: std::path::PathBuf, bytes: Vec<u8>) -> Result<(), RenderError> {
    let write = tokio::task::spawn_blocking(move || {
        let mut file = AtomicWriteFile::open(path)?;

        file.write_all(&bytes)?;

        file.commit()
    })
    .await;

    match write {
        Ok(result) => result.map_err(RenderError::from),
        Err(join_error) => Err(RenderError::Io(io::Error::other(join_error))),
    }
}

struct Layout {
    scale:    usize,
    margin_x: usize,
    margin_y: usize,
}
