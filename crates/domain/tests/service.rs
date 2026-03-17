//! Integration tests for `TaskService` business rules.

mod common;

use domain::{
    error::CoreError,
    model::{Project, Status},
    repository::{TaskFilter, TaskRepository},
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

#[tokio::test]
async fn abandon_from_any_status() {
    let service = common::task_service();
    let project = Project::default();

    // abandon from not_started
    let t1 = service.create("T1", None, project.id, None).await.unwrap();
    service.abandon(&t1.id, None).await.unwrap();

    // abandon from in_progress
    let t2 = service.create("T2", None, project.id, None).await.unwrap();
    service.start(&t2.id, None).await.unwrap();
    service.abandon(&t2.id, None).await.unwrap();

    // abandon from done
    let t3 = service.create("T3", None, project.id, None).await.unwrap();
    service.start(&t3.id, None).await.unwrap();
    service.done(&t3.id, None).await.unwrap();
    service.abandon(&t3.id, None).await.unwrap();
}

#[tokio::test]
async fn backdated_status_change_removes_future_changes() {
    let repo = FakeRepo::default();
    let service = TaskService::new(repo.clone());
    let project = Project::default();

    let task = service
        .create("Task", None, project.id, None)
        .await
        .unwrap();

    let mar10 = common::datetime(common::date(2026, 3, 10), 8);
    let mar15 = common::datetime(common::date(2026, 3, 15), 8);
    let mar16 = common::datetime(common::date(2026, 3, 16), 8);
    let mar12 = common::datetime(common::date(2026, 3, 12), 8);

    // Build history: start -> done -> reset -> start
    service.start(&task.id, Some(mar10)).await.unwrap();
    service.done(&task.id, Some(mar15)).await.unwrap();
    service.reset(&task.id, Some(mar16)).await.unwrap();
    service.start(&task.id, Some(mar16)).await.unwrap();

    assert_eq!(service.status_history(&task.id).await.unwrap().len(), 4);

    // Backdate done to mar12 - must remove changes after mar12
    service.done(&task.id, Some(mar12)).await.unwrap();

    let history = service.status_history(&task.id).await.unwrap();
    // Should have: start@mar10, done@mar12 (mar15/mar16 changes removed)
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].new_status, Status::InProgress);
    assert_eq!(history[0].changed_at, mar10);
    assert_eq!(history[1].new_status, Status::Done);
    assert_eq!(history[1].changed_at, mar12);
}

#[tokio::test]
async fn reorder_shifts_siblings_and_skips_other_parents() {
    let repo = FakeRepo::default();
    let service = TaskService::new(repo.clone());
    let project = Project::default();

    // Root tasks: A(0), B(1), C(2)
    let a = service.create("A", None, project.id, None).await.unwrap();
    let b = service.create("B", None, project.id, None).await.unwrap();
    let c = service.create("C", None, project.id, None).await.unwrap();

    // Child of A - must not be affected by root reorder
    let child = service
        .create("Child", Some(&a.id), project.id, None)
        .await
        .unwrap();

    // Move C to position 0: expected result A->1, B->2, C->0, child unchanged
    let mut all_tasks = service
        .list(&TaskFilter::ByProject(project.id))
        .await
        .unwrap();
    service.reorder(&c.id, 0, &mut all_tasks).await.unwrap();

    let updated_a = repo.find_task_by_id(&a.id).await.unwrap().unwrap();
    let updated_b = repo.find_task_by_id(&b.id).await.unwrap().unwrap();
    let updated_c = repo.find_task_by_id(&c.id).await.unwrap().unwrap();
    let updated_child = repo.find_task_by_id(&child.id).await.unwrap().unwrap();

    assert_eq!(updated_c.order, Some(0));
    assert_eq!(updated_a.order, Some(1));
    assert_eq!(updated_b.order, Some(2));
    // Child of A has order 0 within its own parent - must stay unchanged
    assert_eq!(updated_child.order, Some(0));
}
