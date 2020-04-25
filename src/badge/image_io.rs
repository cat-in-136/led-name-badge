use std::io::{Cursor, Read, Write};

use png::{BitDepth, ColorType, DecodingError, EncodingError};

use crate::badge::BADGE_MSG_FONT_HEIGHT;

#[derive(Debug)]
pub enum BadgeImageWriteError {
    PngEncodeError(EncodingError),
}

impl From<EncodingError> for BadgeImageWriteError {
    fn from(e: EncodingError) -> Self {
        BadgeImageWriteError::PngEncodeError(e)
    }
}

pub fn write_badge_message_to_png<W: Write>(
    message_data: &[u8],
    writer: W,
) -> Result<(), BadgeImageWriteError> {
    let (width, height) = (
        8 * message_data.len() / BADGE_MSG_FONT_HEIGHT,
        BADGE_MSG_FONT_HEIGHT,
    );
    let mut image_data = vec![0u8; width * height];
    for (data_index, &v) in message_data.iter().enumerate() {
        let data_x = (data_index / BADGE_MSG_FONT_HEIGHT) * 8;
        let data_y = data_index % BADGE_MSG_FONT_HEIGHT;
        for i in 0usize..8usize {
            let (x, y) = (data_x + i, data_y);
            let image_data_index = x + y * width as usize;
            if v & (0x80 >> i) as u8 != 0 {
                image_data[image_data_index] = 0xFF;
            }
        }
    }
    let mut encoder = png::Encoder::new(writer, width as u32, height as u32);
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&image_data)?;
    Ok(())
}

#[test]
fn test_write_badge_message_to_png() {
    let mut png_data = Vec::<u8>::new();
    let empty_message_data = &[];
    let mut w = Cursor::new(&mut png_data);
    assert!(write_badge_message_to_png(empty_message_data, w.get_mut()).is_err());

    #[rustfmt::skip]
        let sample_data: [u8; 22] = [
        0xFF, 0x00, 0xAA, 0x55, 0xFF, 0x00, 0xAA, 0x55, 0xFF, 0x00, 0xAA,
        0x00, 0xAA, 0x55, 0xFF, 0x00, 0xAA, 0x55, 0xFF, 0x00, 0xAA, 0x55,
    ];
    #[rustfmt::skip]
        let sample_pixels: Vec<u8> = vec![
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,  0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00,
        0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00,  0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF,
        0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF,  0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,  0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00,
        0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00,  0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF,
        0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF,  0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,  0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00,
        0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00,  0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF,
    ];

    let mut png_data = Vec::<u8>::new();
    let mut w = Cursor::new(&mut png_data);
    assert!(write_badge_message_to_png(&sample_data, w.get_mut()).is_ok());

    assert_eq!(
        &png_data[0..8],
        &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
    );

    let r = Cursor::new(&png_data);
    let decoder = png::Decoder::new(r);
    let (info, mut reader) = decoder.read_info().unwrap();
    assert_eq!(
        (info.width, info.height),
        (8 * 2, BADGE_MSG_FONT_HEIGHT as u32)
    );
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
    assert_eq!(info.color_type, png::ColorType::Grayscale);

    let mut png_pixels = vec![0; (info.width * info.height) as usize];
    reader.next_frame(&mut png_pixels).unwrap();
    assert_eq!(png_pixels, sample_pixels);
}

#[derive(Debug)]
pub enum BadgeImageReadError {
    PngDecodeError(DecodingError),
    UnsupportedPngError(String),
}

impl From<DecodingError> for BadgeImageReadError {
    fn from(e: DecodingError) -> Self {
        BadgeImageReadError::PngDecodeError(e)
    }
}

pub fn read_png_to_badge_message<R: Read>(reader: R) -> Result<Vec<u8>, BadgeImageReadError> {
    let decoder = png::Decoder::new(reader);
    let (info, mut reader) = decoder.read_info()?;

    if info.bit_depth != BitDepth::Eight {
        return Err(BadgeImageReadError::UnsupportedPngError(
            format!("{:?}: only 8bpp PNG supported", info.bit_depth).to_string(),
        ));
    }
    if info.height != BADGE_MSG_FONT_HEIGHT as u32 {
        return Err(BadgeImageReadError::UnsupportedPngError(
            format!(
                "height must be {}px, but height is {}",
                BADGE_MSG_FONT_HEIGHT, info.height
            )
            .to_string(),
        ));
    }

    let byte_per_pixel = match info.color_type {
        ColorType::Grayscale => 1,
        ColorType::RGB => 3,
        ColorType::Indexed => 3,
        ColorType::GrayscaleAlpha => 2,
        ColorType::RGBA => 4,
    };
    let mut buf = vec![0; info.buffer_size()];
    reader.next_frame(&mut buf)?;
    let mut data = vec![0; (info.width as usize + 7) / 8 * BADGE_MSG_FONT_HEIGHT];
    for (i, &v) in buf.iter().step_by(byte_per_pixel).enumerate() {
        let canvas_x = i % info.width as usize;
        let canvas_y = i / info.width as usize;
        let data_x = canvas_x / 8;
        let data_offset = canvas_x % 8;
        let data_y = canvas_y;
        let data_index = data_x * BADGE_MSG_FONT_HEIGHT + data_y;

        if v >= 0x80 {
            data[data_index] |= 0x80u8 >> data_offset as u8;
        }
    }

    Ok(data.to_owned())
}

#[test]
fn test_read_png_to_badge_message() {
    let png_data = vec![
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x0b, 0x01, 0x00, 0x00, 0x00, 0x00, 0x5e,
        0x99, 0x30, 0x94, 0x00, 0x00, 0x00, 0x16, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0x63, 0xfc,
        0xcf, 0xc8, 0xc8, 0xb0, 0x8a, 0x71, 0xd5, 0x6a, 0xc6, 0xd0, 0x55, 0x58, 0xd9, 0x00, 0xb3,
        0x3f, 0x0b, 0x07, 0x82, 0x6b, 0xdc, 0x80, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44,
        0xae, 0x42, 0x60, 0x82,
    ];
    #[rustfmt::skip]
        let sample_data: [u8; 22] = [
        0xFF, 0x00, 0xAA, 0x55, 0xFF, 0x00, 0xAA, 0x55, 0xFF, 0x00, 0xAA,
        0x00, 0xAA, 0x55, 0xFF, 0x00, 0xAA, 0x55, 0xFF, 0x00, 0xAA, 0x55,
    ];

    let r = Cursor::new(&png_data);
    assert_eq!(
        read_png_to_badge_message(r).unwrap().as_slice(),
        &sample_data
    );
}
