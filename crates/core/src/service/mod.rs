//! Service layer — business logic and domain invariants.

mod report;
mod task;

pub use report::{Report, ReportService};
pub use task::TaskService;
