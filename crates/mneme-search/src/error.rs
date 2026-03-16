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

    #[error("embedding error: {0}")]
    Embedding(String),

    #[error("vector store error: {0}")]
    VectorStore(String),

    #[error("model not found: {0}")]
    ModelNotFound(String),
}
