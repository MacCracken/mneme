//! HTTP request handlers.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use mneme_core::note::{CreateNote, Note, NoteWithContent, UpdateNote};
use mneme_core::tag::Tag;
use mneme_search::semantic::HybridResult;

use crate::state::AppState;

// --- Response types ---

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub notes_count: i64,
    pub active_vault: Option<String>,
    pub semantic_available: bool,
    pub vector_count: usize,
    pub embedding_backend: String,
    pub embedding_dimension: usize,
}

#[derive(Deserialize)]
pub struct ListParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub vault: Option<String>,
}

#[derive(Deserialize)]
pub struct SearchParams {
    pub q: String,
    pub limit: Option<usize>,
    pub vault: Option<String>,
    /// Whether to use context-aware retrieval (default: use config setting).
    pub context: Option<bool>,
}

#[derive(Serialize)]
pub struct SearchResponse {
    /// Opaque ID for feedback correlation (encodes arm index).
    pub search_id: String,
    pub results: Vec<SearchResultItem>,
}

#[derive(Serialize)]
pub struct SearchResultItem {
    pub note_id: Uuid,
    pub title: String,
    pub path: String,
    pub snippet: String,
    pub score: f64,
    pub source: String,
    /// Trust score from provenance (0.0–1.0).
    pub trust: f64,
}

#[derive(Deserialize)]
pub struct SearchFeedbackRequest {
    /// The search_id returned from the search endpoint.
    pub search_id: String,
    /// The note ID that the user engaged with.
    pub note_id: Uuid,
    /// The original search query (optional, for training data).
    #[serde(default)]
    pub query: Option<String>,
    /// Position of the clicked result (0-indexed, optional).
    #[serde(default)]
    pub position: Option<usize>,
}

// --- Health ---

pub async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let vs = state.vaults.read().await;
    let (count, vault_name, semantic_available, vector_count, backend_name, embed_dim) =
        match vs.active() {
            Some(vwe) => (
                vwe.vault.vault.count_notes().await.unwrap_or(0),
                Some(vwe.vault.info.name.clone()),
                vwe.semantic().is_available(),
                vwe.semantic().vector_count(),
                vwe.semantic().backend_name().to_string(),
                vwe.semantic().embedding_dimension(),
            ),
            None => (0, None, false, 0, "none".into(), 0),
        };
    Json(HealthResponse {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        notes_count: count,
        active_vault: vault_name,
        semantic_available,
        vector_count,
        embedding_backend: backend_name,
        embedding_dimension: embed_dim,
    })
}

// --- Notes ---

pub async fn create_note(
    State(state): State<AppState>,
    Json(req): Json<CreateNote>,
) -> Result<(StatusCode, Json<NoteWithContent>), (StatusCode, Json<ErrorResponse>)> {
    let vs = state.vaults.read().await;
    let vwe = vs.active().ok_or_else(no_vault)?;

    let result = vwe
        .vault
        .vault
        .create_note(req)
        .await
        .map_err(bad_request)?;

    let _ = vwe.search().index_note(
        result.note.id,
        &result.note.title,
        &result.content,
        &result.tags,
        &result.note.path,
    );
    let _ = vwe
        .semantic()
        .index_note(result.note.id, &result.note.title, &result.content);

    // Publish event (fire-and-forget)
    let vault_id = vwe.vault.info.id;
    let event = mneme_ai::event_bus::MnemeEvent::NoteCreated {
        vault_id,
        note_id: result.note.id,
        title: result.note.title.clone(),
        tags: result.tags.clone(),
    };
    drop(vs);
    tokio::spawn({
        let bus = state.event_bus.clone();
        async move {
            bus.publish(&event).await;
        }
    });

    Ok((StatusCode::CREATED, Json(result)))
}

