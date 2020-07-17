use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io;

#[cfg(feature = "image")]
use crate::image::ImageError;

/// Errors when encoding QR code.
#[derive(Debug)]
pub enum QRCodeError {
    DataTooLong,
    IOError(io::Error),
    #[cfg(feature = "image")]
    ImageError(ImageError),
    ImageSizeTooSmall,
}

impl From<io::Error> for QRCodeError {
    #[inline]
    fn from(error: io::Error) -> Self {
        QRCodeError::IOError(error)
    }
}

#[cfg(feature = "image")]
impl From<ImageError> for QRCodeError {
    #[inline]
    fn from(error: ImageError) -> Self {
        QRCodeError::ImageError(error)
    }
}

impl Display for QRCodeError {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            QRCodeError::DataTooLong => {
                f.write_str("the supplied data does not fit any QR Code version")
            }
            QRCodeError::IOError(error) => Display::fmt(error, f),
            #[cfg(feature = "image")]
            QRCodeError::ImageError(error) => Display::fmt(error, f),
            QRCodeError::ImageSizeTooSmall => {
                f.write_str("image size is too small to draw the whole QR code")
            }
        }
    }
}

impl Error for QRCodeError {}
