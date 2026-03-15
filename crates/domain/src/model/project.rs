//! Project - a logical grouping of tasks.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// A project tracked in the system.
#[derive(Debug, Clone)]
pub struct Project {
    /// Unique identifier.
    pub id: Uuid,
    /// Unique short identifier used in CLI commands (e.g. `work`).
    pub slug: String,
    /// Optional human-readable display title.
    pub title: Option<String>,
    /// Creation timestamp.
    pub created: DateTime<Utc>,
}

impl Project {
    /// Creates a new project with current timestamp.
    #[must_use]
    pub fn new(slug: impl Into<String>, title: Option<impl Into<String>>) -> Self {
        Self {
            id: Uuid::new_v4(),
            slug: slug.into(),
            title: title.map(Into::into),
            created: Utc::now(),
        }
    }
}

impl Default for Project {
    fn default() -> Self {
        Self::new("default", None::<String>)
    }
}
