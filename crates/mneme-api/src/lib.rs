//! Mneme API — HTTP server.
//!
//! Exposes REST endpoints for note CRUD, search, graph queries,
//! and AI operations. Integrates with daimon endpoints.

pub mod advanced_handlers;
pub mod ai_handlers;
pub mod handlers;
pub mod io_handlers;
pub mod router;
pub mod state;
