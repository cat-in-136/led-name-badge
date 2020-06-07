use std::path::Path;

use freetype::{Error, Library};
use freetype::face::LoadFlag;
use freetype::freetype_sys::FT_Pos;

#[derive(Debug)]
struct Canvas {
    width: usize,
    height: usize,
    pixels: Vec<u8>,
}

impl Canvas {
    fn new(width: usize, height: usize) -> Self {
        let pixels = vec![0; width * height];
        Self {
            width,
            height,
            pixels,
        }
    }
}

/// Convert the canvas data into the led badge message data.
fn canvas2vec(canvas: &Canvas) -> Vec<u8> {
    let data_width = (canvas.width + 7) / 8;
    let data_height = canvas.height;

    let mut data = vec![0; data_width * data_height];
    for (i, &v) in canvas.pixels.iter().enumerate() {
        if v > 0 {
            let canvas_x = i % canvas.width;
            let canvas_y = i / canvas.width;

            let data_x = canvas_x / 8;
            let data_offset = canvas_x % 8;
            let data_y = canvas_y;
            let data_index = data_x * canvas.height + data_y;

            data[data_index] |= 0x80u8 >> data_offset as u8;
        }
    }

    data
}

#[test]
fn test_canvas2vec() {
    let vec = vec![0b10101010, 0b11001100, 0b11110000, 0b11111111];
    let canvas = Canvas {
        #[rustfmt::skip]
        pixels: vec![
            1, 0, 1, 0, 1, 0, 1, 0, 1, 1, 1, 1, 0, 0, 0, 0,
            1, 1, 0, 0, 1, 1, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1,
        ],
        width: 16,
        height: 2,
    };
    assert_eq!(canvas2vec(&canvas), vec);

    let vec = vec![
        0b10101010, 0b11001100, 0b11110000, 0b11111111, 0b10000000, 0b10000000,
    ];
    let canvas = Canvas {
        #[rustfmt::skip]
        pixels: vec![
            1, 0, 1, 0, 1, 0, 1, 0, 1, 1, 1, 1, 0, 0, 0, 0, 1,
            1, 1, 0, 0, 1, 1, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        ],
        width: 17,
        height: 2,
    };
    assert_eq!(canvas2vec(&canvas), vec);
}

/// Render text with given font configuration and return the led badge message data.
pub(crate) fn render_text(
    text: &str,
    pixel_height: usize,
    font_path: &Path,
    font_index: usize,
) -> Result<Vec<u8>, Error> {
    fn ftpos2pixel(p: FT_Pos) -> usize {
        p as usize / 64usize
    }
    fn pixel2ftpos(p: usize) -> FT_Pos {
        p as i64 * 64
    }

    let lib = Library::init()?;
    let face = lib.new_face(font_path, font_index as isize)?;

    if face.is_scalable() {
        face.set_pixel_sizes(0, pixel_height as u32)?;
    }

    let mut canvas = {
        let mut width = 0;
        for c in text.chars() {
            face.load_char(c as usize, LoadFlag::RENDER | LoadFlag::TARGET_MONO)?;
            width += ftpos2pixel(face.glyph().advance().x);
        }
        Canvas::new(width, pixel_height)
    };

    let mut pen_x = 0;
    for c in text.chars() {
        face.load_char(c as usize, LoadFlag::RENDER | LoadFlag::TARGET_MONO)?;
        let glyph = face.glyph();
        let bitmap = glyph.bitmap();
        let buffer = bitmap.buffer();

        let face_metrics = face.size_metrics().unwrap();
        let metrics = glyph.metrics();
        let pitch = bitmap.pitch() as usize;
        let rows = bitmap.rows() as usize;
        let pen_start_x = pen_x + ftpos2pixel(metrics.horiBearingX);
        let pen_start_y = if face_metrics.ascender == 0 {
            0 // some font does not have ascend.
        } else {
            ftpos2pixel(
                pixel2ftpos(pixel_height as usize)
                    - (-face_metrics.descender)
                    - metrics.horiBearingY,
            )
        };

        for q in 0..rows {
            for p in 0..pitch {
                for i in 0..8usize {
                    let pixel_val = buffer[q * pitch + p] & (0x80 >> i) as u8;
                    let canvas_x = pen_start_x + p * 8 + i;
                    let canvas_y = pen_start_y + q;
                    if pixel_val != 0 && canvas_x < canvas.width && canvas_y < canvas.height {
                        canvas.pixels[canvas_y * canvas.width + canvas_x] = 1;
                    }
                }
            }
        }

        pen_x += ftpos2pixel(glyph.advance().x);
    }

    Ok(canvas2vec(&canvas))
}

#[test]
fn test_render_text() {
    use crate::badge::font_selector::select_font;
    let (font_path, font_index) = select_font(&["Liberation Sans", "Arial"], Some(10)).unwrap();

    let pixel_data = render_text("Test!", 10, font_path.as_ref(), font_index).unwrap();
    assert!(pixel_data.len() > 0);
    assert_eq!(pixel_data.len() % 10, 0);
    assert_eq!(pixel_data.iter().all(|v| *v == 0), false);
}
