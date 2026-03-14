//! Router configuration.

use axum::Router;
use axum::routing::{delete, get, post, put};

use crate::handlers;
use crate::state::AppState;

/// Build the API router with all routes.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        // Health
        .route("/health", get(handlers::health))
        // Notes
        .route("/v1/notes", get(handlers::list_notes))
        .route("/v1/notes", post(handlers::create_note))
        .route("/v1/notes/{id}", get(handlers::get_note))
        .route("/v1/notes/{id}", put(handlers::update_note))
        .route("/v1/notes/{id}", delete(handlers::delete_note))
        // Search
        .route("/v1/search", get(handlers::search_notes))
        // Tags
        .route("/v1/tags", get(handlers::list_tags))
        .route("/v1/tags/{id}", delete(handlers::delete_tag))
        .with_state(state)
}
