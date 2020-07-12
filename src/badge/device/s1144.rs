use hidapi::{HidApi, HidDevice};

use crate::badge::{Badge, BADGE_MSG_FONT_HEIGHT, BadgeError, DISP_SIZE, N_MESSAGES};

/// Vendor ID of the LED Badge
const BADGE_VID: u16 = 0x0416;
/// Product ID of the LED Badge
const BADGE_PID: u16 = 0x5020;

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

    let mut disp_buf: Vec<u8> = Vec::with_capacity(DISP_SIZE);
    for i in 0..N_MESSAGES {
        let msg_len = badge.messages[i].data.len() / BADGE_MSG_FONT_HEIGHT;
        badge.header.msg_len[i] = (msg_len as u16).to_be();
        disp_buf.extend_from_slice(badge.messages[i].data.as_ref());
    }

    const PAYLOAD_SIZE: usize = 64;
    const REPORT_BUF_LEN: usize = PAYLOAD_SIZE + 1;
    let disp_buf = disp_buf;

    {
        let mut report_buf: Vec<u8> = Vec::with_capacity(REPORT_BUF_LEN);
        report_buf.push(0u8);
        report_buf.extend_from_slice(unsafe { badge.header.as_slice() });
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
