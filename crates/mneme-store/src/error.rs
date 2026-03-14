//! Store error types.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("note not found: {0}")]
    NoteNotFound(String),

    #[error("tag not found: {0}")]
    TagNotFound(String),

    #[error("note already exists at path: {0}")]
    PathConflict(String),

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
}
