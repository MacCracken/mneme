//! Mneme API server entry point.

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use mneme_ai::DaimonClient;
use mneme_api::router::build_router;
use mneme_api::state::{AppState, VaultState};
use mneme_core::config::MnemeConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    // Load config
    let config_path = std::env::var("MNEME_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("mneme")
                .join("mneme.toml")
        });

    let config = if config_path.exists() {
        let data = std::fs::read_to_string(&config_path)?;
        toml::from_str::<MnemeConfig>(&data)?
    } else {
        MnemeConfig::default()
    };

    // Models directory for ONNX embeddings
    let models_dir = std::env::var("MNEME_MODELS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("mneme")
                .join("models")
        });

    // Initialize vault state
    let vault_state = if config.vaults.is_empty() {
        // Legacy single-vault mode
        let vault_dir = std::env::var("MNEME_VAULT_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("mneme")
            });

        tracing::info!("Opening vault at {}", vault_dir.display());
        VaultState::single(&vault_dir, &models_dir).await?
    } else {
        // Multi-vault mode from config
        let registry_path = config.registry_path.clone().unwrap_or_else(|| {
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("mneme")
                .join("registry.toml")
        });

        let mut registry = mneme_store::VaultRegistry::open(&registry_path)?;

        // Register vaults from config that aren't in the registry yet
        for entry in &config.vaults {
            if registry.get_by_name(&entry.name).is_none() {
                let _ = registry.create(entry.name.clone(), entry.path.clone());
            }
        }

        // Set default if specified
        if let Some(default_name) = &config.default_vault
            && let Some(info) = registry.get_by_name(default_name)
        {
            let id = info.id;
            let _ = registry.set_default(id);
        }

        let mut vault_state = VaultState {
            manager: mneme_store::VaultManager::new(registry),
            engines: std::collections::HashMap::new(),
            models_dir: models_dir.clone(),
        };

        // Open the default vault
        if let Some(info) = vault_state.manager.registry().default_vault().cloned() {
            tracing::info!(
                "Opening default vault '{}' at {}",
                info.name,
                info.path.display()
            );
            vault_state.open_vault(info.id).await?;
        }

        vault_state
    };

    let daimon_url = std::env::var("DAIMON_URL").ok();
    let daimon_key = std::env::var("DAIMON_API_KEY").ok();
    let daimon = DaimonClient::new(daimon_url, daimon_key);

    if daimon.health_check().await.unwrap_or(false) {
        tracing::info!("Daimon agent runtime connected");
    } else {
        tracing::warn!("Daimon agent runtime not available — AI features will be limited");
    }

    let event_bus_url = std::env::var("DAIMON_URL").ok();
    let event_bus = mneme_ai::event_bus::EventBusClient::new(event_bus_url, None);

    let agnostic_url = std::env::var("AGNOSTIC_URL").ok();
    let qa_client = mneme_ai::qa_bridge::AgnosticClient::new(agnostic_url);

    let state = AppState {
        vaults: Arc::new(RwLock::new(vault_state)),
        daimon: Arc::new(daimon),
        event_bus: Arc::new(event_bus),
        qa_client: Arc::new(qa_client),
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
