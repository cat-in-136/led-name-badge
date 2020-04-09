extern crate hidapi;

use std::str::FromStr;

use crate::arg_parser::{App, Arg, ArgParseError, ArgValue};
use crate::badge::{Badge, BADGE_SPEED_MAX, BadgeBrightness, BadgeEffect, BadgeError};

mod arg_parser;
mod badge;

fn parse_arguments() -> Result<Box<[ArgValue]>, ArgParseError> {
    let options = vec![
        Arg::new(
            'i',
            Some("msg_number".to_string()),
            "Message Number [0..7]".to_string(),
        ),
        Arg::new('t', Some("msg".to_string()), "Message text".to_string()),
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
    let option = parse_arguments().unwrap_or_else(|err| {
        eprintln!("Error: {:?}", err);
        std::process::exit(1);
    });

    (|| -> Result<i32, BadgeError> {
        let mut badge = Badge::new()?;

        let mut msg_number = 0usize;

        for v in option.iter() {
            use ArgValue::*;
            match v {
                Arg { name: 'i', value } => msg_number = usize::from_str(value.as_str()).unwrap(), // TODO
                Arg { name: 't', value } => {
                    badge.add_text_message(msg_number, &value, &["Liberation Sans", "Arial"])?;
                }
                _ => (),
            }
        }

        badge.set_effects(0, BadgeEffect::Left, BADGE_SPEED_MAX, false, false)?;

        badge.set_brightness(BadgeBrightness::B25);

        badge.send()?;
        Ok(0)
    })()
    .unwrap_or_else(|err| {
        eprintln!("Error: {:?}", err);
        std::process::exit(1);
    });
}
