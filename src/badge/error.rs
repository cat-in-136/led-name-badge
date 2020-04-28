use core::fmt;
use std::io::Error;
use std::path::Path;

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
    /// File IO Error related to specific file
    FileIo(Option<String>, std::io::Error),
    /// Png Reading Error
    PngReadError(Option<String>, BadgeImageReadError),
    /// Png Writing Error
    PngWriteError(Option<String>, BadgeImageWriteError),
    /// No data to write
    NoDataToWrite,
}

impl fmt::Display for BadgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use BadgeError::*;

        match self {
            BadgeNotFound => f.write_str("Badge Not Found"),
            MultipleBadgeFound => f.write_str("Multiple Badge Found"),
            CouldNotOpenDevice(error) => {
                f.write_fmt(format_args!("Could not open device: {}", error))
            }
            MessageNumberOutOfRange(msg_num) => {
                f.write_fmt(format_args!("Wrong message number ({})", msg_num))
            }
            WrongSpeed => f.write_str("Wrong speed value"),
            WrongBrightness => f.write_str("Wrong brightness value"),
            HidIo(_error) => f.write_str("Device IO Error"),
            FontNotFound(error) => f.write_fmt(format_args!("Font Not Found: {}", error)),
            FontLoading(_error) => f.write_str("Failed to load font"),
            FileIo(path, error) => {
                if let Some(path) = path {
                    f.write_fmt(format_args!("File IO Error: {}: {}", path, error))
                } else {
                    f.write_fmt(format_args!("File IO Error: {}", error))
                }
            }
            PngReadError(path, error) => {
                let msg_summary = if let Some(path) = path {
                    format!("Could not open png file: {} :", path)
                } else {
                    "Could not read png data:".to_string()
                };
                let msg_detail = match error {
                    BadgeImageReadError::PngDecodeError(decoding_error) => match decoding_error {
                        DecodingError::IoError(e) => format!("{}", e),
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
                f.write_fmt(format_args!("{}{}", msg_summary, msg_detail))
            }
            PngWriteError(path, error) => {
                let msg_summary = if let Some(path) = path {
                    format!("Could not write to png file: {} :", path)
                } else {
                    "Could not read png data:".to_string()
                };
                let msg_detail = match error {
                    BadgeImageWriteError::PngEncodeError(e) => format!("{}", e),
                };
                f.write_fmt(format_args!("{}{}", msg_summary, msg_detail))
            }
            NoDataToWrite => f.write_str("No data to write"),
        }
    }
}

impl From<HidError> for BadgeError {
    fn from(e: HidError) -> Self {
        BadgeError::HidIo(e)
    }
}

impl From<(&Path, std::io::Error)> for BadgeError {
    fn from((path, e): (&Path, Error)) -> Self {
        BadgeError::FileIo(path.to_str().map_or(None, |v| Some(v.to_string())), e)
    }
}
