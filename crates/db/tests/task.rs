//! Integration tests for `TaskRepository` on `SqliteRepo`.

mod common;

use chrono::NaiveDate;

use domain::{
    model::{Project, Status, StatusChange, Task},
    repository::{ProjectRepository, TaskFilter, TaskRepository},
};

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn save_and_find_task() {
    let (repo, project) = common::repo_with_project().await;
    let task = Task::new("Fix bug", None, project.id);
    repo.save_task(&task).await.unwrap();

    let found = repo.find_task_by_id(&task.id).await.unwrap().unwrap();
    assert_eq!(found.id, task.id);
    assert_eq!(found.title, "Fix bug");
    assert_eq!(found.status(), Status::NotStarted);
    assert!(found.parent.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn find_task_returns_none() {
    let (repo, _) = common::repo_with_project().await;
    assert!(
        repo.find_task_by_id(&uuid::Uuid::new_v4())
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn save_updates_task() {
    let (repo, project) = common::repo_with_project().await;
    let task = Task::new("Old title", None, project.id);
    repo.save_task(&task).await.unwrap();

    let mut updated = task.clone().with_status(Status::InProgress);
    updated.title = "New title".to_owned();
    repo.save_task(&updated).await.unwrap();

    let found = repo.find_task_by_id(&task.id).await.unwrap().unwrap();
    assert_eq!(found.title, "New title");
    assert_eq!(found.status(), Status::InProgress);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn child_tasks_of() {
    let (repo, project) = common::repo_with_project().await;
    let parent = Task::new("Parent", None, project.id);
    repo.save_task(&parent).await.unwrap();

    let mut child1 = Task::new("Child 1", Some(parent.id), project.id);
    child1.order = Some(0);
    repo.save_task(&child1).await.unwrap();

    let mut child2 = Task::new("Child 2", Some(parent.id), project.id);
    child2.order = Some(1);
    repo.save_task(&child2).await.unwrap();

    let children = repo.child_tasks_of(&parent.id).await.unwrap();
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].title, "Child 1");
    assert_eq!(children[1].title, "Child 2");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn child_tasks_of_empty() {
    let (repo, project) = common::repo_with_project().await;
    let task = Task::new("Leaf", None, project.id);
    repo.save_task(&task).await.unwrap();

    assert!(repo.child_tasks_of(&task.id).await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn list_tasks_by_project() {
    let (repo, project) = common::repo_with_project().await;
    let other = Project::new("other", None::<String>);
    repo.save_project(&other).await.unwrap();

    repo.save_task(&Task::new("Work task", None, project.id))
        .await
        .unwrap();
    repo.save_task(&Task::new("Other task", None, other.id))
        .await
        .unwrap();

    let work_tasks = repo
        .list_tasks(&TaskFilter::ByProject(project.id))
        .await
        .unwrap();
    assert_eq!(work_tasks.len(), 1);
    assert_eq!(work_tasks[0].title, "Work task");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn delete_task_removes_it() {
    let (repo, project) = common::repo_with_project().await;
    let task = Task::new("Task", None, project.id);
    repo.save_task(&task).await.unwrap();

    repo.delete_task(&task.id).await.unwrap();
    assert!(repo.find_task_by_id(&task.id).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn delete_task_cascades_children() {
    let (repo, project) = common::repo_with_project().await;
    let parent = Task::new("Parent", None, project.id);
    repo.save_task(&parent).await.unwrap();

    let child = Task::new("Child", Some(parent.id), project.id);
    repo.save_task(&child).await.unwrap();

    repo.delete_task(&parent.id).await.unwrap();
    assert!(repo.find_task_by_id(&child.id).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn delete_nonexistent_task_is_ok() {
    let (repo, _) = common::repo_with_project().await;
    repo.delete_task(&uuid::Uuid::new_v4()).await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn delete_all_tasks_by_project() {
    let (repo, project) = common::repo_with_project().await;
    repo.save_task(&Task::new("A", None, project.id))
        .await
        .unwrap();
    repo.save_task(&Task::new("B", None, project.id))
        .await
        .unwrap();

    repo.delete_all_tasks_by(&project.id).await.unwrap();
    assert!(
        repo.list_tasks(&TaskFilter::ByProject(project.id))
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn save_and_list_status_changes() {
    let (repo, project) = common::repo_with_project().await;
    let task = Task::new("Task", None, project.id);
    repo.save_task(&task).await.unwrap();

    let change = StatusChange::new(task.id, Status::NotStarted, Status::InProgress, None);
    repo.save_task_change(&change).await.unwrap();

    let changes = repo.list_task_changes(&task.id).await.unwrap();
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].old_status, Status::NotStarted);
    assert_eq!(changes[0].new_status, Status::InProgress);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn list_task_changes_on_date() {
    let (repo, project) = common::repo_with_project().await;
    let task = Task::new("Task", None, project.id);
    repo.save_task(&task).await.unwrap();

    let date = NaiveDate::from_ymd_opt(2026, 3, 8).unwrap();
    let dt = date.and_hms_opt(10, 0, 0).unwrap().and_utc();

    let mut change = StatusChange::new(task.id, Status::NotStarted, Status::InProgress, None);
    change.changed_at = dt;
    repo.save_task_change(&change).await.unwrap();

    let on_date = repo.list_task_changes_on(date).await.unwrap();
    assert_eq!(on_date.len(), 1);

    let other_date = NaiveDate::from_ymd_opt(2026, 3, 9).unwrap();
    assert!(
        repo.list_task_changes_on(other_date)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn task_order_preserved() {
    let (repo, project) = common::repo_with_project().await;
    let mut t1 = Task::new("First", None, project.id);
    t1.order = Some(0);
    let mut t2 = Task::new("Second", None, project.id);
    t2.order = Some(1);
    let mut t3 = Task::new("Third", None, project.id);
    t3.order = Some(2);

    repo.save_task(&t3).await.unwrap();
    repo.save_task(&t1).await.unwrap();
    repo.save_task(&t2).await.unwrap();

    let tasks = repo
        .list_tasks(&TaskFilter::ByProject(project.id))
        .await
        .unwrap();
    assert_eq!(tasks[0].title, "First");
    assert_eq!(tasks[1].title, "Second");
    assert_eq!(tasks[2].title, "Third");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn delete_project_cascades_tasks_and_changes() {
    let (repo, project) = common::repo_with_project().await;
    let task = Task::new("Task", None, project.id);
    repo.save_task(&task).await.unwrap();

    let change = StatusChange::new(task.id, Status::NotStarted, Status::InProgress, None);
    repo.save_task_change(&change).await.unwrap();
    repo.delete_project(&project.id).await.unwrap();

    assert!(repo.find_task_by_id(&task.id).await.unwrap().is_none());
    assert!(repo.list_task_changes(&task.id).await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn delete_task_changes_after_cutoff() {
    let (repo, project) = common::repo_with_project().await;
    let task = Task::new("Task", None, project.id);
    repo.save_task(&task).await.unwrap();

    let date = |d: u32| {
        NaiveDate::from_ymd_opt(2026, 3, d)
            .unwrap()
            .and_hms_opt(8, 0, 0)
            .unwrap()
            .and_utc()
    };

    let mut c1 = StatusChange::new(task.id, Status::NotStarted, Status::InProgress, None);
    c1.changed_at = date(10);
    repo.save_task_change(&c1).await.unwrap();

    let mut c2 = StatusChange::new(task.id, Status::InProgress, Status::Done, None);
    c2.changed_at = date(15);
    repo.save_task_change(&c2).await.unwrap();

    let mut c3 = StatusChange::new(task.id, Status::Done, Status::NotStarted, None);
    c3.changed_at = date(16);
    repo.save_task_change(&c3).await.unwrap();

    // Delete changes after mar12 - should remove c2 and c3
    repo.delete_task_changes_after(&task.id, date(12))
        .await
        .unwrap();

    let remaining = repo.list_task_changes(&task.id).await.unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].new_status, Status::InProgress);
}
