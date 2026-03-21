//! Task - the core entity.

use chrono::{DateTime, Utc};
use getset::{CopyGetters, Getters};

use crate::{
    error::{CoreError, CoreResult},
    model::{ProjectId, TaskId, status::Status},
    repository::TaskFilter,
};

/// A task or subtask tracked in the system.
#[derive(Debug, Default, Clone, Getters, CopyGetters)]
pub struct Task {
    /// Unique identifier.
    pub id: TaskId,
    /// Project unique identifier.
    pub project_id: ProjectId,
    /// Display title.
    pub title: String,
    /// Current lifecycle status.
    #[getset(get_copy = "pub")]
    status: Status,
    /// `Id` of the parent task, `None` for root tasks.
    pub parent: Option<TaskId>,
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
    #[must_use]
    pub fn new(title: impl Into<String>, parent: Option<TaskId>, project_id: ProjectId) -> Self {
        Self {
            id: TaskId::new(),
            project_id,
            title: title.into(),
            parent,
            created: Utc::now(),
            updated: Utc::now(),
            ..Task::default()
        }
    }

    /// Creates a new task with a specific creation timestamp.
    #[must_use]
    pub fn new_at(
        title: impl Into<String>,
        parent: Option<TaskId>,
        project_id: ProjectId,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: TaskId::new(),
            project_id,
            title: title.into(),
            parent,
            created: created_at,
            updated: created_at,
            ..Task::default()
        }
    }

    /// Returns `true` if the task has no parent (top-level task).
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.parent.is_none()
    }

    /// Returns a copy of this task with the given status.
    ///
    /// Used to reconstruct historical state from status change log.
    #[must_use]
    pub fn with_status(mut self, status: Status) -> Self {
        self.status = status;
        self
    }

    /// Sets a new status and records the transition timestamp.
    ///
    /// # Errors
    /// - [`CoreError::InvalidStatusTransition`] if the transition is not allowed.
    pub fn update_status(
        &mut self,
        new_status: Status,
        at: Option<DateTime<Utc>>,
    ) -> CoreResult<()> {
        if !self.status.can_transit(&new_status) {
            return Err(CoreError::InvalidStatusTransition {
                from: self.status,
                to: new_status,
            });
        }
        self.status = new_status;
        self.updated = at.unwrap_or_else(Utc::now);
        Ok(())
    }

    /// Returns a [`TaskFilter`] for siblings (tasks sharing the same parent).
    pub fn siblings_filter(&self, project_id: ProjectId) -> TaskFilter {
        self.parent.map_or_else(
            || TaskFilter::RootByProject(project_id),
            TaskFilter::ChildrenOf,
        )
    }
}
