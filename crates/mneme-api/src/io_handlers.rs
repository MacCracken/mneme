//! Import/export and template HTTP handlers.

use std::collections::HashMap;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use mneme_ai::tagger::{self, TagSuggestion};
use mneme_ai::templates::{self, RenderedTemplate, Template};
use mneme_core::note::CreateNote;

use crate::handlers::ErrorResponse;
use crate::state::AppState;

// --- Templates ---

#[derive(Serialize)]
pub struct TemplateListResponse {
    pub templates: Vec<Template>,
}

#[derive(Deserialize)]
pub struct RenderTemplateRequest {
    pub template_name: String,
    pub variables: HashMap<String, String>,
    pub create: Option<bool>,
}

#[derive(Serialize)]
pub struct RenderTemplateResponse {
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub path: Option<String>,
    pub created: bool,
}

/// List available templates.
pub async fn list_templates() -> Json<TemplateListResponse> {
    Json(TemplateListResponse {
        templates: templates::builtin_templates(),
    })
}

/// Render a template (and optionally create the note).
pub async fn render_template(
    State(state): State<AppState>,
    Json(req): Json<RenderTemplateRequest>,
) -> Result<Json<RenderTemplateResponse>, (StatusCode, Json<ErrorResponse>)> {
    let all = templates::builtin_templates();
    let template = all.iter().find(|t| t.name == req.template_name).ok_or((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: format!("Template '{}' not found", req.template_name),
        }),
    ))?;

    let rendered: RenderedTemplate = templates::render_template(template, &req.variables);

    let created = if req.create.unwrap_or(false) {
        let vault = state.vault.read().await;
        let create_req = CreateNote {
            title: rendered.title.clone(),
            path: rendered.path.clone(),
            content: rendered.content.clone(),
            tags: rendered.tags.clone(),
        };

        match vault.create_note(create_req).await {
            Ok(note) => {
                let _ = state.search.index_note(
                    note.note.id,
                    &note.note.title,
                    &note.content,
                    &note.tags,
                    &note.note.path,
                );
                true
            }
            Err(e) => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: e.to_string(),
                    }),
                ));
            }
        }
    } else {
        false
    };

    Ok(Json(RenderTemplateResponse {
        title: rendered.title,
        content: rendered.content,
        tags: rendered.tags,
        path: rendered.path,
        created,
    }))
}

// --- Auto-tagging ---

/// Suggest tags for a note based on content.
pub async fn suggest_tags(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<Vec<TagSuggestion>>, (StatusCode, Json<ErrorResponse>)> {
    let vault = state.vault.read().await;
    let note = vault.get_note(id).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    let existing_tags: Vec<String> = vault
        .list_tags()
        .await
        .map(|tags| tags.into_iter().map(|t| t.name).collect())
        .unwrap_or_default();

    let suggestions = tagger::suggest_tags(&note.content, &existing_tags, 10).map_err(|e| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    Ok(Json(suggestions))
}
