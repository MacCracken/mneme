//! AI-powered HTTP request handlers.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use mneme_ai::concepts::{self, Concept};
use mneme_ai::linker::{AutoLinker, LinkSuggestion};
use mneme_ai::rag::{RagAnswer, RagPipeline};
use mneme_ai::summarizer::{NoteSummary, Summarizer};

use crate::handlers::ErrorResponse;
use crate::state::AppState;

// --- Request/Response types ---

#[derive(Deserialize)]
pub struct RagQueryParams {
    pub q: String,
    pub top_k: Option<usize>,
}

#[derive(Deserialize)]
pub struct SuggestLinksParams {
    pub top_k: Option<usize>,
}

#[derive(Serialize)]
pub struct IngestResponse {
    pub note_id: Uuid,
    pub chunks_ingested: usize,
}

#[derive(Serialize)]
pub struct RagStatsResponse {
    pub index_size: usize,
    pub daimon_available: bool,
}

// --- RAG ---

/// Ask a question across all notes (RAG query).
pub async fn rag_query(
    State(state): State<AppState>,
    Query(params): Query<RagQueryParams>,
) -> Result<Json<RagAnswer>, (StatusCode, Json<ErrorResponse>)> {
    let pipeline = RagPipeline::new((*state.daimon).clone());
    let answer = pipeline.query(&params.q, params.top_k).await.map_err(|e| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;
    Ok(Json(answer))
}

/// Ingest a specific note into the RAG pipeline.
pub async fn rag_ingest_note(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<IngestResponse>, (StatusCode, Json<ErrorResponse>)> {
    let vault = state.vault.read().await;
    let note = vault.get_note(id).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    let pipeline = RagPipeline::new((*state.daimon).clone());
    let chunks = pipeline
        .ingest_note(id, &note.note.title, &note.content)
        .await
        .map_err(|e| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok(Json(IngestResponse {
        note_id: id,
        chunks_ingested: chunks,
    }))
}

/// Get RAG pipeline stats.
pub async fn rag_stats(
    State(state): State<AppState>,
) -> Result<Json<RagStatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let pipeline = RagPipeline::new((*state.daimon).clone());
    match pipeline.stats().await {
        Ok(size) => Ok(Json(RagStatsResponse {
            index_size: size,
            daimon_available: true,
        })),
        Err(_) => Ok(Json(RagStatsResponse {
            index_size: 0,
            daimon_available: false,
        })),
    }
}

// --- Summarization ---

/// Summarize a note's content.
pub async fn summarize_note(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<NoteSummary>, (StatusCode, Json<ErrorResponse>)> {
    let vault = state.vault.read().await;
    let note = vault.get_note(id).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    let summarizer = Summarizer::new((*state.daimon).clone());
    let summary = summarizer.summarize(&note.content).await.map_err(|e| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;
    Ok(Json(summary))
}

// --- Auto-linking ---

/// Suggest links for a note based on content similarity.
pub async fn suggest_links(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<SuggestLinksParams>,
) -> Result<Json<Vec<LinkSuggestion>>, (StatusCode, Json<ErrorResponse>)> {
    let vault = state.vault.read().await;
    let note = vault.get_note(id).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    let linker = AutoLinker::new((*state.daimon).clone());
    let top_k = params.top_k.unwrap_or(5);
    let suggestions = linker
        .suggest_links(id, &note.note.title, &note.content, top_k)
        .await
        .map_err(|e| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;
    Ok(Json(suggestions))
}

// --- Concept extraction ---

/// Extract concepts from a note's content.
pub async fn extract_concepts(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<Concept>>, (StatusCode, Json<ErrorResponse>)> {
    let vault = state.vault.read().await;
    let note = vault.get_note(id).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    let extracted = concepts::extract_concepts(&note.content).map_err(|e| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;
    Ok(Json(extracted))
}
