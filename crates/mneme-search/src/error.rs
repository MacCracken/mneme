//! Search error types.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SearchError {
    #[error("index error: {0}")]
    Index(String),

    #[error("query parse error: {0}")]
    QueryParse(String),

    #[error("tantivy error: {0}")]
    Tantivy(#[from] tantivy::TantivyError),
}
