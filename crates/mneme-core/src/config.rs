//! Configuration types for mneme.toml.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Top-level Mneme configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MnemeConfig {
    /// Default vault name to open on startup.
    #[serde(default)]
    pub default_vault: Option<String>,
    /// Path to the vault registry file.
    #[serde(default)]
    pub registry_path: Option<PathBuf>,
    /// Inline vault definitions (alternative to registry).
    #[serde(default)]
    pub vaults: Vec<VaultConfigEntry>,
}

/// A vault entry in the config file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfigEntry {
    pub name: String,
    pub path: PathBuf,
    #[serde(default)]
    pub description: Option<String>,
    /// Weight multiplier for cross-vault search ranking (default: 1.0).
    #[serde(default = "default_weight")]
    pub search_weight: f64,
}

fn default_weight() -> f64 {
    1.0
}

/// Metadata about a registered vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultInfo {
    pub id: uuid::Uuid,
    pub name: String,
    pub path: PathBuf,
    #[serde(default)]
    pub description: String,
    pub search_weight: f64,
    pub last_opened: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub is_default: bool,
}

impl VaultInfo {
    pub fn new(name: String, path: PathBuf) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            name,
            path,
            description: String::new(),
            search_weight: 1.0,
            last_opened: chrono::Utc::now(),
            is_default: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_empty() {
        let config = MnemeConfig::default();
        assert!(config.default_vault.is_none());
        assert!(config.vaults.is_empty());
    }

    #[test]
    fn vault_config_entry_serde_roundtrip() {
        let entry = VaultConfigEntry {
            name: "work".into(),
            path: PathBuf::from("/home/user/work-notes"),
            description: Some("Work notes".into()),
            search_weight: 1.5,
        };
        let toml_str = toml::to_string(&entry).unwrap();
        let parsed: VaultConfigEntry = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.name, "work");
        assert_eq!(parsed.search_weight, 1.5);
    }

    #[test]
    fn vault_info_new_defaults() {
        let info = VaultInfo::new("test".into(), PathBuf::from("/tmp/test"));
        assert_eq!(info.name, "test");
        assert_eq!(info.search_weight, 1.0);
        assert!(!info.is_default);
    }

    #[test]
    fn full_config_serde_roundtrip() {
        let config = MnemeConfig {
            default_vault: Some("work".into()),
            registry_path: None,
            vaults: vec![
                VaultConfigEntry {
                    name: "work".into(),
                    path: PathBuf::from("/work"),
                    description: None,
                    search_weight: 1.0,
                },
                VaultConfigEntry {
                    name: "personal".into(),
                    path: PathBuf::from("/personal"),
                    description: Some("Personal stuff".into()),
                    search_weight: 0.5,
                },
            ],
        };
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: MnemeConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.default_vault, Some("work".into()));
        assert_eq!(parsed.vaults.len(), 2);
    }
}
