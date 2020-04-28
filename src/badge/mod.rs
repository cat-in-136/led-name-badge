use std::convert::TryFrom;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
#[cfg(test)]
use std::io::Cursor;
use std::mem;
use std::ops::RangeInclusive;
use std::path::Path;
use std::str::FromStr;

use hidapi::{HidApi, HidDevice};

pub use crate::badge::error::BadgeError;
use crate::badge::error::BadgeError::PngWriteError;
use crate::badge::text::{find_font, render_text};

mod error;
mod image_io;
mod text;

/// Vendor ID of the LED Badge
const BADGE_VID: u16 = 0x0416;
/// Product ID of the LED Badge
const BADGE_PID: u16 = 0x5020;

/// Number of messages stored in the LED Badge
pub const N_MESSAGES: usize = 8;

/// Maximum length of message text
const MAX_STR: usize = 255;

/// Maximum number of display memory size
const DISP_SIZE: usize = 32767;

/// Height of the message
const BADGE_MSG_FONT_HEIGHT: usize = 11;

/// Badge Protocol Header (first report to send)
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct BadgeHeader {
    /// magic: "wang",0x00
    pub start: [u8; 5],
    /// badge brightness
    pub brightness: u8,
    /// bit-coded: flash messages
    pub flash: u8,
    /// bit-coded: border messages
    pub border: u8,
    /// config of 8 lines; 0xAB : A-speed[1..8] , B-effect[0..8]
    pub line_conf: [u8; 8],
    /// length lines in BIG endian
    pub msg_len: [u16; N_MESSAGES],
}

impl BadgeHeader {
    /// Transmutes into a slice from the header.
    unsafe fn as_slice(&self) -> &[u8] {
        let view = self as *const _ as *const u8;
        std::slice::from_raw_parts(view, mem::size_of::<Self>())
    }
}

impl Default for BadgeHeader {
    fn default() -> Self {
        Self {
            start: [0x77, 0x61, 0x6e, 0x67, 0x00], // "wang\0
            brightness: 0,
            flash: 0,
            border: 0,
            line_conf: [0x46, 0x41, 0x47, 0x48, 0x40, 0x44, 0x46, 0x47], // "FAGH@DFG"
            msg_len: [0; N_MESSAGES],
        }
    }
}

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
    /// characters as bitmasks (8x11), stuffed together to fill the reports
    data: Vec<u8>,
}

impl Default for BadgeMessage {
    fn default() -> Self {
        Self {
            data: Vec::with_capacity(MAX_STR * BADGE_MSG_FONT_HEIGHT),
        }
    }
}

/// Badge context
pub struct Badge {
    header: BadgeHeader,
    messages: [BadgeMessage; N_MESSAGES],
}

impl Badge {
    /// Open a LED badge device
    ///
    /// # Errors
    ///
    /// If failed to open a LED badge, then an error is returned.
    fn open() -> Result<HidDevice, BadgeError> {
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

        Ok(device)
    }

