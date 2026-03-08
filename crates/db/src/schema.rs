//! `SQLite` schema migrations.

use sqlx::SqlitePool;

use domain::error::{DbError, DbResult};

/// Runs all schema migrations from `migrations/` on the given pool.
///
/// Creates tables if they do not exist. Safe to call on every startup.
///
/// # Errors
/// Returns [`DbError::Migration`] if a migration fails.
#[inline]
pub async fn migrate(pool: &SqlitePool) -> DbResult<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| DbError::Migration(e.to_string()))
}
