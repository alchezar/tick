//! `tick-db` - `SQLite` persistence, repository trait implementations.

mod schema;
mod sqlite;

pub use sqlite::SqliteRepo;
