use std::{error::Error, fmt, io};

#[cfg(feature = "micro-qr")]
use crate::{SymbolErrorCorrection, SymbolVersion};

/// Errors returned while encoding a QR Code symbol.
#[derive(Debug)]
#[non_exhaustive]
pub enum EncodeError {
    /// The encoded bit stream exceeds the selected symbol capacity.
    DataTooLong { required_bits: usize, capacity_bits: usize },
    /// The input contains a byte that is not valid for the requested mode.
    InvalidData { mode: &'static str, byte_offset: usize },
    /// The text cannot be represented by the selected symbol family.
    #[cfg(feature = "micro-qr")]
    TextNotRepresentable { byte_offset: usize, family: &'static str },
    /// The ECI assignment is outside the range supported by QR Code.
    #[cfg(any(feature = "qr", feature = "rmqr"))]
    InvalidEciAssignment(u32),
    /// The FNC1 second-position application indicator is invalid.
    #[cfg(any(feature = "qr", feature = "rmqr"))]
    InvalidApplicationIndicator,
    /// The selected version range is empty or invalid.
    InvalidVersionRange,
    /// The selected symbol family does not support a requested mode.
    #[cfg(feature = "micro-qr")]
    UnsupportedMode { mode: &'static str, family: &'static str },
    /// The selected version does not support the requested error correction level.
    #[cfg(feature = "micro-qr")]
    UnsupportedErrorCorrection {
        version:          SymbolVersion,
        error_correction: SymbolErrorCorrection,
    },
    /// The requested mask number is outside the supported range.
    InvalidMask,
    /// A Structured Append sequence has fewer than one or more than 16 parts.
    #[cfg(feature = "qr")]
    InvalidStructuredAppendPartCount { count: usize },
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DataTooLong {
                required_bits,
                capacity_bits,
            } => write!(
                f,
                "the encoded data needs {required_bits} bits but the selected symbols hold \
                 {capacity_bits} bits"
            ),
            Self::InvalidData {
                mode,
                byte_offset,
            } => {
                write!(f, "invalid {mode} data at byte offset {byte_offset}")
            },
            #[cfg(feature = "micro-qr")]
            Self::TextNotRepresentable {
                byte_offset,
                family,
            } => write!(f, "text at byte offset {byte_offset} cannot be represented by {family}"),
            #[cfg(any(feature = "qr", feature = "rmqr"))]
            Self::InvalidEciAssignment(value) => {
                write!(f, "ECI assignment {value} is outside 0..=999999")
            },
            #[cfg(any(feature = "qr", feature = "rmqr"))]
            Self::InvalidApplicationIndicator => f.write_str("invalid FNC1 application indicator"),
            Self::InvalidVersionRange => f.write_str("invalid symbol version range"),
            #[cfg(feature = "micro-qr")]
            Self::UnsupportedMode {
                mode,
                family,
            } => {
                write!(f, "{mode} mode is not supported by {family}")
            },
            #[cfg(feature = "micro-qr")]
            Self::UnsupportedErrorCorrection {
                version,
                error_correction,
            } => write!(f, "{error_correction:?} error correction is not supported by {version:?}"),
            Self::InvalidMask => f.write_str("the mask number is outside the supported range"),
            #[cfg(feature = "qr")]
            Self::InvalidStructuredAppendPartCount {
                count,
            } => {
                write!(f, "Structured Append requires 1..=16 parts, got {count}")
            },
        }
    }
}

impl Error for EncodeError {}

/// Errors returned while rendering a QR Code symbol.
#[derive(Debug)]
#[non_exhaustive]
pub enum RenderError {
    /// The requested output is too small for an integer module scale and quiet zone.
    ImageSizeTooSmall,
    /// The requested output dimensions overflow a supported image size.
    ImageSizeTooLarge,
    /// An input or output operation failed.
    Io(io::Error),
    #[cfg(feature = "image")]
    /// PNG encoding failed.
    Image(image::ImageError),
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ImageSizeTooSmall => {
                f.write_str("image size is too small to draw the whole symbol")
            },
            Self::ImageSizeTooLarge => f.write_str("image size is too large to generate"),
            Self::Io(error) => fmt::Display::fmt(error, f),
            #[cfg(feature = "image")]
            Self::Image(error) => fmt::Display::fmt(error, f),
        }
    }
}

impl Error for RenderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            #[cfg(feature = "image")]
            Self::Image(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for RenderError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[cfg(feature = "image")]
impl From<image::ImageError> for RenderError {
    fn from(error: image::ImageError) -> Self {
        Self::Image(error)
    }
}
