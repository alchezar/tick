//! Application context - shared state for all command handlers.

use domain::{
    repository::{ProjectRepository, TaskRepository, Transactional},
    service::{ProjectService, TaskService},
};

use crate::config::Config;

/// Shared state passed to all command handlers.
#[derive(Debug)]
pub struct AppContext<R>
where
    R: ProjectRepository + TaskRepository + Transactional,
{
    /// Persistent CLI configuration (active project, etc.).
    pub config: Config,
    /// Project management service.
    pub project_service: ProjectService<R>,
    /// Task management service.
    pub task_service: TaskService<R>,
}
