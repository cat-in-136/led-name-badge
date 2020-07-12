use std::convert::TryFrom;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::io::{Read, Write};
#[cfg(test)]
use std::io::Cursor;
use std::mem;
use std::ops::RangeInclusive;
use std::path::PathBuf;
use std::str::FromStr;

use crate::badge::device::{BadgeType, s1144};
pub use crate::badge::error::BadgeError;
use crate::badge::font_selector::select_font;
use crate::badge::text::render_text;

pub mod device;
mod error;
mod font_selector;
mod image_io;
mod text;

/// Number of messages stored in the LED Badge
pub const N_MESSAGES: usize = 8;

/// Maximum length of message text
const MAX_STR: usize = 255;

/// Maximum number of display memory size
const DISP_SIZE: usize = 32767;

/// Height of the message
const BADGE_MSG_FONT_HEIGHT: usize = 11;

/// Message effect type
#[derive(Debug, PartialEq, Copy, Clone)]
#[allow(dead_code)]
pub enum BadgeEffect {
    Left = 0,
    Right,
    Up,
    Down,
    Freeze,
    Animation,
    Snow,
    Volume,
    Laser,
}

impl BadgeEffect {
    pub fn values() -> impl Iterator<Item = BadgeEffect> {
        (0..)
            .map(|v| BadgeEffect::try_from(v))
            .take_while(|v| v.is_ok())
            .map(|v| v.unwrap())
    }
}

#[test]
fn test_badge_effect_values() {
    let values = BadgeEffect::values().collect::<Vec<_>>();
    assert_eq!(values[0], BadgeEffect::Left);
    assert_eq!(values[values.len() - 1], BadgeEffect::Laser);
}

impl fmt::Display for BadgeEffect {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(format!("{:?}", self).to_lowercase().as_str())
    }
}

#[test]
fn test_badge_effect_display() {
    assert_eq!(BadgeEffect::Left.to_string(), String::from("left"));
}

impl TryFrom<u8> for BadgeEffect {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value <= (BadgeEffect::Laser as u8) {
            Ok(unsafe { mem::transmute(value) })
        } else {
            Err(())
        }
    }
}

#[test]
fn test_badge_effect_from_u8() {
    assert_eq!(BadgeEffect::try_from(0).unwrap(), BadgeEffect::Left);
    assert_eq!(
        BadgeEffect::try_from(BadgeEffect::Down as u8).unwrap(),
        BadgeEffect::Down
    );
    assert_eq!(
        BadgeEffect::try_from(BadgeEffect::Laser as u8).unwrap(),
        BadgeEffect::Laser
    );
    assert_eq!(
        BadgeEffect::try_from((BadgeEffect::Laser as u8) + 1),
        Err(())
    );
}

impl FromStr for BadgeEffect {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        BadgeEffect::values()
            .find(|&v| v.to_string().as_str() == value)
            .map_or(Err(Self::Err::default()), |v| Ok(v))
    }
}

#[test]
fn badge_effect_from_str() {
    assert_eq!(BadgeEffect::from_str("left").unwrap(), BadgeEffect::Left);
    assert_eq!(BadgeEffect::from_str("left2"), Err(()));
}

/// Value range of text animation speed
pub const BADGE_SPEED_RANGE: RangeInclusive<u8> = 1..=8;

/// Value range of LED brightness
pub const BADGE_BRIGHTNESS_RANGE: RangeInclusive<u8> = 0..=4;

/// Badge Protocol Header (first report to send)
#[derive(Debug)]
pub struct BadgeMessage {
    /// blink (flash) messages
    pub blink: bool,
    /// frame (border) messages
    pub frame: bool,
    /// speed[1..8]
    pub speed: u8,
    /// effect[0..8]
    pub effect: BadgeEffect,
    /// characters as bitmasks (8x11), stuffed together to fill the reports
    pub data: Vec<u8>,
}

impl Default for BadgeMessage {
    fn default() -> Self {
        BadgeMessage {
            blink: false,
            frame: false,
            speed: 0,
            effect: BadgeEffect::Left,
            data: Vec::with_capacity(MAX_STR * BADGE_MSG_FONT_HEIGHT),
        }
    }
}

/// Badge context
pub struct Badge {
    /// badge brightness
    pub brightness: u8,
    /// message
    pub messages: [BadgeMessage; N_MESSAGES],
}

impl Badge {
    /// Create Badge config entity
    pub fn new() -> Result<Self, BadgeError> {
        Ok(Badge {
            brightness: 7,
            messages: Default::default(),
        })
    }

