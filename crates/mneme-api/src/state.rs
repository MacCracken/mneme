//! Application state shared across handlers.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use mneme_ai::DaimonClient;
use mneme_ai::rag_eval::RagEvalAggregates;
use mneme_ai::training_export::TrainingLog;
use mneme_search::{ContextBuffer, RetrievalOptimizer, SearchEngine, SemanticEngine};
use mneme_store::VaultManager;
use mneme_store::manager::OpenVault;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Search engines, optimizer, and RAG eval aggregates for an open vault.
pub struct VaultEngines {
    pub search: SearchEngine,
    pub semantic: SemanticEngine,
    pub optimizer: RetrievalOptimizer,
    pub rag_eval: RagEvalAggregates,
    pub context_buffer: ContextBuffer,
    pub training_log: TrainingLog,
}

/// An open vault with its search engines — convenience accessor.
pub struct VaultWithEngines<'a> {
    pub vault: &'a OpenVault,
    pub engines: &'a VaultEngines,
}

impl VaultWithEngines<'_> {
    pub fn search(&self) -> &SearchEngine {
        &self.engines.search
    }

    pub fn semantic(&self) -> &SemanticEngine {
        &self.engines.semantic
    }

    pub fn optimizer(&self) -> &RetrievalOptimizer {
        &self.engines.optimizer
    }
}

/// Application-level vault state: VaultManager + per-vault search engines.
pub struct VaultState {
    pub manager: VaultManager,
    pub engines: HashMap<Uuid, VaultEngines>,
    pub models_dir: PathBuf,
}

impl VaultState {
    /// Create for a single vault (legacy mode).
    pub async fn single(vault_dir: &Path, models_dir: &Path) -> Result<Self, anyhow::Error> {
        let manager = VaultManager::single(vault_dir).await?;
        let mut engines = HashMap::new();

        if let Some(ov) = manager.active() {
            let eng = create_engines(&ov.info.path, models_dir);
            engines.insert(ov.info.id, eng);
        }

        Ok(Self {
            manager,
            engines,
            models_dir: models_dir.to_path_buf(),
        })
    }

    /// Get the active vault with its engines.
    pub fn active(&self) -> Option<VaultWithEngines<'_>> {
        let ov = self.manager.active()?;
        let engines = self.engines.get(&ov.info.id)?;
        Some(VaultWithEngines { vault: ov, engines })
    }

    /// Get mutable engines for the active vault (for recording feedback).
    pub fn active_engines_mut(&mut self) -> Option<&mut VaultEngines> {
        let id = self.manager.active_id()?;
        self.engines.get_mut(&id)
    }

    /// Resolve a vault by name/ID, or fall back to active.
    pub fn resolve(&self, name_or_id: Option<&str>) -> Option<VaultWithEngines<'_>> {
        if let Some(ref_str) = name_or_id {
            let info = self.manager.registry().resolve(ref_str)?;
            let ov = self.manager.get(info.id)?;
            let engines = self.engines.get(&info.id)?;
            Some(VaultWithEngines { vault: ov, engines })
        } else {
            self.active()
        }
    }

    /// Open a vault and create its search engines.
    pub async fn open_vault(&mut self, id: Uuid) -> Result<(), anyhow::Error> {
        self.manager.open_vault(id).await?;
        if !self.engines.contains_key(&id) {
            if let Some(ov) = self.manager.get(id) {
                let eng = create_engines(&ov.info.path, &self.models_dir);
                self.engines.insert(id, eng);
            }
        }
        Ok(())
    }

    /// Switch active vault, opening it if needed.
    pub async fn switch_vault(&mut self, id: Uuid) -> Result<(), anyhow::Error> {
        if !self.engines.contains_key(&id) {
            self.open_vault(id).await?;
        }
        self.manager.switch_vault(id).await?;
        Ok(())
    }

    /// Create and open a new vault.
    pub async fn create_vault(
        &mut self,
        name: String,
        path: PathBuf,
    ) -> Result<(), anyhow::Error> {
        let info = self.manager.create_vault(name, path).await?.clone();
        let eng = create_engines(&info.path, &self.models_dir);
        self.engines.insert(info.id, eng);
        Ok(())
    }

    /// Remove a vault and its engines.
    pub fn remove_vault(&mut self, id: Uuid) -> Result<(), anyhow::Error> {
        self.engines.remove(&id);
        self.manager.remove_vault(id)?;
        Ok(())
    }
}

fn create_engines(vault_path: &Path, models_dir: &Path) -> VaultEngines {
    create_engines_with_config(vault_path, models_dir, None)
}

fn create_engines_with_config(
    vault_path: &Path,
    models_dir: &Path,
    embedding_config: Option<&mneme_search::embedding_backend::EmbeddingConfig>,
) -> VaultEngines {
    let search_dir = vault_path.join(".mneme").join("search-index");
    let search = SearchEngine::open(&search_dir).unwrap_or_else(|e| {
        tracing::warn!("FTS engine failed for {}: {e}", vault_path.display());
        SearchEngine::in_memory().unwrap()
    });

    let vectors_dir = vault_path.join(".mneme").join("vectors");
    let semantic = if let Some(config) = embedding_config {
        SemanticEngine::open_with_config(models_dir, &vectors_dir, config)
    } else {
        SemanticEngine::open(models_dir, &vectors_dir)
    };

    // Load optimizer state from disk, or create fresh
    let optimizer_path = vault_path.join(".mneme").join("optimizer.json");
    let optimizer = if optimizer_path.exists() {
        std::fs::read_to_string(&optimizer_path)
            .ok()
            .and_then(|data| serde_json::from_str::<RetrievalOptimizer>(&data).ok())
            .unwrap_or_default()
    } else {
        RetrievalOptimizer::default()
    };

    let training_log_path = vault_path.join(".mneme").join("training.jsonl");
    let training_log = TrainingLog::open(training_log_path);

    VaultEngines {
        search,
        semantic,
        optimizer,
        rag_eval: RagEvalAggregates::default(),
        context_buffer: ContextBuffer::new(7),
        training_log,
    }
}

/// Persist the optimizer state for a vault.
pub fn save_optimizer(vault_path: &Path, optimizer: &RetrievalOptimizer) {
    let optimizer_path = vault_path.join(".mneme").join("optimizer.json");
    if let Ok(data) = serde_json::to_string_pretty(optimizer) {
        let _ = std::fs::write(&optimizer_path, data);
    }
}

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub vaults: Arc<RwLock<VaultState>>,
    pub daimon: Arc<DaimonClient>,
    pub event_bus: Arc<mneme_ai::event_bus::EventBusClient>,
    pub qa_client: Arc<mneme_ai::qa_bridge::AgnosticClient>,
}
