//! Integration tests for the `Task` domain entity.

use uuid::Uuid;

use domain::model::{Project, Status, Task};

#[test]
fn new_task_has_correct_defaults() {
    let task = Task::new("Write tests", None, Project::default().id);

    assert_eq!(task.title, "Write tests");
    assert_eq!(task.status(), Status::NotStarted);
    assert!(task.parent.is_none());
    assert!(task.order.is_none());
}

#[test]
fn is_root_without_parent() {
    let task = Task::new("Root task", None, Project::default().id);
    assert!(task.is_root());
}

#[test]
fn is_not_root_with_parent() {
    let parent_id = Uuid::new_v4();
    let task = Task::new("Child task", Some(parent_id), Project::default().id);
    assert!(!task.is_root());
}
