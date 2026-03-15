//! CLI argument definitions using clap derive.

use chrono::NaiveDate;
use clap::{Parser, Subcommand};

use crate::types::ShortId;

/// Task tracker with standup report generation.
#[derive(Debug, Parser)]
#[command(name = "tick", version, about)]
pub struct Cli {
    /// Top-level command.
    #[command(subcommand)]
    pub command: Command,
}

/// Top-level commands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Project management.
    #[command(visible_alias = "pr")]
    Project {
        /// Project action (omit to show active project).
        #[command(subcommand)]
        action: Option<ProjectAction>,
    },

    /// Task management.
    #[command(visible_alias = "ts")]
    Task {
        /// Task action.
        #[command(subcommand)]
        action: TaskAction,
    },

    /// Generate standup report.
    #[command(visible_alias = "rp")]
    Report {
        /// Project slug (defaults to active project).
        #[arg(short, long)]
        project: Option<String>,

        /// Copy output to clipboard.
        #[arg(short, long)]
        copy: bool,

        /// Report for a specific date (YYYY-MM-DD).
        #[arg(short, long)]
        date: Option<NaiveDate>,
    },
}

/// Project subcommands.
#[derive(Debug, Subcommand)]
pub enum ProjectAction {
    /// List all projects.
    #[command(visible_alias = "ls")]
    List,

    /// Create a new project.
    #[command(visible_alias = "ad")]
    Add {
        /// Unique short identifier (e.g. `work`).
        slug: String,

        /// Optional display title.
        #[arg(short, long)]
        title: Option<String>,
    },

    /// Switch active project.
    #[command(visible_alias = "sw")]
    Switch {
        /// Project slug to activate.
        slug: String,
    },

    /// Change project display title.
    #[command(visible_alias = "rn")]
    Rename {
        /// Current project slug.
        slug: String,

        /// New display title.
        new_title: String,
    },

    /// Change project slug.
    #[command(visible_alias = "rl")]
    Reslug {
        /// Current slug.
        slug: String,

        /// New slug.
        new_slug: String,
    },

    /// Delete project and all its tasks.
    #[command(visible_alias = "rm")]
    Remove {
        /// Project slug to delete.
        slug: String,
    },
}

/// Task subcommands.
#[derive(Debug, Subcommand)]
pub enum TaskAction {
    /// Add a new task.
    #[command(visible_alias = "ad")]
    Add {
        /// Task title.
        title: String,

        /// Parent task id (creates a subtask).
        #[arg(short, long)]
        under: Option<ShortId>,

        /// Project slug (defaults to active project).
        #[arg(short, long)]
        project: Option<String>,

        /// Creation date (YYYY-MM-DD), defaults to today.
        #[arg(short, long)]
        date: Option<NaiveDate>,
    },

    /// List tasks (tree view).
    #[command(visible_alias = "ls")]
    List {
        /// Include done and blocked tasks.
        #[arg(short, long)]
        all: bool,

        /// Project slug (defaults to active project).
        #[arg(short, long)]
        project: Option<String>,
    },

    /// Set task status to `in_progress`.
    #[command(visible_alias = "st")]
    Start {
        /// Task id.
        id: ShortId,

        /// Date of the status change (YYYY-MM-DD), defaults to now.
        #[arg(short, long)]
        date: Option<NaiveDate>,
    },

    /// Set task status to done.
    #[command(visible_alias = "dn")]
    Done {
        /// Task id.
        id: ShortId,

        /// Date of the status change (YYYY-MM-DD), defaults to now.
        #[arg(short, long)]
        date: Option<NaiveDate>,
    },

    /// Set task status to blocked.
    #[command(visible_alias = "bl")]
    Block {
        /// Task id.
        id: ShortId,

        /// Date of the status change (YYYY-MM-DD), defaults to now.
        #[arg(short, long)]
        date: Option<NaiveDate>,
    },

    /// Set task status to `not_started`.
    #[command(visible_alias = "rs")]
    Reset {
        /// Task id.
        id: ShortId,

        /// Date of the status change (YYYY-MM-DD), defaults to now.
        #[arg(short, long)]
        date: Option<NaiveDate>,
    },

    /// Move task under a new parent or change display order.
    #[command(visible_alias = "mv")]
    Move {
        /// Task id.
        id: ShortId,

        /// New parent task id.
        #[arg(short, long)]
        under: Option<ShortId>,

        /// New sibling display order.
        #[arg(short, long)]
        order: Option<usize>,
    },

    /// Rename a task.
    #[command(visible_alias = "rn")]
    Rename {
        /// Task id.
        id: ShortId,

        /// New title.
        title: String,
    },

    /// Delete task and its children.
    #[command(visible_alias = "rm")]
    Remove {
        /// Task id.
        id: ShortId,
    },
}

impl TaskAction {
    /// Returns the explicit `--project` slug if the variant carries one.
    #[must_use]
    pub fn project(&self) -> Option<&str> {
        match self {
            Self::Add { project, .. } | Self::List { project, .. } => project.as_deref(),
            _ => None,
        }
    }
}
