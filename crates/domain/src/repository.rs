//! Repository traits - persistence contracts for projects and tasks.

use chrono::NaiveDate;
use uuid::Uuid;

use crate::{
    error::CoreResult,
    model::{Project, StatusChange, Task},
};

/// Defines the persistence contract for projects.
/// Implemented by `db/` crate.
pub trait ProjectRepository {
    /// Inserts or updates a project.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    fn save(&self, project: &Project) -> CoreResult<()>;

    /// Returns a project by id.
    ///
    /// Returns `Ok(None)` if the project does not exist - not an error.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    fn find_by_id(&self, id: &Uuid) -> CoreResult<Option<Project>>;

    /// Returns a project by slug.
    ///
    /// Returns `Ok(None)` if the project does not exist - not an error.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    fn find_by_slug(&self, slug: &str) -> CoreResult<Option<Project>>;

    /// Returns all projects.
    ///
    /// Returns `Ok(vec![])` if no projects exist.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    fn list(&self) -> CoreResult<Vec<Project>>;

    /// Deletes a project and all its tasks by id.
    ///
    /// Task cascade is handled at the db level (e.g. `ON DELETE CASCADE`).
    /// Idempotent - returns `Ok(())` if the project does not exist.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    fn delete(&self, project_id: &Uuid) -> CoreResult<()>;
}

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
    fn find_by(&self, id: &Uuid) -> CoreResult<Option<Task>>;

    /// Returns all direct children of the given parent task.
    ///
    /// Returns `Ok(vec![])` if the parent has no children.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    fn children_of(&self, parent: &Uuid) -> CoreResult<Vec<Task>>;

    /// Returns all tasks regardless of status.
    ///
    /// Returns `Ok(vec![])` if no tasks exist.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    fn list_all(&self, project_id: &Uuid) -> CoreResult<Vec<Task>>;

    /// Deletes a task and all its children by id.
    ///
    /// Idempotent — returns `Ok(())` if the task does not exist.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    fn delete(&self, id: &Uuid) -> CoreResult<()>;

    /// Deletes all tasks and all its children by id that related to project.
    ///
    /// Idempotent — returns `Ok(())` if the task does not exist.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    fn delete_all_by(&self, project_id: &Uuid) -> CoreResult<()>;

    /// Saves a status change record.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    fn save_status_change(&self, change: &StatusChange) -> CoreResult<()>;

    /// Returns all status changes for a given task, ordered by `changed_at` ascending.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    fn list_status_changes(&self, task_id: &Uuid) -> CoreResult<Vec<StatusChange>>;

    /// Returns all status changes that occurred on the given date.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    fn list_status_changes_on(&self, date: NaiveDate) -> CoreResult<Vec<StatusChange>>;
}
