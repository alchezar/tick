//! SQLite-backed repository implementation.

use core::cell::RefCell;

use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use tokio::{runtime::Handle, task};
use uuid::Uuid;

use crate::schema;
use domain::{
    error::{CoreError, CoreResult, DbError, DbResult},
    model::{Project, ProjectId, Status, StatusChange, StatusChangeId, Task, TaskId},
    repository::{ProjectRepository, TaskFilter, TaskRepository, TransactionGuard, Transactional},
};

/// `SQLite` repository backed by a connection pool.
///
/// Uses [`RefCell`] for transaction nesting depth because repository traits use `&self`.
#[derive(Debug, Clone)]
pub struct SqliteRepo {
    pool: SqlitePool,
    depth: RefCell<usize>,
}

impl SqliteRepo {
    /// Opens (or creates) a `SQLite` database at the given URL and runs migrations.
    ///
    /// # Errors
    /// Returns [`DbError`] if the connection or migration fails.
    pub async fn open(url: &str) -> DbResult<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(url)
            .await
            .map_err(db_err)?;

        sqlx::raw_sql("PRAGMA journal_mode = WAL")
            .execute(&pool)
            .await
            .map_err(db_err)?;
        sqlx::raw_sql("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .map_err(db_err)?;

        schema::migrate(&pool).await?;

        Ok(Self {
            pool,
            depth: RefCell::new(0),
        })
    }

    /// Opens the default `SQLite` database at the XDG data directory.
    ///
    /// Path: `~/.local/share/tt/tt.db` (or platform equivalent).
    /// Creates the directory if it does not exist.
    /// Respects `DATABASE_URL` env var as an override.
    ///
    /// # Errors
    /// Returns [`DbError`] if the directory cannot be created or the connection fails.
    pub async fn open_default() -> DbResult<Self> {
        if let Ok(url) = std::env::var("DATABASE_URL") {
            return Self::open(&url).await;
        }

        let data_dir = dirs::data_dir()
            .ok_or_else(|| DbError::Query("cannot determine data directory".to_owned()))?;
        let db_dir = data_dir.join("tt");
        std::fs::create_dir_all(&db_dir).map_err(|e| DbError::Query(e.to_string()))?;

        let path = db_dir.join("tt.db");
        let encoded = path.to_string_lossy().replace(' ', "%20");
        Self::open(&format!("sqlite:{encoded}?mode=rwc")).await
    }

    /// Creates an in-memory `SQLite` database with migrations applied.
    ///
    /// Useful for testing.
    ///
    /// # Errors
    /// Returns [`DbError`] if the connection or migration fails.
    pub async fn in_memory() -> DbResult<Self> {
        Self::open("sqlite::memory:").await
    }
}

impl Transactional for SqliteRepo {
    type Guard<'a> = SqliteGuard<'a>;

    async fn begin_transaction(&self) -> CoreResult<Self::Guard<'_>> {
        let need_begin = *self.depth.borrow() == 0;
        if need_begin {
            sqlx::raw_sql("BEGIN")
                .execute(&self.pool)
                .await
                .map_err(db_err)?;
        }
        *self.depth.borrow_mut() += 1;

        Ok(SqliteGuard::new(self))
    }
}

/// RAII transaction guard for [`SqliteRepo`].
///
/// Dropping without calling [`commit_transaction`](TransactionGuard::commit_transaction)
/// triggers a rollback (only at the outermost level).
#[derive(Debug)]
pub struct SqliteGuard<'a> {
    repo: &'a SqliteRepo,
    committed: bool,
}

impl<'a> SqliteGuard<'a> {
    /// Creates a new uncommitted guard tied to the given repo.
    #[must_use]
    pub fn new(repo: &'a SqliteRepo) -> Self {
        Self {
            repo,
            committed: false,
        }
    }
}

impl TransactionGuard for SqliteGuard<'_> {
    async fn commit_transaction(mut self) -> CoreResult<()> {
        self.committed = true;
        let need_commit = {
            let mut depth = self.repo.depth.borrow_mut();
            *depth -= 1;
            *depth == 0
        };
        if need_commit {
            sqlx::raw_sql("COMMIT")
                .execute(&self.repo.pool)
                .await
                .map_err(db_err)?;
        }

        Ok(())
    }
}

impl Drop for SqliteGuard<'_> {
    fn drop(&mut self) {
        if self.committed {
            return;
        }

        let mut depth = self.repo.depth.borrow_mut();
        *depth -= 1;

        if *depth == 0 {
            // Cannot use async in Drop; use blocking approach via the pool's runtime.
            let pool = self.repo.pool.clone();
            task::block_in_place(|| {
                Handle::current().block_on(async {
                    let _ = sqlx::raw_sql("ROLLBACK").execute(&pool).await;
                });
            });
        }
    }
}

