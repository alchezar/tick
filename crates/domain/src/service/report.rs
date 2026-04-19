//! Business logic for standup report generation.

use std::collections::HashSet;

use chrono::{Datelike, Duration, NaiveDate, NaiveTime, Weekday};

use crate::{
    error::CoreResult,
    model::{Project, ProjectId, Status, Task, TaskId},
    repository::{ProjectRepository, TaskFilter, TaskRepository, TransactionGuard, Transactional},
};

/// Structured output of a generated standup report for a single project.
#[derive(Debug)]
pub struct Report {
    /// Project display title (falls back to slug when title is absent).
    pub title: String,
    /// Optional GitHub repository URL for building PR links.
    pub github_url: Option<String>,
    /// Tasks active at end-of-yesterday or with status changes between the
    /// previous workday and yesterday (inclusive), with statuses reconstructed
    /// as of end-of-yesterday.
    pub prev: Vec<Task>,
    /// Morning plan: tasks with their status as of end-of-previous-day.
    pub today: Vec<Task>,
    /// Actual state: tasks with their real end-of-day status.
    pub current: Vec<Task>,
}

impl Report {
    /// Creates a new report for the given date.
    #[must_use]
    pub fn new(
        title: impl Into<String>,
        github_url: Option<String>,
        prev: Vec<Task>,
        today: Vec<Task>,
        current: Vec<Task>,
    ) -> Self {
        Self {
            title: title.into(),
            github_url,
            prev,
            today,
            current,
        }
    }

    /// Renders the report as a formatted string ready to paste into a chat.
    ///
    /// Output starts with the project title, followed by three sections:
    /// - **Previously** - tasks updated on the previous workday (real icons).
    /// - **Today** - morning plan: tasks with their status as of end-of-previous-day.
    /// - **Current** - actual state of today's tasks (real icons).
    ///
    /// Returns an empty string when the project has no relevant tasks.
    #[must_use]
    pub fn render(&self, show_title: bool, include_current: bool) -> String {
        let mut body = String::new();

        if let Some(url) = &self.github_url {
            let links = render_pr_links(&self.current, url);
            if !links.is_empty() {
                body.push_str(&links);
                body.push_str("\n\n");
            }
        }

        if !self.prev.is_empty() {
            body.push_str(" Previously:\n");
            body.push_str(&render_section(&self.prev));
        }

        if !self.today.is_empty() {
            if !body.is_empty() {
                body.push('\n');
            }
            body.push_str(" Today:\n");
            body.push_str(&render_section(&self.today));

            if include_current {
                body.push('\n');
                body.push_str(" Current:\n");
                body.push_str(&render_section(&self.current));
            }
        }

        if body.is_empty() {
            return body;
        }

        let title = if show_title {
            format!("{}\n\n", &self.title)
        } else {
            String::new()
        };
        format!("{title}{body}")
    }
}

