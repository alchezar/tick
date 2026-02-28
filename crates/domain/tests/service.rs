//! Integration tests for `TaskService` business rules.

mod common;

use domain::{
    error::CoreError,
    model::{Project, Status},
};

#[test]
fn done_fails_with_active_child() {
    let service = common::task_service();
    let project = Project::default();
    let parent = service.create("Parent", None, project.id).unwrap();
    service
        .create("Child", Some(&parent.id), project.id)
        .unwrap();

    let err = service.done(&parent.id).unwrap_err();
    assert!(matches!(err, CoreError::TaskHasUnfinishedChildren));
}

#[test]
fn block_cascades_to_children() {
    let service = common::task_service();
    let project = Project::default();
    let parent = service.create("Parent", None, project.id).unwrap();
    let child = service
        .create("Child", Some(&parent.id), project.id)
        .unwrap();
    service.start(&child.id).unwrap();
    service.start(&parent.id).unwrap();

    service.block(&parent.id).unwrap();

    // If cascade worked, child is now Blocked - start(child) must succeed.
    // If cascade did not work, child is still InProgress - start(child) would fail.
    service.start(&child.id).unwrap();
}

#[test]
fn create_exceeds_max_depth() {
    let service = common::task_service();
    let project = Project::default();
    let l0 = service.create("Root", None, project.id).unwrap();
    let l1 = service.create("Level 1", Some(&l0.id), project.id).unwrap();
    let l2 = service.create("Level 2", Some(&l1.id), project.id).unwrap();
    let l3 = service.create("Level 3", Some(&l2.id), project.id).unwrap();

    let err = service
        .create("Level 4", Some(&l3.id), project.id)
        .unwrap_err();
    assert!(matches!(err, CoreError::MaxDepthExceeded));
}

#[test]
fn create_assigns_order_sequentially() {
    let project = Project::default();
    let service = common::task_service();
    let a = service.create("A", None, project.id).unwrap();
    let b = service.create("B", None, project.id).unwrap();
    let c = service.create("C", None, project.id).unwrap();

    assert_eq!(a.order, Some(0));
    assert_eq!(b.order, Some(1));
    assert_eq!(c.order, Some(2));
}

#[test]
fn done_succeeds_without_children() {
    let service = common::task_service();
    let task = service.create("Task", None, Project::default().id).unwrap();
    service.start(&task.id).unwrap();

    service.done(&task.id).unwrap();
}

#[test]
fn start_fails_if_already_in_progress() {
    let service = common::task_service();
    let task = service.create("Task", None, Project::default().id).unwrap();
    service.start(&task.id).unwrap();

    let err = service.start(&task.id).unwrap_err();
    assert!(matches!(err, CoreError::InvalidStatusTransition { .. }));
}

#[test]
fn reset_from_done() {
    let service = common::task_service();
    let task = service.create("Task", None, Project::default().id).unwrap();
    service.start(&task.id).unwrap();
    service.done(&task.id).unwrap();

    // Done -> NotStarted must succeed
    service.reset(&task.id).unwrap();
}

#[test]
fn status_change_recorded_on_transition() {
    let service = common::task_service();
    let task = service.create("Task", None, Project::default().id).unwrap();

    service.start(&task.id).unwrap();

    let history = service.status_history(&task.id).unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].task_id, task.id);
    assert_eq!(history[0].old_status, Status::NotStarted);
    assert_eq!(history[0].new_status, Status::InProgress);
}

#[test]
fn status_change_full_lifecycle() {
    let service = common::task_service();
    let task = service.create("Task", None, Project::default().id).unwrap();

    service.start(&task.id).unwrap();
    service.done(&task.id).unwrap();
    service.reset(&task.id).unwrap();

    let history = service.status_history(&task.id).unwrap();
    assert_eq!(history.len(), 3);

    assert_eq!(history[0].old_status, Status::NotStarted);
    assert_eq!(history[0].new_status, Status::InProgress);

    assert_eq!(history[1].old_status, Status::InProgress);
    assert_eq!(history[1].new_status, Status::Done);

    assert_eq!(history[2].old_status, Status::Done);
    assert_eq!(history[2].new_status, Status::NotStarted);
}

#[test]
fn block_cascade_records_changes_for_children() {
    let service = common::task_service();
    let project = Project::default();
    let parent = service.create("Parent", None, project.id).unwrap();
    let child = service
        .create("Child", Some(&parent.id), project.id)
        .unwrap();
    service.start(&parent.id).unwrap();
    service.start(&child.id).unwrap();

    service.block(&parent.id).unwrap();

    let parent_history = service.status_history(&parent.id).unwrap();
    let child_history = service.status_history(&child.id).unwrap();

    // parent: NotStarted -> InProgress, InProgress -> Blocked
    assert_eq!(parent_history.len(), 2);
    assert_eq!(parent_history[1].old_status, Status::InProgress);
    assert_eq!(parent_history[1].new_status, Status::Blocked);

    // child: NotStarted -> InProgress, InProgress -> Blocked (cascade)
    assert_eq!(child_history.len(), 2);
    assert_eq!(child_history[1].old_status, Status::InProgress);
    assert_eq!(child_history[1].new_status, Status::Blocked);
}
