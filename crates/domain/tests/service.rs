//! Integration tests for `TaskService` business rules.

mod common;

use domain::{
    error::CoreError,
    model::{Project, Status},
    repository::TaskRepository,
    service::TaskService,
};

use common::fake::FakeRepo;

#[tokio::test]
async fn done_fails_with_active_child() {
    let service = common::task_service();
    let project = Project::default();
    let parent = service
        .create("Parent", None, project.id, None)
        .await
        .unwrap();
    service
        .create("Child", Some(&parent.id), project.id, None)
        .await
        .unwrap();

    let err = service.done(&parent.id, None).await.unwrap_err();
    assert!(matches!(err, CoreError::TaskHasUnfinishedChildren));
}

#[tokio::test]
async fn block_cascades_to_children() {
    let service = common::task_service();
    let project = Project::default();
    let parent = service
        .create("Parent", None, project.id, None)
        .await
        .unwrap();
    let child = service
        .create("Child", Some(&parent.id), project.id, None)
        .await
        .unwrap();
    service.start(&child.id, None).await.unwrap();
    service.start(&parent.id, None).await.unwrap();

    service.block(&parent.id, None).await.unwrap();

    // If cascade worked, child is now Blocked - start(child) must succeed.
    // If cascade did not work, child is still InProgress - start(child) would fail.
    service.start(&child.id, None).await.unwrap();
}

#[tokio::test]
async fn create_exceeds_max_depth() {
    let service = common::task_service();
    let project = Project::default();
    let l0 = service
        .create("Root", None, project.id, None)
        .await
        .unwrap();
    let l1 = service
        .create("Level 1", Some(&l0.id), project.id, None)
        .await
        .unwrap();
    let l2 = service
        .create("Level 2", Some(&l1.id), project.id, None)
        .await
        .unwrap();
    let l3 = service
        .create("Level 3", Some(&l2.id), project.id, None)
        .await
        .unwrap();

    // 5th level (l0 -> l1 -> l2 -> l3 -> l4) must be rejected
    let err = service
        .create("Level 4", Some(&l3.id), project.id, None)
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::MaxDepthExceeded));
}

#[tokio::test]
async fn create_assigns_order_sequentially() {
    let project = Project::default();
    let service = common::task_service();
    let a = service.create("A", None, project.id, None).await.unwrap();
    let b = service.create("B", None, project.id, None).await.unwrap();
    let c = service.create("C", None, project.id, None).await.unwrap();

    assert_eq!(a.order, Some(0));
    assert_eq!(b.order, Some(1));
    assert_eq!(c.order, Some(2));
}

#[tokio::test]
async fn done_succeeds_without_children() {
    let service = common::task_service();
    let task = service
        .create("Task", None, Project::default().id, None)
        .await
        .unwrap();
    service.start(&task.id, None).await.unwrap();

    service.done(&task.id, None).await.unwrap();
}

#[tokio::test]
async fn start_fails_if_already_in_progress() {
    let service = common::task_service();
    let task = service
        .create("Task", None, Project::default().id, None)
        .await
        .unwrap();
    service.start(&task.id, None).await.unwrap();

    let err = service.start(&task.id, None).await.unwrap_err();
    assert!(matches!(err, CoreError::InvalidStatusTransition { .. }));
}

#[tokio::test]
async fn reset_from_done() {
    let service = common::task_service();
    let task = service
        .create("Task", None, Project::default().id, None)
        .await
        .unwrap();
    service.start(&task.id, None).await.unwrap();
    service.done(&task.id, None).await.unwrap();

    // Done -> NotStarted must succeed
    service.reset(&task.id, None).await.unwrap();
}

#[tokio::test]
async fn status_change_recorded_on_transition() {
    let service = common::task_service();
    let task = service
        .create("Task", None, Project::default().id, None)
        .await
        .unwrap();

    service.start(&task.id, None).await.unwrap();

    let history = service.status_history(&task.id).await.unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].task_id, task.id);
    assert_eq!(history[0].old_status, Status::NotStarted);
    assert_eq!(history[0].new_status, Status::InProgress);
}

