//! Service layer — business logic and domain invariants.

mod project;
mod report;
mod task;

pub use project::ProjectService;
pub use report::{Report, ReportService, prev_workday, render_all};
pub use task::TaskService;
