//! Mneme Core — zero-I/O types and domain logic.
//!
//! This crate defines the core data model for notes, links, tags,
//! and the knowledge graph. It must have no I/O dependencies.

pub mod calendar;
pub mod frontmatter;
pub mod graph;
pub mod link;
pub mod note;
pub mod plugin;
pub mod tag;
pub mod task;
