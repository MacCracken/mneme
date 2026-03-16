//! Mneme API — HTTP server.
//!
//! Exposes REST endpoints for note CRUD, search, graph queries,
//! vault management, and AI operations.

pub mod advanced_handlers;
pub mod ai_handlers;
pub mod handlers;
pub mod io_handlers;
pub mod router;
pub mod state;
pub mod vault_handlers;
