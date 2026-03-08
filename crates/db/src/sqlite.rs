//! SQLite-backed repository implementation.

use core::cell::RefCell;

use rusqlite::Connection;

use crate::schema;
use domain::{
    error::{CoreResult, DbError, DbResult},
    repository::{TransactionGuard, Transactional},
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
        let conn = Connection::open(path).map_err(|e| DbError::Query(e.to_string()))?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| DbError::Query(e.to_string()))?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(|e| DbError::Query(e.to_string()))?;
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
        self.connection
            .borrow()
            .execute_batch(sql)
            .map_err(|e| DbError::Query(e.to_string()))
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