    /// Create Badge config entity
    pub fn new() -> Result<Self, BadgeError> {
        Ok(Badge {
            header: Default::default(),
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
            let font = find_font(font_names)?;
            let font_size = BADGE_MSG_FONT_HEIGHT as u32;
            let mut pixel_data = render_text(msg, font_size, &font);
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

    /// Add Png-file message
    pub fn add_png_file_message(&mut self, msg_num: usize, path: &Path) -> Result<(), BadgeError> {
        if msg_num >= N_MESSAGES {
            Err(BadgeError::MessageNumberOutOfRange(msg_num))
        } else {
            let file = File::open(path).map_err(|e| {
                BadgeError::FileIo(path.to_str().map_or(None, |v| Some(v.to_string())), e)
            })?;
            let reader = BufReader::new(file);

            let mut pixel_data = image_io::read_png_to_badge_message(reader).map_err(|e| {
                BadgeError::PngReadError(path.to_str().map_or(None, |v| Some(v.to_string())), e)
            })?;
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
            self.header.line_conf[msg_num] =
                (self.header.line_conf[msg_num] & 0xF0u8) | (pat as u8);
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
            self.header.line_conf[msg_num] =
                ((spd - 1) << 4) | (self.header.line_conf[msg_num] & 0x0Fu8);
            Ok(())
        }
    }

    /// Set effect blink
    pub fn set_effect_blink(&mut self, msg_num: usize, blink: bool) -> Result<(), BadgeError> {
        if msg_num >= N_MESSAGES {
            Err(BadgeError::MessageNumberOutOfRange(msg_num))
        } else {
            self.header.flash &= !(0x01u8 << msg_num as u8);
            self.header.flash |= (blink as u8) << msg_num as u8;
            Ok(())
        }
    }

    /// Set effects
    pub fn set_effect_frame(&mut self, msg_num: usize, frame: bool) -> Result<(), BadgeError> {
        if msg_num >= N_MESSAGES {
            Err(BadgeError::MessageNumberOutOfRange(msg_num))
        } else {
            self.header.border &= !(0x01u8 << msg_num as u8);
            self.header.border |= (frame as u8) << msg_num as u8;
            Ok(())
        }
    }

    /// Set brightness
    pub fn set_brightness(&mut self, br: u8) -> Result<(), BadgeError> {
        if !BADGE_BRIGHTNESS_RANGE.contains(&br) {
            Err(BadgeError::WrongBrightness)
        } else {
            self.header.brightness = (br as u8) << 4;
            Ok(())
        }
    }

    /// Send the context information to the device
    ///
    /// # Errors
    ///
    /// If failed to write the data to the device, then an error is returned.
    pub fn send(&mut self) -> Result<(), BadgeError> {
        let device = Badge::open()?;

        let mut disp_buf: Vec<u8> = Vec::with_capacity(DISP_SIZE);
        for i in 0..N_MESSAGES {
            let msg_len = self.messages[i].data.len() / BADGE_MSG_FONT_HEIGHT;
            self.header.msg_len[i] = (msg_len as u16).to_be();
            disp_buf.extend_from_slice(self.messages[i].data.as_ref());
        }

        const PAYLOAD_SIZE: usize = 64;
        const REPORT_BUF_LEN: usize = PAYLOAD_SIZE + 1;
        let disp_buf = disp_buf;

        {
            let mut report_buf: Vec<u8> = Vec::with_capacity(REPORT_BUF_LEN);
            report_buf.push(0u8);
            report_buf.extend_from_slice(unsafe { self.header.as_slice() });
            report_buf.resize(REPORT_BUF_LEN, 0u8);
            device.write(report_buf.as_slice())?;
        }

        for i in (0..disp_buf.len()).step_by(PAYLOAD_SIZE) {
            let disp_buf_range = i..((i + PAYLOAD_SIZE).min(disp_buf.len()));

            let mut report_buf: Vec<u8> = Vec::with_capacity(REPORT_BUF_LEN);
            report_buf.push(0u8);
            report_buf.extend_from_slice(disp_buf[disp_buf_range].as_ref());
            report_buf.resize(REPORT_BUF_LEN, 0u8);
            device.write(report_buf.as_slice())?;
        }

        Ok(())
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

    /// Write png data to file instead of the badge
    pub fn write_to_png_file(&self, msg_num: usize, path: &Path) -> Result<(), BadgeError> {
        if msg_num >= N_MESSAGES {
            Err(BadgeError::MessageNumberOutOfRange(msg_num))
        } else if self.messages[msg_num].data.is_empty() {
            Err(BadgeError::NoDataToWrite)
        } else {
            let message_data = self.messages[msg_num].data.as_slice();
            let file = File::create(path).map_err(|e| {
                BadgeError::FileIo(path.to_str().map_or(None, |v| Some(v.to_string())), e)
            })?;
            let ref mut w = BufWriter::new(file);
            image_io::write_badge_message_to_png(message_data, w)
                .map_err(|e| PngWriteError(path.to_str().map_or(None, |v| Some(v.to_string())), e))
        }
    }
}

#[test]
fn test_badge_new() {
    assert!(Badge::new().is_ok());
}

#[test]
fn add_png_message() {
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
    assert!(badge.add_png_message(N_MESSAGES, reader).is_err());

    let reader = Cursor::new(&corrupted_data);
    assert!(badge.add_png_message(N_MESSAGES - 1, reader).is_err());

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

    assert!(badge.add_text_message(N_MESSAGES, "", font_names).is_err());

    assert!(badge
        .add_text_message(N_MESSAGES - 1, "", font_names)
        .is_ok());
    assert!(badge.messages[N_MESSAGES - 1].data.iter().all(|&v| v == 0));

    assert!(badge.add_text_message(0, "A", font_names).is_ok());
    assert!(badge.messages[0].data.iter().any(|&v| v != 0));
}

#[test]
fn test_badge_set_effect_pattern() {
    let mut badge = Badge::new().unwrap();

    assert!(badge
        .set_effect_pattern(N_MESSAGES, BadgeEffect::Left)
        .is_err());

    assert!(badge
        .set_effect_pattern(N_MESSAGES - 1, BadgeEffect::Laser)
        .is_ok());
    assert_eq!(
        badge.header.line_conf[N_MESSAGES - 1] & 0x0f,
        BadgeEffect::Laser as u8
    );

    assert!(badge.set_effect_pattern(0, BadgeEffect::Left).is_ok());
    assert_eq!(badge.header.line_conf[0] & 0x0f, BadgeEffect::Left as u8);
}

#[test]
fn test_badge_set_effect_speed() {
    let mut badge = Badge::new().unwrap();

    assert!(badge.set_effect_speed(N_MESSAGES, 1).is_err());

    assert!(badge.set_effect_speed(0, 0).is_err());
    assert!(badge.set_effect_speed(0, 9).is_err());

    assert!(badge.set_effect_speed(0, 1).is_ok());
    assert_eq!(badge.header.line_conf[0] & 0xf0, 0 << 4);
    assert!(badge.set_effect_speed(N_MESSAGES - 1, 8).is_ok());
    assert_eq!(badge.header.line_conf[N_MESSAGES - 1] & 0xf0, 7 << 4);
}

#[test]
fn test_badge_set_effect_blink() {
    let mut badge = Badge::new().unwrap();

    assert!(badge.set_effect_blink(N_MESSAGES, true).is_err());

    assert!(badge.set_effect_blink(N_MESSAGES - 1, true).is_ok());
    assert_eq!(
        badge.header.flash & (1 << (N_MESSAGES as u8 - 1)),
        (1 << (N_MESSAGES as u8 - 1))
    );
    assert!(badge.set_effect_blink(N_MESSAGES - 1, false).is_ok());
    assert_eq!(
        badge.header.flash & (1 << (N_MESSAGES as u8 - 1)),
        (0 << (N_MESSAGES as u8 - 1))
    );
}

#[test]
fn test_badge_set_effect_frame() {
    let mut badge = Badge::new().unwrap();

    assert!(badge.set_effect_frame(N_MESSAGES, true).is_err());

    assert!(badge.set_effect_frame(N_MESSAGES - 1, true).is_ok());
    assert_eq!(
        badge.header.border & (1 << (N_MESSAGES as u8 - 1)),
        (1 << (N_MESSAGES as u8 - 1))
    );
    assert!(badge.set_effect_frame(N_MESSAGES - 1, false).is_ok());
    assert_eq!(
        badge.header.border & (1 << (N_MESSAGES as u8 - 1)),
        (0 << (N_MESSAGES as u8 - 1))
    );
}

#[test]
fn test_write_to_png() {
    let mut badge = Badge::new().unwrap();

    let mut png_data = Vec::<u8>::new();
    assert!(badge.write_to_png(N_MESSAGES, &mut png_data).is_err());

    let mut png_data = Vec::<u8>::new();
    assert!(badge.write_to_png(N_MESSAGES - 1, &mut png_data).is_err());

    badge.messages[N_MESSAGES - 1].data = vec![0; BADGE_MSG_FONT_HEIGHT];
    let mut png_data = Vec::<u8>::new();
    let mut w = Cursor::new(&mut png_data);
    assert!(badge.write_to_png(N_MESSAGES - 1, w.get_mut()).is_ok());
    assert_eq!(
        &png_data[0..8],
        &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
    );
}

#[test]
fn test_write_to_png_file() {
    let badge = Badge::new().unwrap();

    let path = Path::new("");
    assert!(badge.write_to_png_file(N_MESSAGES, path).is_err());
    // Success case is not tested in this function.
}
