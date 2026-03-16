//! Vault management HTTP handlers.

use std::path::PathBuf;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use mneme_core::config::VaultInfo;

use crate::handlers::ErrorResponse;
use crate::state::AppState;

#[derive(Serialize)]
pub struct VaultResponse {
    pub id: Uuid,
    pub name: String,
    pub path: String,
    pub description: String,
    pub search_weight: f64,
    pub is_default: bool,
    pub is_active: bool,
}

#[derive(Deserialize)]
pub struct CreateVaultRequest {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateVaultRequest {
    pub description: Option<String>,
    pub search_weight: Option<f64>,
}

fn vault_to_response(info: &VaultInfo, active_id: Option<Uuid>) -> VaultResponse {
    VaultResponse {
        id: info.id,
        name: info.name.clone(),
        path: info.path.display().to_string(),
        description: info.description.clone(),
        search_weight: info.search_weight,
        is_default: info.is_default,
        is_active: active_id == Some(info.id),
    }
}

/// List all registered vaults.
pub async fn list_vaults(
    State(state): State<AppState>,
) -> Result<Json<Vec<VaultResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let vs = state.vaults.read().await;
    let active_id = vs.manager.active_id();
    let vaults: Vec<VaultResponse> = vs
        .manager
        .registry()
        .list()
        .iter()
        .map(|v| vault_to_response(v, active_id))
        .collect();
    Ok(Json(vaults))
}

/// Create/register a new vault.
pub async fn create_vault(
    State(state): State<AppState>,
    Json(req): Json<CreateVaultRequest>,
) -> Result<(StatusCode, Json<VaultResponse>), (StatusCode, Json<ErrorResponse>)> {
    let mut vs = state.vaults.write().await;
    let name = req.name.clone();
    vs.create_vault(req.name, PathBuf::from(&req.path))
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;
    let active_id = vs.manager.active_id();
    let info = vs.manager.registry().get_by_name(&name).unwrap();
    Ok((StatusCode::CREATED, Json(vault_to_response(info, active_id))))
}

/// Get a specific vault's info.
pub async fn get_vault(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<VaultResponse>, (StatusCode, Json<ErrorResponse>)> {
    let vs = state.vaults.read().await;
    let info = vs.manager.registry().get_by_id(id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Vault not found: {id}"),
            }),
        )
    })?;
    let active_id = vs.manager.active_id();
    Ok(Json(vault_to_response(info, active_id)))
}

/// Delete a vault from the registry.
pub async fn delete_vault(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let mut vs = state.vaults.write().await;
    vs.remove_vault(id).map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;
    Ok(StatusCode::NO_CONTENT)
}

/// Switch the active vault.
pub async fn switch_vault(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<VaultResponse>, (StatusCode, Json<ErrorResponse>)> {
    let mut vs = state.vaults.write().await;
    vs.switch_vault(id).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;
    let info = vs.manager.registry().get_by_id(id).unwrap();
    let active_id = vs.manager.active_id();
    Ok(Json(vault_to_response(info, active_id)))
}
