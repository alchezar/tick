//! Model types: core data structures and business rules.

mod status;
mod task;

pub use status::{Status, StatusChange};
pub use task::Task;
