use std::io;

use crate::image::ImageError;

/// Errors when encoding QR code.
#[derive(Debug)]
pub enum QRCodeError {
    IOError(io::Error),
    ImageError(ImageError),
}

impl From<io::Error> for QRCodeError {
    #[inline]
    fn from(error: io::Error) -> Self {
        QRCodeError::IOError(error)
    }
}

impl From<ImageError> for QRCodeError {
    #[inline]
    fn from(error: ImageError) -> Self {
        QRCodeError::ImageError(error)
    }
}
