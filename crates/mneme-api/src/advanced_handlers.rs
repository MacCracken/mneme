//! Advanced feature handlers — tasks, calendar, versioning, flashcards, web clipper.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::Deserialize;
use uuid::Uuid;

use crate::handlers::ErrorResponse;
use crate::state::AppState;

/// Helper to get the active vault.
macro_rules! active_vault {
    ($mgr:expr) => {{
        $mgr.active().ok_or_else(|| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    error: "No active vault".into(),
                }),
            )
        })?
    }};
}

// --- Tasks / Kanban ---

pub async fn get_note_tasks(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<mneme_core::task::TaskBoard>, (StatusCode, Json<ErrorResponse>)> {
    let vs = state.vaults.read().await;
    let ov = active_vault!(vs);
    let note = ov.vault.vault.get_note(id).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;
    let tasks = mneme_core::task::extract_tasks(id, &note.content);
    Ok(Json(mneme_core::task::build_board(tasks)))
}

pub async fn get_all_tasks(
    State(state): State<AppState>,
) -> Result<Json<mneme_core::task::TaskBoard>, (StatusCode, Json<ErrorResponse>)> {
    let vs = state.vaults.read().await;
    let ov = active_vault!(vs);
    let notes = ov.vault.vault.list_notes(1000, 0).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;
    let mut all_tasks = Vec::new();
    for note in &notes {
        if let Ok(full) = ov.vault.vault.get_note(note.id).await {
            all_tasks.extend(mneme_core::task::extract_tasks(note.id, &full.content));
        }
    }
    Ok(Json(mneme_core::task::build_board(all_tasks)))
}

// --- Calendar ---

#[derive(Deserialize)]
pub struct CalendarParams {
    pub year: i32,
    pub month: u32,
}

pub async fn calendar_month(
    State(state): State<AppState>,
    Query(params): Query<CalendarParams>,
) -> Result<Json<mneme_core::calendar::MonthView>, (StatusCode, Json<ErrorResponse>)> {
    let vs = state.vaults.read().await;
    let ov = active_vault!(vs);
    let notes = ov.vault.vault.list_notes(1000, 0).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    let entries: Vec<mneme_core::calendar::CalendarEntry> = notes
        .iter()
        .filter_map(|note| {
            let date = mneme_core::calendar::parse_date_from_title(&note.title)?;
            let entry_type = mneme_core::calendar::detect_entry_type(&note.title);
            Some(mneme_core::calendar::CalendarEntry {
                date,
                note_id: note.id,
                title: note.title.clone(),
                entry_type,
            })
        })
        .collect();

    Ok(Json(mneme_core::calendar::month_view(
        &entries,
        params.year,
        params.month,
    )))
}

// --- Flashcards ---

pub async fn get_note_flashcards(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<mneme_ai::flashcards::Flashcard>>, (StatusCode, Json<ErrorResponse>)> {
    let vs = state.vaults.read().await;
    let ov = active_vault!(vs);
    let note = ov.vault.vault.get_note(id).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;
    let cards = mneme_ai::flashcards::extract_flashcards(id, &note.content);
    Ok(Json(cards))
}

// --- Web Clipper ---

#[derive(Deserialize)]
pub struct ClipHtmlRequest {
    pub html: String,
    pub url: String,
    pub create: Option<bool>,
}

#[derive(Deserialize)]
pub struct ClipBookmarkRequest {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub create: Option<bool>,
}

pub async fn clip_html(
    State(state): State<AppState>,
    Json(req): Json<ClipHtmlRequest>,
) -> Result<Json<mneme_io::web_clipper::ClippedPage>, (StatusCode, Json<ErrorResponse>)> {
    let clipped = mneme_io::web_clipper::clip_html(
        &req.html,
        &req.url,
        &mneme_io::web_clipper::ClipOptions::default(),
    )
    .map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    if req.create.unwrap_or(false) {
        let vs = state.vaults.read().await;
        if let Some(ov) = vs.active() {
            let create_req = mneme_core::note::CreateNote {
                title: clipped.title.clone(),
                path: None,
                content: clipped.content_md.clone(),
                tags: clipped.tags.clone(),
            };
            if let Ok(note) = ov.vault.vault.create_note(create_req).await {
                let _ = ov.search().index_note(
                    note.note.id,
                    &note.note.title,
                    &note.content,
                    &note.tags,
                    &note.note.path,
                );
                let _ = ov
                    .semantic()
                    .index_note(note.note.id, &note.note.title, &note.content);
            }
        }
    }

    Ok(Json(clipped))
}

pub async fn clip_bookmark(
    State(state): State<AppState>,
    Json(req): Json<ClipBookmarkRequest>,
) -> Result<Json<mneme_io::web_clipper::ClippedPage>, (StatusCode, Json<ErrorResponse>)> {
    let clipped = mneme_io::web_clipper::clip_bookmark(
        &req.url,
        req.title.as_deref(),
        req.description.as_deref(),
    );

    if req.create.unwrap_or(false) {
        let vs = state.vaults.read().await;
        if let Some(ov) = vs.active() {
            let create_req = mneme_core::note::CreateNote {
                title: clipped.title.clone(),
                path: None,
                content: clipped.content_md.clone(),
                tags: clipped.tags.clone(),
            };
            if let Ok(note) = ov.vault.vault.create_note(create_req).await {
                let _ = ov.search().index_note(
                    note.note.id,
                    &note.note.title,
                    &note.content,
                    &note.tags,
                    &note.note.path,
                );
                let _ = ov
                    .semantic()
                    .index_note(note.note.id, &note.note.title, &note.content);
            }
        }
    }

    Ok(Json(clipped))
}

// --- Plugins ---

pub async fn list_plugins() -> Json<Vec<mneme_core::plugin::PluginInfo>> {
    Json(vec![])
}