    /// Add text messages
    pub fn add_text_message(
        &mut self,
        msg_num: usize,
        msg: &str,
        font_names: &[&str],
    ) -> Result<(), BadgeError> {
        if msg_num >= N_MESSAGES {
            Err(BadgeError::MessageNumberOutOfRange(msg_num))
        } else if msg.len() == 0 {
            Ok(()) // Do nothing
        } else {
            let pixel_height = BADGE_MSG_FONT_HEIGHT;
            let (font_path, font_index) = font_names
                .get(0)
                .and_then(|&v| {
                    let path = PathBuf::from(v);
                    if path.exists() {
                        Some(Ok((path, 0)))
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| select_font(font_names, Some(pixel_height)))?;

            let mut pixel_data = render_text(msg, pixel_height, font_path.as_ref(), font_index)?;
            mem::swap(&mut self.messages[msg_num].data, &mut pixel_data);
            Ok(())
        }
    }

    /// Add Png message
    pub fn add_png_message<R: Read>(
        &mut self,
        msg_num: usize,
        reader: R,
    ) -> Result<(), BadgeError> {
        if msg_num >= N_MESSAGES {
            Err(BadgeError::MessageNumberOutOfRange(msg_num))
        } else {
            let mut pixel_data = image_io::read_png_to_badge_message(reader)
                .map_err(|e| BadgeError::PngReadError(None, e))?;
            mem::swap(&mut self.messages[msg_num].data, &mut pixel_data);
            Ok(())
        }
    }

    /// Set effect pattern
    pub fn set_effect_pattern(
        &mut self,
        msg_num: usize,
        pat: BadgeEffect,
    ) -> Result<(), BadgeError> {
        if msg_num >= N_MESSAGES {
            Err(BadgeError::MessageNumberOutOfRange(msg_num))
        } else {
            self.messages[msg_num].effect = pat;
            Ok(())
        }
    }

    /// Set effect speed
    pub fn set_effect_speed(&mut self, msg_num: usize, spd: u8) -> Result<(), BadgeError> {
        if msg_num >= N_MESSAGES {
            Err(BadgeError::MessageNumberOutOfRange(msg_num))
        } else if !BADGE_SPEED_RANGE.contains(&spd) {
            Err(BadgeError::WrongSpeed)
        } else {
            self.messages[msg_num].speed = spd;
            Ok(())
        }
    }

    /// Set effect blink
    pub fn set_effect_blink(&mut self, msg_num: usize, blink: bool) -> Result<(), BadgeError> {
        if msg_num >= N_MESSAGES {
            Err(BadgeError::MessageNumberOutOfRange(msg_num))
        } else {
            self.messages[msg_num].blink = blink;
            Ok(())
        }
    }

    /// Set effects
    pub fn set_effect_frame(&mut self, msg_num: usize, frame: bool) -> Result<(), BadgeError> {
        if msg_num >= N_MESSAGES {
            Err(BadgeError::MessageNumberOutOfRange(msg_num))
        } else {
            self.messages[msg_num].frame = frame;
            Ok(())
        }
    }

    /// Set brightness
    pub fn set_brightness(&mut self, br: u8) -> Result<(), BadgeError> {
        if !BADGE_BRIGHTNESS_RANGE.contains(&br) {
            Err(BadgeError::WrongBrightness)
        } else {
            self.brightness = br;
            Ok(())
        }
    }

    /// Send the context information to the device
    ///
    /// # Errors
    ///
    /// If failed to write the data to the device, then an error is returned.\
    pub fn send(&mut self, badge_type: BadgeType) -> Result<(), BadgeError> {
        match badge_type {
            BadgeType::S1144 => s1144::s1144_send(self),
            BadgeType::B1248 => unimplemented!(),
        }
    }

    /// Write png data to the writer instead of badge
    pub fn write_to_png<W: Write>(&self, msg_num: usize, writer: W) -> Result<(), BadgeError> {
        if msg_num >= N_MESSAGES {
            Err(BadgeError::MessageNumberOutOfRange(msg_num))
        } else if self.messages[msg_num].data.is_empty() {
            Err(BadgeError::NoDataToWrite)
        } else {
            let message_data = self.messages[msg_num].data.as_slice();
            image_io::write_badge_message_to_png(message_data, writer)
                .map_err(|e| BadgeError::PngWriteError(None, e))
        }
    }
}

#[test]
fn test_badge_new() {
    assert!(matches!(Badge::new(), Ok(_)));
}

#[test]
fn test_add_png_message() {
    let mut badge = Badge::new().unwrap();

    let valid_8x11_png = vec![
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x0b, 0x01, 0x00, 0x00, 0x00, 0x00, 0x6a,
        0xe0, 0xf1, 0x88, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0x63, 0xf8,
        0x8f, 0x0d, 0x02, 0x00, 0x78, 0x9d, 0x0a, 0xf6, 0xc1, 0x81, 0x34, 0x05, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];
    let corrupted_data = vec![0; 1];

    let reader = Cursor::new(&valid_8x11_png);
    assert!(matches!(
        badge.add_png_message(N_MESSAGES, reader),
        Err(BadgeError::MessageNumberOutOfRange(N_MESSAGES))
    ));

    let reader = Cursor::new(&corrupted_data);
    assert!(matches!(
        badge.add_png_message(N_MESSAGES - 1, reader),
        Err(BadgeError::PngReadError(None, _))
    ));

    let reader = Cursor::new(&valid_8x11_png);
    assert!(badge.add_png_message(N_MESSAGES - 1, reader).is_ok());
    assert_eq!(
        badge.messages[N_MESSAGES - 1].data,
        &[0xff; BADGE_MSG_FONT_HEIGHT]
    );
}

#[test]
fn test_badge_add_text_message() {
    let mut badge = Badge::new().unwrap();
    let font_names = &["Liberation Sans", "Arial"];

    assert!(matches!(
        badge.add_text_message(N_MESSAGES, "", font_names),
        Err(BadgeError::MessageNumberOutOfRange(N_MESSAGES))
    ));

    assert!(matches!(
        badge.add_text_message(N_MESSAGES - 1, "", font_names),
        Ok(())
    ));
    assert!(badge.messages[N_MESSAGES - 1].data.iter().all(|&v| v == 0));

    assert!(matches!(badge.add_text_message(0, "A", font_names), Ok(())));
    assert!(badge.messages[0].data.iter().any(|&v| v != 0));
}

#[test]
fn test_badge_set_effect_pattern() {
    let mut badge = Badge::new().unwrap();

    assert!(matches!(
        badge.set_effect_pattern(N_MESSAGES, BadgeEffect::Left),
        Err(BadgeError::MessageNumberOutOfRange(N_MESSAGES))
    ));

    assert!(matches!(
        badge.set_effect_pattern(N_MESSAGES - 1, BadgeEffect::Laser),
        Ok(())
    ));
    assert_eq!(badge.messages[N_MESSAGES - 1].effect, BadgeEffect::Laser);

    assert!(matches!(
        badge.set_effect_pattern(0, BadgeEffect::Left),
        Ok(())
    ));
    assert_eq!(badge.messages[0].effect, BadgeEffect::Left);
}

#[test]
fn test_badge_set_effect_speed() {
    let mut badge = Badge::new().unwrap();

    assert!(matches!(
        badge.set_effect_speed(N_MESSAGES, 1),
        Err(BadgeError::MessageNumberOutOfRange(N_MESSAGES))
    ));

    assert!(matches!(
        badge.set_effect_speed(0, 0),
        Err(BadgeError::WrongSpeed)
    ));
    assert!(matches!(
        badge.set_effect_speed(0, 9),
        Err(BadgeError::WrongSpeed)
    ));

    assert!(matches!(badge.set_effect_speed(0, 1), Ok(())));
    assert_eq!(badge.messages[0].speed, 1);
    assert!(matches!(badge.set_effect_speed(N_MESSAGES - 1, 8), Ok(())));
    assert_eq!(badge.messages[N_MESSAGES - 1].speed, 8);
}

#[test]
fn test_badge_set_effect_blink() {
    let mut badge = Badge::new().unwrap();

    assert!(matches!(
        badge.set_effect_blink(N_MESSAGES, true),
        Err(BadgeError::MessageNumberOutOfRange(N_MESSAGES))
    ));

    assert!(matches!(
        badge.set_effect_blink(N_MESSAGES - 1, true),
        Ok(())
    ));
    assert_eq!(badge.messages[N_MESSAGES - 1].blink, true);
    assert!(matches!(
        badge.set_effect_blink(N_MESSAGES - 1, false),
        Ok(())
    ));
    assert_eq!(badge.messages[N_MESSAGES - 1].blink, false);
}

#[test]
fn test_badge_set_effect_frame() {
    let mut badge = Badge::new().unwrap();

    assert!(matches!(
        badge.set_effect_frame(N_MESSAGES, true),
        Err(BadgeError::MessageNumberOutOfRange(N_MESSAGES))
    ));

    assert!(matches!(
        badge.set_effect_frame(N_MESSAGES - 1, true),
        Ok(())
    ));
    assert_eq!(badge.messages[N_MESSAGES - 1].frame, true);
    assert!(matches!(
        badge.set_effect_frame(N_MESSAGES - 1, false),
        Ok(())
    ));
    assert_eq!(badge.messages[N_MESSAGES - 1].frame, false);
}

#[test]
fn test_write_to_png() {
    let mut badge = Badge::new().unwrap();

    let mut png_data = Vec::<u8>::new();
    assert!(matches!(
        badge.write_to_png(N_MESSAGES, &mut png_data),
        Err(BadgeError::MessageNumberOutOfRange(N_MESSAGES))
    ));

    let mut png_data = Vec::<u8>::new();
    assert!(matches!(
        badge.write_to_png(N_MESSAGES - 1, &mut png_data),
        Err(BadgeError::NoDataToWrite)
    ));

    badge.messages[N_MESSAGES - 1].data = vec![0; BADGE_MSG_FONT_HEIGHT];
    let mut png_data = Vec::<u8>::new();
    let mut w = Cursor::new(&mut png_data);
    assert!(matches!(
        badge.write_to_png(N_MESSAGES - 1, w.get_mut()),
        Ok(())
    ));
    assert_eq!(
        &png_data[0..8],
        &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
    );
}
