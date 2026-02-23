use core::fmt::Debug;

use freetype::Error as FtError;
use hidapi::HidError;

use crate::badge::font_selector::FontSelectorError;
use crate::badge::image_io::{BadgeImageReadError, BadgeImageWriteError};

fn format_io_error_path(path: &Option<String>) -> String {
    if let Some(p) = path {
        format!(" : {}", p)
    } else {
        String::new()
    }
}

/// Describes an error related to the LED Badge operation
#[derive(thiserror::Error, Debug)]
pub enum BadgeError {
    /// Badge Not Found i.e. the LED Badge is not connected to the PC.
    #[error("Badge Not Found")]
    BadgeNotFound,
    /// Multiple Badge Found
    #[error("Multiple Badge Found")]
    MultipleBadgeFound,
    /// Could not open device
    #[error("Could not open device: {0}")]
    CouldNotOpenDevice(HidError),
    /// Out of Index of the message number
    #[error("Wrong message number ({0})")]
    MessageNumberOutOfRange(usize),
    /// Wrong speed value
    #[error("Wrong speed value")]
    WrongSpeed,
    /// Wrong brightness value
    #[error("Wrong brightness value")]
    WrongBrightness,
    /// HID IO Error.
    #[error("Device IO Error")]
    HidIo(HidError),
    /// Font Not Found
    #[error("Failed to find font: {0}")]
    FontNotFound(FontSelectorError),
    /// Font Loading Error
    #[error("Failed to load font: {0}")]
    FontLoading(FtError),
    /// File IO Error
    #[error("File IO Error: {source_error}{path_info}", source_error = .1, path_info = format_io_error_path(.0))]
    FileIo(Option<String>, #[source] std::io::Error),
    /// Png Reading Error
    #[error("Could not read the png file: {source_error}{path_info}", source_error = .1, path_info = format_io_error_path(.0))]
    PngReadError(Option<String>, #[source] BadgeImageReadError),
    #[error("Could not write the png file: {source_error}{path_info}", source_error = .1, path_info = format_io_error_path(.0))]
    PngWriteError(Option<String>, #[source] BadgeImageWriteError),
    /// No data to write
    #[error("No data to write")]
    NoDataToWrite,
}

impl From<hidapi::HidError> for BadgeError {
    fn from(err: hidapi::HidError) -> Self {
        BadgeError::HidIo(err)
    }
}

impl From<FontSelectorError> for BadgeError {
    fn from(err: FontSelectorError) -> Self {
        BadgeError::FontNotFound(err)
    }
}

impl From<freetype::Error> for BadgeError {
    fn from(err: freetype::Error) -> Self {
        BadgeError::FontLoading(err)
    }
}
