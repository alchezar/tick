//! Business logic for standup report generation.

use std::collections::HashSet;

use chrono::{Datelike, Duration, NaiveDate, Weekday};
use uuid::Uuid;

use crate::{
    error::CoreResult,
    model::{Project, Status, Task},
    repository::{ProjectRepository, TaskRepository, TransactionGuard, Transactional},
};

/// Structured output of a generated standup report for a single project.
#[derive(Debug)]
pub struct Report {
    /// Project display title (falls back to slug when title is absent).
    pub title: String,
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
    pub fn new(
        title: impl Into<String>,
        prev: Vec<Task>,
        current: Vec<Task>,
        date: NaiveDate,
    ) -> Self {
        Self {
            title: title.into(),
            prev,
            current,
            date,
        }
    }

    /// Renders the report as a formatted string ready to paste into a chat.
    ///
    /// Output starts with the project title, followed by three sections:
    /// - **Previously** - tasks updated on the previous workday (real icons).
    /// - **Today** - morning plan: tasks created or completed today show ❌,
    ///   others keep their real icon.
    /// - **Current** - actual state of today's tasks (real icons).
    ///
    /// Returns an empty string when the project has no relevant tasks.
    #[inline]
    #[must_use]
    pub fn render(&self) -> String {
        let mut body = String::new();

        if !self.prev.is_empty() {
            body.push_str("Previously:\n");
            body.push_str(&render_section(&self.prev, None));
        }

        if !self.current.is_empty() {
            if !body.is_empty() {
                body.push('\n');
            }
            body.push_str("Today:\n");
            body.push_str(&render_section(&self.current, Some(self.date)));

            body.push('\n');
            body.push_str("Current:\n");
            body.push_str(&render_section(&self.current, None));
        }

        if body.is_empty() {
            return body;
        }

        format!("{}\n\n{}", self.title, body)
    }
}

/// Combines multiple per-project reports into a single output string.
///
/// Empty reports (projects with no relevant tasks) are skipped.
#[inline]
#[must_use]
pub fn render_all(reports: &[Report]) -> String {
    reports
        .iter()
        .map(Report::render)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

// -----------------------------------------------------------------------------

/// Encapsulates all logic for building standup reports.
///
/// Fetches tasks from the repository and partitions them
/// into "Previously" and "Today" sections.
#[derive(Debug)]
pub struct ReportService<R>
where
    R: ProjectRepository + TaskRepository + Transactional,
{
    repo: R,
}

impl<R> ReportService<R>
where
    R: ProjectRepository + TaskRepository + Transactional,
{
    /// Creates a new `ReportService` with the given repository.
    #[inline]
    #[must_use]
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    /// Generates a standup report for the given project and date.
    ///
    /// Uses `project.title` as the report header, falling back to `project.slug`.
    ///
    /// # Errors
    /// Returns an error if the repository operation fails.
    #[inline]
    pub fn generate(&self, date: NaiveDate, project: &Project) -> CoreResult<Report> {
        let title = project.title.as_deref().unwrap_or(&project.slug);
        let tx = self.repo.begin_transaction()?;

        let report = Report::new(
            title,
            self.tasks_prev(date, &project.id)?,
            self.tasks_today(date, &project.id)?,
            date,
        );

        tx.commit_transaction()?;
        Ok(report)
    }

    /// Generates standup reports for all projects on the given date.
    ///
    /// # Errors
    /// Returns an error if the repository operation fails.
    #[inline]
    pub fn generate_all(&self, date: NaiveDate) -> CoreResult<Vec<Report>> {
        let tx = self.repo.begin_transaction()?;

        let reports = self
            .repo
            .list_projects()?
            .into_iter()
            .map(|project| self.generate(date, &project))
            .collect::<CoreResult<_>>()?;

        tx.commit_transaction()?;
        Ok(reports)
    }

    // -------------------------------------------------------------------------

    /// Returns tasks for the Previously section.
    ///
    /// All tasks that had a status change on the previous workday,
    /// shown with their status as of that day.
    fn tasks_prev(&self, date: NaiveDate, project_id: &Uuid) -> CoreResult<Vec<Task>> {
        let prev_day = prev_workday(date);
        self.tasks_snapshot(prev_day, project_id)
    }

    /// Returns tasks for the Today / Current sections.
    ///
    /// Includes tasks that were active on `date` or had a status change on `date`,
    /// each with status reconstructed from the change log.
    fn tasks_today(&self, date: NaiveDate, project_id: &Uuid) -> CoreResult<Vec<Task>> {
        self.tasks_snapshot(date, project_id)
    }

    /// Builds a snapshot of tasks relevant to `date`:
    /// those that were active on `date` or had a status change on `date`,
    /// each reconstructed with the status it had at end-of-day.
    fn tasks_snapshot(&self, date: NaiveDate, project_id: &Uuid) -> CoreResult<Vec<Task>> {
        let tx = self.repo.begin_transaction()?;
        let changed_ids = self
            .repo
            .list_task_changes_on(date)?
            .iter()
            .map(|c| c.task_id)
            .collect::<HashSet<_>>();

        let mut seen = HashSet::new();
        let mut tasks = Vec::new();
        for task in self
            .repo
            .list_tasks(project_id)?
            .into_iter()
            .filter(|task| task.created.date_naive() <= date)
        {
            let status = self.status_at(&task.id, date)?;
            if (status.is_active() || changed_ids.contains(&task.id)) && seen.insert(task.id) {
                tasks.push(task.with_status(status));
            }
        }
        tx.commit_transaction()?;
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
            .list_task_changes(task_id)?
            .iter()
            .rev()
            .find(|c| c.changed_at < cutoff)
            .map_or(Status::NotStarted, |c| c.new_status);

        Ok(status)
    }
}

/// Returns the previous workday for `date`.
///
/// Monday/Sunday -> Friday, Saturday -> Friday, other days -> previous day.
#[inline]
#[must_use]
#[doc(hidden)]
pub fn prev_workday(date: NaiveDate) -> NaiveDate {
    match date.weekday() {
        Weekday::Mon => date - Duration::days(3),
        Weekday::Sun => date - Duration::days(2),
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
