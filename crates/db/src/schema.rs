//! `SQLite` schema migrations.

use rusqlite::Connection;

use domain::error::{DbError, DbResult};

const MIGRATION_001: &str = include_str!("../migrations/0001_initial.sql");

/// Runs all schema migrations on the given connection.
///
/// Creates tables if they do not exist. Safe to call on every startup.
///
/// # Errors
/// Returns [`DbError::Migration`] if a migration statement fails.
#[inline]
pub fn migrate(connection: &Connection) -> DbResult<()> {
    connection
        .execute_batch(MIGRATION_001)
        .map_err(|e| DbError::Migration(e.to_string()))
}
