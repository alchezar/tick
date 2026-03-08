//! SQLite-backed repository implementation.

use core::cell::RefCell;

use rusqlite::Connection;

use crate::schema;
use domain::error::{DbError, DbResult};

/// `SQLite` repository wrapping a single connection.
///
/// Uses [`RefCell`] for interior mutability because repository traits use `&self`.
#[allow(dead_code)]
#[derive(Debug)]
pub struct SqliteRepo {
    connection: RefCell<Connection>,
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
        })
    }

    /// Creates an in-memory `SQLite` database with migrations applied.
    ///
    /// Useful for testing.
    ///
    /// # Errors
    /// Returns [`DbError`] if the connection or migration fails.
    #[inline]
    pub fn in_memory() -> Result<Self, DbError> {
        Self::open(":memory:")
    }
}
