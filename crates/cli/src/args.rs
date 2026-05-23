//! CLI argument definitions using clap derive.

use chrono::NaiveDate;
use clap::{Parser, Subcommand};

use crate::types::ShortId;

/// Task tracker with standup report generation.
#[derive(Debug, Parser)]
#[command(name = "tt", version, about)]
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

    /// Task management (defaults to list).
    #[command(visible_alias = "ts")]
    Task {
        /// Task action (omit to list tasks).
        #[command(subcommand)]
        action: Option<TaskAction>,
    },

    /// Generate standup report.
    #[command(visible_alias = "rp")]
    Report {
        /// Project slug (defaults to active project).
        #[arg(short, long)]
        project: Option<String>,

        /// Report for all projects.
        #[arg(short, long)]
        all: bool,

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

        /// Optional GitHub repository URL.
        #[arg(short, long)]
        github: Option<String>,
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

    /// Set GitHub repository URL.
    #[command(visible_alias = "gh")]
    Github {
        /// Project slug.
        slug: String,

        /// GitHub repository URL (omit to clear).
        url: Option<String>,
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
        parent: Option<ShortId>,

        /// Project slug (defaults to active project).
        #[arg(short = 'P', long)]
        project: Option<String>,

        /// Creation date (YYYY-MM-DD), defaults to today.
        #[arg(short, long)]
        date: Option<NaiveDate>,

        /// Pull request number.
        #[arg(short, long)]
        number: Option<u32>,
    },

    /// List tasks (tree view).
    #[command(visible_alias = "ls")]
    List {
        /// Include done and blocked tasks from specific date (YYYY-MM-DD).
        #[arg(short, long, group = "period")]
        from: Option<NaiveDate>,

        /// Include done and blocked tasks until specific date (YYYY-MM-DD).
        #[arg(short, long, group = "period")]
        until: Option<NaiveDate>,

        /// Include done and blocked tasks.
        #[arg(short, long, group = "period")]
        all: bool,

        /// Show subtree of a specific task.
        #[arg(short, long, group = "scope")]
        subtree: Option<ShortId>,

        /// Project slug (defaults to active project).
        #[arg(short, long, group = "scope")]
        project: Option<String>,
    },

    /// Set task status to `in_progress`.
    #[command(visible_alias = "st")]
    Start {
        /// Task ids.
        #[arg(required = true)]
        ids: Vec<ShortId>,

        /// Date of the status change (YYYY-MM-DD), defaults to now.
        #[arg(short, long)]
        date: Option<NaiveDate>,
    },

    /// Set task status to done.
    #[command(visible_alias = "dn")]
    Done {
        /// Task ids.
        #[arg(required = true)]
        ids: Vec<ShortId>,

        /// Date of the status change (YYYY-MM-DD), defaults to now.
        #[arg(short, long)]
        date: Option<NaiveDate>,
    },

    /// Set task status to blocked.
    #[command(visible_alias = "bl")]
    Block {
        /// Task ids.
        #[arg(required = true)]
        ids: Vec<ShortId>,

        /// Date of the status change (YYYY-MM-DD), defaults to now.
        #[arg(short, long)]
        date: Option<NaiveDate>,
    },

    /// Mark task as abandoned.
    #[command(visible_alias = "ab")]
    Abandon {
        /// Task ids.
        #[arg(required = true)]
        ids: Vec<ShortId>,

        /// Date of the status change (YYYY-MM-DD), defaults to now.
        #[arg(short, long)]
        date: Option<NaiveDate>,
    },

    /// Set task status to `not_started`.
    #[command(visible_alias = "rs")]
    Reset {
        /// Task ids.
        #[arg(required = true)]
        ids: Vec<ShortId>,

        /// Date of the status change (YYYY-MM-DD), defaults to now.
        #[arg(short, long)]
        date: Option<NaiveDate>,
    },

    /// Move task to a new parent or change display order.
    #[command(visible_alias = "mv")]
    Move {
        /// Task id.
        id: ShortId,

        /// New parent task id.
        #[arg(short, long, group = "action")]
        parent: Option<ShortId>,

        /// Move N positions up among siblings (default 1).
        #[arg(short, long, group = "action", num_args = 0..=1, default_missing_value = "1")]
        up: Option<u8>,

        /// Move N positions down among siblings (default 1).
        #[arg(short, long, group = "action", num_args = 0..=1, default_missing_value = "1")]
        down: Option<u8>,

        /// New sibling display order.
        #[arg(short, long, group = "action")]
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

    /// Set pull request number for a task.
    #[command(visible_alias = "ln")]
    Link {
        /// Task id.
        id: ShortId,

        /// Pull request number (omit to clear).
        number: Option<u32>,
    },

    /// Delete task and its children.
    #[command(visible_alias = "rm")]
    Remove {
        /// Task id.
        id: ShortId,
    },
}

impl Default for TaskAction {
    fn default() -> Self {
        Self::List {
            from: None,
            until: None,
            all: false,
            subtree: None,
            project: None,
        }
    }
}
