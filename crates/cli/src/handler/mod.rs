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
