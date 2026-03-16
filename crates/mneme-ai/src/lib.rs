//! Mneme AI — intelligent pipelines.
//!
//! Provides summarization, auto-linking, concept extraction, and RAG
//! by integrating with daimon's `/v1/rag/*` and `/v1/knowledge/*` APIs
//! and local models via Synapse.

pub mod client;
pub mod clustering;
pub mod concepts;
pub mod consolidation;
pub mod creative;
pub mod error;
pub mod flashcards;
pub mod linker;
pub mod multimodal;
pub mod rag;
pub mod rag_eval;
pub mod summarizer;
pub mod tagger;
pub mod templates;
pub mod temporal;
pub mod translator;
pub mod writer;

pub use client::DaimonClient;
pub use error::AiError;
