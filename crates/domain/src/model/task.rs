//! Task — the core  entity.

use chrono::{DateTime, Utc};
use getset::{CopyGetters, Getters};
use uuid::Uuid;

use crate::error::{CoreError, CoreResult};
use crate::model::status::Status;

/// A task or subtask tracked in the system.
#[derive(Debug, Default, Clone, Getters, CopyGetters)]
pub struct Task {
    /// Unique identifier.
    pub id: Uuid,
    /// Display title.
    pub title: String,
    /// Current lifecycle status.
    #[getset(get_copy = "pub")]
    status: Status,
    /// `Id` of the parent task, `None` for root tasks.
    pub parent: Option<Uuid>,
    /// Display order among siblings.
    pub order: Option<usize>,
    /// Creation timestamp.
    pub created: DateTime<Utc>,
    /// Timestamp of the last status change.
    /// Updated only on status transitions, not on renames or reorders.
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

    /// Sets a new status and records the transition timestamp.
    ///
    /// # Errors
    /// - [`CoreError::InvalidStatusTransition`] if the transition is not allowed.
    #[inline]
    pub fn update_status(&mut self, new_status: Status) -> CoreResult<()> {
        if !self.status.can_transit(&new_status) {
            return Err(CoreError::InvalidStatusTransition {
                from: self.status,
                to: new_status,
            });
        }
        self.status = new_status;
        self.updated = Utc::now();
        Ok(())
    }
}
