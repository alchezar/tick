//! Integration tests for `ProjectService` business rules.

mod common;

use domain::{
    error::CoreError,
    repository::TaskRepository,
    service::{ProjectService, TaskService},
};

use common::fake::FakeRepo;

#[test]
fn create_project_succeeds() {
    let service = common::project_service();
    let project = service.create("work", None).unwrap();

    assert_eq!(project.slug, "work");
    assert!(project.title.is_none());
}

#[test]
fn create_project_with_title() {
    let service = common::project_service();
    let project = service.create("work", Some("Work Projects")).unwrap();

    assert_eq!(project.slug, "work");
    assert_eq!(project.title.as_deref(), Some("Work Projects"));
}

#[test]
fn create_duplicate_slug_fails() {
    let service = common::project_service();
    service.create("work", None).unwrap();

    let err = service.create("work", Some("Another")).unwrap_err();
    assert!(matches!(err, CoreError::ProjectAlreadyExists { slug } if slug == "work"));
}

#[test]
fn find_by_slug_returns_project() {
    let service = common::project_service();
    let created = service.create("work", None).unwrap();

    let found = service.find_by("work").unwrap();
    assert_eq!(found.id, created.id);
    assert_eq!(found.slug, "work");
}

#[test]
fn find_by_slug_not_found() {
    let service = common::project_service();

    let err = service.find_by("missing").unwrap_err();
    assert!(matches!(err, CoreError::ProjectNotFound { slug } if slug == "missing"));
}

#[test]
fn list_returns_all_projects() {
    let service = common::project_service();
    service.create("alpha", None).unwrap();
    service.create("beta", None).unwrap();
    service.create("gamma", None).unwrap();

    let projects = service.list().unwrap();
    assert_eq!(projects.len(), 3);
}

#[test]
fn delete_removes_project() {
    let service = common::project_service();
    service.create("work", None).unwrap();

    service.delete("work").unwrap();

    let err = service.find_by("work").unwrap_err();
    assert!(matches!(err, CoreError::ProjectNotFound { .. }));
}

#[test]
fn delete_not_found() {
    let service = common::project_service();

    let err = service.delete("missing").unwrap_err();
    assert!(matches!(err, CoreError::ProjectNotFound { slug } if slug == "missing"));
}

#[test]
fn delete_cascades_tasks() {
    let repo = FakeRepo::default();
    let project_svc = ProjectService::new(repo.clone());
    let task_svc = TaskService::new(repo.clone());

    let project = project_svc.create("work", None).unwrap();
    task_svc.create("Task A", None, project.id).unwrap();
    task_svc.create("Task B", None, project.id).unwrap();

    project_svc.delete("work").unwrap();

    // Tasks should be gone too
    let tasks = repo.list_tasks(&project.id).unwrap();
    assert!(tasks.is_empty());
}

#[test]
fn tasks_isolated_between_projects() {
    let repo = FakeRepo::default();
    let project_svc = ProjectService::new(repo.clone());
    let task_svc = TaskService::new(repo.clone());

    let work = project_svc.create("work", None).unwrap();
    let personal = project_svc.create("personal", None).unwrap();

    task_svc.create("Work task", None, work.id).unwrap();
    task_svc.create("Personal task", None, personal.id).unwrap();

    let work_tasks = repo.list_tasks(&work.id).unwrap();
    let personal_tasks = repo.list_tasks(&personal.id).unwrap();

    assert_eq!(work_tasks.len(), 1);
    assert_eq!(work_tasks[0].title, "Work task");
    assert_eq!(personal_tasks.len(), 1);
    assert_eq!(personal_tasks[0].title, "Personal task");
}

#[test]
fn delete_project_cleans_status_changes() {
    let repo = FakeRepo::default();
    let project_svc = ProjectService::new(repo.clone());
    let task_svc = TaskService::new(repo.clone());

    let project = project_svc.create("work", None).unwrap();
    let task = task_svc.create("Task", None, project.id).unwrap();
    task_svc.start(&task.id).unwrap();

    project_svc.delete("work").unwrap();

    let changes = repo.list_task_changes(&task.id).unwrap();
    assert!(changes.is_empty());
}