#[tokio::test]
async fn status_change_full_lifecycle() {
    let service = common::task_service();
    let task = service
        .create("Task", None, Project::default().id, None)
        .await
        .unwrap();

    service.start(&task.id, None).await.unwrap();
    service.done(&task.id, None).await.unwrap();
    service.reset(&task.id, None).await.unwrap();

    let history = service.status_history(&task.id).await.unwrap();
    assert_eq!(history.len(), 3);

    assert_eq!(history[0].old_status, Status::NotStarted);
    assert_eq!(history[0].new_status, Status::InProgress);

    assert_eq!(history[1].old_status, Status::InProgress);
    assert_eq!(history[1].new_status, Status::Done);

    assert_eq!(history[2].old_status, Status::Done);
    assert_eq!(history[2].new_status, Status::NotStarted);
}

#[tokio::test]
async fn block_cascade_records_changes_for_children() {
    let service = common::task_service();
    let project = Project::default();
    let parent = service
        .create("Parent", None, project.id, None)
        .await
        .unwrap();
    let child = service
        .create("Child", Some(&parent.id), project.id, None)
        .await
        .unwrap();
    service.start(&parent.id, None).await.unwrap();
    service.start(&child.id, None).await.unwrap();

    service.block(&parent.id, None).await.unwrap();

    let parent_history = service.status_history(&parent.id).await.unwrap();
    let child_history = service.status_history(&child.id).await.unwrap();

    // parent: NotStarted -> InProgress, InProgress -> Blocked
    assert_eq!(parent_history.len(), 2);
    assert_eq!(parent_history[1].old_status, Status::InProgress);
    assert_eq!(parent_history[1].new_status, Status::Blocked);

    // child: NotStarted -> InProgress, InProgress -> Blocked (cascade)
    assert_eq!(child_history.len(), 2);
    assert_eq!(child_history[1].old_status, Status::InProgress);
    assert_eq!(child_history[1].new_status, Status::Blocked);
}

#[tokio::test]
async fn create_at_fourth_level_fails() {
    // SPEC: "up to 4 levels of nesting"
    let service = common::task_service();
    let project = Project::default();
    let l0 = service
        .create("Task", None, project.id, None)
        .await
        .unwrap();
    let l1 = service
        .create("Subtask", Some(&l0.id), project.id, None)
        .await
        .unwrap();
    let l2 = service
        .create("Sub-subtask", Some(&l1.id), project.id, None)
        .await
        .unwrap();
    let l3 = service
        .create("Sub-sub-subtask", Some(&l2.id), project.id, None)
        .await
        .unwrap();

    // 5th level must be rejected
    let result = service
        .create("Too deep", Some(&l3.id), project.id, None)
        .await;
    assert!(matches!(result, Err(CoreError::MaxDepthExceeded)));
}

#[tokio::test]
async fn move_subtree_exceeds_depth() {
    let service = common::task_service();
    let project = Project::default();

    // Tree 1: a -> b -> c (3 levels)
    let a = service.create("A", None, project.id, None).await.unwrap();
    let b = service
        .create("B", Some(&a.id), project.id, None)
        .await
        .unwrap();
    let _c = service
        .create("C", Some(&b.id), project.id, None)
        .await
        .unwrap();

    // Tree 2: x -> y (2 levels)
    let x = service.create("X", None, project.id, None).await.unwrap();
    let y = service
        .create("Y", Some(&x.id), project.id, None)
        .await
        .unwrap();

    // Move A under Y: x -> y -> a -> b -> c = 5 levels, exceeds max 4
    let result = service.move_to_parent(&a.id, Some(&y.id)).await;
    assert!(matches!(result, Err(CoreError::MaxDepthExceeded)));
}

#[tokio::test]
async fn create_order_no_duplicates_after_delete() {
    let repo = FakeRepo::default();
    let task_svc = TaskService::new(repo);
    let project = Project::default();

    let _a = task_svc.create("A", None, project.id, None).await.unwrap(); // order 0
    let b = task_svc.create("B", None, project.id, None).await.unwrap(); // order 1
    let c = task_svc.create("C", None, project.id, None).await.unwrap(); // order 2

    task_svc.delete(&b.id).await.unwrap();

    let d = task_svc.create("D", None, project.id, None).await.unwrap();
    // d.order must not collide with c.order
    assert_ne!(d.order, c.order);
}

#[tokio::test]
async fn delete_task_cleans_status_changes() {
    let repo = FakeRepo::default();
    let task_svc = TaskService::new(repo.clone());
    let project = Project::default();

    let task = task_svc
        .create("Task", None, project.id, None)
        .await
        .unwrap();
    task_svc.start(&task.id, None).await.unwrap();

    task_svc.delete(&task.id).await.unwrap();

    let changes = repo.list_task_changes(&task.id).await.unwrap();
    assert!(changes.is_empty());
}
