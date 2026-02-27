//! Fake implementations of repository traits for use in integration tests.

use core::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use chrono::NaiveDate;
use uuid::Uuid;

use domain::{
    error::CoreResult,
    model::{StatusChange, Task},
    repository::TaskRepository,
};

/// In-memory implementation of `TaskRepository` for use in tests.
///
/// Clone is cheap — all clones share the same underlying data via `Rc`.
#[derive(Debug, Default, Clone)]
pub struct FakeRepo {
    tasks: Rc<RefCell<HashMap<Uuid, Task>>>,
    status_changes: Rc<RefCell<Vec<StatusChange>>>,
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
    fn list_all(&self) -> CoreResult<Vec<Task>> {
        Ok(self.tasks.borrow().values().cloned().collect())
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

    #[inline]
    fn save_status_change(&self, change: &StatusChange) -> CoreResult<()> {
        self.status_changes.borrow_mut().push(change.clone());
        Ok(())
    }

    #[inline]
    fn list_status_changes(&self, task_id: &Uuid) -> CoreResult<Vec<StatusChange>> {
        Ok(self
            .status_changes
            .borrow()
            .iter()
            .filter(|c| &c.task_id == task_id)
            .cloned()
            .collect())
    }

    #[inline]
    fn list_status_changes_on(&self, date: NaiveDate) -> CoreResult<Vec<StatusChange>> {
        Ok(self
            .status_changes
            .borrow()
            .iter()
            .filter(|c| c.changed_at.date_naive() == date)
            .cloned()
            .collect())
    }
}
