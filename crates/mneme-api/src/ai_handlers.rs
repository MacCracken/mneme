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

// --- AI Writing ---

#[derive(Deserialize)]
pub struct WriteAssistRequest {
    pub action: mneme_ai::writer::WriteAction,
    pub text: String,
    pub context: Option<String>,
}

/// AI writing assistance (complete, reword, expand).
pub async fn write_assist(
    State(state): State<AppState>,
    Json(req): Json<WriteAssistRequest>,
) -> Result<Json<mneme_ai::writer::WriteResult>, (StatusCode, Json<ErrorResponse>)> {
    let writer = mneme_ai::writer::Writer::new((*state.daimon).clone());
    let write_req = mneme_ai::writer::WriteRequest {
        action: req.action,
        text: req.text,
        context: req.context,
    };
    let result = writer.assist(&write_req).await.map_err(|e| {
        (StatusCode::UNPROCESSABLE_ENTITY, Json(ErrorResponse { error: e.to_string() }))
    })?;
    Ok(Json(result))
}

// --- Translation ---

/// Translate a note's content.
pub async fn translate_note(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<TranslateParams>,
) -> Result<Json<mneme_ai::translator::TranslateResult>, (StatusCode, Json<ErrorResponse>)> {
    let vault = state.vault.read().await;
    let note = vault.get_note(id).await.map_err(|e| {
        (StatusCode::NOT_FOUND, Json(ErrorResponse { error: e.to_string() }))
    })?;

    let translator = mneme_ai::translator::Translator::new((*state.daimon).clone());
    let req = mneme_ai::translator::TranslateRequest {
        content: note.content,
        target_language: params.lang.clone(),
        source_language: params.source_lang.clone(),
        preserve_formatting: true,
    };
    let result = translator.translate(&req).await.map_err(|e| {
        (StatusCode::UNPROCESSABLE_ENTITY, Json(ErrorResponse { error: e.to_string() }))
    })?;
    Ok(Json(result))
}

#[derive(Deserialize)]
pub struct TranslateParams {
    pub lang: String,
    pub source_lang: Option<String>,
}

/// List supported languages.
pub async fn list_languages() -> Json<Vec<mneme_ai::translator::Language>> {
    Json(mneme_ai::translator::Translator::supported_languages())
}

// --- Temporal Analysis ---

/// Temporal analysis across all notes.
pub async fn temporal_analysis(
    State(state): State<AppState>,
) -> Result<Json<mneme_ai::temporal::TemporalReport>, (StatusCode, Json<ErrorResponse>)> {
    let vault = state.vault.read().await;
    let notes = vault.list_notes(1000, 0).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() }))
    })?;

    let mut snapshots = Vec::new();
    for note in &notes {
        if let Ok(full) = vault.get_note(note.id).await {
            snapshots.push(mneme_ai::temporal::NoteSnapshot {
                title: full.note.title,
                content: full.content,
                tags: full.tags,
                created_at: full.note.created_at,
                updated_at: full.note.updated_at,
            });
        }
    }

    let report = mneme_ai::temporal::analyze_temporal(&snapshots).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() }))
    })?;
    Ok(Json(report))
}
