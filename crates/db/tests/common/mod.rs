//! Shared test helpers for db integration tests.

#![allow(unused)]

use db::SqliteRepo;
use domain::{model::Project, repository::ProjectRepository};

pub fn repo() -> SqliteRepo {
    SqliteRepo::in_memory().unwrap()
}

pub fn repo_with_project() -> (SqliteRepo, Project) {
    let repo = repo();
    let project = Project::new("work", None::<String>);
    repo.save_project(&project).unwrap();
    (repo, project)
}
