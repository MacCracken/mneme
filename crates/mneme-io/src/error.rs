//! I/O error types.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum IoError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("source directory not found: {0}")]
    SourceNotFound(String),

    #[error("output directory not writable: {0}")]
    OutputNotWritable(String),

    #[error("parse error in {path}: {reason}")]
    Parse { path: String, reason: String },

    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),
}
