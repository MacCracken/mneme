//! Consolidation HTTP handlers — stale notes, duplicates, vault health, merge suggestions.

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use serde::Deserialize;

use mneme_ai::consolidation::{
    self, ConsolidationReport, DuplicatePair, MergeSuggestion, NoteContent, StaleNote,
};

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
    /// Minimum similarity to report as duplicate (default: 0.7).
    pub threshold: Option<f64>,
    pub limit: Option<usize>,
    /// Detection method: "jaccard", "semantic", or "both" (default: "jaccard").
    pub method: Option<String>,
}

#[derive(Deserialize)]
pub struct ConsolidateParams {
    pub duplicate_threshold: Option<f64>,
    pub stale_days: Option<i64>,
}

#[derive(Deserialize)]
pub struct MergeParams {
    pub note_a_id: uuid::Uuid,
    pub note_b_id: uuid::Uuid,
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

    let notes = load_note_contents(&vwe).await?;
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
    let method = params.method.as_deref().unwrap_or("jaccard");

    let vs = state.vaults.read().await;
    let vwe = vs.active().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "No active vault".into(),
            }),
        )
    })?;

    let notes = load_note_contents(&vwe).await?;

    let mut dups = Vec::new();

    if method == "jaccard" || method == "both" {
        dups.extend(consolidation::detect_duplicates(&notes, threshold));
    }

    if method == "semantic" || method == "both" {
        // Build semantic similarity map by querying the vector store per note
        let semantic = vwe.semantic();
        let mut similarity_map = Vec::new();
        for note in &notes {
            let text = format!("{}\n{}", note.title, note.content);
            if let Ok(results) = semantic.find_similar_to(&text, threshold, 10) {
                let similars: Vec<(uuid::Uuid, String, f64)> = results
                    .into_iter()
                    .filter_map(|r| {
                        let id = r.note_id?;
                        if id == note.id {
                            return None;
                        }
                        Some((id, r.title.unwrap_or_default(), r.score))
                    })
                    .collect();
                if !similars.is_empty() {
                    similarity_map.push((note.id, note.title.clone(), similars));
                }
            }
        }
        dups.extend(consolidation::detect_duplicates_semantic(
            &similarity_map,
            threshold,
        ));
    }

    dups.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
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

    let notes = load_note_contents(&vwe).await?;
    let report = consolidation::consolidate(&notes, threshold, stale_days);

    Ok(Json(report))
}

/// Suggest how to merge two duplicate notes via LLM.
pub async fn suggest_merge(
    State(state): State<AppState>,
    Json(params): Json<MergeParams>,
) -> Result<Json<MergeSuggestion>, (StatusCode, Json<ErrorResponse>)> {
    let vs = state.vaults.read().await;
    let vwe = vs.active().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "No active vault".into(),
            }),
        )
    })?;

    let note_a = vwe
        .vault
        .vault
        .get_note(params.note_a_id)
        .await
        .map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Note A: {e}"),
                }),
            )
        })?;

    let note_b = vwe
        .vault
        .vault
        .get_note(params.note_b_id)
        .await
        .map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Note B: {e}"),
                }),
            )
        })?;

    let merge_resp = state
        .daimon
        .suggest_merge(
            &note_a.note.title,
            &note_a.content,
            &note_b.note.title,
            &note_b.content,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse {
                    error: format!("Daimon: {e}"),
                }),
            )
        })?;

    let keep_id = if merge_resp.keep == "b" {
        params.note_b_id
    } else {
        params.note_a_id
    };

    Ok(Json(MergeSuggestion {
        note_a_id: params.note_a_id,
        note_b_id: params.note_b_id,
        keep_id,
        merged_title: merge_resp.merged_title,
        merged_content: merge_resp.merged_content,
        rationale: merge_resp.rationale,
        confidence: merge_resp.confidence,
    }))
}

/// Load all notes with their content for consolidation analysis.
async fn load_note_contents(
    vwe: &crate::state::VaultWithEngines<'_>,
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
                last_accessed: note.last_accessed,
            });
        }
    }

    Ok(contents)
}
