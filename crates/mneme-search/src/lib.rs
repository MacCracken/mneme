//! Mneme Search — full-text and semantic search.
//!
//! Provides full-text indexing via Tantivy and semantic/vector search
//! via local ONNX embeddings + usearch ANN index, with optional
//! fallback to daimon's `/v1/vectors/*` endpoints.

pub mod context_buffer;
pub mod embedding_backend;
pub mod engine;
pub mod query_dsl;
pub mod error;
pub mod retrieval_optimizer;
pub mod semantic;
pub mod semantic_engine;

#[cfg(feature = "local-vectors")]
pub mod embedder;
#[cfg(feature = "local-vectors")]
pub mod vector_store;

pub mod cross_vault;

pub use context_buffer::ContextBuffer;
pub use engine::SearchEngine;
pub use error::SearchError;
pub use retrieval_optimizer::RetrievalOptimizer;
pub use semantic_engine::SemanticEngine;
