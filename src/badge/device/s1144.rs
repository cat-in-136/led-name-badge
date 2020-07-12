use std::mem;

use hidapi::{HidApi, HidDevice};

use crate::badge::{Badge, BADGE_MSG_FONT_HEIGHT, BadgeEffect, BadgeError, DISP_SIZE, N_MESSAGES};

/// Vendor ID of the LED Badge
const BADGE_VID: u16 = 0x0416;
/// Product ID of the LED Badge
const BADGE_PID: u16 = 0x5020;

/// Badge Protocol Header (first report to send)
#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct BadgeHeader {
    /// magic: "wang",0x00
    start: [u8; 5],
    /// badge brightness
    brightness: u8,
    /// bit-coded: flash messages
    flash: u8,
    /// bit-coded: border messages
    border: u8,
    /// config of 8 lines; 0xAB : A-speed[1..8] , B-effect[0..8]
    line_conf: [u8; 8],
    /// length lines in BIG endian
    msg_len: [u16; N_MESSAGES],
}

impl BadgeHeader {
    /// Transmutes into a slice from the header.
    unsafe fn as_slice(&self) -> &[u8] {
        let view = self as *const _ as *const u8;
        std::slice::from_raw_parts(view, mem::size_of::<Self>())
    }

    /// Set effect pattern
    fn set_effect_pattern(&mut self, msg_num: usize, pat: BadgeEffect) {
        self.line_conf[msg_num] = (self.line_conf[msg_num] & 0xF0u8) | (pat as u8);
    }

    /// Set effect speed
    fn set_effect_speed(&mut self, msg_num: usize, spd: u8) {
        self.line_conf[msg_num] = ((spd - 1) << 4) | (self.line_conf[msg_num] & 0x0Fu8);
    }

    /// Set effect blink
    fn set_effect_blink(&mut self, msg_num: usize, blink: bool) {
        self.flash &= !(0x01u8 << msg_num as u8);
        self.flash |= (blink as u8) << msg_num as u8;
    }

    /// Set effects
    fn set_effect_frame(&mut self, msg_num: usize, frame: bool) {
        self.border &= !(0x01u8 << msg_num as u8);
        self.border |= (frame as u8) << msg_num as u8;
    }

    /// Set brightness
    fn set_brightness(&mut self, br: u8) {
        self.brightness = (br as u8) << 4;
    }

    /// Load from badge object
    fn load(&mut self, badge: &Badge) {
        self.set_brightness(badge.brightness);
        for i in 0..N_MESSAGES {
            let message = &badge.messages[i];

            self.set_effect_blink(i, message.blink);
            self.set_effect_frame(i, message.frame);
            self.set_effect_speed(i, message.speed);
            self.set_effect_pattern(i, message.effect);

            let msg_len = message.data.len() / BADGE_MSG_FONT_HEIGHT;
            self.msg_len[i] = (msg_len as u16).to_be();
        }
    }
}

#[test]
fn test_badge_header_set_effect_pattern() {
    let mut header: BadgeHeader = Default::default();

    header.set_effect_pattern(N_MESSAGES - 1, BadgeEffect::Laser);
    assert_eq!(
        header.line_conf[N_MESSAGES - 1] & 0x0f,
        BadgeEffect::Laser as u8
    );

    header.set_effect_pattern(0, BadgeEffect::Left);
    assert_eq!(header.line_conf[0] & 0x0f, BadgeEffect::Left as u8);
}

#[test]
fn test_badge_header_set_effect_speed() {
    let mut header: BadgeHeader = Default::default();

    header.set_effect_speed(0, 1);
    assert_eq!(header.line_conf[0] & 0xf0, 0 << 4);
    header.set_effect_speed(N_MESSAGES - 1, 8);
    assert_eq!(header.line_conf[N_MESSAGES - 1] & 0xf0, 7 << 4);
}

#[test]
fn test_badge_header_set_effect_blink() {
    let mut header: BadgeHeader = Default::default();

    header.set_effect_blink(N_MESSAGES - 1, true);
    assert_eq!(
        header.flash & (1 << (N_MESSAGES as u8 - 1)),
        (1 << (N_MESSAGES as u8 - 1))
    );
    header.set_effect_blink(N_MESSAGES - 1, false);
    assert_eq!(
        header.flash & (1 << (N_MESSAGES as u8 - 1)),
        (0 << (N_MESSAGES as u8 - 1))
    );
}

#[test]
fn test_badge_header_set_effect_frame() {
    let mut header: BadgeHeader = Default::default();

    header.set_effect_frame(N_MESSAGES - 1, true);
    assert_eq!(
        header.border & (1 << (N_MESSAGES as u8 - 1)),
        (1 << (N_MESSAGES as u8 - 1))
    );
    header.set_effect_frame(N_MESSAGES - 1, false);
    assert_eq!(
        header.border & (1 << (N_MESSAGES as u8 - 1)),
        (0 << (N_MESSAGES as u8 - 1))
    );
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

/// Open a LED badge device
///
/// # Errors
///
/// If failed to open a LED badge, then an error is returned.
fn s1144_open() -> Result<HidDevice, BadgeError> {
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

/// Send the context information to the device
///
/// # Errors
///
/// If failed to write the data to the device, then an error is returned.
pub fn s1144_send(badge: &mut Badge) -> Result<(), BadgeError> {
    let device = s1144_open()?;

    let mut header = BadgeHeader::default();
    header.load(badge);

    let mut disp_buf: Vec<u8> = Vec::with_capacity(DISP_SIZE);
    for i in 0..N_MESSAGES {
        disp_buf.extend_from_slice(badge.messages[i].data.as_ref());
    }

    const PAYLOAD_SIZE: usize = 64;
    const REPORT_BUF_LEN: usize = PAYLOAD_SIZE + 1;
    let disp_buf = disp_buf;

    {
        let mut report_buf: Vec<u8> = Vec::with_capacity(REPORT_BUF_LEN);
        report_buf.push(0u8);
        report_buf.extend_from_slice(unsafe { header.as_slice() });
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
