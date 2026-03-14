//! Plugin system — extensibility framework for Mneme.
//!
//! Defines the plugin trait and registry for extending Mneme
//! with custom note processors, importers, and exporters.

use serde::{Deserialize, Serialize};

/// Plugin metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub capabilities: Vec<PluginCapability>,
}

/// What a plugin can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    /// Process note content (transform, validate, enrich)
    NoteProcessor,
    /// Import from external source
    Importer,
    /// Export to external format
    Exporter,
    /// Custom search provider
    SearchProvider,
    /// Custom AI pipeline
    AiPipeline,
}

/// A hook point where plugins can intercept operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookPoint {
    /// Before a note is created
    PreCreate,
    /// After a note is created
    PostCreate,
    /// Before a note is updated
    PreUpdate,
    /// After a note is updated
    PostUpdate,
    /// Before a note is deleted
    PreDelete,
    /// After a note is deleted
    PostDelete,
    /// Before search results are returned
    PreSearch,
    /// After indexing a note
    PostIndex,
}

/// Plugin registry — tracks installed plugins and their capabilities.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginRegistry {
    pub plugins: Vec<PluginInfo>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a plugin.
    pub fn register(&mut self, plugin: PluginInfo) {
        // Don't register duplicates
        if !self.plugins.iter().any(|p| p.name == plugin.name) {
            self.plugins.push(plugin);
        }
    }

    /// Unregister a plugin by name.
    pub fn unregister(&mut self, name: &str) -> bool {
        let before = self.plugins.len();
        self.plugins.retain(|p| p.name != name);
        self.plugins.len() < before
    }

    /// Find plugins with a specific capability.
    pub fn find_by_capability(&self, cap: PluginCapability) -> Vec<&PluginInfo> {
        self.plugins
            .iter()
            .filter(|p| p.capabilities.contains(&cap))
            .collect()
    }

    /// List all registered plugins.
    pub fn list(&self) -> &[PluginInfo] {
        &self.plugins
    }

    /// Check if a plugin is registered.
    pub fn has_plugin(&self, name: &str) -> bool {
        self.plugins.iter().any(|p| p.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_plugin(name: &str, caps: Vec<PluginCapability>) -> PluginInfo {
        PluginInfo {
            name: name.into(),
            version: "1.0.0".into(),
            description: format!("Test plugin: {name}"),
            author: "test".into(),
            capabilities: caps,
        }
    }

    #[test]
    fn register_and_list() {
        let mut reg = PluginRegistry::new();
        reg.register(test_plugin(
            "markdown-lint",
            vec![PluginCapability::NoteProcessor],
        ));
        reg.register(test_plugin("csv-import", vec![PluginCapability::Importer]));
        assert_eq!(reg.list().len(), 2);
    }

    #[test]
    fn no_duplicates() {
        let mut reg = PluginRegistry::new();
        reg.register(test_plugin("test", vec![]));
        reg.register(test_plugin("test", vec![]));
        assert_eq!(reg.list().len(), 1);
    }

    #[test]
    fn unregister() {
        let mut reg = PluginRegistry::new();
        reg.register(test_plugin("removeme", vec![]));
        assert!(reg.unregister("removeme"));
        assert!(!reg.has_plugin("removeme"));
        assert!(!reg.unregister("nonexistent"));
    }

    #[test]
    fn find_by_capability() {
        let mut reg = PluginRegistry::new();
        reg.register(test_plugin(
            "proc1",
            vec![PluginCapability::NoteProcessor],
        ));
        reg.register(test_plugin("imp1", vec![PluginCapability::Importer]));
        reg.register(test_plugin(
            "proc2",
            vec![PluginCapability::NoteProcessor, PluginCapability::Exporter],
        ));

        let processors = reg.find_by_capability(PluginCapability::NoteProcessor);
        assert_eq!(processors.len(), 2);

        let importers = reg.find_by_capability(PluginCapability::Importer);
        assert_eq!(importers.len(), 1);
    }

    #[test]
    fn serialization() {
        let info = test_plugin("test", vec![PluginCapability::NoteProcessor]);
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("note_processor"));
    }
}
