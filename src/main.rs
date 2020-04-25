extern crate hidapi;

use core::fmt;
use std::fmt::Formatter;
use std::path::Path;
use std::str::FromStr;

use crate::arg_parser::{App, Arg, ArgParseError, ArgValue};
use crate::badge::{Badge, BADGE_BRIGHTNESS_RANGE, BADGE_SPEED_RANGE, BadgeEffect, BadgeError};

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
                    .iter()
                    .map(|&v| v.to_string())
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
                    badge.add_text_message(
                        msg_number,
                        &value.as_ref().unwrap(),
                        &["Liberation Sans", "Arial"],
                    )?;
                }
                Arg {
                    id: CliArgumentId::p,
                    value,
                } => {
                    let path = value.as_ref().unwrap();
                    let path = Path::new(path.as_str());
                    badge.add_png_file_message(msg_number, path)?;
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
                                    .iter()
                                    .map(|&v| v.to_string())
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
                    let path = Path::new(value.as_ref().unwrap());
                    badge.write_to_png_file(msg_number, path)?;
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
            badge.send()?;
        }
        Ok(0)
    })()
    .unwrap_or_else(|err| {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    });
}
