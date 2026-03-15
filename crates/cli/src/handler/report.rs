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
pub async fn handle<R>(
    project: Option<&str>,
    copy: bool,
    date: Option<NaiveDate>,
    context: &AppContext<R>,
) -> CliResult<()>
where
    R: ProjectRepository + TaskRepository + Transactional,
{
    let date = date.unwrap_or_else(|| Local::now().date_naive());

    let output = if let Some(slug) = project {
        let project = context.project_service.find_by(slug).await?;
        let report = context.report_service.generate(date, &project).await?;
        report.render()
    } else {
        let reports = context.report_service.generate_all(date).await?;
        service::render_all(&reports)
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
        print!("{output}");
    }

    Ok(())
}
