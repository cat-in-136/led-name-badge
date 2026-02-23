use std::ffi::CString;
use std::path::PathBuf;

use fontconfig::{Fontconfig, Pattern};

/// Describes font finder error
#[derive(thiserror::Error, Debug, Clone)]
pub enum FontSelectorError {
    /// Caused by fontconfig internal error
    #[error("Internal Error")]
    FontConfigError,
    /// Font Not Found
    #[error("Font Not Found: {0}")]
    FontNotFound(String),
}

/// Select font and returns font path and index
///
/// # Errors
///
/// Return Err if no font is matched to given font_names
pub(crate) fn select_font(
    font_names: &[&str],
    font_size: Option<usize>,
) -> Result<(PathBuf, usize), FontSelectorError> {
    let fc = Fontconfig::new().ok_or(FontSelectorError::FontConfigError)?;
    if font_names.is_empty() {
        return Err(FontSelectorError::FontNotFound("-".to_string()));
    }

    let mut pattern = Pattern::new(&fc);
    for &font_name in font_names {
        let name_cstr = CString::new("family").unwrap();
        let font_name_cstr = CString::new(font_name).unwrap();
        pattern.add_string(name_cstr.as_c_str(), font_name_cstr.as_c_str());
    }
    if let Some(size) = font_size {
        let name_cstr = CString::new("size").unwrap();
        pattern.add_integer(name_cstr.as_c_str(), size as i32);
    }

    let font_match = pattern.font_match();
    if let (Some(filename), Some(index)) = (font_match.filename(), font_match.face_index()) {
        Ok((PathBuf::from(filename), index as usize))
    } else {
        let x = font_names.join(", ");
        Err(FontSelectorError::FontNotFound(x))
    }
}

#[test]
fn test_select_font() {
    assert!(matches!(
        select_font(&["Liberation Sans", "Arial"], None),
        Ok((_, 0))
    ));
    assert!(matches!(
        select_font(&["Liberation Sans", "Arial"], Some(24)),
        Ok((_, 0))
    ));
    assert!(select_font(&[], None).is_err());
    // assert!(find_font(&["NOT-EXIST-FONT-NAME"], Some(1)).is_err());
}