extern crate hidapi;

use std::error::Error;
use std::fmt;
use std::fmt::Debug;
use std::mem;

use font_kit::error::{FontLoadingError, GlyphLoadingError, SelectionError};
use hidapi::{HidApi, HidDevice, HidError};

use crate::badge::text::{find_font, render_text};

mod text;

/// Vendor ID of the LED Badge
const BADGE_VID: u16 = 0x0416;
/// Product ID of the LED Badge
const BADGE_PID: u16 = 0x5020;

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
    /// IO Error.
    Io(HidError),
    /// Font Not Found
    FontNotFound(SelectionError),
    /// Font Loading Error
    FontLoading(FontLoadingError),
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
            Io(_error) => f.write_str("IO Error"),
            FontNotFound(error) => {
                f.write_str(format!("Font Not Found: {}", error.description()).as_str())
            }
            FontLoading(_error) => f.write_str("Failed to load font"),
        }
    }
}

impl From<HidError> for BadgeError {
    fn from(e: HidError) -> Self {
        BadgeError::Io(e)
    }
}

/// Number of messages stored in the LED Badge
pub const N_MESSAGES: usize = 8;

/// Maximum length of message text
pub const MAX_STR: usize = 255;

/// Maximum number of display memory size
pub const DISP_SIZE: usize = 32767;

/// Height of the message
pub const BADGE_MSG_FONT_HEIGHT: usize = 11;

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
#[derive(Debug, PartialEq)]
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

/// LED brightness
#[derive(Debug, PartialEq)]
pub enum BadgeBrightness {
    B100 = 0,
    B75,
    B50,
    B25,
}

/// Maximum text animation speed
pub const BADGE_SPEED_MAX: u8 = 8;
/// Minimum text animation speed
pub const BADGE_SPEED_MIN: u8 = 1;

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
    device: HidDevice,
    header: BadgeHeader,
    messages: [BadgeMessage; N_MESSAGES],
}

impl Badge {
    /// Open a LED badge device
    ///
    /// ### Errors
    ///
    /// If failed to open a LED badge, then an error is returned.
    pub fn new() -> Result<Self, BadgeError> {
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

        Ok(Badge {
            device,
            header: Default::default(),
            messages: Default::default(),
        })
    }

    /// Add text messages
    pub fn add_text_message(&mut self, msg_num: usize, msg: &str, font_names: &[&str]) -> Result<(), BadgeError> {
        if msg_num >= N_MESSAGES {
            Err(BadgeError::MessageNumberOutOfRange(msg_num))
        } else if msg.len() == 0 {
            Ok(()) // Do nothing
        } else {
            let font = find_font(font_names)?;
            let mut pixel_data = render_text(msg, 11, &font);
            mem::swap(&mut self.messages[msg_num].data, &mut pixel_data);
            Ok(())
        }
    }

    /// Set effects
    pub fn set_effects(
        &mut self,
        msg_num: usize,
        pat: BadgeEffect,
        spd: u8,
        blink: bool,
        frame: bool,
    ) -> Result<(), BadgeError> {
        if msg_num >= N_MESSAGES {
            Err(BadgeError::MessageNumberOutOfRange(msg_num))
        } else if spd < BADGE_SPEED_MIN || BADGE_SPEED_MAX < spd {
            Err(BadgeError::WrongSpeed)
        } else {
            self.header.line_conf[msg_num] = (spd << 4) | (pat as u8);
            self.header.flash &= !(0x01u8 << msg_num as u8);
            self.header.flash |= (blink as u8) << msg_num as u8;
            self.header.border &= !(0x01u8 << msg_num as u8);
            self.header.border |= (frame as u8) << msg_num as u8;
            Ok(())
        }
    }

    /// Set brightness
    pub fn set_brightness(&mut self, br: BadgeBrightness) {
        self.header.brightness = (br as u8) << 4;
    }

    /// Send the context information to the device
    ///
    /// ### Errors
    ///
    /// If failed to write the data to the device, then an error is returned.
    pub fn send(&mut self) -> Result<(), BadgeError> {
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
            self.device.write(report_buf.as_slice())?;
        }

        for i in (0..disp_buf.len()).step_by(PAYLOAD_SIZE) {
            let disp_buf_range = i..((i + PAYLOAD_SIZE).min(disp_buf.len()));

            let mut report_buf: Vec<u8> = Vec::with_capacity(REPORT_BUF_LEN);
            report_buf.push(0u8);
            report_buf.extend_from_slice(disp_buf[disp_buf_range].as_ref());
            report_buf.resize(REPORT_BUF_LEN, 0u8);
            self.device.write(report_buf.as_slice())?;
        }

        Ok(())
    }
}
