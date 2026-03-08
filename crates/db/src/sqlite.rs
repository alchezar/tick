//! SQLite-backed repository implementation.

use core::cell::RefCell;

use chrono::{DateTime, Utc};
use rusqlite::{Connection, Error, Result, Row, params, types::Type};
use uuid::Uuid;

use crate::schema;
use domain::{
    error::{CoreError, CoreResult, DbError, DbResult},
    model::Project,
    repository::{ProjectRepository, TransactionGuard, Transactional},
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

/// Converts a `rusqlite::Error` into a [`DbError::Query`].
#[allow(clippy::needless_pass_by_value)]
fn db_err(e: Error) -> DbError {
    DbError::Query(e.to_string())
}

/// Converts a `rusqlite::Error` into a [`CoreError::Storage`].
#[allow(clippy::needless_pass_by_value)]
fn core_err(e: Error) -> CoreError {
    CoreError::Storage(db_err(e))
}

/// Maps a single row to a [`Project`].
fn row_to_project(row: &Row<'_>) -> Result<Project> {
    let id: String = row.get(0)?;
    let slug: String = row.get(1)?;
    let title: Option<String> = row.get(2)?;
    let created_at: String = row.get(3)?;

    let id = Uuid::parse_str(&id)
        .map_err(|e| Error::FromSqlConversionFailure(0, Type::Text, Box::new(e)))?;
    let created = created_at
        .parse::<DateTime<Utc>>()
        .map_err(|e| Error::FromSqlConversionFailure(0, Type::Text, Box::new(e)))?;

    Ok(Project {
        id,
        slug,
        title,
        created,
    })
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
