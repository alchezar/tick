//! Custom CLI types.

use core::fmt::{Display, Formatter, Result as FmtResult};
use core::str::FromStr;

use uuid::Uuid;

use crate::error::{CliError, CliResult};
use domain::{
    model::Task,
    repository::{TaskRepository, Transactional},
    service::TaskService,
};

/// Task id - accepts full UUID or a short prefix (min 8 hex chars).
///
/// Resolved to a full [`Uuid`] via [`ShortId::to_uuid`].
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

    /// Resolves this prefix to a full [`Uuid`] by fetching tasks from the service.
    ///
    /// Tries full UUID parse first, then scans project tasks for a unique prefix match.
    ///
    /// # Errors
    /// - [`CliError::IdNotFound`] if no task matches.
    /// - [`CliError::IdAmbiguous`] if multiple tasks match.
    pub async fn to_uuid<R>(
        &self,
        task_service: &TaskService<R>,
        project_id: &Uuid,
    ) -> CliResult<Uuid>
    where
        R: TaskRepository + Transactional,
    {
        if let Ok(uuid) = self.0.parse::<Uuid>() {
            return Ok(uuid);
        }

        let tasks = task_service.list(project_id).await?;
        Self::find_by_prefix(&self.0, &tasks)
    }

    /// Finds a unique task matching the given hex prefix.
    fn find_by_prefix(prefix: &str, tasks: &[Task]) -> CliResult<Uuid> {
        let matches = tasks
            .iter()
            .filter(|t| t.id.simple().to_string().starts_with(prefix))
            .collect::<Vec<_>>();

        match matches.len() {
            0 => Err(CliError::IdNotFound {
                prefix: prefix.to_owned(),
            }),
            1 => Ok(matches[0].id),
            _ => Err(CliError::IdAmbiguous {
                prefix: prefix.to_owned(),
                count: matches.len(),
            }),
        }
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
