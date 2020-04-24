use core::fmt;
use std::error::Error;

use font_kit::error::{FontLoadingError, SelectionError};
use hidapi::HidError;
use png::DecodingError;

use crate::badge::image_io::{BadgeImageReadError, BadgeImageWriteError};

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
    /// Png Reading Error,
    PngReadError(Option<String>, BadgeImageReadError),
    /// Png Writing Error,
    PngWriteError(Option<String>, BadgeImageWriteError),
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
            PngReadError(path, error) => {
                let msg_summary = if let Some(path) = path {
                    format!("Could not open png file: {} :", path)
                } else {
                    "Could not read png data:".to_string()
                };
                let msg_detail = match error {
                    BadgeImageReadError::PngDecodeError(decoding_error) => match decoding_error {
                        DecodingError::IoError(e) => e.description().to_string(),
                        DecodingError::Format(data) => data.to_string(),
                        DecodingError::InvalidSignature => {
                            ("Broken File (Invalid signature)".to_string())
                        }
                        DecodingError::CrcMismatch { .. } => "Broken file (CRC Error)".to_string(),
                        DecodingError::Other(data) => data.to_string(),
                        DecodingError::CorruptFlateStream => "Corrupted Flate Stream".to_string(),
                        DecodingError::LimitsExceeded => "Limits Exceeded".to_string(),
                    },
                    BadgeImageReadError::UnsupportedPngError(data) => data.to_string(),
                };
                f.write_str(format!("{}{}", msg_summary, msg_detail).as_str())
            }
            PngWriteError(path, error) => {
                let msg_summary = if let Some(path) = path {
                    format!("Could not write to png file: {} :", path)
                } else {
                    "Could not read png data:".to_string()
                };
                let msg_detail = match error {
                    BadgeImageWriteError::PngEncodeError(e) => e.description(),
                };
                f.write_str(format!("{}{}", msg_summary, msg_detail).as_str())
            }
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
