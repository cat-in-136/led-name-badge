use crate::badge::{Badge, BadgeError};

mod b1248;
mod s1144;

/// Badge type
#[derive(Debug, PartialEq, Copy, Clone)]
#[allow(dead_code)]
pub enum BadgeType {
    S1144 = 0,
    B1248,
}

/// Send the context information to the given device
///
/// # Errors
///
/// If failed to write the data to the device, then an error is returned.
pub fn device_send(badge_type: BadgeType, badge: &Badge) -> Result<(), BadgeError> {
    match badge_type {
        BadgeType::S1144 => s1144::s1144_send(badge),
        BadgeType::B1248 => unimplemented!(),
    }
}
