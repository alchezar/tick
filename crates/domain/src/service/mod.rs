//! Service layer — business logic and domain invariants.

mod report;
mod task;

pub use report::{Report, ReportService, prev_workday};
pub use task::TaskService;
