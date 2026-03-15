//! CLI error types.

use core::error::Error;
use core::fmt::{Display, Formatter, Result as FmtResult};

/// Errors produced by the CLI layer.
#[derive(Debug)]
pub enum CliError {
    /// Task id prefix is too short.
    TooShortId {
        /// Number of hex characters provided.
        got: usize,
        /// Minimum required.
        min: usize,
    },
    /// Task id contains non-hex characters.
    InvalidHexId {
        /// The original input.
        input: String,
    },
}

impl Display for CliError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::TooShortId { got, min } => {
                write!(
                    f,
                    "id too short: expected at least {min} hex chars, got {got}"
                )
            }
            Self::InvalidHexId { input } => {
                write!(f, "invalid id: '{input}' is not a valid hex string")
            }
        }
    }
}

impl Error for CliError {}

/// Shorthand `Result` type for CLI operations.
pub type CliResult<T> = Result<T, CliError>;
