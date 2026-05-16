//! `tt` - CLI entry point.

use core::cell::RefCell;
use std::process;

use clap::Parser;

use cli::{
    args::{Cli, Command},
    config::Config,
    context::AppContext,
    error::{CliError, CliResult},
    guard::RemoveGuard,
    handler::{project, report, task},
};
use db::SqliteRepo;
use domain::service::{ProjectService, ReportService, TaskService};

#[tokio::main]
async fn main() {
    if let Err(err) = run().await
        && !matches!(err, CliError::Aborted)
    {
        eprintln!("error: {err}");
        process::exit(1);
    }
}

async fn run() -> CliResult<()> {
    let cli = Cli::parse();
    let config = Config::load()?;
    let repo = SqliteRepo::open_default()
        .await
        .map_err(|err| CliError::Domain(err.into()))?;

    let mut context = AppContext {
        config,
        project_service: ProjectService::new(repo.clone()),
        task_service: TaskService::new(repo.clone()),
        report_service: ReportService::new(repo),
        confirmer: RefCell::new(RemoveGuard::default()),
    };

    match cli.command {
        Command::Project { action } => project::handle(action, &mut context).await,
        Command::Task { action } => task::handle(action, &context).await,
        Command::Report {
            project,
            all,
            copy,
            date,
        } => report::handle(project.as_deref(), all, copy, date, &context).await,
    }
}
