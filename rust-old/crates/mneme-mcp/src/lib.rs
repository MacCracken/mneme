//! Mneme MCP — Model Context Protocol server.
//!
//! Stdio-based JSON-RPC 2.0 server exposing 5 tools for Claude integration:
//! - `mneme_create_note` — create a new note
//! - `mneme_search` — search notes
//! - `mneme_get_note` — retrieve a note
//! - `mneme_update_note` — update a note
//! - `mneme_query_graph` — query the knowledge graph

pub mod protocol;
pub mod tools;
