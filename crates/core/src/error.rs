//! Defines custom error type for the `core` module.

use thiserror::Error;
use uuid::Uuid;

use crate::domain::Status;

/// Maximum allowed task nesting depth.
pub const MAX_DEPTH: usize = 3;

/// Domain-level errors for the `core` crate.
#[derive(Debug, Error)]
pub enum CoreError {
    /// No task found with the given id.
    #[error("task not found: {id}")]
    TaskNotFound {
        /// `Id` of the missing task.
        id: Uuid,
    },
    /// Status transition is not allowed by domain rules.
    #[error("invalid status transition: {from:?} -> {to:?}")]
    InvalidStatusTransition {
        /// Current status.
        from: Status,
        /// Requested status.
        to: Status,
    },
    /// Task nesting exceeds the maximum allowed depth of 3.
    #[error("max nesting depth ({MAX_DEPTH}) exceeded")]
    MaxDepthExceeded,
    /// Cannot mark a task as done while it has unfinished children.
    #[error("task has unfinished children")]
    TaskHasUnfinishedChildren,
}

/// Shorthand `Result` type for all `core` operations.
pub type CoreResult<T> = Result<T, CoreError>;
