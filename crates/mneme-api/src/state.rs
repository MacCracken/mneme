//! Application state shared across handlers.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use mneme_ai::DaimonClient;
use mneme_search::{SearchEngine, SemanticEngine};
use mneme_store::VaultManager;
use mneme_store::manager::OpenVault;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Search engines associated with an open vault.
pub struct VaultEngines {
    pub search: SearchEngine,
    pub semantic: SemanticEngine,
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
    let search_dir = vault_path.join(".mneme").join("search-index");
    let search = SearchEngine::open(&search_dir).unwrap_or_else(|e| {
        tracing::warn!("FTS engine failed for {}: {e}", vault_path.display());
        SearchEngine::in_memory().unwrap()
    });

    let vectors_dir = vault_path.join(".mneme").join("vectors");
    let semantic = SemanticEngine::open(models_dir, &vectors_dir);

    VaultEngines { search, semantic }
}

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub vaults: Arc<RwLock<VaultState>>,
    pub daimon: Arc<DaimonClient>,
}
