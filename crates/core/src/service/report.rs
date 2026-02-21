//! Business logic for standup report generation.

use chrono::{Datelike, Duration, NaiveDate, Weekday};

use crate::{domain::Task, error::CoreResult, repository::TaskRepository};

/// Structured output of a generated standup report.
#[derive(Debug)]
pub struct Report {
    /// Tasks that were `done` or `blocked` on the previous workday.
    pub prev: Vec<Task>,
    /// Tasks that are `not_started` or `in_progress` as of today.
    pub today: Vec<Task>,
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
fn prev_workday(date: NaiveDate) -> NaiveDate {
    match date.weekday() {
        Weekday::Mon => date - Duration::days(3),
        _ => date - Duration::days(1),
    }
}
