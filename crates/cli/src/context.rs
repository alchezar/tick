//! Application context - shared state for all command handlers.

use core::cell::RefCell;

use crate::{
    config::Config,
    error::{CliError, CliResult},
};
use domain::{
    model::Project,
    repository::{ProjectRepository, TaskRepository, Transactional},
    service::{ProjectService, ReportService, TaskService},
};

/// Shared state passed to all command handlers.
#[derive(Debug)]
pub struct AppContext<R, C>
where
    R: ProjectRepository + TaskRepository + Transactional,
{
    /// Persistent CLI configuration (active project, etc.).
    pub config: Config,
    /// Project management service.
    pub project_service: ProjectService<R>,
    /// Task management service.
    pub task_service: TaskService<R>,
    /// Standup report service.
    pub report_service: ReportService<R>,
    /// Confirmation guard for destructive operations.
    pub confirmer: RefCell<C>,
}

impl<R, C> AppContext<R, C>
where
    R: ProjectRepository + TaskRepository + Transactional,
{
    /// Resolves project from an optional slug, falling back to the active project.
    ///
    /// # Errors
    ///
    /// - [`CliError::NoActiveProject`] if no slug is given and no active project is set.
    pub async fn resolve_project(&self, project_slug: Option<&str>) -> CliResult<Project> {
        let project_slug = project_slug
            .or(self.config.active_project.as_deref())
            .ok_or(CliError::NoActiveProject)?;
        Ok(self.project_service.find_by(project_slug).await?)
    }
}
