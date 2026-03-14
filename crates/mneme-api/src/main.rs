//! Mneme API server entry point.

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use mneme_api::router::build_router;
use mneme_api::state::AppState;
use mneme_search::SearchEngine;
use mneme_store::Vault;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let vault_dir = std::env::var("MNEME_VAULT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("mneme")
        });

    tracing::info!("Opening vault at {}", vault_dir.display());
    let vault = Vault::open(&vault_dir).await?;

    let search_dir = vault_dir.join(".mneme").join("search-index");
    let search = SearchEngine::open(&search_dir)?;

    let state = AppState {
        vault: Arc::new(RwLock::new(vault)),
        search: Arc::new(search),
    };

    let app = build_router(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let bind = std::env::var("MNEME_BIND").unwrap_or_else(|_| "127.0.0.1:3838".into());
    tracing::info!("Listening on {bind}");

    let listener = tokio::net::TcpListener::bind(&bind).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