/// Combines multiple per-project reports into a single output string.
///
/// Empty reports (projects with no relevant tasks) are skipped.
#[must_use]
pub fn render_all(reports: &[Report], include_current: bool) -> String {
    reports
        .iter()
        .map(|r| r.render(true, include_current))
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
    pub async fn generate(&self, date: NaiveDate, project: &Project) -> CoreResult<Report> {
        let title = project.title.as_deref().unwrap_or(&project.slug);
        let tx = self.repo.begin_transaction().await?;

        let (today, current) = self.tasks_today(date, &project.id).await?;
        let report = Report::new(
            title,
            project.github_url.clone(),
            self.tasks_prev(date, &project.id).await?,
            today,
            current,
        );

        tx.commit_transaction().await?;
        Ok(report)
    }

    /// Generates standup reports for all projects on the given date.
    ///
    /// # Errors
    /// Returns an error if the repository operation fails.
    pub async fn generate_all(&self, date: NaiveDate) -> CoreResult<Vec<Report>> {
        let tx = self.repo.begin_transaction().await?;

        let mut reports = Vec::new();
        for project in self.repo.list_projects().await? {
            reports.push(self.generate(date, &project).await?);
        }

        tx.commit_transaction().await?;
        Ok(reports)
    }

    // -------------------------------------------------------------------------

    /// Returns tasks for the Previously section.
    ///
    /// Covers the whole span since the previous workday: for Monday this
    /// aggregates Friday, Saturday and Sunday; for Sunday - Friday and
    /// Saturday; for Saturday - Friday; for other days just the previous day.
    /// Each task is shown with its end-of-yesterday status.
    async fn tasks_prev(&self, date: NaiveDate, project_id: &ProjectId) -> CoreResult<Vec<Task>> {
        let start = prev_workday(date);
        let end = date - Duration::days(1);

        let mut changed_ids = HashSet::new();
        let mut cur = start;
        while cur <= end {
            for change in self.repo.list_task_changes_on(cur).await? {
                changed_ids.insert(change.task_id);
            }
            cur += Duration::days(1);
        }

        let mut seen = HashSet::new();
        let mut tasks = Vec::new();
        for task in self
            .repo
            .list_tasks(&TaskFilter::CreatedBefore(*project_id, end))
            .await?
        {
            let status = self.status_at(&task.id, end).await?;
            if (status.is_active() || changed_ids.contains(&task.id))
                && status.is_reportable()
                && seen.insert(task.id)
            {
                tasks.push(task.with_status(status));
            }
        }
        Ok(tasks)
    }

    /// Returns tasks for the Today (morning plan) and Current sections.
    ///
    /// Today tasks have their status as of end-of-previous-day.
    /// Current tasks have their real end-of-day status.
    async fn tasks_today(
        &self,
        date: NaiveDate,
        project_id: &ProjectId,
    ) -> CoreResult<(Vec<Task>, Vec<Task>)> {
        let current = self.tasks_on(date, project_id).await?;
        let yesterday = date - Duration::days(1);

        let mut today = Vec::new();
        for task in &current {
            let morning = self.status_at(&task.id, yesterday).await?;
            today.push(task.clone().with_status(morning));
        }

        Ok((today, current))
    }

    /// Builds a snapshot of tasks relevant to `date`:
    /// those that were active or had a status change on `date`,
    /// each with their end-of-day status.
    async fn tasks_on(&self, date: NaiveDate, project_id: &ProjectId) -> CoreResult<Vec<Task>> {
        let changed_ids = self
            .repo
            .list_task_changes_on(date)
            .await?
            .iter()
            .map(|c| c.task_id)
            .collect::<HashSet<_>>();

        let mut seen = HashSet::new();
        let mut tasks = Vec::new();
        for task in self
            .repo
            .list_tasks(&TaskFilter::CreatedBefore(*project_id, date))
            .await?
        {
            let status = self.status_at(&task.id, date).await?;
            if (status.is_active() || changed_ids.contains(&task.id))
                && status.is_reportable()
                && seen.insert(task.id)
            {
                tasks.push(task.with_status(status));
            }
        }
        Ok(tasks)
    }

    /// Reconstructs the status a task had at end-of-day on `date`.
    ///
    /// Replays all status changes up to (and including) `date`.
    /// Returns `NotStarted` if the task had no changes by that date.
    async fn status_at(&self, task_id: &TaskId, date: NaiveDate) -> CoreResult<Status> {
        let next_day = date + Duration::days(1);
        let cutoff = next_day.and_time(NaiveTime::MIN).and_utc();

        let status = self
            .repo
            .list_task_changes(task_id)
            .await?
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
#[must_use]
#[doc(hidden)]
pub fn prev_workday(date: NaiveDate) -> NaiveDate {
    match date.weekday() {
        Weekday::Mon => date - Duration::days(3),
        Weekday::Sun => date - Duration::days(2),
        _ => date - Duration::days(1),
    }
}

/// Renders a flat list of tasks as an indented hierarchy string.
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
        &format!("{} {} {}\n", indent, task.status().icon(), task.title),
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

/// Renders PR links for active tasks that have a pull request number.
fn render_pr_links(tasks: &[Task], github_url: &str) -> String {
    let mut links = tasks
        .iter()
        .filter(|t| t.status().is_active() && t.pull_request_number.is_some())
        .filter_map(|t| {
            t.pull_request_number
                .map(|n| format!("{github_url}/pull/{n}"))
        })
        .collect::<Vec<_>>();
    links.sort();
    links.dedup();
    links.join("\n")
}
