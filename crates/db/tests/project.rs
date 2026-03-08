//! Integration tests for `ProjectRepository` on `SqliteRepo`.

mod common;

use domain::{
    model::Project,
    repository::{ProjectRepository, TransactionGuard, Transactional},
};

#[test]
fn save_and_find_by_id() {
    let repo = common::repo();
    let project = Project::new("work", Some("Work"));

    repo.save_project(&project).unwrap();
    let found = repo.find_project_by_id(&project.id).unwrap().unwrap();

    assert_eq!(found.id, project.id);
    assert_eq!(found.slug, "work");
    assert_eq!(found.title.as_deref(), Some("Work"));
}

#[test]
fn find_by_id_returns_none() {
    let repo = common::repo();
    let result = repo.find_project_by_id(&uuid::Uuid::new_v4()).unwrap();

    assert!(result.is_none());
}

#[test]
fn save_and_find_by_slug() {
    let repo = common::repo();
    let project = Project::new("work", None::<String>);

    repo.save_project(&project).unwrap();
    let found = repo.find_project_by_slug("work").unwrap().unwrap();

    assert_eq!(found.id, project.id);
    assert!(found.title.is_none());
}

#[test]
fn find_by_slug_returns_none() {
    let repo = common::repo();
    let result = repo.find_project_by_slug("missing").unwrap();

    assert!(result.is_none());
}

#[test]
fn save_updates_existing() {
    let repo = common::repo();
    let mut project = Project::new("work", None::<String>);

    repo.save_project(&project).unwrap();
    project.slug = "updated".to_owned();
    project.title = Some("Updated Title".to_owned());
    repo.save_project(&project).unwrap();
    let found = repo.find_project_by_id(&project.id).unwrap().unwrap();

    assert_eq!(found.slug, "updated");
    assert_eq!(found.title.as_deref(), Some("Updated Title"));
}

#[test]
fn list_projects_empty() {
    let repo = common::repo();
    let projects = repo.list_projects().unwrap();

    assert!(projects.is_empty());
}

#[test]
fn list_projects_returns_all() {
    let repo = common::repo();

    repo.save_project(&Project::new("alpha", None::<String>))
        .unwrap();
    repo.save_project(&Project::new("beta", None::<String>))
        .unwrap();
    repo.save_project(&Project::new("gamma", None::<String>))
        .unwrap();
    let projects = repo.list_projects().unwrap();

    assert_eq!(projects.len(), 3);
}

#[test]
fn delete_project_removes_it() {
    let repo = common::repo();
    let project = Project::new("work", None::<String>);

    repo.save_project(&project).unwrap();
    repo.delete_project(&project.id).unwrap();

    assert!(repo.find_project_by_id(&project.id).unwrap().is_none());
}

#[test]
fn delete_nonexistent_is_ok() {
    let repo = common::repo();
    repo.delete_project(&uuid::Uuid::new_v4()).unwrap();
}

#[test]
fn transaction_commit_persists() {
    let repo = common::repo();
    let tx = repo.begin_transaction().unwrap();

    repo.save_project(&Project::new("work", None::<String>))
        .unwrap();
    tx.commit_transaction().unwrap();

    assert!(repo.find_project_by_slug("work").unwrap().is_some());
}

#[test]
fn transaction_rollback_on_drop() {
    let repo = common::repo();
    {
        let _tx = repo.begin_transaction().unwrap();
        repo.save_project(&Project::new("work", None::<String>))
            .unwrap();
        // guard dropped without commit
    }

    assert!(repo.find_project_by_slug("work").unwrap().is_none());
}

#[test]
fn nested_transaction_commit() {
    let repo = common::repo();
    let outer = repo.begin_transaction().unwrap();

    repo.save_project(&Project::new("alpha", None::<String>))
        .unwrap();

    let inner = repo.begin_transaction().unwrap();
    repo.save_project(&Project::new("beta", None::<String>))
        .unwrap();
    inner.commit_transaction().unwrap();
    outer.commit_transaction().unwrap();

    assert_eq!(repo.list_projects().unwrap().len(), 2);
}

#[test]
fn nested_transaction_inner_drop_rollbacks_all() {
    let repo = common::repo();
    let outer = repo.begin_transaction().unwrap();

    repo.save_project(&Project::new("alpha", None::<String>))
        .unwrap();
    {
        let _inner = repo.begin_transaction().unwrap();
        repo.save_project(&Project::new("beta", None::<String>))
            .unwrap();
        // inner dropped without commit
    }

    // outer cannot commit because inner already decremented depth,
    // but the real rollback happened when depth reached 0 on inner drop
    // so data is already rolled back
    drop(outer);

    assert!(repo.list_projects().unwrap().is_empty());
}
