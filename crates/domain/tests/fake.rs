//! Fake implementations of repository traits for use in integration tests.

use core::cell::RefCell;
use std::collections::HashMap;

use chrono::NaiveDate;
use uuid::Uuid;

use domain::{error::CoreResult, model::Task, repository::TaskRepository};

/// In-memory implementation of `TaskRepository` for use in tests.
#[derive(Debug, Default)]
pub struct FakeRepo {
    tasks: RefCell<HashMap<Uuid, Task>>,
}

impl TaskRepository for FakeRepo {
    #[inline]
    fn save(&self, task: &Task) -> CoreResult<()> {
        self.tasks.borrow_mut().insert(task.id, task.clone());
        Ok(())
    }

    #[inline]
    fn find_by_id(&self, id: &Uuid) -> CoreResult<Option<Task>> {
        Ok(self.tasks.borrow().get(id).cloned())
    }

    #[inline]
    fn children_of(&self, parent: &Uuid) -> CoreResult<Vec<Task>> {
        Ok(self
            .tasks
            .borrow()
            .values()
            .filter(|t| t.parent == Some(*parent))
            .cloned()
            .collect())
    }

    #[inline]
    fn list_active(&self) -> CoreResult<Vec<Task>> {
        Ok(self
            .tasks
            .borrow()
            .values()
            .filter(|t| t.status.is_active())
            .cloned()
            .collect())
    }

    #[inline]
    fn list_all(&self) -> CoreResult<Vec<Task>> {
        Ok(self.tasks.borrow().values().cloned().collect())
    }

    #[inline]
    fn list_updated_on(&self, date: NaiveDate) -> CoreResult<Vec<Task>> {
        Ok(self
            .tasks
            .borrow()
            .values()
            .filter(|t| t.updated.date_naive() == date)
            .cloned()
            .collect())
    }

    #[inline]
    fn delete(&self, id: &Uuid) -> CoreResult<()> {
        let children: Vec<Uuid> = self
            .tasks
            .borrow()
            .values()
            .filter(|t| t.parent == Some(*id))
            .map(|t| t.id)
            .collect();
        for child_id in children {
            self.delete(&child_id)?;
        }
        self.tasks.borrow_mut().remove(id);
        Ok(())
    }
}
