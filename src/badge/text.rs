extern crate euclid;

use euclid::Point2D;
use euclid::Size2D;
use font_kit::canvas::{Canvas, Format, RasterizationOptions};
use font_kit::error::GlyphLoadingError;
use font_kit::family_name::FamilyName;
use font_kit::font::Font;
use font_kit::hinting::HintingOptions;
use font_kit::loader::FontTransform;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;

use crate::badge::error::BadgeError;

/// A continence method to find font and returns a Font.
///
/// # Errors
///
/// Return Err if no font is matched to given font_names or if failed to load the font.
pub(crate) fn find_font(font_names: &[&str]) -> Result<Font, BadgeError> {
    let family_names = font_names.iter().map(|&v| String::from(v));
    let family_names = family_names.map(|v| FamilyName::Title(v));
    let font = SystemSource::new()
        .select_best_match(&family_names.collect::<Vec<_>>(), &Properties::new())
        .map_err(|err| BadgeError::FontNotFound(err))?
        .load()
        .map_err(|err| BadgeError::FontLoading(err))?;

    Ok(font)
}

#[test]
fn test_find_font() {
    let font = SystemSource::new()
        .select_best_match(
            &[
                FamilyName::Title(String::from("Liberation Sans")),
                FamilyName::Title(String::from("Arial")),
                FamilyName::SansSerif,
            ],
            &Properties::new(),
        )
        .unwrap()
        .load()
        .unwrap();

    assert_eq!(
        find_font(&["Liberation Sans", "Arial"])
            .unwrap()
            .family_name(),
        font.family_name()
    );
    assert!(find_font(&[]).is_err());
    assert!(find_font(&["NOT-EXIST-FONT-NAME"]).is_err());
}

/// Convert the canvas data into the led badge message data.
fn canvas2vec(canvas: &Canvas) -> Vec<u8> {
    let canvas_size = canvas.size;
    let data_width = (canvas_size.width as usize + 7) / 8;
    let data_height = canvas_size.height as usize;

    let mut data = vec![0; data_width * data_height];
    for (i, &v) in canvas.pixels.iter().enumerate() {
        if v > 0 {
            let canvas_x = i % canvas_size.width as usize;
            let canvas_y = i / canvas_size.width as usize;

            let data_x = canvas_x / 8;
            let data_offset = canvas_x % 8;
            let data_y = canvas_y;
            let data_index = data_x * canvas_size.height as usize + data_y;

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
        size: Size2D::new(16, 2),
        stride: 0,
        format: Format::A8,
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
        size: Size2D::new(17, 2),
        stride: 0,
        format: Format::A8,
    };
    assert_eq!(canvas2vec(&canvas), vec);
}

/// Render text with given font configuration and return the led badge message data.
pub(crate) fn render_text(text: &str, font_size: u32, font: &Font) -> Vec<u8> {
    let width = text
        .chars()
        .map(|c| font.glyph_for_char(c))
        .fold(0u32, |x, v| {
            x + v
                .ok_or(GlyphLoadingError::NoSuchGlyph)
                .and_then(|glyph_id| {
                    font.raster_bounds(
                        glyph_id,
                        font_size as f32,
                        &FontTransform::identity(),
                        &Point2D::new(x as f32, font_size as f32),
                        HintingOptions::None,
                        RasterizationOptions::Bilevel,
                    )
                })
                .and_then(|bounds| Ok(bounds.size.width as u32))
                .unwrap_or(0)
        });

    let mut canvas = Canvas::new(&Size2D::new(width, font_size), Format::A8);
    text.chars()
        .map(|c| font.glyph_for_char(c))
        .fold(0u32, |x, v| {
            x + v
                .ok_or(GlyphLoadingError::NoSuchGlyph)
                .and_then(|glyph_id| {
                    font.rasterize_glyph(
                        &mut canvas,
                        glyph_id,
                        font_size as f32,
                        &FontTransform::identity(),
                        &Point2D::new(x as f32, font_size as f32),
                        HintingOptions::None,
                        RasterizationOptions::Bilevel,
                    )
                    .and(Ok(glyph_id))
                })
                .and_then(|glyph_id| {
                    font.raster_bounds(
                        glyph_id,
                        font_size as f32,
                        &FontTransform::identity(),
                        &Point2D::new(x as f32, font_size as f32),
                        HintingOptions::None,
                        RasterizationOptions::Bilevel,
                    )
                })
                .and_then(|bounds| Ok(bounds.size.width as u32))
                .unwrap_or(0)
        });

    canvas2vec(&canvas)
}

#[test]
fn test_render_text() {
    let font = SystemSource::new()
        .select_best_match(
            &[
                FamilyName::Title(String::from("Liberation Sans")),
                FamilyName::Title(String::from("Arial")),
                FamilyName::SansSerif,
            ],
            &Properties::new(),
        )
        .unwrap()
        .load()
        .unwrap();

    let pixel_data = render_text("Test!", 10, &font);
    assert!(pixel_data.len() > 0);
    assert_eq!(pixel_data.len() % 10, 0);
    assert_eq!(pixel_data.iter().all(|v| *v == 0), false);
}
