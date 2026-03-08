//! Integration tests for `ProjectService` business rules.

mod common;

use domain::{
    error::CoreError,
    repository::TaskRepository,
    service::{ProjectService, TaskService},
};

use common::fake::FakeRepo;

#[tokio::test]
async fn create_project_succeeds() {
    let service = common::project_service();
    let project = service.create("work", None).await.unwrap();

    assert_eq!(project.slug, "work");
    assert!(project.title.is_none());
}

#[tokio::test]
async fn create_project_with_title() {
    let service = common::project_service();
    let project = service.create("work", Some("Work Projects")).await.unwrap();

    assert_eq!(project.slug, "work");
    assert_eq!(project.title.as_deref(), Some("Work Projects"));
}

#[tokio::test]
async fn create_duplicate_slug_fails() {
    let service = common::project_service();
    service.create("work", None).await.unwrap();

    let err = service.create("work", Some("Another")).await.unwrap_err();
    assert!(matches!(err, CoreError::ProjectAlreadyExists { slug } if slug == "work"));
}

#[tokio::test]
async fn find_by_slug_returns_project() {
    let service = common::project_service();
    let created = service.create("work", None).await.unwrap();

    let found = service.find_by("work").await.unwrap();
    assert_eq!(found.id, created.id);
    assert_eq!(found.slug, "work");
}

#[tokio::test]
async fn find_by_slug_not_found() {
    let service = common::project_service();

    let err = service.find_by("missing").await.unwrap_err();
    assert!(matches!(err, CoreError::ProjectNotFound { slug } if slug == "missing"));
}

#[tokio::test]
async fn list_returns_all_projects() {
    let service = common::project_service();
    service.create("alpha", None).await.unwrap();
    service.create("beta", None).await.unwrap();
    service.create("gamma", None).await.unwrap();

    let projects = service.list().await.unwrap();
    assert_eq!(projects.len(), 3);
}

#[tokio::test]
async fn delete_removes_project() {
    let service = common::project_service();
    service.create("work", None).await.unwrap();

    service.delete("work").await.unwrap();

    let err = service.find_by("work").await.unwrap_err();
    assert!(matches!(err, CoreError::ProjectNotFound { .. }));
}

#[tokio::test]
async fn delete_not_found() {
    let service = common::project_service();

    let err = service.delete("missing").await.unwrap_err();
    assert!(matches!(err, CoreError::ProjectNotFound { slug } if slug == "missing"));
}

#[tokio::test]
async fn delete_cascades_tasks() {
    let repo = FakeRepo::default();
    let project_svc = ProjectService::new(repo.clone());
    let task_svc = TaskService::new(repo.clone());

    let project = project_svc.create("work", None).await.unwrap();
    task_svc.create("Task A", None, project.id).await.unwrap();
    task_svc.create("Task B", None, project.id).await.unwrap();

    project_svc.delete("work").await.unwrap();

    // Tasks should be gone too
    let tasks = repo.list_tasks(&project.id).await.unwrap();
    assert!(tasks.is_empty());
}

#[tokio::test]
async fn tasks_isolated_between_projects() {
    let repo = FakeRepo::default();
    let project_svc = ProjectService::new(repo.clone());
    let task_svc = TaskService::new(repo.clone());

    let work = project_svc.create("work", None).await.unwrap();
    let personal = project_svc.create("personal", None).await.unwrap();

    task_svc.create("Work task", None, work.id).await.unwrap();
    task_svc
        .create("Personal task", None, personal.id)
        .await
        .unwrap();

    let work_tasks = repo.list_tasks(&work.id).await.unwrap();
    let personal_tasks = repo.list_tasks(&personal.id).await.unwrap();

    assert_eq!(work_tasks.len(), 1);
    assert_eq!(work_tasks[0].title, "Work task");
    assert_eq!(personal_tasks.len(), 1);
    assert_eq!(personal_tasks[0].title, "Personal task");
}

#[tokio::test]
async fn delete_project_cleans_status_changes() {
    let repo = FakeRepo::default();
    let project_svc = ProjectService::new(repo.clone());
    let task_svc = TaskService::new(repo.clone());

    let project = project_svc.create("work", None).await.unwrap();
    let task = task_svc.create("Task", None, project.id).await.unwrap();
    task_svc.start(&task.id).await.unwrap();

    project_svc.delete("work").await.unwrap();

    let changes = repo.list_task_changes(&task.id).await.unwrap();
    assert!(changes.is_empty());
}

#[tokio::test]
async fn rename_updates_title() {
    let service = common::project_service();
    service.create("work", Some("Old Title")).await.unwrap();

    service.rename("work", "New Title").await.unwrap();

    let project = service.find_by("work").await.unwrap();
    assert_eq!(project.title.as_deref(), Some("New Title"));
}

#[tokio::test]
async fn rename_sets_title_when_none() {
    let service = common::project_service();
    service.create("work", None).await.unwrap();

    service.rename("work", "Added Title").await.unwrap();

    let project = service.find_by("work").await.unwrap();
    assert_eq!(project.title.as_deref(), Some("Added Title"));
}

#[tokio::test]
async fn reslug_changes_slug() {
    let service = common::project_service();
    service.create("old", Some("My Project")).await.unwrap();

    service.reslug("old", "new").await.unwrap();

    let err = service.find_by("old").await.unwrap_err();
    assert!(matches!(err, CoreError::ProjectNotFound { .. }));

    let project = service.find_by("new").await.unwrap();
    assert_eq!(project.title.as_deref(), Some("My Project"));
}

#[tokio::test]
async fn reslug_duplicate_fails() {
    let service = common::project_service();
    service.create("alpha", None).await.unwrap();
    service.create("beta", None).await.unwrap();

    let err = service.reslug("alpha", "beta").await.unwrap_err();
    assert!(matches!(err, CoreError::ProjectAlreadyExists { slug } if slug == "beta"));
}

#[tokio::test]
async fn reslug_not_found() {
    let service = common::project_service();

    let err = service.reslug("missing", "new").await.unwrap_err();
    assert!(matches!(err, CoreError::ProjectNotFound { slug } if slug == "missing"));
}

#[tokio::test]
async fn rename_not_found() {
    let service = common::project_service();

    let err = service.rename("missing", "Title").await.unwrap_err();
    assert!(matches!(err, CoreError::ProjectNotFound { slug } if slug == "missing"));
}
