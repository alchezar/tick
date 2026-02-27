//! Task status, allowed transitions, and status change history.

use chrono::{DateTime, Utc};
use uuid::Uuid;

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
}

impl Status {
    /// Returns `true` if transition from current status to `to` is allowed.
    #[inline]
    #[must_use]
    pub fn can_transit(&self, to: &Self) -> bool {
        matches!(
            (self, to),
            (_, Status::NotStarted)
                | (Status::NotStarted | Status::Blocked, Status::InProgress)
                | (Status::NotStarted | Status::InProgress, Status::Blocked)
                | (Status::InProgress, Status::Done)
        )
    }

    /// Returns `true` if the task is actionable (shown in Today section of the report).
    #[inline]
    #[must_use]
    pub fn is_active(&self) -> bool {
        matches!(self, Status::NotStarted | Status::InProgress)
    }

    /// Returns `true` if the task is no longer actionable (shown in Previously section of the report).
    #[inline]
    #[must_use]
    pub fn is_closed(&self) -> bool {
        matches!(self, Status::Done | Status::Blocked)
    }

    /// Returns the emoji icon representing this status in the report output.
    #[inline]
    #[must_use]
    pub fn icon(&self) -> &'static str {
        match self {
            Self::NotStarted => "❌",
            Self::InProgress => "🔄",
            Self::Done => "✅",
            Self::Blocked => "🛑",
        }
    }
}

/// A single status transition event for a task.
#[derive(Debug, Clone)]
pub struct StatusChange {
    /// Unique identifier.
    pub id: Uuid,
    /// The task this change belongs to.
    pub task_id: Uuid,
    /// Status before the transition.
    pub old_status: Status,
    /// Status after the transition.
    pub new_status: Status,
    /// When the transition occurred.
    pub changed_at: DateTime<Utc>,
}

impl StatusChange {
    /// Creates a new status change record with a generated id.
    #[inline]
    #[must_use]
    pub fn new(task_id: Uuid, old_status: Status, new_status: Status) -> Self {
        Self {
            id: Uuid::new_v4(),
            task_id,
            old_status,
            new_status,
            changed_at: Utc::now(),
        }
    }
}
