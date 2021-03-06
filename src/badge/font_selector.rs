use std::{error, fmt};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;
use std::ptr::null_mut;
use std::sync::Once;

use fontconfig::fontconfig::{
    FcChar8, FcConfig, FcConfigSubstitute, FcDefaultSubstitute, FcFontMatch,
    FcInitLoadConfigAndFonts, FcMatchPattern, FcPattern, FcPatternAddInteger, FcPatternAddString,
    FcPatternCreate, FcPatternDestroy, FcPatternGetInteger, FcPatternGetString, FcResultMatch,
    FcResultNoMatch,
};

static INIT_FC: Once = Once::new();
static mut FC_CONFIG: *mut FcConfig = null_mut();

/// Initialize font finder.
fn init() -> Result<(), FontSelectorError> {
    INIT_FC.call_once(|| unsafe {
        FC_CONFIG = FcInitLoadConfigAndFonts();
    });

    if unsafe { FC_CONFIG }.is_null() {
        Err(FontSelectorError::FontConfigError)
    } else {
        Ok(())
    }
}

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
pub struct FontPattern {
    pattern: *mut FcPattern,
}

impl FontPattern {
    /// Create a new pattern
    fn new() -> Result<Self, FontSelectorError> {
        init()?;
        Ok(Self {
            pattern: unsafe { FcPatternCreate() },
        })
    }

    /// Create a new instance from raw pointer
    fn from_pattern(pattern: *mut FcPattern) -> Result<Self, FontSelectorError> {
        init()?;
        Ok(Self { pattern })
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

    /// Get matcher pattern i.e. wrapper function to `FcFontMatch`
    fn font_match(&mut self) -> Option<Self> {
        let font_pat = unsafe {
            FcConfigSubstitute(FC_CONFIG, self.pattern, FcMatchPattern);
            FcDefaultSubstitute(self.pattern);

            let mut _result = FcResultNoMatch;
            FcFontMatch(FC_CONFIG, self.pattern, &mut _result)
        };

        if font_pat.is_null() {
            None
        } else {
            Self::from_pattern(font_pat).ok()
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

#[test]
fn test_font_match() {
    let mut pattern = FontPattern::new().unwrap();
    pattern.add_string("family", "Liberation Sans");
    pattern.add_string("family", "Arial");

    let font_pattern = pattern.font_match().unwrap();
    let path = font_pattern.get_string("file", 0).unwrap();
    assert!(std::path::PathBuf::from(path).exists());

    assert_eq!(font_pattern.get_integer("index", 0), Some(0));
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
    if font_names.is_empty() {
        return Err(FontSelectorError::FontNotFound("-".to_string()));
    }

    let mut pattern = FontPattern::new()?;
    for &font_name in font_names {
        pattern.add_string("family", font_name);
    }
    if let Some(size) = font_size {
        pattern.add_integer("size", size as i32);
    }

    pattern
        .font_match()
        .and_then(|font_pattern| {
            font_pattern.get_string("file", 0).map(|v| {
                let index = font_pattern.get_integer("index", 0).unwrap_or(0);
                (PathBuf::from(v), index as usize)
            })
        })
        .ok_or_else(|| {
            let x = font_names.join(", ");
            FontSelectorError::FontNotFound(x)
        })
}

#[test]
fn test_select_font() {
    assert!(matches!(
        select_font(&["Liberation Sans", "Arial"], None),
        Ok((_, 0))
    ));
    assert!(select_font(&[], None).is_err());
    // assert!(find_font(&["NOT-EXIST-FONT-NAME"], Some(1)).is_err());
}
