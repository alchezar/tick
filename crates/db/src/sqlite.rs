//! SQLite-backed repository implementation.

use core::cell::RefCell;
use std::error;

use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{Connection, Error, Result, Row, params, types::Type};
use uuid::Uuid;

use crate::schema;
use domain::{
    error::{CoreError, CoreResult, DbError, DbResult},
    model::{Project, Status, StatusChange, Task},
    repository::{ProjectRepository, TaskRepository, TransactionGuard, Transactional},
};

/// `SQLite` repository wrapping a single connection.
///
/// Uses [`RefCell`] for interior mutability because repository traits use `&self`.
/// Transaction nesting is tracked via a depth counter.
#[derive(Debug)]
pub struct SqliteRepo {
    connection: RefCell<Connection>,
    depth: RefCell<usize>,
}

impl SqliteRepo {
    /// Opens (or creates) a `SQLite` database at the given path and runs migrations.
    ///
    /// # Errors
    /// Returns [`DbError`] if the connection or migration fails.
    #[inline]
    pub fn open(path: &str) -> DbResult<Self> {
        let conn = Connection::open(path).map_err(db_err)?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(db_err)?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(db_err)?;
        schema::migrate(&conn)?;
        Ok(Self {
            connection: RefCell::new(conn),
            depth: RefCell::new(0),
        })
    }

    /// Creates an in-memory `SQLite` database with migrations applied.
    ///
    /// Useful for testing.
    ///
    /// # Errors
    /// Returns [`DbError`] if the connection or migration fails.
    #[inline]
    pub fn in_memory() -> DbResult<Self> {
        Self::open(":memory:")
    }

    /// Executes a raw SQL statement on the underlying connection.
    fn execute_sql(&self, sql: &str) -> DbResult<()> {
        self.connection.borrow().execute_batch(sql).map_err(db_err)
    }
}

impl Transactional for SqliteRepo {
    type Guard<'a> = SqliteGuard<'a>;

    #[inline]
    fn begin_transaction(&self) -> CoreResult<Self::Guard<'_>> {
        let mut depth = self.depth.borrow_mut();
        if *depth == 0 {
            self.execute_sql("BEGIN")?;
        }
        *depth += 1;

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
    #[inline]
    #[must_use]
    pub fn new(repo: &'a SqliteRepo) -> Self {
        Self {
            repo,
            committed: false,
        }
    }
}

impl TransactionGuard for SqliteGuard<'_> {
    #[inline]
    fn commit_transaction(mut self) -> CoreResult<()> {
        self.committed = true;
        let mut depth = self.repo.depth.borrow_mut();
        *depth -= 1;
        if *depth == 0 {
            self.repo.execute_sql("COMMIT")?;
        }

        Ok(())
    }
}

impl Drop for SqliteGuard<'_> {
    #[inline]
    fn drop(&mut self) {
        if self.committed {
            return;
        }

        let mut depth = self.repo.depth.borrow_mut();
        *depth -= 1;

        if *depth == 0 {
            let _ = self.repo.execute_sql("ROLLBACK");
        }
    }
}

impl ProjectRepository for SqliteRepo {
    #[inline]
    fn save_project(&self, project: &Project) -> CoreResult<()> {
        self.connection
            .borrow()
            .execute(
                "INSERT INTO projects (id, slug, title, created_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(id) DO UPDATE SET
                     slug = excluded.slug,
                     title = excluded.title",
                params![
                    project.id.to_string(),
                    project.slug,
                    project.title,
                    project.created.to_rfc3339(),
                ],
            )
            .map_err(db_err)?;

        Ok(())
    }

    #[inline]
    fn find_project_by_id(&self, id: &Uuid) -> CoreResult<Option<Project>> {
        self.connection
            .borrow()
            .prepare("SELECT id, slug, title, created_at FROM projects WHERE id = ?1")
            .map_err(db_err)?
            .query_map(params![id.to_string()], row_to_project)
            .map_err(db_err)?
            .next()
            .transpose()
            .map_err(core_err)
    }

    #[inline]
    fn find_project_by_slug(&self, slug: &str) -> CoreResult<Option<Project>> {
        self.connection
            .borrow()
            .prepare("SELECT id, slug, title, created_at FROM projects WHERE slug = ?1")
            .map_err(db_err)?
            .query_map(params![slug], row_to_project)
            .map_err(db_err)?
            .next()
            .transpose()
            .map_err(core_err)
    }

