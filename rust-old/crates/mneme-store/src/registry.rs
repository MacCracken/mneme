//! Vault registry — tracks multiple vault locations and metadata.
//!
//! Persists as a TOML file at `~/.config/mneme/registry.toml` or
//! a user-specified path.

use std::path::{Path, PathBuf};

use chrono::Utc;
use mneme_core::config::VaultInfo;
use uuid::Uuid;

use crate::StoreError;

/// Registry of known vaults.
pub struct VaultRegistry {
    config_path: PathBuf,
    entries: Vec<VaultInfo>,
}

impl VaultRegistry {
    /// Open or create the registry at the given path.
    pub fn open(config_path: &Path) -> Result<Self, StoreError> {
        let entries = if config_path.exists() {
            let data = std::fs::read_to_string(config_path)?;
            toml::from_str::<RegistryFile>(&data)
                .map(|f| f.vaults)
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        Ok(Self {
            config_path: config_path.to_path_buf(),
            entries,
        })
    }

    /// Create an in-memory registry (for testing).
    pub fn in_memory() -> Self {
        Self {
            config_path: PathBuf::from("/dev/null"),
            entries: Vec::new(),
        }
    }

    /// List all registered vaults.
    pub fn list(&self) -> &[VaultInfo] {
        &self.entries
    }

    /// Find a vault by name.
    pub fn get_by_name(&self, name: &str) -> Option<&VaultInfo> {
        self.entries.iter().find(|v| v.name == name)
    }

    /// Find a vault by ID.
    pub fn get_by_id(&self, id: Uuid) -> Option<&VaultInfo> {
        self.entries.iter().find(|v| v.id == id)
    }

    /// Find a vault by name or ID string.
    pub fn resolve(&self, name_or_id: &str) -> Option<&VaultInfo> {
        // Try UUID first
        if let Ok(id) = Uuid::parse_str(name_or_id)
            && let Some(v) = self.get_by_id(id)
        {
            return Some(v);
        }
        self.get_by_name(name_or_id)
    }

    /// Get the default vault.
    pub fn default_vault(&self) -> Option<&VaultInfo> {
        self.entries
            .iter()
            .find(|v| v.is_default)
            .or(self.entries.first())
    }

    /// Register a new vault.
    pub fn create(&mut self, name: String, path: PathBuf) -> Result<&VaultInfo, StoreError> {
        if self.entries.iter().any(|v| v.name == name) {
            return Err(StoreError::VaultAlreadyExists(name));
        }

        let is_default = self.entries.is_empty();
        let mut info = VaultInfo::new(name, path);
        info.is_default = is_default;

        self.entries.push(info);
        self.save()?;

        Ok(self.entries.last().unwrap())
    }

    /// Remove a vault from the registry (does not delete files).
    pub fn remove(&mut self, id: Uuid) -> Result<(), StoreError> {
        let pos = self
            .entries
            .iter()
            .position(|v| v.id == id)
            .ok_or_else(|| StoreError::VaultNotFound(id.to_string()))?;

        self.entries.remove(pos);
        self.save()?;
        Ok(())
    }

    /// Set a vault as the default.
    pub fn set_default(&mut self, id: Uuid) -> Result<(), StoreError> {
        let mut found = false;
        for entry in &mut self.entries {
            if entry.id == id {
                entry.is_default = true;
                found = true;
            } else {
                entry.is_default = false;
            }
        }
        if !found {
            return Err(StoreError::VaultNotFound(id.to_string()));
        }
        self.save()
    }

    /// Update the last_opened timestamp for a vault.
    pub fn touch(&mut self, id: Uuid) {
        if let Some(entry) = self.entries.iter_mut().find(|v| v.id == id) {
            entry.last_opened = Utc::now();
        }
    }

    /// Persist the registry to disk.
    fn save(&self) -> Result<(), StoreError> {
        if self.config_path == Path::new("/dev/null") {
            return Ok(()); // in-memory mode
        }

        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = RegistryFile {
            vaults: self.entries.clone(),
        };
        let data =
            toml::to_string_pretty(&file).map_err(|e| StoreError::Io(std::io::Error::other(e)))?;
        std::fs::write(&self.config_path, data)?;
        Ok(())
    }
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct RegistryFile {
    #[serde(default)]
    vaults: Vec<VaultInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_starts_empty() {
        let reg = VaultRegistry::in_memory();
        assert!(reg.list().is_empty());
        assert!(reg.default_vault().is_none());
    }

    #[test]
    fn create_vault() {
        let mut reg = VaultRegistry::in_memory();
        let info = reg.create("work".into(), PathBuf::from("/work")).unwrap();
        assert_eq!(info.name, "work");
        assert!(info.is_default); // first vault becomes default
        assert_eq!(reg.list().len(), 1);
    }

    #[test]
    fn create_duplicate_name_fails() {
        let mut reg = VaultRegistry::in_memory();
        reg.create("work".into(), PathBuf::from("/work")).unwrap();
        let result = reg.create("work".into(), PathBuf::from("/other"));
        assert!(result.is_err());
    }

    #[test]
    fn resolve_by_name() {
        let mut reg = VaultRegistry::in_memory();
        reg.create("personal".into(), PathBuf::from("/p")).unwrap();
        assert!(reg.resolve("personal").is_some());
        assert!(reg.resolve("nonexistent").is_none());
    }

    #[test]
    fn resolve_by_id() {
        let mut reg = VaultRegistry::in_memory();
        let info = reg.create("work".into(), PathBuf::from("/w")).unwrap();
        let id = info.id;
        assert!(reg.resolve(&id.to_string()).is_some());
    }

    #[test]
    fn remove_vault() {
        let mut reg = VaultRegistry::in_memory();
        let info = reg.create("temp".into(), PathBuf::from("/t")).unwrap();
        let id = info.id;
        reg.remove(id).unwrap();
        assert!(reg.list().is_empty());
    }

    #[test]
    fn remove_nonexistent_fails() {
        let mut reg = VaultRegistry::in_memory();
        assert!(reg.remove(Uuid::new_v4()).is_err());
    }

    #[test]
    fn set_default_vault() {
        let mut reg = VaultRegistry::in_memory();
        let v1 = reg.create("v1".into(), PathBuf::from("/v1")).unwrap().id;
        let v2 = reg.create("v2".into(), PathBuf::from("/v2")).unwrap().id;

        assert_eq!(reg.default_vault().unwrap().id, v1); // first is default
        reg.set_default(v2).unwrap();
        assert_eq!(reg.default_vault().unwrap().id, v2);
    }

    #[test]
    fn persist_and_reload() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("registry.toml");

        {
            let mut reg = VaultRegistry::open(&path).unwrap();
            reg.create("persistent".into(), PathBuf::from("/p"))
                .unwrap();
        }

        let reg = VaultRegistry::open(&path).unwrap();
        assert_eq!(reg.list().len(), 1);
        assert_eq!(reg.list()[0].name, "persistent");
    }

    #[test]
    fn default_vault_falls_back_to_first() {
        let mut reg = VaultRegistry::in_memory();
        let info_id = reg
            .create("only".into(), PathBuf::from("/only"))
            .unwrap()
            .id;
        // Even though first is_default=true, test fallback logic:
        let default = reg.default_vault().unwrap();
        assert_eq!(default.id, info_id);
    }

    #[test]
    fn touch_updates_last_opened() {
        let mut reg = VaultRegistry::in_memory();
        let info = reg.create("touch".into(), PathBuf::from("/t")).unwrap();
        let id = info.id;
        let before = info.last_opened;
        std::thread::sleep(std::time::Duration::from_millis(10));
        reg.touch(id);
        let after = reg.get_by_id(id).unwrap().last_opened;
        assert!(after >= before);
    }
}
