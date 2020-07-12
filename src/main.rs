extern crate hidapi;

use core::fmt;
use core::fmt::Formatter;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read};
use std::path::Path;
use std::str::FromStr;

use crate::arg_parser::{App, Arg, ArgParseError, ArgValue};
use crate::badge::{Badge, BADGE_BRIGHTNESS_RANGE, BADGE_SPEED_RANGE, BadgeEffect, BadgeError};
use crate::badge::device::BadgeType;

mod arg_parser;
mod badge;

#[derive(Debug)]
enum CliError {
    ArgParseError(ArgParseError),
    BadgeError(BadgeError),
    CliError(String),
}

impl From<ArgParseError> for CliError {
    fn from(e: ArgParseError) -> Self {
        CliError::ArgParseError(e)
    }
}

impl From<BadgeError> for CliError {
    fn from(e: BadgeError) -> Self {
        CliError::BadgeError(e)
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            CliError::ArgParseError(e) => e.fmt(f),
            CliError::BadgeError(e) => e.fmt(f),
            CliError::CliError(str) => f.write_str(str.as_str()),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[allow(non_camel_case_types)]
enum CliArgumentId {
    i,
    t,
    T,
    F,
    p,
    s,
    e,
    b,
    f,
    B,
    o,
    h,
}

fn parse_arguments() -> Result<Box<[ArgValue<CliArgumentId>]>, ArgParseError> {
    let options = vec![
        Arg::new(
            CliArgumentId::i,
            'i',
            Some("msg_number".to_string()),
            "Message Number [0..7]".to_string(),
        ),
        Arg::new(
            CliArgumentId::t,
            't',
            Some("msg".to_string()),
            "Message text".to_string(),
        ),
        Arg::new(
            CliArgumentId::T,
            'T',
            Some("file".to_string()),
            "Message text read from file".to_string(),
        ),
        Arg::new(
            CliArgumentId::F,
            'F',
            Some("font".to_string()),
            "Font family name or font file path".to_string(),
        ),
        Arg::new(
            CliArgumentId::p,
            'p',
            Some("file".to_string()),
            "Load message png file".to_string(),
        ),
        Arg::new(
            CliArgumentId::s,
            's',
            Some("speed".to_string()),
            "Message speed [1..8]".to_string(),
        ),
        Arg::new(
            CliArgumentId::e,
            'e',
            Some("effect".to_string()),
            format!(
                "Message effect\n[{}]",
                BadgeEffect::values()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            )
            .to_string(),
        ),
        Arg::new(CliArgumentId::b, 'b', None, "Blink message".to_string()),
        Arg::new(
            CliArgumentId::f,
            'f',
            None,
            "Set frame for message".to_string(),
        ),
        Arg::new(
            CliArgumentId::B,
            'B',
            Some("brightness".to_string()),
            "LED brightness [0..3]".to_string(),
        ),
        Arg::new(
            CliArgumentId::o,
            'o',
            Some("pngfile".to_string()),
            "Write to png file instead of badge".to_string(),
        ),
        Arg::new(
            CliArgumentId::h,
            'h',
            None,
            "show this help message".to_string(),
        ),
    ];

    let arguments = std::env::args().skip(1).collect::<Vec<String>>();
    let app = App::new(&options);
    let values = app.parse(&arguments)?;

    if values.iter().any(|option| match option {
        ArgValue::Arg {
            id: CliArgumentId::h,
            ..
        } => true,
        _ => false,
    }) {
        println!(
            "{}\n\nUSAGE:\n    {} [OPTIONS]\n\nOPTIONS:\n{}",
            env!("CARGO_PKG_DESCRIPTION"),
            std::env::args().nth(0).unwrap(),
            app.help_option_message(),
        );
        std::process::exit(0);
    } else {
        Ok(values)
    }
}

/// CLI entry point
fn main() {
    (|| -> Result<i32, CliError> {
        let option = parse_arguments()?;

        let mut badge = Badge::new()?;
        let mut msg_number = 0;
        let mut disable_send_to_badge = false;
        let mut font_family = Vec::with_capacity(1);
        const DEFAULT_FONT_FAMILY: [&'static str; 2] = ["Liberation Sans", "Arial"];

        for v in option.iter() {
            use ArgValue::*;

            match v {
                Arg {
                    id: CliArgumentId::i,
                    value,
                } => {
                    msg_number = match usize::from_str(value.as_ref().unwrap().as_str()) {
                        Ok(i) if (i <= 7) => Ok(i),
                        _ => Err(CliError::CliError(format!(
                            "-i '{}': wrong value. specify [0..7]",
                            value.as_ref().unwrap()
                        ))),
                    }?;
                }
                Arg {
                    id: CliArgumentId::t,
                    value,
                } => {
                    let font_names = if font_family.is_empty() {
                        DEFAULT_FONT_FAMILY.as_ref()
                    } else {
                        font_family.as_ref()
                    };

                    badge.add_text_message(msg_number, &value.as_ref().unwrap(), font_names)?;
                }
                Arg {
                    id: CliArgumentId::T,
                    value,
                } => {
                    let msg = (|| -> Result<String, std::io::Error> {
                        let file = File::open(Path::new(&value.as_ref().unwrap()))?;
                        let mut msg = String::new();
                        BufReader::new(file).read_to_string(&mut msg)?;
                        Ok(msg)
                    })()
                    .map_err(|e| CliError::BadgeError(BadgeError::FileIo(value.clone(), e)))?;

                    let font_names = if font_family.is_empty() {
                        DEFAULT_FONT_FAMILY.as_ref()
                    } else {
                        font_family.as_ref()
                    };

                    badge.add_text_message(msg_number, msg.as_str(), font_names)?;
                }
                Arg {
                    id: CliArgumentId::F,
                    value,
                } => {
                    if !font_family.is_empty() {
                        font_family.clear();
                    }
                    font_family.push(value.as_ref().unwrap().as_str());
                }
                Arg {
                    id: CliArgumentId::p,
                    value,
                } => {
                    let file = File::open(Path::new(&value.as_ref().unwrap()))
                        .map_err(|e| CliError::BadgeError(BadgeError::FileIo(value.clone(), e)))?;
                    let reader = BufReader::new(&file);
                    badge.add_png_message(msg_number, reader)?;
                }
                Arg {
                    id: CliArgumentId::s,
                    value,
                } => {
                    let msg_speed = match u8::from_str(value.as_ref().unwrap().as_str()) {
                        Ok(i) if BADGE_SPEED_RANGE.contains(&i) => Ok(i),
                        _ => Err(CliError::CliError(format!(
                            "-s '{}': wrong value. specify [1..8]",
                            value.as_ref().unwrap()
                        ))),
                    }?;
                    badge.set_effect_speed(msg_number, msg_speed)?;
                }
                Arg {
                    id: CliArgumentId::e,
                    value,
                } => {
                    let msg_effect = BadgeEffect::from_str(value.as_ref().unwrap().as_str())
                        .map_err(|_err| {
                            CliError::CliError(format!(
                                "-e '{}': wrong value. specify [{}]",
                                value.as_ref().unwrap(),
                                BadgeEffect::values()
                                    .map(|v| v.to_string())
                                    .collect::<Vec<_>>()
                                    .join(","),
                            ))
                        })?;
                    badge.set_effect_pattern(msg_number, msg_effect)?;
                }
                Arg {
                    id: CliArgumentId::b,
                    value: _,
                } => {
                    badge.set_effect_blink(msg_number, true)?;
                }
                Arg {
                    id: CliArgumentId::f,
                    value: _,
                } => {
                    badge.set_effect_frame(msg_number, true)?;
                }
                Arg {
                    id: CliArgumentId::B,
                    value,
                } => {
                    let msg_brightness = match u8::from_str(value.as_ref().unwrap().as_str()) {
                        Ok(i) if BADGE_BRIGHTNESS_RANGE.contains(&i) => Ok(i),
                        _ => Err(CliError::CliError(format!(
                            "-B '{}': wrong value. specify [0..3]",
                            value.as_ref().unwrap()
                        ))),
                    }?;
                    badge.set_brightness(msg_brightness)?;
                }
                Arg {
                    id: CliArgumentId::o,
                    value,
                } => {
                    let file = File::create(Path::new(&value.as_ref().unwrap()))
                        .map_err(|e| CliError::BadgeError(BadgeError::FileIo(value.clone(), e)))?;
                    let writer = BufWriter::new(&file);
                    badge.write_to_png(msg_number, writer)?;
                    disable_send_to_badge = true;
                }
                Arg {
                    id: CliArgumentId::h,
                    value: _,
                } => (),
                Value { .. } => (),
            }
        }

        if !disable_send_to_badge {
            badge.send(BadgeType::S1144)?;
        }
        Ok(0)
    })()
    .unwrap_or_else(|err| {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    });
}
