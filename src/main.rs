extern crate hidapi;

use std::error::Error;
use std::fmt;
use std::fmt::{Debug, Write};
use std::process::exit;

use hidapi::{HidApi, HidDevice, HidError};

/// Vendor ID of the LED Badge
const BADGE_VID: u16 = 0x0416;
/// Product ID of the LED Badge
const BADGE_PID: u16 = 0x5020;

#[derive(Debug)]
pub enum BadgeError {
    /// Badge Not Found i.e. the LED Badge is not connected to the PC.
    BadgeNotFound,
    /// Multiple Badge Found
    MultipleBadgeFound,
    /// Could not open device
    CouldNotOpenDevice(HidError),
    /// IO Error.
    Io(HidError),
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
            Io(_error) => f.write_str("IO Error"),
        }
    }
}

impl From<HidError> for BadgeError {
    fn from(e: HidError) -> Self {
        BadgeError::Io(e)
    }
}

pub struct Badge {
    device: HidDevice,
}

impl Badge {
    fn new() -> Result<Self, BadgeError> {
        let api = HidApi::new()?;

        match api
            .device_list()
            .filter(|info| info.vendor_id() == BADGE_VID && info.product_id() == BADGE_PID)
            .count()
        {
            0 => Err(BadgeError::BadgeNotFound),
            1 => Ok(()),
            _ => Err(BadgeError::MultipleBadgeFound),
        }?;

        let device = api
            .open(BADGE_VID, BADGE_PID)
            .map_err(|e| BadgeError::CouldNotOpenDevice(e))?;

        Ok(Badge { device })
    }
}

fn main() {
    let _badge = match Badge::new() {
        Ok(badge) => badge,
        Err(err) => {
            eprintln!("Error: {}", err);
            exit(1);
            return;
        }
    };

    println!("Badge Found!");
    exit(0);
}
