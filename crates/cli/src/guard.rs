//! Interactive confirmation guards for destructive operations.

use std::io::{self, BufRead, StdinLock, Stdout, Write};

use crate::error::{CliError, CliResult};

/// Confirmation strategy for destructive operations.
pub trait Confirm {
    /// Asks the user to confirm deletion of the entity described by `label`.
    ///
    /// # Errors
    ///
    /// Returns [`CliError::Aborted`] if the user declines.
    fn confirm(&mut self, label: &str) -> CliResult<()>;
}

/// Interactive guard that prompts stdin/stdout.
///
/// Only `y` or `Y` confirms; everything else (including empty input) aborts.
#[derive(Debug)]
pub struct RemoveGuard<R, W> {
    reader: R,
    writer: W,
}

impl Default for RemoveGuard<StdinLock<'static>, Stdout> {
    fn default() -> Self {
        Self {
            reader: io::stdin().lock(),
            writer: io::stdout(),
        }
    }
}

impl<R, W> RemoveGuard<R, W>
where
    R: BufRead,
    W: Write,
{
    /// Creates a guard with custom reader/writer (useful for testing).
    pub fn with_io(reader: R, writer: W) -> Self {
        Self { reader, writer }
    }
}

impl<R, W> Confirm for RemoveGuard<R, W>
where
    R: BufRead,
    W: Write,
{
    fn confirm(&mut self, label: &str) -> CliResult<()> {
        write!(self.writer, "You really want to delete {label}? [y/N] ")
            .map_err(|e| CliError::Io(e.to_string()))?;
        self.writer
            .flush()
            .map_err(|e| CliError::Io(e.to_string()))?;

        let mut answer = String::new();
        self.reader
            .read_line(&mut answer)
            .map_err(|e| CliError::Io(e.to_string()))?;

        answer
            .trim()
            .eq_ignore_ascii_case("y")
            .then_some(())
            .ok_or(CliError::Aborted)
    }
}

/// No-op guard that always confirms. Used in tests.
#[derive(Debug)]
pub struct AutoConfirm;

impl Confirm for AutoConfirm {
    fn confirm(&mut self, _label: &str) -> CliResult<()> {
        Ok(())
    }
}
