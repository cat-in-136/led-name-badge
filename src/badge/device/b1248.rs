use std::convert::TryFrom;
use std::mem;

use hidapi::{HidApi, HidDevice};

use crate::badge::{Badge, BADGE_MSG_FONT_HEIGHT, BadgeEffect, BadgeError, DISP_SIZE, N_MESSAGES};

/// Vendor ID of the LED Badge
const BADGE_VID: u16 = 0x0483;
/// Product ID of the LED Badge
const BADGE_PID: u16 = 0x5750;

/// Message Offset/Length information in the Badge Protocol Configuration (second report to send)
#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct BadgeMessageOffsetLength {
    /// Fix number `0x08` to indicate the coding
    header: u8,
    /// Offset to starting message data (set a proper value even if the message length is zero)
    offset: u8,
    /// ? (should be set to zero)
    reserved: u8,
    /// message length
    length: u8,
}

impl BadgeMessageOffsetLength {
    /// Transmutes into a slice from the field.
    unsafe fn as_slice(&self) -> &[u8] {
        let view = self as *const _ as *const u8;
        std::slice::from_raw_parts(view, mem::size_of::<Self>())
    }
}

impl Default for BadgeMessageOffsetLength {
    fn default() -> Self {
        Self {
            header: 0x08,
            offset: 0,
            reserved: 0x00,
            length: 0,
        }
    }
}

/// Badge Protocol Configuration (second report to send)
#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct BadgeMessageConfiguration {
    /// Frame/Speed/Blink/Effect for each message
    effect: [u8; N_MESSAGES],
    /// Offset/Length for each message
    offset_length: [BadgeMessageOffsetLength; N_MESSAGES],
}

impl BadgeMessageConfiguration {
    /// Transmutes into a slice from the header.
    fn as_vec(&self) -> Vec<u8> {
        let mut vec = Vec::with_capacity(N_MESSAGES * (1 + 4) + 2);
        vec.push(0x00u8); // the first byte is zero
        vec.extend_from_slice(&self.effect);
        vec.push(0x00u8);
        for i in 0..N_MESSAGES {
            vec.extend_from_slice( unsafe {self.offset_length[i].as_slice() });
        }
        vec
    }

    /// Load from badge object
    fn load(&mut self, badge: &Badge) {
        let mut offset = 0u8;
        for i in 0..N_MESSAGES {
            let message = &badge.messages[i];

            self.effect[i] = if message.frame { 0b10000000 } else { 0 }
                | (((message.speed - 1) & 0b111) << 4)
                | if message.blink { 0b00001000 } else { 0 }
                | ((message.effect as u8) & 0b111);

            self.offset_length[i].offset = offset;
            let msg_len = message.data.len() / BADGE_MSG_FONT_HEIGHT;
            self.offset_length[i].length = msg_len as u8;

            offset += msg_len as u8;
        }
    }
}

#[test]
fn test_badge_message_configuration_load() {
    let mut msg_config = BadgeMessageConfiguration::default();

    let mut badge = Badge::new().unwrap();
    for i in 0..N_MESSAGES {
        badge
            .set_effect_pattern(
                i,
                BadgeEffect::try_from((N_MESSAGES - i - 1) as u8).unwrap(),
            )
            .unwrap();
        badge.set_effect_blink(i, true).unwrap();
        badge.set_effect_speed(i, (i + 1) as u8).unwrap();
        badge.set_effect_frame(i, true).unwrap();

        badge.messages[i]
            .data
            .extend_from_slice(&[i as u8; BADGE_MSG_FONT_HEIGHT]);
    }
    msg_config.load(&badge);
    assert_eq!(
        msg_config.effect,
        [0x8F, 0x9E, 0xAD, 0xBC, 0xCB, 0xDA, 0xE9, 0xF8]
    );
    for i in 0..N_MESSAGES {
        assert_eq!(msg_config.offset_length[i].header, 0x08);
        assert_eq!(msg_config.offset_length[i].offset, i as u8);
        assert_eq!(msg_config.offset_length[i].length, 1);
    }
}

impl Default for BadgeMessageConfiguration {
    fn default() -> Self {
        Self {
            effect: Default::default(),
            offset_length: Default::default(),
        }
    }
}

/// Open a LED badge device
///
/// # Errors
///
/// If failed to open a LED badge, then an error is returned.
fn b1248_open() -> Result<HidDevice, BadgeError> {
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
pub fn b1248_send(badge: &Badge) -> Result<(), BadgeError> {
    let device = b1248_open()?;

    let mut msg_config = BadgeMessageConfiguration::default();
    msg_config.load(badge);

    const PAYLOAD_SIZE: usize = 64;
    const REPORT_BUF_LEN: usize = PAYLOAD_SIZE + 1;

    // first report -- "Hello"
    {
        let mut report_buf = vec![0x00, 0x48, 0x65, 0x6c, 0x6c, 0x6f]; // "Hello"
        report_buf.resize(REPORT_BUF_LEN, 0u8);
        device.write(report_buf.as_slice()).unwrap();
    }

    // second report -- Message configuration
    {
        let mut report_buf = Vec::with_capacity(REPORT_BUF_LEN);
        report_buf.push(0u8);
        report_buf.extend_from_slice(&msg_config.as_vec());
        report_buf.resize(REPORT_BUF_LEN, 0u8);
        device.write(report_buf.as_slice())?;
    }

    // Message lines
    for j in 0..BADGE_MSG_FONT_HEIGHT {
        let mut report_buf: Vec<u8> = vec![0x00; REPORT_BUF_LEN];
        for msg_no in 0..N_MESSAGES {
            let offset = msg_config.offset_length[msg_no].offset + 1;
            for (i, &v) in badge.messages[msg_no]
                .data
                .iter()
                .skip(j)
                .step_by(BADGE_MSG_FONT_HEIGHT)
                .enumerate()
            {
                report_buf[offset as usize + i] = v;
            }
        }
        report_buf.resize(REPORT_BUF_LEN, 0u8);
        device.write(report_buf.as_slice())?;
    }

    // last report -- Dummy line
    {
        let mut report_buf: Vec<u8> = Vec::with_capacity(REPORT_BUF_LEN);
        report_buf.resize(REPORT_BUF_LEN, 0u8);
        device.write(report_buf.as_slice())?;
    }

    Ok(())
}
