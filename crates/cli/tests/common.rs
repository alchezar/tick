//! Shared test helpers for CLI integration tests.

#![allow(unused)]

use tempfile::TempDir;

use cli::{
    config::{CONFIG_FILE, Config},
    context::AppContext,
};
use db::SqliteRepo;
use domain::service::{ProjectService, ReportService, TaskService};

/// Creates a test context with in-memory DB and temp config directory.
///
/// Returns the context and the temp dir (must be kept alive for config persistence).
///
/// # Panics
/// Panics if the temp dir, config, or in-memory DB cannot be created.
pub async fn context() -> (AppContext<SqliteRepo>, TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let config = Config::load_from(&dir.path().join(CONFIG_FILE)).unwrap();
    let repo = SqliteRepo::in_memory().await.unwrap();
    let ctx = AppContext {
        config,
        project_service: ProjectService::new(repo.clone()),
        task_service: TaskService::new(repo.clone()),
        report_service: ReportService::new(repo),
    };
    (ctx, dir)
}

/// Creates a test context with an active project "work".
///
/// # Panics
/// Panics if the context or project cannot be created.
pub async fn setup() -> (AppContext<SqliteRepo>, TempDir) {
    let (mut ctx, dir) = context().await;
    ctx.project_service.create("work", None).await.unwrap();
    ctx.config.active_project = Some("work".to_owned());
    (ctx, dir)
}
