//! Handler for standup report commands.

use arboard::Clipboard;
use chrono::{Local, NaiveDate};

use crate::{
    context::AppContext,
    error::{CliError, CliResult},
};
use domain::{
    repository::{ProjectRepository, TaskRepository, Transactional},
    service,
};

/// Generates and prints a standup report.
///
/// # Errors
/// Returns [`CliError`] on domain or config errors.
pub async fn handle<R, C>(
    project: Option<&str>,
    all: bool,
    copy: bool,
    date: Option<NaiveDate>,
    context: &AppContext<R, C>,
) -> CliResult<()>
where
    R: ProjectRepository + TaskRepository + Transactional,
{
    let date = date.unwrap_or_else(|| Local::now().date_naive());
    let output = if all {
        let reports = context.report_service.generate_all(date).await?;
        service::render_all(&reports, !copy)
    } else {
        let project = context.resolve_project(project).await?;
        let report = context.report_service.generate(date, &project).await?;
        report.render(!copy, !copy)
    };

    if output.is_empty() {
        println!("no tasks to report");
        return Ok(());
    }

    if copy {
        Clipboard::new()
            .and_then(|mut cb| cb.set_text(&output))
            .map_err(|e| CliError::Clipboard(e.to_string()))?;
        println!("copied to clipboard");
    } else {
        print!("{}", super::terminal_emoji(&output));
    }

    Ok(())
}
