//! `tick` - CLI entry point.

use std::process;

use clap::Parser;

use cli::{
    args::{Cli, Command},
    config::Config,
    context::AppContext,
    error::{CliError, CliResult},
    handler::{project, report, task},
};
use db::SqliteRepo;
use domain::service::{ProjectService, ReportService, TaskService};

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

async fn run() -> CliResult<()> {
    dotenv::dotenv().ok();

    let cli = Cli::parse();
    let config = Config::load()?;
    let repo = SqliteRepo::open_default()
        .await
        .map_err(|e| CliError::Domain(e.into()))?;

    let mut context = AppContext {
        config,
        project_service: ProjectService::new(repo.clone()),
        task_service: TaskService::new(repo.clone()),
        report_service: ReportService::new(repo),
    };

    match cli.command {
        Command::Project { action } => project::handle(action, &mut context).await,
        Command::Task { action } => task::handle(action, &context).await,
        Command::Report {
            project,
            copy,
            date,
        } => report::handle(project.as_deref(), copy, date, &context).await,
    }
}
