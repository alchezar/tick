//! Application context - shared state for all command handlers.

use core::cell::RefCell;

use crate::config::Config;
use domain::{
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