pub async fn get_note(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<NoteWithContent>, (StatusCode, Json<ErrorResponse>)> {
    let note = {
        let vs = state.vaults.read().await;
        let vwe = vs.active().ok_or_else(no_vault)?;
        vwe.vault.vault.get_note(id).await.map_err(not_found)?
    };

    // Record note access in the context buffer
    let mut vs = state.vaults.write().await;
    if let Some(eng) = vs.active_engines_mut() {
        eng.context_buffer.push(id);
    }

    Ok(Json(note))
}

pub async fn list_notes(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<Vec<Note>>, (StatusCode, Json<ErrorResponse>)> {
    let vs = state.vaults.read().await;
    let vwe = vs.resolve(params.vault.as_deref()).ok_or_else(no_vault)?;
    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);
    let notes = vwe
        .vault
        .vault
        .list_notes(limit, offset)
        .await
        .map_err(internal)?;
    Ok(Json(notes))
}

pub async fn update_note(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateNote>,
) -> Result<Json<NoteWithContent>, (StatusCode, Json<ErrorResponse>)> {
    let vs = state.vaults.read().await;
    let vwe = vs.active().ok_or_else(no_vault)?;
    let result = vwe
        .vault
        .vault
        .update_note(id, req)
        .await
        .map_err(not_found)?;

    let _ = vwe.search().index_note(
        result.note.id,
        &result.note.title,
        &result.content,
        &result.tags,
        &result.note.path,
    );
    let _ = vwe
        .semantic()
        .index_note(result.note.id, &result.note.title, &result.content);

    // Publish update event
    let vault_id = vwe.vault.info.id;
    let event = mneme_ai::event_bus::MnemeEvent::NoteUpdated {
        vault_id,
        note_id: result.note.id,
        title: result.note.title.clone(),
    };
    drop(vs);
    tokio::spawn({
        let bus = state.event_bus.clone();
        async move {
            bus.publish(&event).await;
        }
    });

    Ok(Json(result))
}

pub async fn delete_note(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let vs = state.vaults.read().await;
    let vwe = vs.active().ok_or_else(no_vault)?;

    let vault_id = vwe.vault.info.id;
    let _ = vwe.search().remove_note(id);
    let _ = vwe.semantic().remove_note(id);

    vwe.vault.vault.delete_note(id).await.map_err(not_found)?;

    // Publish delete event
    drop(vs);
    tokio::spawn({
        let bus = state.event_bus.clone();
        async move {
            bus.publish(&mneme_ai::event_bus::MnemeEvent::NoteDeleted {
                vault_id,
                note_id: id,
            })
            .await;
        }
    });

    Ok(StatusCode::NO_CONTENT)
}

// --- Search ---

pub async fn search_notes(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<SearchResponse>, (StatusCode, Json<ErrorResponse>)> {
    let vs = state.vaults.read().await;
    let vwe = vs.resolve(params.vault.as_deref()).ok_or_else(no_vault)?;
    let limit = params.limit.unwrap_or(20);

    // Select blend weights from the optimizer
    let (arm_idx, weights) = vwe.optimizer().select_arm();
    let search_id = format!("s:{}:{}", arm_idx, vwe.optimizer().total_searches);

    let ft_results = vwe.search().search(&params.q, limit).map_err(bad_request)?;

    // Context-aware semantic search: fuse query with context buffer if available
    let use_context = params.context.unwrap_or(true);
    let sem_results = if use_context && !vwe.engines.context_buffer.is_empty() {
        // Build context embedding from recent notes
        let recent_ids: Vec<uuid::Uuid> = vwe
            .engines
            .context_buffer
            .recent_ids()
            .iter()
            .copied()
            .collect();
        let mut embeddings = Vec::new();
        for id in &recent_ids {
            // Re-embed the note title as a lightweight context signal
            if let Some(note) = vwe
                .vault
                .vault
                .list_notes(1000, 0)
                .await
                .ok()
                .and_then(|notes| notes.into_iter().find(|n| n.id == *id))
                && let Ok(Some(emb)) = vwe.semantic().embed(&note.title)
            {
                embeddings.push((*id, emb));
            }
        }
        if let Some(ctx_emb) = vwe.engines.context_buffer.context_embedding(&embeddings) {
            vwe.semantic()
                .context_search(&params.q, &ctx_emb, 0.7, limit)
                .unwrap_or_default()
        } else {
            vwe.semantic().search(&params.q, limit).unwrap_or_default()
        }
    } else {
        vwe.semantic().search(&params.q, limit).unwrap_or_default()
    };

    // Build trust map for provenance-based scoring
    let trust_map: std::collections::HashMap<Uuid, f64> = vwe
        .vault
        .vault
        .list_notes(1000, 0)
        .await
        .unwrap_or_default()
        .iter()
        .map(|n| (n.id, n.trust_score()))
        .collect();

    let mut items: Vec<SearchResultItem> = if sem_results.is_empty() {
        ft_results
            .into_iter()
            .map(|r| {
                let trust = trust_map.get(&r.note_id).copied().unwrap_or(1.0);
                SearchResultItem {
                    note_id: r.note_id,
                    title: r.title,
                    path: r.path,
                    snippet: r.snippet,
                    score: r.score as f64 * trust,
                    source: "fulltext".into(),
                    trust,
                }
            })
            .collect()
    } else {
        let ft_tuples: Vec<_> = ft_results
            .into_iter()
            .map(|r| (r.note_id, r.title, r.path, r.snippet, r.score))
            .collect();
        let hybrid =
            mneme_search::semantic::weighted_hybrid_merge(ft_tuples, sem_results, limit, &weights);
        hybrid_to_items(hybrid, &trust_map)
    };

    // Re-sort after trust boost
    items.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

    // Drop the read lock before acquiring write for recording
    drop(vs);

    // Record that this arm was used for a search
    let mut vs = state.vaults.write().await;
    if let Some(eng) = vs.active_engines_mut() {
        eng.optimizer.record_search(arm_idx);
    }

    Ok(Json(SearchResponse {
        search_id,
        results: items,
    }))
}

/// Record search feedback — the user clicked on a result.
pub async fn search_feedback(
    State(state): State<AppState>,
    Json(req): Json<SearchFeedbackRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Parse arm index from search_id (format: "s:{arm_idx}:{search_count}")
    let arm_idx: usize = req
        .search_id
        .strip_prefix("s:")
        .and_then(|s| s.split(':').next())
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| bad_request("Invalid search_id format"))?;

    let mut vs = state.vaults.write().await;
    // Get vault path before mutable borrow of engines
    let vault_path = vs.manager.active().map(|ov| ov.info.path.clone());

    // Look up note title for training log (before mutable borrow)
    let note_title = if let Some(ov) = vs.manager.active() {
        ov.vault
            .get_note(req.note_id)
            .await
            .ok()
            .map(|n| n.note.title)
            .unwrap_or_default()
    } else {
        String::new()
    };

    let arm_name = vs
        .active_engines_mut()
        .map(|eng| {
            let stats = eng.optimizer.arm_stats();
            stats
                .get(arm_idx)
                .map(|a| a.name.clone())
                .unwrap_or_default()
        })
        .unwrap_or_default();

    if let Some(eng) = vs.active_engines_mut() {
        eng.optimizer.record_feedback(arm_idx);

        // Log to training data
        if let Some(query) = &req.query {
            let _ =
                eng.training_log
                    .append(&mneme_ai::training_export::TrainingRecord::SearchClick {
                        timestamp: chrono::Utc::now(),
                        query: query.clone(),
                        clicked_note_id: req.note_id,
                        clicked_note_title: note_title,
                        search_arm: arm_name,
                        position: req.position.unwrap_or(0),
                    });
        }

        // Persist optimizer state periodically (every 10 feedbacks)
        if eng.optimizer.total_successes % 10 == 0
            && let Some(path) = &vault_path
        {
            crate::state::save_optimizer(path, &eng.optimizer);
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Get retrieval optimizer stats.
pub async fn optimizer_stats(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let vs = state.vaults.read().await;
    let vwe = vs.active().ok_or_else(no_vault)?;
    let opt = vwe.optimizer();
    Ok(Json(serde_json::json!({
        "total_searches": opt.total_searches,
        "total_successes": opt.total_successes,
        "arms": opt.arm_stats(),
    })))
}

fn hybrid_to_items(
    results: Vec<HybridResult>,
    trust_map: &std::collections::HashMap<Uuid, f64>,
) -> Vec<SearchResultItem> {
    results
        .into_iter()
        .map(|r| {
            let trust = trust_map.get(&r.note_id).copied().unwrap_or(1.0);
            SearchResultItem {
                note_id: r.note_id,
                title: r.title,
                path: r.path,
                snippet: r.snippet,
                score: r.score * trust,
                source: format!("{:?}", r.source).to_lowercase(),
                trust,
            }
        })
        .collect()
}

// --- Tags ---

pub async fn list_tags(
    State(state): State<AppState>,
) -> Result<Json<Vec<Tag>>, (StatusCode, Json<ErrorResponse>)> {
    let vs = state.vaults.read().await;
    let vwe = vs.active().ok_or_else(no_vault)?;
    let tags = vwe.vault.vault.list_tags().await.map_err(internal)?;
    Ok(Json(tags))
}

pub async fn delete_tag(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let vs = state.vaults.read().await;
    let vwe = vs.active().ok_or_else(no_vault)?;
    vwe.vault.vault.delete_tag(id).await.map_err(not_found)?;
    Ok(StatusCode::NO_CONTENT)
}

// --- Error helpers ---

fn no_vault() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorResponse {
            error: "No active vault".into(),
        }),
    )
}

fn bad_request(e: impl std::fmt::Display) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: e.to_string(),
        }),
    )
}

fn not_found(e: impl std::fmt::Display) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: e.to_string(),
        }),
    )
}

fn internal(e: impl std::fmt::Display) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: e.to_string(),
        }),
    )
}
