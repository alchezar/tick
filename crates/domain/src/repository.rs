//! Repository traits - persistence contracts for projects and tasks.

use chrono::NaiveDate;
use uuid::Uuid;

use crate::{
    error::CoreResult,
    model::{Project, StatusChange, Task},
};

/// Provides `RAII`-based transaction demarcation.
///
/// Implementations must support nesting via a depth counter:
/// only the outermost `begin`/`commit` pair issues real SQL statements;
/// inner pairs simply adjust the counter.
pub trait Transactional {
    /// Guard type returned by [`begin_transaction`](Transactional::begin_transaction).
    type Guard<'a>: TransactionGuard
    where
        Self: 'a;

    /// Opens a transaction (or increments nesting depth) and returns a guard.
    ///
    /// All repository calls made while the guard is alive participate in
    /// the same transaction. If the guard is dropped without
    /// [`commit_transaction`](TransactionGuard::commit_transaction),
    /// the implementation should roll back automatically.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    async fn begin_transaction(&self) -> CoreResult<Self::Guard<'_>>;
}

/// `RAII` transaction guard returned by [`Transactional::begin_transaction`].
///
/// Consuming [`commit_transaction`](TransactionGuard::commit_transaction)
/// persists all changes made since `begin_transaction`.
/// Dropping the guard without committing triggers an automatic rollback
/// (e.g. via [`Drop`] in the `SQLite` implementation).
pub trait TransactionGuard {
    /// Commits the transaction (or decrements nesting depth).
    ///
    /// Only the outermost commit issues a real `COMMIT`.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    async fn commit_transaction(self) -> CoreResult<()>;
}

/// Defines the persistence contract for projects.
/// Implemented by `db/` crate.
pub trait ProjectRepository {
    /// Inserts or updates a project.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    async fn save_project(&self, project: &Project) -> CoreResult<()>;

    /// Returns a project by id.
    ///
    /// Returns `Ok(None)` if the project does not exist - not an error.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    async fn find_project_by_id(&self, id: &Uuid) -> CoreResult<Option<Project>>;

    /// Returns a project by slug.
    ///
    /// Returns `Ok(None)` if the project does not exist - not an error.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    async fn find_project_by_slug(&self, slug: &str) -> CoreResult<Option<Project>>;

    /// Returns all projects.
    ///
    /// Returns `Ok(vec![])` if no projects exist.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    async fn list_projects(&self) -> CoreResult<Vec<Project>>;

    /// Deletes a project and all its tasks by id.
    ///
    /// Task cascade is handled at the db level (e.g. `ON DELETE CASCADE`).
    /// Idempotent - returns `Ok(())` if the project does not exist.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    async fn delete_project(&self, project_id: &Uuid) -> CoreResult<()>;
}

/// Defines the persistence contract for tasks.
/// Implemented by `db/` crate.
pub trait TaskRepository {
    /// Inserts or updates a task.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    async fn save_task(&self, task: &Task) -> CoreResult<()>;

    /// Returns a task by id.
    ///
    /// Returns `Ok(None)` if the task does not exist - not an error.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    async fn find_task_by_id(&self, id: &Uuid) -> CoreResult<Option<Task>>;

    /// Finds a task id by hex prefix within a project.
    ///
    /// Returns `Ok(None)` if no task matches.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    async fn find_task_by_id_prefix(
        &self,
        project_id: &Uuid,
        id_prefix: &str,
    ) -> CoreResult<Option<Uuid>>;

    /// Returns all direct children of the given parent task.
    ///
    /// Returns `Ok(vec![])` if the parent has no children.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    async fn child_tasks_of(&self, parent: &Uuid) -> CoreResult<Vec<Task>>;

    /// Returns all tasks regardless of status.
    ///
    /// Returns `Ok(vec![])` if no tasks exist.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    async fn list_tasks(&self, project_id: &Uuid) -> CoreResult<Vec<Task>>;

    /// Deletes a task and all its children by id.
    ///
    /// Idempotent - returns `Ok(())` if the task does not exist.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    async fn delete_task(&self, id: &Uuid) -> CoreResult<()>;

    /// Deletes all tasks and all its children by id that related to project.
    ///
    /// Idempotent - returns `Ok(())` if the task does not exist.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    async fn delete_all_tasks_by(&self, project_id: &Uuid) -> CoreResult<()>;

    /// Saves a status change record.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    async fn save_task_change(&self, change: &StatusChange) -> CoreResult<()>;

    /// Returns all status changes for a given task, ordered by `changed_at` ascending.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    async fn list_task_changes(&self, task_id: &Uuid) -> CoreResult<Vec<StatusChange>>;

    /// Returns all status changes that occurred on the given date.
    ///
    /// # Errors
    /// Returns an error if the underlying storage operation fails.
    async fn list_task_changes_on(&self, date: NaiveDate) -> CoreResult<Vec<StatusChange>>;
}
