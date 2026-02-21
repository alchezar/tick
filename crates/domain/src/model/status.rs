//! Task status and allowed transitions.

/// Represents the lifecycle state of a task.
#[derive(Debug, Default, Clone, PartialEq)]
pub enum Status {
    /// Task has not been started yet.
    #[default]
    NotStarted,
    /// Task is currently being worked on.
    InProgress,
    /// Task has been completed.
    Done,
    /// Task is blocked and cannot progress.
    Blocked,
}

impl Status {
    /// Returns `true` if transition from current status to `to` is allowed.
    #[inline]
    #[must_use]
    pub fn can_transit(&self, to: &Self) -> bool {
        matches!(
            (self, to),
            (_, Status::NotStarted)
                | (Status::NotStarted | Status::Blocked, Status::InProgress)
                | (Status::NotStarted | Status::InProgress, Status::Blocked)
                | (Status::InProgress, Status::Done)
        )
    }

    /// Returns `true` if the task is actionable (shown in Today section of the report).
    #[inline]
    #[must_use]
    pub fn is_active(&self) -> bool {
        matches!(self, Status::NotStarted | Status::InProgress)
    }

    /// Returns `true` if the task is no longer actionable (shown in Previously section of the report).
    #[inline]
    #[must_use]
    pub fn is_closed(&self) -> bool {
        matches!(self, Status::Done | Status::Blocked)
    }

    /// Returns the emoji icon representing this status in the report output.
    #[inline]
    #[must_use]
    pub fn icon(&self) -> &'static str {
        match self {
            Self::NotStarted => "❌",
            Self::InProgress => "🔄",
            Self::Done => "✅",
            Self::Blocked => "🛑",
        }
    }
}
