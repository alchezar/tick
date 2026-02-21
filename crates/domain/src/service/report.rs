//! Business logic for standup report generation.

use std::collections::HashSet;

use chrono::{Datelike, Duration, NaiveDate, Weekday};

use crate::{error::CoreResult, model::Task, repository::TaskRepository};

/// Structured output of a generated standup report.
#[derive(Debug)]
pub struct Report {
    /// Tasks that were `done` or `blocked` on the previous workday.
    pub prev: Vec<Task>,
    /// Tasks that are `not_started` or `in_progress` as of today.
    pub today: Vec<Task>,
}

impl Report {
    /// Renders the report as a formatted string ready to paste into a chat.
    ///
    /// Each nesting level adds one ` -` prefix segment followed by the status icon and title.
    #[inline]
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = String::new();

        if !self.prev.is_empty() {
            out.push_str("Previously:\n");
            out.push_str(&render_section(&self.prev));
        }

        if !self.today.is_empty() {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str("Today:\n");
            out.push_str(&render_section(&self.today));
        }

        out
    }
}

/// Encapsulates all logic for building standup reports.
///
/// Fetches tasks from the repository and partitions them
/// into "Previously" and "Today" sections.
#[derive(Debug)]
pub struct ReportService<R>
where
    R: TaskRepository,
{
    repo: R,
}

impl<R> ReportService<R>
where
    R: TaskRepository,
{
    /// Creates a new `ReportService` with the given repository.
    #[inline]
    #[must_use]
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    /// Generates a standup report for the given date.
    ///
    /// # Errors
    /// Returns an error if the repository operation fails.
    #[inline]
    pub fn generate(&self, date: NaiveDate) -> CoreResult<Report> {
        Ok(Report {
            prev: self.tasks_prev(date)?,
            today: self.tasks_today(date)?,
        })
    }

    /// Returns active tasks (`not_started` or `in_progress`).
    ///
    /// `date` is reserved for future use when task history is supported.
    fn tasks_today(&self, _date: NaiveDate) -> CoreResult<Vec<Task>> {
        self.repo.list_active()
    }

    /// Returns tasks that were closed (`done` or `blocked`) on the previous workday before `date`.
    ///
    /// Accounts for weekends: on Monday includes Friday, Saturday, and Sunday.
    fn tasks_prev(&self, date: NaiveDate) -> CoreResult<Vec<Task>> {
        let prev = prev_workday(date);
        let tasks = self.repo.list_updated_on(prev)?;
        Ok(tasks.into_iter().filter(|t| t.status.is_closed()).collect())
    }
}

/// Returns the previous workday for `date`.
///
/// Monday -> Friday (skips Saturday and Sunday).
/// Any other day -> previous calendar day.
#[inline]
#[must_use]
#[doc(hidden)]
pub fn prev_workday(date: NaiveDate) -> NaiveDate {
    match date.weekday() {
        Weekday::Mon => date - Duration::days(3),
        _ => date - Duration::days(1),
    }
}

/// Renders a flat list of tasks as an indented hierarchy string.
///
/// Tasks whose parent is absent from the list are treated as roots (depth 1).
fn render_section(tasks: &[Task]) -> String {
    let ids = tasks.iter().map(|t| t.id).collect::<HashSet<_>>();

    let mut roots = tasks
        .iter()
        .filter(|t| t.parent.is_none_or(|p| !ids.contains(&p)))
        .collect::<Vec<_>>();
    roots.sort_by_key(|t| t.order);

    let mut out = String::new();
    for root in roots {
        render_task(root, tasks, 1, &mut out);
    }
    out
}

/// Recursively appends a task and its children to `out`.
fn render_task(task: &Task, all: &[Task], depth: usize, out: &mut String) {
    let indent = " -".repeat(depth);
    String::push_str(
        out,
        &format!("{} {} {}\n", indent, task.status.icon(), task.title),
    );

    let mut children = all
        .iter()
        .filter(|t| t.parent == Some(task.id))
        .collect::<Vec<_>>();
    children.sort_by_key(|t| t.order);

    for child in children {
        render_task(child, all, depth + 1, out);
    }
}
