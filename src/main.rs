extern crate hidapi;

use std::fmt;
use std::fmt::Debug;
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
    /// IO Error.
    Io(HidError),
}

impl fmt::Display for BadgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use BadgeError::*;

        match self {
            BadgeNotFound => f.write_str("Badge Not Found"),
            MultipleBadgeFound => f.write_str("Multiple Badge Found"),
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

        let num_badge = api
            .device_list()
            .filter(|info| info.vendor_id() == BADGE_VID && info.product_id() == BADGE_PID)
            .count();
        if num_badge == 0 {
            return Err(BadgeError::BadgeNotFound);
        } else if num_badge > 1 {
            return Err(BadgeError::MultipleBadgeFound);
        }

        let device = api.open(BADGE_VID, BADGE_PID)?;

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