    #[inline]
    fn list_projects(&self) -> CoreResult<Vec<Project>> {
        self.connection
            .borrow()
            .prepare("SELECT id, slug, title, created_at FROM projects ORDER BY created_at")
            .map_err(db_err)?
            .query_map([], row_to_project)
            .map_err(db_err)?
            .collect::<Result<Vec<_>>>()
            .map_err(core_err)
    }

    #[inline]
    fn delete_project(&self, project_id: &Uuid) -> CoreResult<()> {
        self.connection
            .borrow()
            .execute(
                "DELETE FROM projects WHERE id = ?1",
                params![project_id.to_string()],
            )
            .map_err(db_err)?;

        Ok(())
    }
}

impl TaskRepository for SqliteRepo {
    #[inline]
    fn save_task(&self, task: &Task) -> CoreResult<()> {
        self.connection
            .borrow()
            .execute(
                "INSERT INTO tasks (id, project_id, title, status, parent_id, display_order, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(id) DO UPDATE SET
                     title = excluded.title,
                     status = excluded.status,
                     parent_id = excluded.parent_id,
                     display_order = excluded.display_order,
                     updated_at = excluded.updated_at",
                params![
                    task.id.to_string(),
                    task.project_id.to_string(),
                    task.title,
                    task.status().as_str(),
                    task.parent.map(|id| id.to_string()),
                    task.order.map(i64::try_from).transpose().map_err(db_err)?,
                    task.created.to_rfc3339(),
                    task.updated.to_rfc3339(),
                ],
            )
            .map_err(db_err)?;
        Ok(())
    }

    #[inline]
    fn find_task_by(&self, id: &Uuid) -> CoreResult<Option<Task>> {
        self.connection
            .borrow()
            .prepare(
                "SELECT id, project_id, title, status, parent_id, display_order, created_at, updated_at
                 FROM tasks WHERE id = ?1",
            )
            .map_err(db_err)?
            .query_map(params![id.to_string()], row_to_task)
            .map_err(db_err)?
            .next()
            .transpose()
            .map_err(core_err)
    }

    #[inline]
    fn child_tasks_of(&self, parent: &Uuid) -> CoreResult<Vec<Task>> {
        self.connection
            .borrow()
            .prepare(
                "SELECT id, project_id, title, status, parent_id, display_order, created_at, updated_at
                 FROM tasks WHERE parent_id = ?1 ORDER BY display_order",
            )
            .map_err(db_err)?
            .query_map(params![parent.to_string()], row_to_task)
            .map_err(db_err)?
            .collect::<Result<Vec<_>>>()
            .map_err(core_err)
    }

    #[inline]
    fn list_tasks(&self, project_id: &Uuid) -> CoreResult<Vec<Task>> {
        self.connection
            .borrow()
            .prepare(
                "SELECT id, project_id, title, status, parent_id, display_order, created_at, updated_at
                 FROM tasks WHERE project_id = ?1 ORDER BY display_order",
            )
            .map_err(db_err)?
            .query_map(params![project_id.to_string()], row_to_task)
            .map_err(db_err)?
            .collect::<Result<Vec<_>>>()
            .map_err(core_err)
    }

    #[inline]
    fn delete_task(&self, id: &Uuid) -> CoreResult<()> {
        self.connection
            .borrow()
            .execute("DELETE FROM tasks WHERE id = ?1", params![id.to_string()])
            .map_err(db_err)?;
        Ok(())
    }

    #[inline]
    fn delete_all_tasks_by(&self, project_id: &Uuid) -> CoreResult<()> {
        self.connection
            .borrow()
            .execute(
                "DELETE FROM tasks WHERE project_id = ?1",
                params![project_id.to_string()],
            )
            .map_err(db_err)?;
        Ok(())
    }

