//! Task status, allowed transitions, and status change history.

use core::fmt;
use core::str::FromStr;

use chrono::{DateTime, Utc};
use fmt::{Display, Formatter, Result as FmtResult};

use crate::{
    error::{CoreError, CoreResult},
    model::{StatusChangeId, TaskId},
};

/// Represents the lifecycle state of a task.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum Status {
    /// Task has not been started yet.
    #[default]
    NotStarted,
    /// Task is currently being worked on.
    InProgress,
    /// Task has been completed.
    Done,
    /// Task is blocked and cannot progress.
    Blocked,
    /// Task was abandoned and is no longer relevant.
    Abandoned,
}

impl Status {
    /// Returns the database TEXT representation of this status.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::InProgress => "in_progress",
            Self::Done => "done",
            Self::Blocked => "blocked",
            Self::Abandoned => "abandoned",
        }
    }

    /// Returns `true` if transition from current status to `to` is allowed.
    #[must_use]
    pub fn can_transit(&self, to: &Self) -> bool {
        matches!(
            (self, to),
            (_, Status::NotStarted | Status::Abandoned)
                | (Status::NotStarted | Status::Blocked, Status::InProgress)
                | (
                    Status::NotStarted | Status::InProgress,
                    Status::Blocked | Status::Done
                )
        )
    }

    /// Returns `true` if the task is actionable (shown in Today section of the
    /// report).
    #[must_use]
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            Status::NotStarted | Status::InProgress | Status::Blocked
        )
    }

    /// Returns `true` if the task is no longer actionable (shown in Previously
    /// section of the report).
    #[must_use]
    pub fn is_closed(&self) -> bool {
        matches!(self, Status::Done | Status::Abandoned)
    }

    /// Returns `true` if the task should appear in reports.
    #[must_use]
    pub fn is_reportable(&self) -> bool {
        *self != Status::Abandoned
    }

    /// Returns the emoji icon representing this status in the report output.
    #[must_use]
    pub fn icon(&self) -> &'static str {
        match self {
            Self::NotStarted => "❌",
            Self::InProgress => "🔄",
            Self::Done => "✅",
            Self::Blocked => "🛑",
            Self::Abandoned => "🚫",
        }
    }
}

impl FromStr for Status {
    type Err = CoreError;

    fn from_str(s: &str) -> CoreResult<Self> {
        match s {
            "not_started" => Ok(Self::NotStarted),
            "in_progress" => Ok(Self::InProgress),
            "done" => Ok(Self::Done),
            "blocked" => Ok(Self::Blocked),
            "abandoned" => Ok(Self::Abandoned),
            other => Err(CoreError::ParseStatusError(other.to_owned())),
        }
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str(self.as_str())
    }
}

/// A single status transition event for a task.
#[derive(Debug, Clone)]
pub struct StatusChange {
    /// Unique identifier.
    pub id: StatusChangeId,
    /// The task this change belongs to.
    pub task_id: TaskId,
    /// Status before the transition.
    pub old_status: Status,
    /// Status after the transition.
    pub new_status: Status,
    /// When the transition occurred.
    pub changed_at: DateTime<Utc>,
}

impl StatusChange {
    /// Creates a new status change record with a generated id.
    #[must_use]
    pub fn new(
        task_id: TaskId,
        old_status: Status,
        new_status: Status,
        changed_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id: StatusChangeId::new(),
            task_id,
            old_status,
            new_status,
            changed_at: changed_at.unwrap_or_else(Utc::now),
        }
    }
}
