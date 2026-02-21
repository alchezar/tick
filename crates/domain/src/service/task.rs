//! Business logic for task management.

use uuid::Uuid;

use crate::{
    error::{CoreError, CoreResult, MAX_DEPTH},
    model::{Status, Task},
    repository::TaskRepository,
};

/// Encapsulates all business rules for task management.
///
/// Acts as the intermediary between the CLI/TUI/API layer and the repository.
/// All domain invariants (nesting depth, status transitions, cascading deletes)
/// are enforced here.
#[derive(Debug)]
pub struct TaskService<R>
where
    R: TaskRepository,
{
    repo: R,
}

impl<R> TaskService<R>
where
    R: TaskRepository,
{
    /// Creates a new `TaskService` with the given repository.
    #[inline]
    #[must_use]
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    /// Creates a new task and persists it.
    ///
    /// Assigns `order` as the next sibling position among existing siblings.
    ///
    /// # Errors
    /// - [`CoreError::MaxDepthExceeded`] if nesting depth would exceed 3.
    /// - Returns an error if the persistence operation fails.
    #[inline]
    pub fn create(&self, title: &str, parent: Option<&Uuid>) -> CoreResult<Task> {
        self.check_depth(parent)?;

        let siblings = match parent {
            Some(id) => self.repo.children_of(id)?,
            None => self
                .repo
                .list_all()?
                .into_iter()
                .filter(Task::is_root)
                .collect(),
        };

        let mut task = Task::new(title, parent.copied());
        task.order = Some(siblings.len());

        self.repo.save(&task)?;
        Ok(task)
    }

    /// Sets status to [`Status::InProgress`].
    ///
    /// # Errors
    /// - [`CoreError::TaskNotFound`] if no task exists with the given id.
    /// - [`CoreError::InvalidStatusTransition`] if the transition is not allowed.
    /// - Returns an error if the persistence operation fails.
    #[inline]
    pub fn start(&self, task_id: &Uuid) -> CoreResult<()> {
        self.update_status(task_id, Status::InProgress)
    }

    /// Resets status to [`Status::NotStarted`].
    ///
    /// # Errors
    /// - [`CoreError::TaskNotFound`] if no task exists with the given id.
    /// - Returns an error if the persistence operation fails.
    #[inline]
    pub fn reset(&self, task_id: &Uuid) -> CoreResult<()> {
        self.update_status(task_id, Status::NotStarted)
    }

    /// Marks a task as [`Status::Done`].
    ///
    /// # Errors
    /// - [`CoreError::TaskNotFound`] if no task exists with the given id.
    /// - [`CoreError::TaskHasUnfinishedChildren`] if any child task is still active.
    /// - Returns an error if the persistence operation fails.
    #[inline]
    pub fn done(&self, task_id: &Uuid) -> CoreResult<()> {
        if self
            .repo
            .children_of(task_id)?
            .iter()
            .any(|c| c.status.is_active())
        {
            return Err(CoreError::TaskHasUnfinishedChildren);
        }

        self.update_status(task_id, Status::Done)
    }

    /// Marks a task as [`Status::Blocked`] and cascades to all active descendants.
    ///
    /// # Errors
    /// - [`CoreError::TaskNotFound`] if no task exists with the given id.
    /// - [`CoreError::InvalidStatusTransition`] if the transition is not allowed.
    /// - Returns an error if the persistence operation fails.
    #[inline]
    pub fn block(&self, task_id: &Uuid) -> CoreResult<()> {
        self.update_status(task_id, Status::Blocked)?;
        self.block_children(task_id)
    }

    /// Moves a task under a new parent, or promotes it to root if `parent_id` is `None`.
    ///
    /// # Errors
    /// - [`CoreError::TaskNotFound`] if the task does not exist.
    /// - [`CoreError::MaxDepthExceeded`] if the move would exceed nesting depth of 3.
    /// - Returns an error if the persistence operation fails.
    #[inline]
    pub fn move_to_parent(&self, task_id: &Uuid, parent_id: Option<&Uuid>) -> CoreResult<()> {
        let mut task = self.find_task(task_id)?;
        self.check_depth(parent_id)?;

        task.parent = parent_id.copied();
        self.repo.save(&task)
    }

    /// Renames a task.
    ///
    /// # Errors
    /// - [`CoreError::TaskNotFound`] if no task exists with the given id.
    /// - Returns an error if the persistence operation fails.
    #[inline]
    pub fn rename(&self, task_id: &Uuid, title: &str) -> CoreResult<()> {
        let mut task = self.find_task(task_id)?;

        title.clone_into(&mut task.title);
        self.repo.save(&task)
    }

    /// Changes the display order of a task among its siblings.
    ///
    /// # Errors
    /// - [`CoreError::TaskNotFound`] if no task exists with the given id.
    /// - Returns an error if the persistence operation fails.
    #[inline]
    pub fn reorder(&self, task_id: &Uuid, order: usize) -> CoreResult<()> {
        let mut task = self.find_task(task_id)?;

        task.order = Some(order);
        self.repo.save(&task)
    }

    /// Deletes a task and all its children recursively.
    ///
    /// Idempotent — returns `Ok(())` if the task does not exist.
    ///
    /// # Errors
    /// Returns an error if the persistence operation fails.
    #[inline]
    pub fn delete(&self, task_id: &Uuid) -> CoreResult<()> {
        self.repo.delete(task_id)
    }

    // -------------------------------------------------------------------------

    /// Finds a task by id or returns [`CoreError::TaskNotFound`].
    fn find_task(&self, id: &Uuid) -> CoreResult<Task> {
        self.repo
            .find_by_id(id)?
            .ok_or(CoreError::TaskNotFound { id: *id })
    }

    /// Updates the status of a task.
    ///
    /// # Errors
    /// - [`CoreError::TaskNotFound`] if no task exists with the given id.
    /// - [`CoreError::InvalidStatusTransition`] if the transition is not allowed.
    /// - Returns an error if the persistence operation fails.
    #[inline]
    fn update_status(&self, task_id: &Uuid, new_status: Status) -> CoreResult<()> {
        let mut task = self.find_task(task_id)?;

        if !task.status.can_transit(&new_status) {
            return Err(CoreError::InvalidStatusTransition {
                from: task.status,
                to: new_status,
            });
        }

        task.status = new_status;
        task.touch();
        self.repo.save(&task)
    }

    /// Recursively blocks all active descendants of the given task.
    fn block_children(&self, parent_id: &Uuid) -> CoreResult<()> {
        for mut child in self.repo.children_of(parent_id)? {
            if child.status.is_active() {
                child.status = Status::Blocked;
                child.touch();
                self.repo.save(&child)?;
                self.block_children(&child.id)?;
            }
        }
        Ok(())
    }

    /// Checks that placing a task under `parent` would not exceed [`MAX_DEPTH`].
    ///
    /// Walks up the parent chain and returns [`CoreError::MaxDepthExceeded`]
    /// as soon as the depth limit is reached, without traversing further.
    fn check_depth(&self, parent: Option<&Uuid>) -> CoreResult<()> {
        let Some(mut current) = parent.copied() else {
            return Ok(());
        };
        let mut depth = 0_usize;

        loop {
            let Some(task) = self.repo.find_by_id(&current)? else {
                break;
            };
            let Some(parent_id) = task.parent else { break };

            depth += 1;
            if depth >= MAX_DEPTH {
                return Err(CoreError::MaxDepthExceeded);
            }
            current = parent_id;
        }

        Ok(())
    }
}
