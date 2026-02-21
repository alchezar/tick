//! Repository trait — persistence contract for tasks.

use uuid::Uuid;

use crate::{domain::Task, error::CoreResult};

/// Defines the persistence contract for tasks.
/// Implemented by `db/` crate.
pub trait TaskRepository {
    /// Inserts or updates a task.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    fn save(&self, task: &Task) -> CoreResult<()>;

    /// Returns a task by id.
    ///
    /// Returns `Ok(None)` if the task does not exist — not an error.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    fn find_by_id(&self, id: &Uuid) -> CoreResult<Option<Task>>;

    /// Returns all direct children of the given parent task.
    ///
    /// Returns `Ok(vec![])` if the parent has no children.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    fn children_of(&self, parent: &Uuid) -> CoreResult<Vec<Task>>;

    /// Returns all tasks with `not_started` or `in_progress` status.
    ///
    /// Returns `Ok(vec![])` if no active tasks exist.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    fn list_active(&self) -> CoreResult<Vec<Task>>;

    /// Returns all tasks regardless of status.
    ///
    /// Returns `Ok(vec![])` if no tasks exist.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    fn list_all(&self) -> CoreResult<Vec<Task>>;

    /// Deletes a task and all its children by id.
    ///
    /// Idempotent — returns `Ok(())` if the task does not exist.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    fn delete(self, id: &Uuid) -> CoreResult<()>;
}
