//! Mneme Store — persistence layer.
//!
//! Manages SQLite database for note metadata, tags, and links.
//! Note content is stored as plain Markdown files on disk.

pub mod db;
pub mod error;
pub mod files;
pub mod sharing;
pub mod vault;
pub mod versioning;

pub use error::StoreError;
pub use vault::Vault;
