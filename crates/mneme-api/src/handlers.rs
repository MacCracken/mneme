//! HTTP request handlers.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use mneme_core::note::{CreateNote, Note, NoteWithContent, UpdateNote};
use mneme_core::tag::Tag;

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
}

#[derive(Deserialize)]
pub struct ListParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Deserialize)]
pub struct SearchParams {
    pub q: String,
    pub limit: Option<usize>,
}

#[derive(Serialize)]
pub struct SearchResultResponse {
    pub note_id: Uuid,
    pub title: String,
    pub path: String,
    pub snippet: String,
    pub score: f32,
}

// --- Health ---

pub async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let vault = state.vault.read().await;
    let count = vault.count_notes().await.unwrap_or(0);
    Json(HealthResponse {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        notes_count: count,
    })
}

// --- Notes ---

pub async fn create_note(
    State(state): State<AppState>,
    Json(req): Json<CreateNote>,
) -> Result<(StatusCode, Json<NoteWithContent>), (StatusCode, Json<ErrorResponse>)> {
    let vault = state.vault.read().await;

    let result = vault.create_note(req).await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    // Index for search
    let _ = state.search.index_note(
        result.note.id,
        &result.note.title,
        &result.content,
        &result.tags,
        &result.note.path,
    );

    Ok((StatusCode::CREATED, Json(result)))
}

pub async fn get_note(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<NoteWithContent>, (StatusCode, Json<ErrorResponse>)> {
    let vault = state.vault.read().await;
    let note = vault.get_note(id).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;
    Ok(Json(note))
}

pub async fn list_notes(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<Vec<Note>>, (StatusCode, Json<ErrorResponse>)> {
    let vault = state.vault.read().await;
    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);
    let notes = vault.list_notes(limit, offset).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;
    Ok(Json(notes))
}

pub async fn update_note(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateNote>,
) -> Result<Json<NoteWithContent>, (StatusCode, Json<ErrorResponse>)> {
    let vault = state.vault.read().await;
    let result = vault.update_note(id, req).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    // Re-index for search
    let _ = state.search.index_note(
        result.note.id,
        &result.note.title,
        &result.content,
        &result.tags,
        &result.note.path,
    );

    Ok(Json(result))
}

pub async fn delete_note(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let vault = state.vault.read().await;

    // Remove from search index
    let _ = state.search.remove_note(id);

    vault.delete_note(id).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;
    Ok(StatusCode::NO_CONTENT)
}

// --- Search ---

pub async fn search_notes(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<Vec<SearchResultResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let limit = params.limit.unwrap_or(20);
    let results = state.search.search(&params.q, limit).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    Ok(Json(
        results
            .into_iter()
            .map(|r| SearchResultResponse {
                note_id: r.note_id,
                title: r.title,
                path: r.path,
                snippet: r.snippet,
                score: r.score,
            })
            .collect(),
    ))
}

// --- Tags ---

pub async fn list_tags(
    State(state): State<AppState>,
) -> Result<Json<Vec<Tag>>, (StatusCode, Json<ErrorResponse>)> {
    let vault = state.vault.read().await;
    let tags = vault.list_tags().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;
    Ok(Json(tags))
}

pub async fn delete_tag(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let vault = state.vault.read().await;
    vault.delete_tag(id).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;
    Ok(StatusCode::NO_CONTENT)
}
