//! CLI error types.

use core::error::Error;
use core::fmt::{Display, Formatter, Result as FmtResult};
use std::path::PathBuf;

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
    /// I/O error (stdin/stdout).
    Io(String),
    /// Invalid date supplied to `--date`.
    InvalidDate {
        /// The date that could not be converted.
        date: String,
    },
    /// One or more operations in a batch command failed.
    BatchFailed {
        /// Number of operations that failed.
        failed: usize,
        /// Total number of operations attempted in the batch.
        total: usize,
    },
    /// User declined a destructive operation.
    Aborted,
    /// Domain-level error (task/project not found, invalid transition, etc.).
    Domain(CoreError),
}

impl Display for CliError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::IdTooShort { got, min } => {
                write!(f, "expected at least {min} hex chars, got {got}")
            }
            Self::IdInvalidHex { input } => {
                write!(f, "'{input}' is not a valid hex string")
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
            Self::Io(source) => {
                write!(f, "I/O error: {source}")
            }
            Self::InvalidDate { date } => {
                write!(f, "'{date}' cannot be converted to a timestamp")
            }
            Self::BatchFailed { failed, total } => {
                write!(f, "{failed} of {total} operations failed")
            }
            Self::Aborted => {
                write!(f, "aborted")
            }
            Self::NoActiveProject => {
                write!(f, "no active project set, use `tt project switch <slug>`")
            }
            Self::Domain(e) => Display::fmt(e, f),
        }
    }
}

impl Error for CliError {}

impl From<CoreError> for CliError {
    fn from(err: CoreError) -> Self {
        Self::Domain(err)
    }
}

/// Shorthand `Result` type for CLI operations.
pub type CliResult<T> = Result<T, CliError>;
