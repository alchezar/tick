//! Business logic for task management.

use uuid::Uuid;

use crate::{
    error::{CoreError, CoreResult, MAX_DEPTH},
    model::{Status, StatusChange, Task},
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
    pub fn create(&self, title: &str, parent: Option<&Uuid>, project_id: Uuid) -> CoreResult<Task> {
        self.check_depth(parent, 0)?;

        let siblings = match parent {
            Some(id) => self.repo.child_tasks_of(id)?,
            None => self
                .repo
                .list_tasks(&project_id)?
                .into_iter()
                .filter(Task::is_root)
                .collect(),
        };

        let mut task = Task::new(title, parent.copied(), project_id);
        let next_order = siblings
            .iter()
            .filter_map(|s| s.order)
            .max()
            .map_or(0, |m| m + 1);
        task.order = Some(next_order);

        self.repo.save_task(&task)?;
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
            .child_tasks_of(task_id)?
            .iter()
            .any(|c| c.status().is_active())
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
        self.check_depth(parent_id, self.subtree_depth(task_id)?)?;

        task.parent = parent_id.copied();
        self.repo.save_task(&task)
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
        self.repo.save_task(&task)
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
        self.repo.save_task(&task)
    }

    /// Returns the full status change history for a task, ordered by `changed_at`.
    ///
    /// # Errors
    /// Returns an error if the persistence operation fails.
    #[inline]
    pub fn status_history(&self, task_id: &Uuid) -> CoreResult<Vec<StatusChange>> {
        self.repo.list_task_changes(task_id)
    }

    /// Deletes a task and all its children recursively.
    ///
    /// Idempotent — returns `Ok(())` if the task does not exist.
    ///
    /// # Errors
    /// Returns an error if the persistence operation fails.
    #[inline]
    pub fn delete(&self, task_id: &Uuid) -> CoreResult<()> {
        self.repo.delete_task(task_id)
    }

    // -------------------------------------------------------------------------

    /// Finds a task by id or returns [`CoreError::TaskNotFound`].
    fn find_task(&self, id: &Uuid) -> CoreResult<Task> {
        self.repo
            .find_task_by(id)?
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
        let old_status = task.status();
        task.update_status(new_status)?;
        self.repo.save_task(&task)?;
        self.repo
            .save_task_change(&StatusChange::new(task.id, old_status, new_status))
    }

    /// Recursively blocks all active descendants of the given task.
    fn block_children(&self, parent_id: &Uuid) -> CoreResult<()> {
        for mut child in self.repo.child_tasks_of(parent_id)? {
            if child.status().is_active() {
                let old_status = child.status();
                child.update_status(Status::Blocked)?;
                self.repo.save_task(&child)?;
                self.repo.save_task_change(&StatusChange::new(
                    child.id,
                    old_status,
                    Status::Blocked,
                ))?;
                self.block_children(&child.id)?;
            }
        }
        Ok(())
    }

    /// Returns the maximum depth among all descendants of a task (0 if no children).
    fn subtree_depth(&self, task_id: &Uuid) -> CoreResult<usize> {
        let children = self.repo.child_tasks_of(task_id)?;
        if children.is_empty() {
            return Ok(0);
        }
        let mut max = 0;
        for child in &children {
            max = max.max(1 + self.subtree_depth(&child.id)?);
        }
        Ok(max)
    }

    /// Returns the depth of a task (1 for root, 2 for child of root, etc.).
    fn depth_of(&self, task_id: &Uuid) -> CoreResult<usize> {
        let mut depth = 1_usize;
        let mut current = *task_id;
        while let Some(task) = self.repo.find_task_by(&current)? {
            match task.parent {
                Some(id) => {
                    depth += 1;
                    current = id;
                }
                None => break,
            }
        }
        Ok(depth)
    }

    /// Checks that placing a node (with `extra_depth` levels below it)
    /// under `parent` would not exceed [`MAX_DEPTH`].
    ///
    /// - `create` passes `extra_depth = 0` (new leaf).
    /// - `move_to_parent` passes `extra_depth = subtree_depth(task_id)`.
    fn check_depth(&self, parent: Option<&Uuid>, extra_depth: usize) -> CoreResult<()> {
        let base = match parent {
            Some(id) => self.depth_of(id)?,
            None => 0,
        };
        if base + 1 + extra_depth > MAX_DEPTH {
            return Err(CoreError::MaxDepthExceeded);
        }
        Ok(())
    }
}
