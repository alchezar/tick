//! Fake implementations of repository traits for use in integration tests.

use core::cell::RefCell;
use std::{collections::HashMap, rc::Rc};

use chrono::NaiveDate;
use uuid::Uuid;

use domain::{
    error::CoreResult,
    model::{Project, StatusChange, Task},
    repository::{ProjectRepository, TaskRepository, TransactionGuard, Transactional},
};

/// In-memory implementation of repository traits for use in tests.
///
/// Clone is cheap - all clones share the same underlying data via `Rc`.
#[derive(Debug, Default, Clone)]
pub struct FakeRepo {
    projects: Rc<RefCell<HashMap<Uuid, Project>>>,
    tasks: Rc<RefCell<HashMap<Uuid, Task>>>,
    status_changes: Rc<RefCell<Vec<StatusChange>>>,
}

pub struct FakeGuard;

impl TransactionGuard for FakeGuard {
    async fn commit_transaction(self) -> CoreResult<()> {
        Ok(())
    }
}

impl Transactional for FakeRepo {
    type Guard<'a> = FakeGuard;
    async fn begin_transaction(&self) -> CoreResult<Self::Guard<'_>> {
        Ok(FakeGuard)
    }
}

impl ProjectRepository for FakeRepo {
    async fn save_project(&self, project: &Project) -> CoreResult<()> {
        self.projects
            .borrow_mut()
            .insert(project.id, project.clone());
        Ok(())
    }

    async fn find_project_by_id(&self, id: &Uuid) -> CoreResult<Option<Project>> {
        Ok(self.projects.borrow().get(id).cloned())
    }

    async fn find_project_by_slug(&self, slug: &str) -> CoreResult<Option<Project>> {
        Ok(self
            .projects
            .borrow()
            .values()
            .find(|project| project.slug == slug)
            .cloned())
    }

    async fn list_projects(&self) -> CoreResult<Vec<Project>> {
        Ok(self.projects.borrow().values().cloned().collect())
    }

    async fn delete_project(&self, project_id: &Uuid) -> CoreResult<()> {
        let task_ids = self
            .tasks
            .borrow()
            .values()
            .filter(|task| task.project_id == *project_id)
            .map(|task| task.id)
            .collect::<Vec<_>>();
        self.status_changes
            .borrow_mut()
            .retain(|change| !task_ids.contains(&change.task_id));
        self.tasks
            .borrow_mut()
            .retain(|_, task| task.project_id != *project_id);
        self.projects.borrow_mut().remove(project_id);
        Ok(())
    }
}

impl TaskRepository for FakeRepo {
    async fn save_task(&self, task: &Task) -> CoreResult<()> {
        self.tasks.borrow_mut().insert(task.id, task.clone());
        Ok(())
    }

    async fn find_task_by(&self, id: &Uuid) -> CoreResult<Option<Task>> {
        Ok(self.tasks.borrow().get(id).cloned())
    }

    async fn child_tasks_of(&self, parent: &Uuid) -> CoreResult<Vec<Task>> {
        Ok(self
            .tasks
            .borrow()
            .values()
            .filter(|task| task.parent == Some(*parent))
            .cloned()
            .collect())
    }

    async fn list_tasks(&self, project_id: &Uuid) -> CoreResult<Vec<Task>> {
        Ok(self
            .tasks
            .borrow()
            .values()
            .filter(|task| task.project_id == *project_id)
            .cloned()
            .collect())
    }

    async fn delete_task(&self, id: &Uuid) -> CoreResult<()> {
        let children = self
            .tasks
            .borrow()
            .values()
            .filter(|task| task.parent == Some(*id))
            .map(|task| task.id)
            .collect::<Vec<_>>();
        for child_id in children {
            Box::pin(self.delete_task(&child_id)).await?;
        }
        self.status_changes
            .borrow_mut()
            .retain(|change| &change.task_id != id);
        self.tasks.borrow_mut().remove(id);
        Ok(())
    }

    async fn delete_all_tasks_by(&self, project_id: &Uuid) -> CoreResult<()> {
        self.tasks
            .borrow_mut()
            .retain(|_, task| task.project_id != *project_id);
        Ok(())
    }

    async fn save_task_change(&self, change: &StatusChange) -> CoreResult<()> {
        self.status_changes.borrow_mut().push(change.clone());
        Ok(())
    }

    async fn list_task_changes(&self, task_id: &Uuid) -> CoreResult<Vec<StatusChange>> {
        Ok(self
            .status_changes
            .borrow()
            .iter()
            .filter(|change| &change.task_id == task_id)
            .cloned()
            .collect())
    }

    async fn list_task_changes_on(&self, date: NaiveDate) -> CoreResult<Vec<StatusChange>> {
        Ok(self
            .status_changes
            .borrow()
            .iter()
            .filter(|change| change.changed_at.date_naive() == date)
            .cloned()
            .collect())
    }
}
