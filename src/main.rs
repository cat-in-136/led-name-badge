extern crate hidapi;

use crate::badge::{Badge, BADGE_SPEED_MAX, BadgeBrightness, BadgeEffect, BadgeError};

mod badge;

/// CLI entry point
fn main() {
    let error_label = (|| -> Result<i32, BadgeError> {
        let mut badge = Badge::new()?;

        badge.add_text_message(0, "Game", &["Liberation Sans", "Arial"])?;
        badge.set_effects(0, BadgeEffect::Left, BADGE_SPEED_MAX, false, false)?;

        badge.set_brightness(BadgeBrightness::B25);

        badge.send()?;
        Ok(0)
    })()
    .unwrap_or_else(|err| {
        eprintln!("Error: {:?}", err);
        1
    });
    std::process::exit(error_label);
}
