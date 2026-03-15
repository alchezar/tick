//! Custom CLI types.

use core::fmt::{Display, Formatter, Result as FmtResult};
use core::str::FromStr;

use uuid::Uuid;

use crate::error::{CliError, CliResult};

/// Task id - accepts full UUID or a short prefix (min 8 hex chars).
///
/// Resolved to a full [`Uuid`] via [`TaskService::find_by_prefix`](domain::service::TaskService::find_by_prefix).
#[derive(Debug, Clone)]
pub struct ShortId(String);

impl ShortId {
    /// Minimum prefix length (first segment of UUID).
    pub const MIN_LEN: usize = 8;

    /// Returns the hex prefix (dashes stripped).
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<Uuid> for ShortId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid.simple().to_string()[..Self::MIN_LEN].to_owned())
    }
}

impl Display for ShortId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str(&self.0)
    }
}

impl FromStr for ShortId {
    type Err = CliError;

    fn from_str(s: &str) -> CliResult<Self> {
        let clean = s.replace('-', "");

        (clean.len() >= Self::MIN_LEN)
            .then_some(())
            .ok_or(CliError::IdTooShort {
                got: clean.len(),
                min: Self::MIN_LEN,
            })?;

        clean
            .chars()
            .all(|c| c.is_ascii_hexdigit())
            .then_some(())
            .ok_or_else(|| CliError::IdInvalidHex {
                input: s.to_owned(),
            })?;

        Ok(Self(clean))
    }
}
