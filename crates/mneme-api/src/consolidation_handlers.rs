//! Consolidation HTTP handlers — stale notes, duplicates, vault health.

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use serde::Deserialize;

use mneme_ai::consolidation::{self, ConsolidationReport, DuplicatePair, NoteContent, StaleNote};

use crate::handlers::ErrorResponse;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct StaleParams {
    /// Number of days since last update to consider a note stale (default: 90).
    pub days: Option<i64>,
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct DuplicateParams {
    /// Minimum Jaccard similarity to report as duplicate (default: 0.7).
    pub threshold: Option<f64>,
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct ConsolidateParams {
    pub duplicate_threshold: Option<f64>,
    pub stale_days: Option<i64>,
}

/// List stale notes that haven't been updated recently.
pub async fn stale_notes(
    State(state): State<AppState>,
    Query(params): Query<StaleParams>,
) -> Result<Json<Vec<StaleNote>>, (StatusCode, Json<ErrorResponse>)> {
    let days = params.days.unwrap_or(90);
    let limit = params.limit.unwrap_or(50);

    let vs = state.vaults.read().await;
    let vwe = vs.active().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "No active vault".into(),
            }),
        )
    })?;

    let notes = load_note_contents(vwe).await?;
    let mut stale = consolidation::detect_stale(&notes, days);
    stale.truncate(limit);

    Ok(Json(stale))
}

/// Detect near-duplicate notes based on content similarity.
pub async fn duplicate_notes(
    State(state): State<AppState>,
    Query(params): Query<DuplicateParams>,
) -> Result<Json<Vec<DuplicatePair>>, (StatusCode, Json<ErrorResponse>)> {
    let threshold = params.threshold.unwrap_or(0.7);
    let limit = params.limit.unwrap_or(20);

    let vs = state.vaults.read().await;
    let vwe = vs.active().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "No active vault".into(),
            }),
        )
    })?;

    let notes = load_note_contents(vwe).await?;
    let mut dups = consolidation::detect_duplicates(&notes, threshold);
    dups.truncate(limit);

    Ok(Json(dups))
}

/// Run a full consolidation pass (duplicates + stale detection).
pub async fn consolidate(
    State(state): State<AppState>,
    Query(params): Query<ConsolidateParams>,
) -> Result<Json<ConsolidationReport>, (StatusCode, Json<ErrorResponse>)> {
    let threshold = params.duplicate_threshold.unwrap_or(0.7);
    let stale_days = params.stale_days.unwrap_or(90);

    let vs = state.vaults.read().await;
    let vwe = vs.active().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "No active vault".into(),
            }),
        )
    })?;

    let notes = load_note_contents(vwe).await?;
    let report = consolidation::consolidate(&notes, threshold, stale_days);

    Ok(Json(report))
}

/// Load all notes with their content for consolidation analysis.
async fn load_note_contents(
    vwe: crate::state::VaultWithEngines<'_>,
) -> Result<Vec<NoteContent>, (StatusCode, Json<ErrorResponse>)> {
    let all_notes = vwe.vault.vault.list_notes(1000, 0).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    let mut contents = Vec::with_capacity(all_notes.len());
    for note in &all_notes {
        if let Ok(full) = vwe.vault.vault.get_note(note.id).await {
            contents.push(NoteContent {
                id: note.id,
                title: note.title.clone(),
                path: note.path.clone(),
                content: full.content,
                updated_at: note.updated_at,
            });
        }
    }

    Ok(contents)
}