/// Converts any error into a [`DbError::Query`].
#[allow(clippy::needless_pass_by_value)]
fn db_err(e: impl ToString) -> DbError {
    DbError::Query(e.to_string())
}

/// Converts any error into a [`CoreError::Storage`].
#[allow(clippy::needless_pass_by_value)]
fn core_err(e: impl ToString) -> CoreError {
    CoreError::Storage(db_err(e))
}

/// Intermediate row for projects.
struct ProjectRow {
    id: String,
    slug: String,
    title: Option<String>,
    github_url: Option<String>,
    created_at: String,
}

impl TryFrom<ProjectRow> for Project {
    type Error = CoreError;

    fn try_from(r: ProjectRow) -> CoreResult<Self> {
        Ok(Self {
            id: ProjectId::from(Uuid::parse_str(&r.id).map_err(core_err)?),
            slug: r.slug,
            title: r.title,
            github_url: r.github_url,
            created: r.created_at.parse::<DateTime<Utc>>().map_err(core_err)?,
        })
    }
}

/// Intermediate row for tasks.
struct TaskRow {
    id: String,
    project_id: String,
    title: String,
    status: String,
    parent_id: Option<String>,
    display_order: Option<i64>,
    pull_request: Option<i64>,
    branch_name: Option<String>,
    created_at: String,
    updated_at: String,
}

impl TryFrom<TaskRow> for Task {
    type Error = CoreError;

    fn try_from(r: TaskRow) -> CoreResult<Self> {
        let id = TaskId::from(Uuid::parse_str(&r.id).map_err(core_err)?);
        let project_id = ProjectId::from(Uuid::parse_str(&r.project_id).map_err(core_err)?);
        let parent = r
            .parent_id
            .as_deref()
            .map(|s| Uuid::parse_str(s).map(TaskId::from))
            .transpose()
            .map_err(core_err)?;
        let status = r.status.parse::<Status>().map_err(core_err)?;

        let mut task = Task::new(r.title, parent, project_id).with_status(status);
        task.id = id;
        task.order = r
            .display_order
            .map(usize::try_from)
            .transpose()
            .map_err(core_err)?;
        task.pull_request_number = r
            .pull_request
            .map(u32::try_from)
            .transpose()
            .map_err(core_err)?;
        task.branch_name = r.branch_name;
        task.created = r.created_at.parse::<DateTime<Utc>>().map_err(core_err)?;
        task.updated = r.updated_at.parse::<DateTime<Utc>>().map_err(core_err)?;

        Ok(task)
    }
}

/// Intermediate row for status changes.
struct StatusChangeRow {
    id: String,
    task_id: String,
    old_status: String,
    new_status: String,
    changed_at: String,
}

impl TryFrom<StatusChangeRow> for StatusChange {
    type Error = CoreError;

    fn try_from(r: StatusChangeRow) -> CoreResult<Self> {
        Ok(Self {
            id: StatusChangeId::from(Uuid::parse_str(&r.id).map_err(core_err)?),
            task_id: TaskId::from(Uuid::parse_str(&r.task_id).map_err(core_err)?),
            old_status: r.old_status.parse::<Status>().map_err(core_err)?,
            new_status: r.new_status.parse::<Status>().map_err(core_err)?,
            changed_at: r.changed_at.parse::<DateTime<Utc>>().map_err(core_err)?,
        })
    }
}

