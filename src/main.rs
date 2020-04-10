extern crate hidapi;

use core::fmt;
use std::fmt::Formatter;
use std::str::FromStr;

use crate::arg_parser::{App, Arg, ArgParseError, ArgValue};
use crate::badge::{Badge, BADGE_SPEED_MAX, BadgeBrightness, BadgeEffect, BadgeError};

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
        let mut msg_speed = 1u8;

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
                }
                Arg { name: 's', value } => {
                    msg_speed = match u8::from_str(value.as_str()) {
                        Ok(i) if ((0 < i) && (i <= 8)) => Ok(i),
                        _ => Err(CliError::CliError(format!(
                            "'{}': wrong value. specify [1..8]",
                            value
                        ))),
                    }?
                }
                _ => (),
            }
        }

        badge.set_effects(0, BadgeEffect::Left, msg_speed, false, false)?;

        badge.set_brightness(BadgeBrightness::B25);

        badge.send()?;
        Ok(0)
    })()
    .unwrap_or_else(|err| {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    });
}
