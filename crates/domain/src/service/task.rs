//! Business logic for task management.

use chrono::{DateTime, Utc};

use crate::{
    error::{CoreError, CoreResult, MAX_DEPTH},
    model::{ProjectId, Status, StatusChange, Task, TaskId},
    repository::{TaskFilter, TaskRepository, TransactionGuard, Transactional},
};

/// Encapsulates all business rules for task management.
///
/// Acts as the intermediary between the CLI/TUI/API layer and the repository.
/// All domain invariants (nesting depth, status transitions, cascading deletes)
/// are enforced here.
#[derive(Debug)]
pub struct TaskService<R>
where
    R: TaskRepository + Transactional,
{
    repo: R,
}

impl<R> TaskService<R>
where
    R: TaskRepository + Transactional,
{
    /// Creates a new `TaskService` with the given repository.
    #[must_use]
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    /// Creates a new task and persists it.
    ///
    /// Assigns `order` as the next sibling position among existing siblings.
    ///
    /// # Errors
    ///
    /// - [`CoreError::MaxDepthExceeded`] if nesting depth would exceed 3.
    /// - Returns an error if the persistence operation fails.
    pub async fn create(
        &self,
        title: &str,
        parent_id: Option<TaskId>,
        project_id: ProjectId,
        created_at: Option<DateTime<Utc>>,
        pull_request_number: Option<u32>,
        branch_name: Option<String>,
    ) -> CoreResult<Task> {
        self.check_depth(parent_id, 0).await?;

        let tx = self.repo.begin_transaction().await?;

        let mut task = match created_at {
            Some(at) => Task::new_at(title, parent_id, project_id, at),
            None => Task::new(title, parent_id, project_id),
        };
        let siblings = self
            .repo
            .list_tasks(&task.siblings_filter(project_id))
            .await?;
        let next_order = siblings
            .iter()
            .filter_map(|task| task.order)
            .max()
            .map_or(0, |max| max + 1);
        task.order = Some(next_order);
        task.pull_request_number = pull_request_number;
        task.branch_name = branch_name;
        self.repo.save_task(&task).await?;

        tx.commit_transaction().await?;
        Ok(task)
    }

    /// Sets status to [`Status::InProgress`].
    ///
    /// # Errors
    ///
    /// - [`CoreError::TaskNotFound`] if no task exists with the given id.
    /// - [`CoreError::InvalidStatusTransition`] if the transition is not allowed.
    /// - Returns an error if the persistence operation fails.
    pub async fn start(&self, task_id: &TaskId, at: Option<DateTime<Utc>>) -> CoreResult<()> {
        self.update_status(task_id, Status::InProgress, at).await
    }

    /// Resets status to [`Status::NotStarted`].
    ///
    /// # Errors
    ///
    /// - [`CoreError::TaskNotFound`] if no task exists with the given id.
    /// - Returns an error if the persistence operation fails.
    pub async fn reset(&self, task_id: &TaskId, at: Option<DateTime<Utc>>) -> CoreResult<()> {
        self.update_status(task_id, Status::NotStarted, at).await
    }

    /// Marks a task as [`Status::Done`].
    ///
    /// # Errors
    ///
    /// - [`CoreError::TaskNotFound`] if no task exists with the given id.
    /// - [`CoreError::TaskHasUnfinishedChildren`] if any child task is still active.
    /// - Returns an error if the persistence operation fails.
    pub async fn done(&self, task_id: &TaskId, at: Option<DateTime<Utc>>) -> CoreResult<()> {
        if self
            .repo
            .child_tasks_of(task_id)
            .await?
            .iter()
            .any(|task| task.status().is_active())
        {
            return Err(CoreError::TaskHasUnfinishedChildren);
        }

        self.update_status(task_id, Status::Done, at).await
    }

    /// Marks a task as [`Status::Abandoned`].
    ///
    /// # Errors
    ///
    /// - [`CoreError::TaskNotFound`] if no task exists with the given id.
    /// - Returns an error if the persistence operation fails.
    pub async fn abandon(&self, task_id: &TaskId, at: Option<DateTime<Utc>>) -> CoreResult<()> {
        self.update_status(task_id, Status::Abandoned, at).await
    }

    /// Marks a task as [`Status::Blocked`] and cascades to all active descendants.
    ///
    /// # Errors
    ///
    /// - [`CoreError::TaskNotFound`] if no task exists with the given id.
    /// - [`CoreError::InvalidStatusTransition`] if the transition is not allowed.
    /// - Returns an error if the persistence operation fails.
    pub async fn block(&self, task_id: &TaskId, at: Option<DateTime<Utc>>) -> CoreResult<()> {
        let tx = self.repo.begin_transaction().await?;

        self.update_status(task_id, Status::Blocked, at).await?;
        self.block_children(task_id, at).await?;

        tx.commit_transaction().await
    }

    /// Moves a task under a new parent, or promotes it to root if `parent_id` is `None`.
    ///
    /// # Errors
    ///
    /// - [`CoreError::TaskNotFound`] if the task does not exist.
    /// - [`CoreError::MaxDepthExceeded`] if the move would exceed nesting depth of 3.
    /// - Returns an error if the persistence operation fails.
    pub async fn move_to_parent(
        &self,
        task_id: &TaskId,
        new_parent_id: Option<TaskId>,
        project_id: ProjectId,
    ) -> CoreResult<()> {
        self.check_cycle(task_id, new_parent_id).await?;
        self.check_depth(new_parent_id, self.subtree_depth(task_id).await?)
            .await?;

        let mut task = self.find_task(task_id).await?;
        task.parent = new_parent_id;

        let siblings = self
            .repo
            .list_tasks(&task.siblings_filter(project_id))
            .await?;
        let next_order = siblings
            .iter()
            .filter(|task| task.id != *task_id)
            .filter_map(|task| task.order)
            .max()
            .map_or(0, |max| max + 1);
        task.order = Some(next_order);

        self.repo.save_task(&task).await
    }

    /// Sets or clears the pull request number for a task.
    ///
    /// # Errors
    ///
    /// - [`CoreError::TaskNotFound`] if no task exists with the given id.
    /// - Returns an error if the persistence operation fails.
    pub async fn set_pull_request(
        &self,
        task_id: &TaskId,
        pr: Option<u32>,
        branch: Option<String>,
    ) -> CoreResult<()> {
        let mut task = self.find_task(task_id).await?;
        task.pull_request_number = pr;
        task.branch_name = branch;
        self.repo.save_task(&task).await
    }

    /// Renames a task.
    ///
    /// # Errors
    ///
    /// - [`CoreError::TaskNotFound`] if no task exists with the given id.
    /// - Returns an error if the persistence operation fails.
    pub async fn rename(&self, task_id: &TaskId, title: &str) -> CoreResult<()> {
        let mut task = self.find_task(task_id).await?;

        title.clone_into(&mut task.title);
        self.repo.save_task(&task).await
    }

    /// Changes the display order of a task among its siblings.
    ///
    /// # Errors
    ///
    /// - [`CoreError::TaskNotFound`] if no task exists with the given id.
    /// - Returns an error if the persistence operation fails.
    pub async fn reorder(
        &self,
        task_id: &TaskId,
        new_order: usize,
        siblings: &mut [Task],
    ) -> CoreResult<()> {
        let tx = self.repo.begin_transaction().await?;

        let mut task = self.find_task(task_id).await?;
        task.order = Some(new_order);
        self.repo.save_task(&task).await?;

        for sibling in siblings.iter_mut() {
            if sibling.id == *task_id || sibling.parent != task.parent {
                continue;
            }
            if let Some(ref mut sibling_order) = sibling.order
                && *sibling_order >= new_order
            {
                *sibling_order += 1;
                self.repo.save_task(sibling).await?;
            }
        }

        tx.commit_transaction().await
    }

    /// Swaps the display order of two sibling tasks.
    ///
    /// # Errors
    ///
    /// - [`CoreError::TaskNotFound`] if either task does not exist.
    /// - Returns an error if the persistence operation fails.
    pub async fn swap_order(
        &self,
        id_a: &TaskId,
        order_a: usize,
        id_b: &TaskId,
        order_b: usize,
    ) -> CoreResult<()> {
        let tx = self.repo.begin_transaction().await?;

        let mut task_a = self.find_task(id_a).await?;
        let mut task_b = self.find_task(id_b).await?;
        task_a.order = Some(order_b);
        task_b.order = Some(order_a);
        self.repo.save_task(&task_a).await?;
        self.repo.save_task(&task_b).await?;

        tx.commit_transaction().await
    }

    /// Resolves a hex id prefix to a full [`TaskId`].
    ///
    /// # Errors
    ///
    /// - [`CoreError::TaskPrefixNotFound`] if no task matches the prefix.
    /// - Returns an error if the persistence operation fails.
    pub async fn find_by_prefix(&self, id_prefix: &str) -> CoreResult<TaskId> {
        self.repo
            .find_task_by_id_prefix(id_prefix)
            .await?
            .ok_or_else(|| CoreError::TaskPrefixNotFound {
                prefix: id_prefix.to_owned(),
            })
    }

    /// Returns all tasks in a project.
    ///
    /// # Errors
    /// Returns an error if the persistence operation fails.
    pub async fn list(&self, filter: &TaskFilter) -> CoreResult<Vec<Task>> {
        self.repo.list_tasks(filter).await
    }

    /// Returns the full status change history for a task, ordered by `changed_at`.
    ///
    /// # Errors
    /// Returns an error if the persistence operation fails.
    pub async fn status_history(&self, task_id: &TaskId) -> CoreResult<Vec<StatusChange>> {
        self.repo.list_task_changes(task_id).await
    }

    /// Deletes a task and all its children recursively.
    ///
    /// Idempotent - returns `Ok(())` if the task does not exist.
    ///
    /// # Errors
    /// Returns an error if the persistence operation fails.
    pub async fn delete(&self, task_id: &TaskId) -> CoreResult<()> {
        self.repo.delete_task(task_id).await
    }

    /// Finds a task by id or returns [`CoreError::TaskNotFound`].
    ///
    /// # Errors
    /// Returns an error if the persistence operation fails.
    pub async fn find_task(&self, id: &TaskId) -> CoreResult<Task> {
        self.repo
            .find_task_by_id(id)
            .await?
            .ok_or(CoreError::TaskNotFound { id: *id })
    }

    // -------------------------------------------------------------------------

    /// Updates the status of a task.
    ///
    /// # Errors
    /// - [`CoreError::TaskNotFound`] if no task exists with the given id.
    /// - [`CoreError::InvalidStatusTransition`] if the transition is not allowed.
    /// - Returns an error if the persistence operation fails.
    async fn update_status(
        &self,
        task_id: &TaskId,
        new_status: Status,
        at: Option<DateTime<Utc>>,
    ) -> CoreResult<()> {
        let mut task = self.find_task(task_id).await?;
        self.update_status_inner(&mut task, new_status, at).await
    }

    /// Applies a status transition to a task, saves it and records the change.
    async fn update_status_inner(
        &self,
        task: &mut Task,
        new_status: Status,
        at: Option<DateTime<Utc>>,
    ) -> CoreResult<()> {
        let old_status = task.status();
        let status_change = StatusChange::new(task.id, old_status, new_status, at);
        task.update_status(new_status, at)?;
        let tx = self.repo.begin_transaction().await?;

        self.repo.save_task(task).await?;
        if at.is_some() {
            self.repo
                .delete_task_changes_after(&task.id, status_change.changed_at)
                .await?;
        }
        self.repo.save_task_change(&status_change).await?;

        tx.commit_transaction().await
    }

    /// Recursively blocks all active descendants of the given task.
    async fn block_children(
        &self,
        parent_id: &TaskId,
        at: Option<DateTime<Utc>>,
    ) -> CoreResult<()> {
        for mut child in self.repo.child_tasks_of(parent_id).await? {
            if child.status().is_active() {
                self.update_status_inner(&mut child, Status::Blocked, at)
                    .await?;
                Box::pin(self.block_children(&child.id, at)).await?;
            }
        }
        Ok(())
    }

    /// Returns the maximum depth among all descendants of a task (0 if no children).
    async fn subtree_depth(&self, task_id: &TaskId) -> CoreResult<usize> {
        let children = self.repo.child_tasks_of(task_id).await?;
        if children.is_empty() {
            return Ok(0);
        }
        let mut max = 0;
        for child in &children {
            max = max.max(1 + Box::pin(self.subtree_depth(&child.id)).await?);
        }
        Ok(max)
    }

    /// Returns the depth of a task (1 for root, 2 for child of root, etc.).
    async fn depth_of(&self, task_id: TaskId) -> CoreResult<usize> {
        let mut depth = 1_usize;
        let mut current = task_id;
        while let Some(task) = self.repo.find_task_by_id(&current).await? {
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
    async fn check_depth(&self, parent: Option<TaskId>, extra_depth: usize) -> CoreResult<()> {
        let base = match parent {
            Some(id) => self.depth_of(id).await?,
            None => 0,
        };
        if base + 1 + extra_depth > MAX_DEPTH {
            return Err(CoreError::MaxDepthExceeded);
        }
        Ok(())
    }

    /// Ensures that `parent` is not a descendant of `task_id`.
    async fn check_cycle(&self, task_id: &TaskId, parent: Option<TaskId>) -> CoreResult<()> {
        let Some(pid) = parent else {
            return Ok(());
        };

        let mut current = pid;
        while let Some(ancestor) = self.repo.find_task_by_id(&current).await? {
            if ancestor.id == *task_id {
                return Err(CoreError::CyclicParentage);
            }
            match ancestor.parent {
                Some(id) => current = id,
                None => break,
            }
        }

        Ok(())
    }
}
