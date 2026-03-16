//! Vault manager — manages multiple open vaults.
//!
//! Note: Search engines (FTS, semantic) are NOT owned here to avoid
//! a circular dependency between mneme-store and mneme-search.
//! The caller is responsible for creating search engines for each vault.

use std::collections::HashMap;
use std::path::PathBuf;

use uuid::Uuid;

use mneme_core::config::VaultInfo;

use crate::StoreError;
use crate::registry::VaultRegistry;
use crate::vault::Vault;

/// A vault with its resolved info.
pub struct OpenVault {
    pub info: VaultInfo,
    pub vault: Vault,
}

/// Manages multiple vaults, lazily opening them as needed.
pub struct VaultManager {
    registry: VaultRegistry,
    active_id: Option<Uuid>,
    open_vaults: HashMap<Uuid, OpenVault>,
}

impl VaultManager {
    /// Create a manager from an existing registry.
    pub fn new(registry: VaultRegistry) -> Self {
        let active_id = registry.default_vault().map(|v| v.id);
        Self {
            registry,
            active_id,
            open_vaults: HashMap::new(),
        }
    }

    /// Get the vault registry.
    pub fn registry(&self) -> &VaultRegistry {
        &self.registry
    }

    /// Get a mutable reference to the vault registry.
    pub fn registry_mut(&mut self) -> &mut VaultRegistry {
        &mut self.registry
    }

    /// Get the active vault ID.
    pub fn active_id(&self) -> Option<Uuid> {
        self.active_id
    }

    /// Switch the active vault. Opens it if not already open.
    pub async fn switch_vault(&mut self, id: Uuid) -> Result<(), StoreError> {
        if !self.open_vaults.contains_key(&id) {
            self.open_vault(id).await?;
        }
        self.active_id = Some(id);
        self.registry.touch(id);
        Ok(())
    }

    /// Get the active open vault.
    pub fn active(&self) -> Option<&OpenVault> {
        self.active_id.and_then(|id| self.open_vaults.get(&id))
    }

    /// Get a specific open vault by ID.
    pub fn get(&self, id: Uuid) -> Option<&OpenVault> {
        self.open_vaults.get(&id)
    }

    /// Get all currently open vaults.
    pub fn open_vaults(&self) -> impl Iterator<Item = &OpenVault> {
        self.open_vaults.values()
    }

    /// Get all open vault IDs.
    pub fn open_vault_ids(&self) -> Vec<Uuid> {
        self.open_vaults.keys().copied().collect()
    }

    /// Open a vault by its registry ID.
    pub async fn open_vault(&mut self, id: Uuid) -> Result<(), StoreError> {
        if self.open_vaults.contains_key(&id) {
            return Ok(());
        }

        let info = self
            .registry
            .get_by_id(id)
            .ok_or_else(|| StoreError::VaultNotFound(id.to_string()))?
            .clone();

        let vault = Vault::open(&info.path).await?;

        self.open_vaults.insert(id, OpenVault { info, vault });
        self.registry.touch(id);

        if self.active_id.is_none() {
            self.active_id = Some(id);
        }

        Ok(())
    }

    /// Close a vault, releasing its resources.
    pub fn close_vault(&mut self, id: Uuid) {
        self.open_vaults.remove(&id);
        if self.active_id == Some(id) {
            self.active_id = self.open_vaults.keys().next().copied();
        }
    }

    /// Register and open a new vault.
    pub async fn create_vault(
        &mut self,
        name: String,
        path: PathBuf,
    ) -> Result<&VaultInfo, StoreError> {
        let info = self.registry.create(name, path)?.clone();
        let vault = Vault::open(&info.path).await?;
        let id = info.id;
        self.open_vaults.insert(id, OpenVault { info, vault });
        if self.active_id.is_none() {
            self.active_id = Some(id);
        }
        Ok(self.registry.get_by_id(id).unwrap())
    }

    /// Remove a vault from the registry and close it.
    pub fn remove_vault(&mut self, id: Uuid) -> Result<(), StoreError> {
        self.close_vault(id);
        self.registry.remove(id)
    }

    /// Create a single-vault manager (for backwards compatibility).
    pub async fn single(vault_dir: &std::path::Path) -> Result<Self, StoreError> {
        let mut registry = VaultRegistry::in_memory();
        let info = registry
            .create("default".into(), vault_dir.to_path_buf())?
            .clone();

        let vault = Vault::open(&info.path).await?;
        let id = info.id;

        let mut mgr = Self {
            registry,
            active_id: Some(id),
            open_vaults: HashMap::new(),
        };
        mgr.open_vaults.insert(id, OpenVault { info, vault });
        Ok(mgr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn test_manager() -> (VaultManager, TempDir) {
        let vault_dir = TempDir::new().unwrap();
        let mgr = VaultManager::single(vault_dir.path()).await.unwrap();
        (mgr, vault_dir)
    }

    #[tokio::test]
    async fn single_vault_manager() {
        let (mgr, _vd) = test_manager().await;
        assert!(mgr.active_id().is_some());
        assert!(mgr.active().is_some());
        assert_eq!(mgr.registry().list().len(), 1);
    }

    #[tokio::test]
    async fn active_vault_works() {
        let (mgr, _vd) = test_manager().await;
        let active = mgr.active().unwrap();
        assert_eq!(active.info.name, "default");
    }

    #[tokio::test]
    async fn create_second_vault() {
        let (mut mgr, _vd) = test_manager().await;
        let second_dir = TempDir::new().unwrap();
        let info = mgr
            .create_vault("second".into(), second_dir.path().to_path_buf())
            .await
            .unwrap();
        assert_eq!(info.name, "second");
        assert_eq!(mgr.registry().list().len(), 2);
    }

    #[tokio::test]
    async fn switch_vault() {
        let (mut mgr, _vd) = test_manager().await;
        let second_dir = TempDir::new().unwrap();
        let info = mgr
            .create_vault("second".into(), second_dir.path().to_path_buf())
            .await
            .unwrap();
        let second_id = info.id;

        mgr.switch_vault(second_id).await.unwrap();
        assert_eq!(mgr.active_id(), Some(second_id));
        assert_eq!(mgr.active().unwrap().info.name, "second");
    }

    #[tokio::test]
    async fn close_vault_switches_active() {
        let (mut mgr, _vd) = test_manager().await;
        let second_dir = TempDir::new().unwrap();
        let info = mgr
            .create_vault("second".into(), second_dir.path().to_path_buf())
            .await
            .unwrap();
        let second_id = info.id;

        mgr.switch_vault(second_id).await.unwrap();
        mgr.close_vault(second_id);
        assert!(mgr.active_id().is_some());
        assert_ne!(mgr.active_id(), Some(second_id));
    }

    #[tokio::test]
    async fn remove_vault() {
        let (mut mgr, _vd) = test_manager().await;
        let second_dir = TempDir::new().unwrap();
        let info = mgr
            .create_vault("second".into(), second_dir.path().to_path_buf())
            .await
            .unwrap();
        let second_id = info.id;

        mgr.remove_vault(second_id).unwrap();
        assert_eq!(mgr.registry().list().len(), 1);
        assert!(mgr.get(second_id).is_none());
    }

    #[tokio::test]
    async fn get_specific_vault() {
        let (mgr, _vd) = test_manager().await;
        let id = mgr.active_id().unwrap();
        assert!(mgr.get(id).is_some());
        assert!(mgr.get(Uuid::new_v4()).is_none());
    }
}
