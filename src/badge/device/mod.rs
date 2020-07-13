use crate::badge::{Badge, BadgeError};

mod b1248;
mod s1144;

/// Badge type
#[derive(Debug, PartialEq, Copy, Clone)]
#[allow(dead_code)]
pub enum BadgeType {
    Auto = 0,
    S1144,
    B1248,
}

/// Find device and then send the context information to the device
///
/// # Errors
///
/// If failed to write the data to the device, then an error is returned.
fn find_device_and_send(badge: &Badge) -> Result<(), BadgeError> {
    for send_attempt in [s1144::s1144_send, b1248::b1248_send].iter() {
        match send_attempt(badge) {
            Err(BadgeError::BadgeNotFound) => (), // go to the next attempt
            result => return result,
        }
    }
    Err(BadgeError::BadgeNotFound)
}

/// Send the context information to the given device
///
/// # Errors
///
/// If failed to write the data to the device, then an error is returned.
pub fn device_send(badge_type: BadgeType, badge: &Badge) -> Result<(), BadgeError> {
    match badge_type {
        BadgeType::Auto => find_device_and_send(badge),
        BadgeType::S1144 => s1144::s1144_send(badge),
        BadgeType::B1248 => b1248::b1248_send(badge),
    }
}
