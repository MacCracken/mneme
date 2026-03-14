//! Router configuration.

use axum::Router;
use axum::routing::{delete, get, post, put};

use crate::advanced_handlers;
use crate::ai_handlers;
use crate::handlers;
use crate::io_handlers;
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
        // AI — RAG
        .route("/v1/ai/rag/query", get(ai_handlers::rag_query))
        .route("/v1/ai/rag/stats", get(ai_handlers::rag_stats))
        .route("/v1/ai/rag/ingest/{id}", post(ai_handlers::rag_ingest_note))
        // AI — Summarization
        .route("/v1/ai/summarize/{id}", get(ai_handlers::summarize_note))
        // AI — Auto-linking
        .route("/v1/ai/suggest-links/{id}", get(ai_handlers::suggest_links))
        // AI — Concept extraction
        .route("/v1/ai/concepts/{id}", get(ai_handlers::extract_concepts))
        // AI — Auto-tagging
        .route("/v1/ai/suggest-tags/{id}", get(io_handlers::suggest_tags))
        // Templates
        .route("/v1/templates", get(io_handlers::list_templates))
        .route("/v1/templates/render", post(io_handlers::render_template))
        // AI — Writing
        .route("/v1/ai/write", post(ai_handlers::write_assist))
        // AI — Translation
        .route("/v1/ai/translate/{id}", get(ai_handlers::translate_note))
        .route("/v1/ai/languages", get(ai_handlers::list_languages))
        // AI — Temporal
        .route("/v1/ai/temporal", get(ai_handlers::temporal_analysis))
        // Export — PDF
        .route("/v1/export/pdf/{id}", get(io_handlers::export_note_pdf))
        // Tasks / Kanban
        .route("/v1/tasks", get(advanced_handlers::get_all_tasks))
        .route("/v1/tasks/{id}", get(advanced_handlers::get_note_tasks))
        // Calendar
        .route("/v1/calendar", get(advanced_handlers::calendar_month))
        // Flashcards
        .route("/v1/flashcards/{id}", get(advanced_handlers::get_note_flashcards))
        // Web Clipper
        .route("/v1/clip/html", post(advanced_handlers::clip_html))
        .route("/v1/clip/bookmark", post(advanced_handlers::clip_bookmark))
        // Plugins
        .route("/v1/plugins", get(advanced_handlers::list_plugins))
        .with_state(state)
}
