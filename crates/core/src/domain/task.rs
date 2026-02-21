//! Task — the core domain entity.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::status::Status;

/// A task or subtask tracked in the system.
#[derive(Debug, Default, Clone)]
pub struct Task {
    /// Unique identifier.
    pub id: Uuid,
    /// Display title.
    pub title: String,
    /// Current lifecycle status.
    pub status: Status,
    /// Id of the parent task, `None` for root tasks.
    pub parent: Option<Uuid>,
    /// Display order among siblings.
    pub order: Option<usize>,
    /// Creation timestamp.
    pub created: DateTime<Utc>,
    /// Last modification timestamp.
    pub updated: DateTime<Utc>,
}

impl Task {
    /// Creates a new task with `NotStarted` status and current timestamp.
    #[inline]
    #[must_use]
    pub fn new(title: impl Into<String>, parent: Option<Uuid>) -> Self {
        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            parent,
            created: Utc::now(),
            updated: Utc::now(),
            ..Task::default()
        }
    }

    /// Returns `true` if the task has no parent (top-level task).
    #[inline]
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.parent.is_none()
    }

    /// Updates `updated` timestamp to the current time.
    #[inline]
    pub fn touch(&mut self) {
        self.updated = Utc::now();
    }
}
