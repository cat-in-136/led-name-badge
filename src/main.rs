extern crate hidapi;

use core::fmt;
use std::fmt::Formatter;
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

fn parse_arguments() -> Result<Box<[ArgValue]>, ArgParseError> {
    let options = vec![
        Arg::new(
            'i',
            Some("msg_number".to_string()),
            "Message Number [0..7]".to_string(),
        ),
        Arg::new('t', Some("msg".to_string()), "Message text".to_string()),
        Arg::new(
            's',
            Some("speed".to_string()),
            "Message speed [1..8]".to_string(),
        ),
        Arg::new(
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
        Arg::new(
            'B',
            Some("brightness".to_string()),
            "LED brightness [0..3]".to_string(),
        ),
        Arg::new('h', None, "show this help message".to_string()),
    ];

    let arguments = std::env::args().skip(1).collect::<Vec<String>>();
    let app = App::new(&options);
    let values = app.parse(&arguments)?;

    if values.iter().any(|option| match option {
        ArgValue::FlagArg { name: 'h' } => true,
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

        let mut msg_number = 0usize;
        let mut msg_speed = *BADGE_SPEED_RANGE.end();
        let mut msg_effect = BadgeEffect::Left;
        let mut msg_brightness = *BADGE_SPEED_RANGE.end();

        for v in option.iter() {
            use ArgValue::*;
            match v {
                Arg { name: 'i', value } => {
                    msg_number = match usize::from_str(value.as_str()) {
                        Ok(i) if (i <= 7) => Ok(i),
                        _ => Err(CliError::CliError(format!(
                            "'{}': wrong value. specify [0..7]",
                            value
                        ))),
                    }?
                }
                Arg { name: 't', value } => {
                    badge.add_text_message(msg_number, &value, &["Liberation Sans", "Arial"])?;
                    badge.set_effects(msg_number, msg_effect, msg_speed, false, false)?;
                }
                Arg { name: 's', value } => {
                    msg_speed = match u8::from_str(value.as_str()) {
                        Ok(i) if BADGE_SPEED_RANGE.contains(&i) => Ok(i),
                        _ => Err(CliError::CliError(format!(
                            "'{}': wrong value. specify [1..8]",
                            value
                        ))),
                    }?
                }
                Arg { name: 'e', value } => {
                    msg_effect = BadgeEffect::from_str(value.as_str()).map_err(|_err| {
                        CliError::CliError(format!(
                            "'{}': wrong value. specify [{}]",
                            value,
                            BadgeEffect::values()
                                .iter()
                                .map(|&v| v.to_string())
                                .collect::<Vec<_>>()
                                .join(","),
                        ))
                    })?
                }
                Arg { name: 'B', value } => {
                    msg_brightness = match u8::from_str(value.as_str()) {
                        Ok(i) if BADGE_BRIGHTNESS_RANGE.contains(&i) => Ok(i - 1),
                        _ => Err(CliError::CliError(format!(
                            "'{}': wrong value. specify [0..3]",
                            value
                        ))),
                    }?
                }
                _ => (),
            }
        }

        badge.set_brightness(msg_brightness)?;

        badge.send()?;
        Ok(0)
    })()
    .unwrap_or_else(|err| {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    });
}