    #[inline]
    fn save_task_change(&self, change: &StatusChange) -> CoreResult<()> {
        self.connection
            .borrow()
            .execute(
                "INSERT INTO status_changes (id, task_id, old_status, new_status, changed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    change.id.to_string(),
                    change.task_id.to_string(),
                    change.old_status.as_str(),
                    change.new_status.as_str(),
                    change.changed_at.to_rfc3339(),
                ],
            )
            .map_err(db_err)?;
        Ok(())
    }

    #[inline]
    fn list_task_changes(&self, task_id: &Uuid) -> CoreResult<Vec<StatusChange>> {
        self.connection
            .borrow()
            .prepare(
                "SELECT id, task_id, old_status, new_status, changed_at
                 FROM status_changes WHERE task_id = ?1 ORDER BY changed_at",
            )
            .map_err(db_err)?
            .query_map(params![task_id.to_string()], row_to_status_change)
            .map_err(db_err)?
            .collect::<Result<Vec<_>>>()
            .map_err(core_err)
    }

    #[inline]
    fn list_task_changes_on(&self, date: NaiveDate) -> CoreResult<Vec<StatusChange>> {
        let start = date
            .and_hms_opt(0, 0, 0)
            .expect("valid midnight")
            .and_utc()
            .to_rfc3339();
        let end = date
            .succ_opt()
            .expect("valid next day")
            .and_hms_opt(0, 0, 0)
            .expect("valid midnight")
            .and_utc()
            .to_rfc3339();
        self.connection
            .borrow()
            .prepare(
                "SELECT id, task_id, old_status, new_status, changed_at
                 FROM status_changes WHERE changed_at >= ?1 AND changed_at < ?2 ORDER BY changed_at",
            )
            .map_err(db_err)?
            .query_map(params![start, end], row_to_status_change)
            .map_err(db_err)?
            .collect::<Result<Vec<_>>>()
            .map_err(core_err)
    }
}

// -----------------------------------------------------------------------------

/// Converts a `rusqlite::Error` into a [`DbError::Query`].
#[allow(clippy::needless_pass_by_value)]
fn db_err(e: impl ToString) -> DbError {
    DbError::Query(e.to_string())
}

/// Converts a `rusqlite::Error` into a [`CoreError::Storage`].
#[allow(clippy::needless_pass_by_value)]
fn core_err(e: Error) -> CoreError {
    CoreError::Storage(db_err(e))
}

/// Wraps a parse error as [`Error::FromSqlConversionFailure`].
fn parse_err<E: error::Error + Send + Sync + 'static>(e: E) -> Error {
    Error::FromSqlConversionFailure(0, Type::Text, Box::new(e))
}

/// Maps a single row to a [`Project`].
fn row_to_project(row: &Row<'_>) -> Result<Project> {
    let id = row.get::<_, String>(0)?;
    let slug = row.get::<_, String>(1)?;
    let title = row.get::<_, Option<String>>(2)?;
    let created_at = row.get::<_, String>(3)?;

    let id = Uuid::parse_str(&id).map_err(parse_err)?;
    let created = created_at.parse::<DateTime<Utc>>().map_err(parse_err)?;

    Ok(Project {
        id,
        slug,
        title,
        created,
    })
}

/// Maps a single row to a [`Task`].
fn row_to_task(row: &Row<'_>) -> Result<Task> {
    let id = row.get::<_, String>(0)?;
    let project_id = row.get::<_, String>(1)?;
    let title = row.get::<_, String>(2)?;
    let status = row.get::<_, String>(3)?;
    let parent_id = row.get::<_, Option<String>>(4)?;
    let order = row.get::<_, Option<i64>>(5)?;
    let created_at = row.get::<_, String>(6)?;
    let updated_at = row.get::<_, String>(7)?;

    let id = Uuid::parse_str(&id).map_err(parse_err)?;
    let project_id = Uuid::parse_str(&project_id).map_err(parse_err)?;
    let parent = parent_id
        .as_deref()
        .map(Uuid::parse_str)
        .transpose()
        .map_err(parse_err)?;
    let created = created_at.parse::<DateTime<Utc>>().map_err(parse_err)?;
    let updated = updated_at.parse::<DateTime<Utc>>().map_err(parse_err)?;

    let status = status.parse::<Status>().map_err(parse_err)?;
    let mut task = Task::new(title, parent, project_id).with_status(status);
    task.id = id;
    task.order = order.map(usize::try_from).transpose().map_err(parse_err)?;
    task.created = created;
    task.updated = updated;

    Ok(task)
}

/// Maps a single row to a [`StatusChange`].
fn row_to_status_change(row: &Row<'_>) -> Result<StatusChange> {
    let id = row.get::<_, String>(0)?;
    let task_id = row.get::<_, String>(1)?;
    let old_status = row.get::<_, String>(2)?;
    let new_status = row.get::<_, String>(3)?;
    let changed_at = row.get::<_, String>(4)?;

    let id = Uuid::parse_str(&id).map_err(parse_err)?;
    let task_id = Uuid::parse_str(&task_id).map_err(parse_err)?;
    let changed_at = changed_at.parse::<DateTime<Utc>>().map_err(parse_err)?;

    Ok(StatusChange {
        id,
        task_id,
        old_status: old_status.parse::<Status>().map_err(parse_err)?,
        new_status: new_status.parse::<Status>().map_err(parse_err)?,
        changed_at,
    })
}
