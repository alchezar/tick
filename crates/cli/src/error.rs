//! CLI error types.

use std::path::PathBuf;

use core::error::Error;
use core::fmt::{Display, Formatter, Result as FmtResult};

use domain::error::CoreError;

/// Errors produced by the CLI layer.
#[derive(Debug)]
pub enum CliError {
    /// Task id prefix is too short.
    IdTooShort {
        /// Number of hex characters provided.
        got: usize,
        /// Minimum required.
        min: usize,
    },
    /// Task id contains non-hex characters.
    IdInvalidHex {
        /// The original input.
        input: String,
    },
    /// Config file cannot be read or parsed.
    ConfigRead {
        /// Path to the config file.
        path: PathBuf,
        /// Underlying error message.
        source: String,
    },
    /// Config file cannot be written.
    ConfigWrite {
        /// Path to the config file.
        path: PathBuf,
        /// Underlying error message.
        source: String,
    },
    /// XDG data directory could not be determined.
    NoDataDir,
    /// No active project set.
    NoActiveProject,
    /// Clipboard operation failed.
    Clipboard(String),
    /// Domain-level error (task/project not found, invalid transition, etc.).
    Domain(CoreError),
}

impl Display for CliError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::IdTooShort { got, min } => {
                write!(
                    f,
                    "id too short: expected at least {min} hex chars, got {got}"
                )
            }
            Self::IdInvalidHex { input } => {
                write!(f, "invalid id: '{input}' is not a valid hex string")
            }
            Self::ConfigRead { path, source } => {
                write!(f, "cannot read config {}: {source}", path.display())
            }
            Self::ConfigWrite { path, source } => {
                write!(f, "cannot write config {}: {source}", path.display())
            }
            Self::NoDataDir => {
                write!(f, "cannot determine XDG data directory")
            }
            Self::Clipboard(source) => {
                write!(f, "clipboard error: {source}")
            }
            Self::NoActiveProject => {
                write!(f, "no active project set, use `tick project switch <slug>`")
            }
            Self::Domain(e) => Display::fmt(e, f),
        }
    }
}

impl Error for CliError {}

impl From<CoreError> for CliError {
    fn from(e: CoreError) -> Self {
        Self::Domain(e)
    }
}

/// Shorthand `Result` type for CLI operations.
pub type CliResult<T> = Result<T, CliError>;