impl ProjectRepository for SqliteRepo {
    async fn save_project(&self, project: &Project) -> CoreResult<()> {
        let id = project.id.to_string();
        let created = project.created.to_rfc3339();
        sqlx::query!(
            r"
                INSERT INTO projects (id, slug, title, github_url, created_at)
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT(id) DO UPDATE SET
                    slug = excluded.slug,
                    title = excluded.title,
                    github_url = excluded.github_url
            ",
            id,
            project.slug,
            project.title,
            project.github_url,
            created,
        )
        .execute(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(())
    }

    async fn find_project_by_id(&self, id: &ProjectId) -> CoreResult<Option<Project>> {
        let id = id.to_string();
        sqlx::query_as!(
            ProjectRow,
            r"
                SELECT id, slug, title, github_url, created_at
                FROM projects
                WHERE id = $1
            ",
            id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?
        .map(Project::try_from)
        .transpose()
    }

    async fn find_project_by_slug(&self, slug: &str) -> CoreResult<Option<Project>> {
        sqlx::query_as!(
            ProjectRow,
            r"
                SELECT id, slug, title, github_url, created_at
                FROM projects
                WHERE slug = $1
            ",
            slug,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?
        .map(Project::try_from)
        .transpose()
    }

    async fn list_projects(&self) -> CoreResult<Vec<Project>> {
        sqlx::query_as!(
            ProjectRow,
            r"
                SELECT id, slug, title, github_url, created_at
                FROM projects
                ORDER BY created_at
            ",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?
        .into_iter()
        .map(Project::try_from)
        .collect()
    }

    async fn delete_project(&self, project_id: &ProjectId) -> CoreResult<()> {
        let id = project_id.to_string();
        sqlx::query!(
            r"
                DELETE FROM projects
                WHERE id = $1
            ",
            id,
        )
        .execute(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(())
    }
}

impl TaskRepository for SqliteRepo {
    async fn save_task(&self, task: &Task) -> CoreResult<()> {
        let id = task.id.to_string();
        let project_id = task.project_id.to_string();
        let status = task.status().as_str().to_owned();
        let parent_id = task.parent.map(|p| p.to_string());
        let order = task.order.map(i64::try_from).transpose().map_err(db_err)?;
        let pull_request = task.pull_request_number.map(i64::from);
        let branch_name = task.branch_name.clone();
        let created = task.created.to_rfc3339();
        let updated = task.updated.to_rfc3339();
        sqlx::query!(
            r"
                INSERT INTO tasks (id, project_id, title, status, parent_id, display_order, pull_request, branch_name, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                ON CONFLICT(id) DO UPDATE SET
                    title = excluded.title,
                    status = excluded.status,
                    parent_id = excluded.parent_id,
                    display_order = excluded.display_order,
                    pull_request = excluded.pull_request,
                    branch_name = excluded.branch_name,
                    updated_at = excluded.updated_at
            ",
            id,
            project_id,
            task.title,
            status,
            parent_id,
            order,
            pull_request,
            branch_name,
            created,
            updated,
        )
        .execute(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(())
    }

    async fn find_task_by_id(&self, id: &TaskId) -> CoreResult<Option<Task>> {
        let id = id.to_string();
        sqlx::query_as!(
            TaskRow,
            r"
                SELECT id, project_id, title, status, parent_id, display_order, pull_request, branch_name, created_at, updated_at
                FROM tasks
                WHERE id = $1
            ",
            id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?
        .map(Task::try_from)
        .transpose()
    }

    async fn find_task_by_id_prefix(&self, id_prefix: &str) -> CoreResult<Option<TaskId>> {
        let pattern = format!("{id_prefix}%");

        sqlx::query_as!(
            TaskRow,
            r"
                SELECT id, project_id, title, status, parent_id, display_order, pull_request, branch_name, created_at, updated_at
                FROM tasks
                WHERE id LIKE $1
                LIMIT 1
            ",
            pattern,
        )
          .fetch_optional(&self.pool)
          .await
          .map_err(db_err)?
          .map(|r| Uuid::parse_str(&r.id).map(TaskId::from).map_err(core_err))
          .transpose()
    }

    async fn child_tasks_of(&self, parent: &TaskId) -> CoreResult<Vec<Task>> {
        let parent = parent.to_string();
        sqlx::query_as!(
            TaskRow,
            r"
                SELECT id, project_id, title, status, parent_id, display_order, pull_request, branch_name, created_at, updated_at
                FROM tasks
                WHERE parent_id = $1
                ORDER BY display_order
            ",
            parent,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?
        .into_iter()
        .map(Task::try_from)
        .collect()
    }

    async fn list_tasks(&self, filter: &TaskFilter) -> CoreResult<Vec<Task>> {
        match filter {
            TaskFilter::ByProject(id) => {
                let id = id.to_string();
                sqlx::query_as!(
                    TaskRow,
                    r"
                        SELECT id, project_id, title, status, parent_id, display_order, pull_request, branch_name, created_at, updated_at
                        FROM tasks
                        WHERE project_id = $1
                        ORDER BY display_order
                    ",
                    id,
                )
                .fetch_all(&self.pool)
                .await
                .map_err(db_err)?
                .into_iter()
                .map(Task::try_from)
                .collect()
            }
            TaskFilter::RootByProject(project_id) => {
                let project_id = project_id.to_string();
                sqlx::query_as!(
                        TaskRow,
                        r"
                            SELECT id, project_id, title, status, parent_id, display_order, pull_request, branch_name, created_at, updated_at
                            FROM tasks
                            WHERE project_id = $1 AND parent_id IS NULL
                            ORDER BY display_order
                        ",
                        project_id,
                    )
                  .fetch_all(&self.pool)
                  .await
                  .map_err(db_err)?
                  .into_iter()
                  .map(Task::try_from)
                  .collect()
            }
            TaskFilter::ChildrenOf(parent_id) => {
                let parent_id = parent_id.to_string();
                sqlx::query_as!(
                    TaskRow,
                    r"
                        SELECT id, project_id, title, status, parent_id, display_order, pull_request, branch_name, created_at, updated_at
                        FROM tasks
                        WHERE parent_id = $1
                        ORDER BY display_order
                    ",
                    parent_id,
                )
                .fetch_all(&self.pool)
                .await
                .map_err(db_err)?
                .into_iter()
                .map(Task::try_from)
                .collect()
            }
            TaskFilter::ActiveByProject(project_id, date) => {
                let project_id = project_id.to_string();
                let date = date.to_string();
                sqlx::query_as!(
                    TaskRow,
                    r"
                        SELECT t.id, t.project_id, t.title, t.status, t.parent_id, t.display_order, t.pull_request, t.branch_name, t.created_at, t.updated_at
                        FROM tasks t
                        WHERE t.project_id = $1
                          AND (
                            t.status IN ('not_started', 'in_progress', 'blocked')
                            OR (t.status IN ('done', 'abandoned') AND substr(t.updated_at, 1, 10) = $2)
                          )
                        ORDER BY t.display_order
                    ",
                    project_id,
                    date,
                )
                .fetch_all(&self.pool)
                .await
                .map_err(db_err)?
                .into_iter()
                .map(Task::try_from)
                .collect()
            }
            TaskFilter::CreatedBefore(project_id, date) => {
                let project_id = project_id.to_string();
                let date = date.to_string();
                sqlx::query_as!(
                    TaskRow,
                    r"
                        SELECT id, project_id, title, status, parent_id, display_order, pull_request, branch_name, created_at, updated_at
                        FROM tasks
                        WHERE project_id = $1 AND substr(created_at, 1, 10) <= $2
                        ORDER BY display_order
                    ",
                    project_id,
                    date,
                )
                .fetch_all(&self.pool)
                .await
                .map_err(db_err)?
                .into_iter()
                .map(Task::try_from)
                .collect()
            }
        }
    }

    async fn delete_task(&self, id: &TaskId) -> CoreResult<()> {
        let id = id.to_string();
        sqlx::query!(
            r"
                DELETE FROM tasks
                WHERE id = $1
            ",
            id,
        )
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn delete_all_tasks_by(&self, project_id: &ProjectId) -> CoreResult<()> {
        let project_id = project_id.to_string();
        sqlx::query!(
            r"
                DELETE FROM tasks
                WHERE project_id = $1
            ",
            project_id,
        )
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn save_task_change(&self, change: &StatusChange) -> CoreResult<()> {
        let id = change.id.to_string();
        let task_id = change.task_id.to_string();
        let old_status = change.old_status.as_str().to_owned();
        let new_status = change.new_status.as_str().to_owned();
        let changed_at = change.changed_at.to_rfc3339();
        sqlx::query!(
            r"
                INSERT INTO status_changes (id, task_id, old_status, new_status, changed_at)
                VALUES ($1, $2, $3, $4, $5)
            ",
            id,
            task_id,
            old_status,
            new_status,
            changed_at,
        )
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn list_task_changes(&self, task_id: &TaskId) -> CoreResult<Vec<StatusChange>> {
        let task_id = task_id.to_string();
        sqlx::query_as!(
            StatusChangeRow,
            r"
                SELECT id, task_id, old_status, new_status, changed_at
                FROM status_changes
                WHERE task_id = $1
                ORDER BY changed_at
            ",
            task_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?
        .into_iter()
        .map(StatusChange::try_from)
        .collect()
    }

    async fn list_task_changes_on(&self, date: NaiveDate) -> CoreResult<Vec<StatusChange>> {
        let start = date.and_time(NaiveTime::MIN).and_utc().to_rfc3339();
        let end = (date + Duration::days(1))
            .and_time(NaiveTime::MIN)
            .and_utc()
            .to_rfc3339();
        sqlx::query_as!(
            StatusChangeRow,
            r"
                SELECT id, task_id, old_status, new_status, changed_at
                FROM status_changes
                WHERE changed_at >= $1 AND changed_at < $2
                ORDER BY changed_at
            ",
            start,
            end,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?
        .into_iter()
        .map(StatusChange::try_from)
        .collect()
    }

    async fn delete_task_changes_after(
        &self,
        task_id: &TaskId,
        after: DateTime<Utc>,
    ) -> CoreResult<()> {
        let task_id = task_id.to_string();
        let after = after.to_rfc3339();
        sqlx::query!(
            r"
                DELETE FROM status_changes
                WHERE task_id = $1 AND changed_at > $2
            ",
            task_id,
            after,
        )
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }
}
