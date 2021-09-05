use std::{error, fmt};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;
use std::ptr::null_mut;

use fontconfig::{Fontconfig, Pattern};
use fontconfig_sys::fontconfig::{
    FcChar8, FcPattern, FcPatternAddInteger, FcPatternAddString, FcPatternCreate, FcPatternDestroy,
    FcPatternGetInteger, FcPatternGetString, FcResultMatch,
};

/// Describes font finder error
#[derive(Debug, Clone)]
pub enum FontSelectorError {
    /// Caused by fontconfig internal error
    FontConfigError,
    /// Font Not Found
    FontNotFound(String),
}

impl fmt::Display for FontSelectorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FontSelectorError::FontConfigError => f.write_str("Internal Error"),
            FontSelectorError::FontNotFound(fonts) => {
                f.write_fmt(format_args!("Font Not Found: {}", fonts))
            }
        }
    }
}

impl error::Error for FontSelectorError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

/// Wrapper of `FcPattern`
#[derive(Debug)]
struct FontPattern {
    pattern: *mut FcPattern,
}

impl FontPattern {
    /// Create a new pattern
    fn new() -> Result<Self, FontSelectorError> {
        let pattern = unsafe { FcPatternCreate() };
        if pattern.is_null() {
            Err(FontSelectorError::FontConfigError)
        } else {
            Ok(Self { pattern })
        }
    }

    /// Create a new instance from raw pointer
    fn from_pattern(pattern: *mut FcPattern) -> Self {
        Self { pattern }
    }

    /// Add a string to the pattern i.e. wrapper function to `FcPatternAddString`
    fn add_string(&mut self, name: &str, val: &str) {
        let name_c = CString::new(name).unwrap();
        let object = name_c.as_ptr() as *const c_char;

        let val_c = CString::new(val).unwrap();
        let s = val_c.as_ptr() as *const FcChar8;

        unsafe { FcPatternAddString(self.pattern, object, s) };
    }

    /// Get a string from the pattern i.e. wrapper function to `FcPatternGetString`
    fn get_string(&self, name: &str, n: usize) -> Option<String> {
        let name_c = CString::new(name).unwrap();
        let object = name_c.as_ptr() as *const c_char;

        let mut s = null_mut();
        if unsafe { FcPatternGetString(*&self.pattern, object, n as c_int, &mut s) }
            == FcResultMatch
        {
            let str = unsafe { CStr::from_ptr(s as *mut c_char) }
                .to_string_lossy()
                .into_owned();
            Some(str)
        } else {
            None
        }
    }

    /// Add an integer to the pattern i.e. wrapper function to `FcPatternAddInteger`
    fn add_integer(&mut self, name: &str, val: i32) {
        let name_c = CString::new(name).unwrap();
        let object = name_c.as_ptr() as *const c_char;

        let i = val as c_int;
        unsafe { FcPatternAddInteger(self.pattern, object, i) };
    }

    /// Get an integer from the pattern i.e. wrapper function to `FcPatternGetInteger`
    fn get_integer(&self, name: &str, n: usize) -> Option<i32> {
        let name_c = CString::new(name).unwrap();
        let object = name_c.as_ptr() as *const c_char;

        let mut i = 0 as c_int;
        if unsafe { FcPatternGetInteger(*&self.pattern, object, n as c_int, &mut i) }
            == FcResultMatch
        {
            Some(i)
        } else {
            None
        }
    }
}

#[test]
fn test_pattern_new() {
    let pattern = FontPattern::new().unwrap();
    assert!(!pattern.pattern.is_null());
}

#[test]
fn test_pattern_add_string_get_string() {
    let mut pattern = FontPattern::new().unwrap();
    pattern.add_string("family", "Open Sans");
    assert_eq!(
        pattern.get_string("family", 0),
        Some("Open Sans".to_string())
    );
    assert_eq!(pattern.get_string("family", 1), None);
    pattern.add_string("family", "Liberation Sans");
    assert_eq!(
        pattern.get_string("family", 0),
        Some("Open Sans".to_string())
    );
    assert_eq!(
        pattern.get_string("family", 1),
        Some("Liberation Sans".to_string())
    );
    assert_eq!(pattern.get_string("family", 2), None);
}

#[test]
fn test_pattern_add_integer_get_integer() {
    let mut pattern = FontPattern::new().unwrap();
    pattern.add_integer("size", 48);
    assert_eq!(pattern.get_integer("size", 0), Some(48));
    assert_eq!(pattern.get_integer("size", 1), None);
}

impl Drop for FontPattern {
    fn drop(&mut self) {
        unsafe { FcPatternDestroy(self.pattern) }
    }
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
    let mut pat = FontPattern::from_pattern(pattern.pat);
    for &font_name in font_names {
        pat.add_string("family", font_name);
    }
    if let Some(size) = font_size {
        pat.add_integer("size", size as i32);
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
