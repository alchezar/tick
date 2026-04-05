//! Command handlers - bridge between CLI args and domain services.

use std::borrow::Cow;

pub mod project;
pub mod report;
pub mod task;

/// Appends VS16 (`\u{FE0F}`) after `❌` so terminals render the emoji variant.
fn terminal_emoji(text: &str) -> Cow<'_, str> {
    if text.contains('❌') {
        Cow::Owned(text.replace('❌', "❌\u{FE0F}"))
    } else {
        Cow::Borrowed(text)
    }
}

/// Formats a colored terminal pull request link.
#[must_use]
pub fn pull_request_link(repo_link: &str, pr_number: u32) -> String {
    // Formats an OSC 8 clickable terminal pull request link.
    // format!("\x1b]8;;{repo_link}/pull/{pr_number}\x1b\\(#{pr_number})\x1b]8;;\x1b\\")

    let blue_color = "\x1b[34m";
    let dark_color = "\x1b[90m";
    let default_color = "\x1b[0m";
    format!("{dark_color}{repo_link}/pull/{blue_color}{pr_number}{default_color}")
}
