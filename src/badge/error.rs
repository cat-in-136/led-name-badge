use core::fmt;
use std::error::Error;

use font_kit::error::{FontLoadingError, SelectionError};
use hidapi::HidError;
use png::EncodingError;

/// Describes an error related to the LED Badge operation
#[derive(Debug)]
pub enum BadgeError {
    /// Badge Not Found i.e. the LED Badge is not connected to the PC.
    BadgeNotFound,
    /// Multiple Badge Found
    MultipleBadgeFound,
    /// Could not open device
    CouldNotOpenDevice(HidError),
    /// Out of Index of the message number
    MessageNumberOutOfRange(usize),
    /// Wrong speed value
    WrongSpeed,
    /// Wrong brightness value
    WrongBrightness,
    /// HID IO Error.
    HidIo(HidError),
    /// Font Not Found
    FontNotFound(SelectionError),
    /// Font Loading Error
    FontLoading(FontLoadingError),
    /// File IO Error
    FileIo(std::io::Error),
    /// Png Encoding Error
    PngEncodingError(String),
}

impl fmt::Display for BadgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use BadgeError::*;

        match self {
            BadgeNotFound => f.write_str("Badge Not Found"),
            MultipleBadgeFound => f.write_str("Multiple Badge Found"),
            CouldNotOpenDevice(error) => {
                f.write_str(format!("Could not open device: {}", error.description()).as_str())
            }
            MessageNumberOutOfRange(msg_num) => {
                f.write_str(format!("Wrong message number ({})", msg_num).as_str())
            }
            WrongSpeed => f.write_str("Wrong speed value"),
            WrongBrightness => f.write_str("Wrong brightness value"),
            HidIo(_error) => f.write_str("Device IO Error"),
            FontNotFound(error) => {
                f.write_str(format!("Font Not Found: {}", error.description()).as_str())
            }
            FontLoading(_error) => f.write_str("Failed to load font"),
            FileIo(error) => {
                f.write_str(format!("File IO Error: {}", error.description()).as_str())
            }
            PngEncodingError(msg) => f.write_str(format!("PNG Encoding Error: {}", msg).as_str()),
        }
    }
}

impl From<HidError> for BadgeError {
    fn from(e: HidError) -> Self {
        BadgeError::HidIo(e)
    }
}

impl From<std::io::Error> for BadgeError {
    fn from(e: std::io::Error) -> Self {
        BadgeError::FileIo(e)
    }
}

impl From<png::EncodingError> for BadgeError {
    fn from(e: png::EncodingError) -> Self {
        match e {
            EncodingError::IoError(e) => BadgeError::FileIo(e),
            EncodingError::Format(data) => BadgeError::PngEncodingError(data.to_string()),
        }
    }
}
