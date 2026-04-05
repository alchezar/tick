//! Project - a logical grouping of tasks.

use chrono::{DateTime, Utc};

use crate::model::ProjectId;

/// A project tracked in the system.
#[derive(Debug, Clone)]
pub struct Project {
    /// Unique identifier.
    pub id: ProjectId,
    /// Unique short identifier used in CLI commands (e.g. `work`).
    pub slug: String,
    /// Optional human-readable display title.
    pub title: Option<String>,
    /// Optional GitHub repository URL (e.g. `https://github.com/owner/repo`).
    pub github_url: Option<String>,
    /// Creation timestamp.
    pub created: DateTime<Utc>,
}

impl Project {
    /// Creates a new project with current timestamp.
    #[must_use]
    pub fn new(slug: impl Into<String>, title: Option<impl Into<String>>) -> Self {
        Self {
            id: ProjectId::new(),
            slug: slug.into(),
            title: title.map(Into::into),
            github_url: None,
            created: Utc::now(),
        }
    }
}

impl Default for Project {
    fn default() -> Self {
        Self::new("default", None::<String>)
    }
}
