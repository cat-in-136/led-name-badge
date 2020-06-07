use core::fmt;
use core::fmt::Debug;
use std::error;

use freetype::Error as FtError;
use hidapi::HidError;

use crate::badge::font_finder::FontFinderError;
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
    FontNotFound(FontFinderError),
    /// Font Loading Error
    FontLoading(FtError),
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
            FontNotFound(error) => f.write_fmt(format_args!("Failed to find font: {}", error)),
            FontLoading(error) => f.write_fmt(format_args!("Failed to load font: {}", error)),
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
                f.write_fmt(format_args!("{}{}", msg_summary, error))
            }
            PngWriteError(path, error) => {
                let msg_summary = if let Some(path) = path {
                    format!("Could not write to png file: {} :", path)
                } else {
                    "Could not read png data:".to_string()
                };
                f.write_fmt(format_args!("{}{}", msg_summary, error))
            }
            NoDataToWrite => f.write_str("No data to write"),
        }
    }
}

impl error::Error for BadgeError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            BadgeError::BadgeNotFound => None,
            BadgeError::MultipleBadgeFound => None,
            BadgeError::CouldNotOpenDevice(e) => Some(e),
            BadgeError::MessageNumberOutOfRange(_) => None,
            BadgeError::WrongSpeed => None,
            BadgeError::WrongBrightness => None,
            BadgeError::HidIo(e) => Some(e),
            BadgeError::FontNotFound(e) => Some(e),
            BadgeError::FontLoading(e) => Some(e),
            BadgeError::FileIo(_, e) => Some(e),
            BadgeError::PngReadError(_, e) => Some(e),
            BadgeError::PngWriteError(_, e) => Some(e),
            BadgeError::NoDataToWrite => None,
        }
    }
}

impl From<FontFinderError> for BadgeError {
    fn from(e: FontFinderError) -> Self {
        BadgeError::FontNotFound(e)
    }
}

impl From<FtError> for BadgeError {
    fn from(e: FtError) -> Self {
        BadgeError::FontLoading(e)
    }
}

impl From<HidError> for BadgeError {
    fn from(e: HidError) -> Self {
        BadgeError::HidIo(e)
    }
}
