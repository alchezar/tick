//! Shared test helpers for db integration tests.

#![allow(unused)]

use db::SqliteRepo;

use domain::{model::Project, repository::ProjectRepository};

pub async fn repo() -> SqliteRepo {
    SqliteRepo::in_memory().await.unwrap()
}

pub async fn repo_with_project() -> (SqliteRepo, Project) {
    let repo = repo().await;
    let project = Project::new("work", None::<String>);
    repo.save_project(&project).await.unwrap();
    (repo, project)
}
