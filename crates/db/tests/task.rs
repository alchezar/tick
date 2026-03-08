//! Integration tests for `TaskRepository` on `SqliteRepo`.

mod common;

use chrono::NaiveDate;
use domain::{
    model::{Project, Status, StatusChange, Task},
    repository::{ProjectRepository, TaskRepository},
};

#[test]
fn save_and_find_task() {
    let (repo, project) = common::repo_with_project();
    let task = Task::new("Fix bug", None, project.id);
    repo.save_task(&task).unwrap();

    let found = repo.find_task_by(&task.id).unwrap().unwrap();
    assert_eq!(found.id, task.id);
    assert_eq!(found.title, "Fix bug");
    assert_eq!(found.status(), Status::NotStarted);
    assert!(found.parent.is_none());
}

#[test]
fn find_task_returns_none() {
    let (repo, _) = common::repo_with_project();
    assert!(repo.find_task_by(&uuid::Uuid::new_v4()).unwrap().is_none());
}

#[test]
fn save_updates_task() {
    let (repo, project) = common::repo_with_project();
    let task = Task::new("Old title", None, project.id);
    repo.save_task(&task).unwrap();

    let mut updated = task.clone().with_status(Status::InProgress);
    updated.title = "New title".to_owned();
    repo.save_task(&updated).unwrap();

    let found = repo.find_task_by(&task.id).unwrap().unwrap();
    assert_eq!(found.title, "New title");
    assert_eq!(found.status(), Status::InProgress);
}

#[test]
fn child_tasks_of() {
    let (repo, project) = common::repo_with_project();
    let parent = Task::new("Parent", None, project.id);
    repo.save_task(&parent).unwrap();

    let mut child1 = Task::new("Child 1", Some(parent.id), project.id);
    child1.order = Some(0);
    repo.save_task(&child1).unwrap();

    let mut child2 = Task::new("Child 2", Some(parent.id), project.id);
    child2.order = Some(1);
    repo.save_task(&child2).unwrap();

    let children = repo.child_tasks_of(&parent.id).unwrap();
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].title, "Child 1");
    assert_eq!(children[1].title, "Child 2");
}

#[test]
fn child_tasks_of_empty() {
    let (repo, project) = common::repo_with_project();
    let task = Task::new("Leaf", None, project.id);
    repo.save_task(&task).unwrap();

    assert!(repo.child_tasks_of(&task.id).unwrap().is_empty());
}

#[test]
fn list_tasks_by_project() {
    let (repo, project) = common::repo_with_project();
    let other = Project::new("other", None::<String>);
    repo.save_project(&other).unwrap();

    repo.save_task(&Task::new("Work task", None, project.id))
        .unwrap();
    repo.save_task(&Task::new("Other task", None, other.id))
        .unwrap();

    let work_tasks = repo.list_tasks(&project.id).unwrap();
    assert_eq!(work_tasks.len(), 1);
    assert_eq!(work_tasks[0].title, "Work task");
}

#[test]
fn delete_task_removes_it() {
    let (repo, project) = common::repo_with_project();
    let task = Task::new("Task", None, project.id);
    repo.save_task(&task).unwrap();

    repo.delete_task(&task.id).unwrap();
    assert!(repo.find_task_by(&task.id).unwrap().is_none());
}

#[test]
fn delete_task_cascades_children() {
    let (repo, project) = common::repo_with_project();
    let parent = Task::new("Parent", None, project.id);
    repo.save_task(&parent).unwrap();

    let child = Task::new("Child", Some(parent.id), project.id);
    repo.save_task(&child).unwrap();

    repo.delete_task(&parent.id).unwrap();
    assert!(repo.find_task_by(&child.id).unwrap().is_none());
}

#[test]
fn delete_nonexistent_task_is_ok() {
    let (repo, _) = common::repo_with_project();
    repo.delete_task(&uuid::Uuid::new_v4()).unwrap();
}

#[test]
fn delete_all_tasks_by_project() {
    let (repo, project) = common::repo_with_project();
    repo.save_task(&Task::new("A", None, project.id)).unwrap();
    repo.save_task(&Task::new("B", None, project.id)).unwrap();

    repo.delete_all_tasks_by(&project.id).unwrap();
    assert!(repo.list_tasks(&project.id).unwrap().is_empty());
}

#[test]
fn save_and_list_status_changes() {
    let (repo, project) = common::repo_with_project();
    let task = Task::new("Task", None, project.id);
    repo.save_task(&task).unwrap();

    let change = StatusChange::new(task.id, Status::NotStarted, Status::InProgress);
    repo.save_task_change(&change).unwrap();

    let changes = repo.list_task_changes(&task.id).unwrap();
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].old_status, Status::NotStarted);
    assert_eq!(changes[0].new_status, Status::InProgress);
}

#[test]
fn list_task_changes_on_date() {
    let (repo, project) = common::repo_with_project();
    let task = Task::new("Task", None, project.id);
    repo.save_task(&task).unwrap();

    let date = NaiveDate::from_ymd_opt(2026, 3, 8).unwrap();
    let dt = date.and_hms_opt(10, 0, 0).unwrap().and_utc();

    let mut change = StatusChange::new(task.id, Status::NotStarted, Status::InProgress);
    change.changed_at = dt;
    repo.save_task_change(&change).unwrap();

    let on_date = repo.list_task_changes_on(date).unwrap();
    assert_eq!(on_date.len(), 1);

    let other_date = NaiveDate::from_ymd_opt(2026, 3, 9).unwrap();
    assert!(repo.list_task_changes_on(other_date).unwrap().is_empty());
}

#[test]
fn task_order_preserved() {
    let (repo, project) = common::repo_with_project();
    let mut t1 = Task::new("First", None, project.id);
    t1.order = Some(0);
    let mut t2 = Task::new("Second", None, project.id);
    t2.order = Some(1);
    let mut t3 = Task::new("Third", None, project.id);
    t3.order = Some(2);

    repo.save_task(&t3).unwrap();
    repo.save_task(&t1).unwrap();
    repo.save_task(&t2).unwrap();

    let tasks = repo.list_tasks(&project.id).unwrap();
    assert_eq!(tasks[0].title, "First");
    assert_eq!(tasks[1].title, "Second");
    assert_eq!(tasks[2].title, "Third");
}

#[test]
fn delete_project_cascades_tasks_and_changes() {
    let (repo, project) = common::repo_with_project();
    let task = Task::new("Task", None, project.id);
    repo.save_task(&task).unwrap();

    let change = StatusChange::new(task.id, Status::NotStarted, Status::InProgress);
    repo.save_task_change(&change).unwrap();

    repo.delete_project(&project.id).unwrap();

    assert!(repo.find_task_by(&task.id).unwrap().is_none());
    assert!(repo.list_task_changes(&task.id).unwrap().is_empty());
}
