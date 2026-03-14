//! Mneme AI — intelligent pipelines.
//!
//! Provides summarization, auto-linking, concept extraction, and RAG
//! by integrating with daimon's `/v1/rag/*` and `/v1/knowledge/*` APIs
//! and local models via Synapse.

pub mod client;
pub mod concepts;
pub mod error;
pub mod linker;
pub mod rag;
pub mod summarizer;

pub use client::DaimonClient;
pub use error::AiError;
