//! Business logic for standup report generation.

use std::collections::HashSet;

use chrono::{Datelike, Duration, NaiveDate, Weekday};
use uuid::Uuid;

use crate::{
    error::CoreResult,
    model::{Status, Task},
    repository::TaskRepository,
};

/// Structured output of a generated standup report.
#[derive(Debug)]
pub struct Report {
    /// Tasks whose `updated_at` falls on the previous workday.
    pub prev: Vec<Task>,
    /// Active tasks (`not_started` / `in_progress`) plus any task updated today,
    /// deduplicated by id. Used for both Today (planned) and Current (actual) sections.
    pub current: Vec<Task>,
    /// Report date, used to determine "morning plan" icons in the Today section.
    date: NaiveDate,
}

impl Report {
    /// Creates a new report for the given date.
    #[inline]
    #[must_use]
    pub fn new(prev: Vec<Task>, current: Vec<Task>, date: NaiveDate) -> Self {
        Self {
            prev,
            current,
            date,
        }
    }

    /// Renders the report as a formatted string ready to paste into a chat.
    ///
    /// Three sections:
    /// - **Previously** — tasks updated on the previous workday (real icons).
    /// - **Today** — morning plan: tasks created or completed today show ❌,
    ///   others keep their real icon.
    /// - **Current** — actual state of today's tasks (real icons).
    #[inline]
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = String::new();

        if !self.prev.is_empty() {
            out.push_str("Previously:\n");
            out.push_str(&render_section(&self.prev, None));
        }

        if !self.current.is_empty() {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str("Today:\n");
            out.push_str(&render_section(&self.current, Some(self.date)));

            out.push('\n');
            out.push_str("Current:\n");
            out.push_str(&render_section(&self.current, None));
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
        Ok(Report::new(
            self.tasks_prev(date)?,
            self.tasks_today(date)?,
            date,
        ))
    }

    /// Returns tasks for the Previously section.
    ///
    /// All tasks that had a status change on the previous workday,
    /// shown with their status as of that day.
    fn tasks_prev(&self, date: NaiveDate) -> CoreResult<Vec<Task>> {
        let prev_day = prev_workday(date);
        self.tasks_snapshot(prev_day)
    }

    /// Returns tasks for the Today / Current sections.
    ///
    /// Includes tasks that were active on `date` or had a status change on `date`,
    /// each with status reconstructed from the change log.
    fn tasks_today(&self, date: NaiveDate) -> CoreResult<Vec<Task>> {
        self.tasks_snapshot(date)
    }

    /// Builds a snapshot of tasks relevant to `date`:
    /// those that were active on `date` or had a status change on `date`,
    /// each reconstructed with the status it had at end-of-day.
    fn tasks_snapshot(&self, date: NaiveDate) -> CoreResult<Vec<Task>> {
        let changed_ids = self
            .repo
            .list_status_changes_on(date)?
            .iter()
            .map(|c| c.task_id)
            .collect::<HashSet<_>>();

        let mut seen = HashSet::new();
        let tasks = self
            .repo
            .list_all()?
            .into_iter()
            .filter(|task| task.created.date_naive() <= date)
            .filter_map(|task| {
                let status = self.status_at(&task.id, date).ok()?;
                (status.is_active() || changed_ids.contains(&task.id))
                    .then_some(task.with_status(status))
            })
            .filter(|t| seen.insert(t.id))
            .collect();

        Ok(tasks)
    }

    /// Reconstructs the status a task had at end-of-day on `date`.
    ///
    /// Replays all status changes up to (and including) `date`.
    /// Returns `NotStarted` if the task had no changes by that date.
    fn status_at(&self, task_id: &Uuid, date: NaiveDate) -> CoreResult<Status> {
        let next_day = date + Duration::days(1);
        let cutoff = next_day
            .and_hms_opt(0, 0, 0)
            .expect("valid midnight")
            .and_utc();

        let status = self
            .repo
            .list_status_changes(task_id)?
            .iter()
            .rev()
            .find(|c| c.changed_at < cutoff)
            .map_or(Status::NotStarted, |c| c.new_status);

        Ok(status)
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

/// Returns the display icon for a task.
///
/// When `today` is `Some(date)`, applies "morning plan" logic:
/// tasks created on that date or already `Done` are shown as `NotStarted`.
fn task_icon(task: &Task, today: Option<NaiveDate>) -> &'static str {
    if let Some(date) = today
        && (task.created.date_naive() == date || task.status() == Status::Done)
    {
        return Status::NotStarted.icon();
    }

    task.status().icon()
}

/// Renders a flat list of tasks as an indented hierarchy string.
///
/// When `today` is `Some(date)`, icons follow the "morning plan" rule.
/// When `None`, real status icons are used.
fn render_section(tasks: &[Task], today: Option<NaiveDate>) -> String {
    let ids = tasks.iter().map(|t| t.id).collect::<HashSet<_>>();

    let mut roots = tasks
        .iter()
        .filter(|t| t.parent.is_none_or(|p| !ids.contains(&p)))
        .collect::<Vec<_>>();
    roots.sort_by_key(|t| t.order);

    let mut out = String::new();
    for root in roots {
        render_task(root, tasks, 1, today, &mut out);
    }
    out
}

/// Recursively appends a task and its children to `out`.
fn render_task(
    task: &Task,
    all: &[Task],
    depth: usize,
    today: Option<NaiveDate>,
    out: &mut String,
) {
    let indent = " -".repeat(depth);
    String::push_str(
        out,
        &format!("{} {} {}\n", indent, task_icon(task, today), task.title),
    );

    let mut children = all
        .iter()
        .filter(|t| t.parent == Some(task.id))
        .collect::<Vec<_>>();
    children.sort_by_key(|t| t.order);

    for child in children {
        render_task(child, all, depth + 1, today, out);
    }
}
