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
    /// Tasks whose `updated_at` falls on the previous workday.
    pub prev: Vec<Task>,
    /// Tasks with status changes on weekend days falling strictly between
    /// the previous workday and today. Empty when there is no weekend span
    /// to report (Tuesday-Saturday) or when no changes happened on those days.
    pub weekend: Vec<Task>,
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
        weekend: Vec<Task>,
        today: Vec<Task>,
        current: Vec<Task>,
    ) -> Self {
        Self {
            title: title.into(),
            github_url,
            prev,
            weekend,
            today,
            current,
        }
    }

    /// Renders the report as a formatted string ready to paste into a chat.
    ///
    /// Output starts with the project title, followed by:
    /// - **Previously** - tasks updated on the previous workday.
    /// - **Weekend** - tasks with status changes on weekend days since the
    ///   previous workday. Only rendered when non-empty.
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

        if !self.weekend.is_empty() {
            if !body.is_empty() {
                body.push('\n');
            }
            body.push_str(" Weekend:\n");
            body.push_str(&render_section(&self.weekend));
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
        .map(|report| report.render(true, include_current))
        .filter(|string| !string.is_empty())
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
    ///
    /// Returns an error if the repository operation fails.
    pub async fn generate(&self, date: NaiveDate, project: &Project) -> CoreResult<Report> {
        let title = project.title.as_deref().unwrap_or(&project.slug);
        let tx = self.repo.begin_transaction().await?;

        let (today, current) = self.tasks_today(date, &project.id).await?;
        let report = Report::new(
            title,
            project.github_url.clone(),
            self.tasks_prev(date, &project.id).await?,
            self.tasks_weekend(date, &project.id).await?,
            today,
            current,
        );

        tx.commit_transaction().await?;
        Ok(report)
    }

    /// Generates standup reports for all projects on the given date.
    ///
    /// # Errors
    ///
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
    /// All tasks that had a status change on the previous workday,
    /// shown with their status as of that day.
    async fn tasks_prev(&self, date: NaiveDate, project_id: &ProjectId) -> CoreResult<Vec<Task>> {
        let prev_day = prev_workday(date);
        self.tasks_on(prev_day, project_id).await
    }

    /// Returns tasks for the Weekend section.
    ///
    /// Mirrors [`Self::tasks_on`] semantics (active at end-of-yesterday or
    /// changed in the range), but the "changed" predicate is limited to
    /// weekend days (Sat/Sun) strictly between the previous workday and
    /// `date`. For Sunday reports this covers Saturday; for Monday - Saturday
    /// and Sunday. Returns an empty vector on other weekdays or when no
    /// weekend changes occurred - in that case the section is suppressed.
    async fn tasks_weekend(
        &self,
        date: NaiveDate,
        project_id: &ProjectId,
    ) -> CoreResult<Vec<Task>> {
        let prev = prev_workday(date);
        let yesterday = date - Duration::days(1);

        let mut changed_ids = HashSet::new();
        let mut cur = prev + Duration::days(1);
        while cur <= yesterday {
            if matches!(cur.weekday(), Weekday::Sat | Weekday::Sun) {
                for change in self.repo.list_task_changes_on(cur).await? {
                    changed_ids.insert(change.task_id);
                }
            }
            cur += Duration::days(1);
        }

        if changed_ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut seen = HashSet::new();
        let mut tasks = Vec::new();
        for task in self
            .repo
            .list_tasks(&TaskFilter::CreatedBefore(*project_id, yesterday))
            .await?
        {
            let status = self.status_at(&task.id, yesterday).await?;
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
            .map(|change| change.task_id)
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
            .find(|change| change.changed_at < cutoff)
            .map_or(Status::NotStarted, |change| change.new_status);

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
    let ids = tasks.iter().map(|task| task.id).collect::<HashSet<_>>();

    let mut roots = tasks
        .iter()
        .filter(|task| {
            task.parent
                .is_none_or(|parent_id| !ids.contains(&parent_id))
        })
        .collect::<Vec<_>>();
    roots.sort_by_key(|task| task.order);

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
        .filter(|child| child.parent == Some(task.id))
        .collect::<Vec<_>>();
    children.sort_by_key(|child| child.order);

    for child in children {
        render_task(child, all, depth + 1, out);
    }
}

/// Renders PR links for active tasks that have a pull request number.
fn render_pr_links(tasks: &[Task], github_url: &str) -> String {
    let mut links = tasks
        .iter()
        .filter(|task| task.status().is_active() && task.pull_request_number.is_some())
        .filter_map(|task| {
            task.pull_request_number
                .map(|num| format!("{github_url}/pull/{num}"))
        })
        .collect::<Vec<_>>();
    links.sort();
    links.dedup();
    links.join("\n")
}
