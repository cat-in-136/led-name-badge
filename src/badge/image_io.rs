use core::fmt;
use core::fmt::{Debug, Formatter};
use std::error;
use std::io::{Read, Write};
#[cfg(test)]
use std::io::Cursor;

use png::{BitDepth, ColorType, Decoder, DecodingError, Encoder, EncodingError};

use crate::badge::BADGE_MSG_FONT_HEIGHT;

#[derive(Debug)]
pub enum BadgeImageWriteError {
    PngEncodeError(EncodingError),
}

impl fmt::Display for BadgeImageWriteError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            BadgeImageWriteError::PngEncodeError(e) => (e as &dyn fmt::Display).fmt(f),
        }
    }
}

impl error::Error for BadgeImageWriteError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            BadgeImageWriteError::PngEncodeError(e) => Some(e),
        }
    }
}

impl From<EncodingError> for BadgeImageWriteError {
    fn from(e: EncodingError) -> Self {
        BadgeImageWriteError::PngEncodeError(e)
    }
}

#[derive(Debug)]
pub enum BadgeImageReadError {
    PngDecodeError(DecodingError),
    UnsupportedPngError(String),
}

impl fmt::Display for BadgeImageReadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            BadgeImageReadError::PngDecodeError(decoding_error) => match decoding_error {
                DecodingError::IoError(e) => (e as &dyn fmt::Display).fmt(f),
                DecodingError::Format(data) => f.write_str(data.as_ref()),
                DecodingError::InvalidSignature => f.write_str("Broken File (Invalid signature)"),
                DecodingError::CrcMismatch { .. } => f.write_str("Broken file (CRC Error)"),
                DecodingError::Other(data) => f.write_str(data.as_ref()),
                DecodingError::CorruptFlateStream => f.write_str("Corrupted Flate Stream"),
                DecodingError::LimitsExceeded => f.write_str("Limits Exceeded"),
            },
            BadgeImageReadError::UnsupportedPngError(data) => f.write_str(data.as_ref()),
        }
    }
}
impl error::Error for BadgeImageReadError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            BadgeImageReadError::PngDecodeError(e) => Some(e),
            BadgeImageReadError::UnsupportedPngError(_) => None,
        }
    }
}

impl From<DecodingError> for BadgeImageReadError {
    fn from(e: DecodingError) -> Self {
        BadgeImageReadError::PngDecodeError(e)
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
    let mut encoder = Encoder::new(writer, width as u32, height as u32);
    encoder.set_color(ColorType::Grayscale);
    encoder.set_depth(BitDepth::Eight);
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
    let decoder = Decoder::new(r);
    let (info, mut reader) = decoder.read_info().unwrap();
    assert_eq!(
        (info.width, info.height),
        (8 * 2, BADGE_MSG_FONT_HEIGHT as u32)
    );
    assert_eq!(info.bit_depth, BitDepth::Eight);
    assert_eq!(info.color_type, ColorType::Grayscale);

    let mut png_pixels = vec![0; (info.width * info.height) as usize];
    reader.next_frame(&mut png_pixels).unwrap();
    assert_eq!(png_pixels, sample_pixels);
}

pub fn read_png_to_badge_message<R: Read>(reader: R) -> Result<Vec<u8>, BadgeImageReadError> {
    let decoder = Decoder::new(reader);
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
    fn create_png_data(
        width: u32,
        color_type: ColorType,
        bit_depth: BitDepth,
        data: &[u8],
    ) -> Vec<u8> {
        let mut png_data = Vec::new();
        {
            let w = Cursor::new(&mut png_data);
            let mut encoder = Encoder::new(w, width, BADGE_MSG_FONT_HEIGHT as u32);
            encoder.set_color(color_type);
            encoder.set_depth(bit_depth);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(data).unwrap();
        }
        png_data
    }

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

    let png_data = create_png_data(16, ColorType::Grayscale, BitDepth::Eight, &sample_pixels);
    let r = Cursor::new(&png_data);
    assert_eq!(
        read_png_to_badge_message(r).unwrap().as_slice(),
        &sample_data
    );

    // ColorType::GrayscaleAlpha
    let sample_pixels_gray_alpha = sample_pixels
        .iter()
        .flat_map(|&v| vec![v, 0xFF])
        .collect::<Vec<u8>>();
    let png_data = create_png_data(
        16,
        ColorType::GrayscaleAlpha,
        BitDepth::Eight,
        &sample_pixels_gray_alpha,
    );
    let r = Cursor::new(&png_data);
    assert_eq!(
        read_png_to_badge_message(r).unwrap().as_slice(),
        &sample_data
    );

    // ColorType::RGB
    let sample_pixels_rgb = sample_pixels
        .iter()
        .flat_map(|&v| vec![v, v, v])
        .collect::<Vec<u8>>();
    let png_data = create_png_data(16, ColorType::RGB, BitDepth::Eight, &sample_pixels_rgb);
    let r = Cursor::new(&png_data);
    assert_eq!(
        read_png_to_badge_message(r).unwrap().as_slice(),
        &sample_data
    );

    // ColorType::RGBA
    let sample_pixels_rgba = sample_pixels
        .iter()
        .flat_map(|&v| vec![v, v, v, 255])
        .collect::<Vec<u8>>();
    let png_data = create_png_data(16, ColorType::RGBA, BitDepth::Eight, &sample_pixels_rgba);
    let r = Cursor::new(&png_data);
    assert_eq!(
        read_png_to_badge_message(r).unwrap().as_slice(),
        &sample_data
    );
}
