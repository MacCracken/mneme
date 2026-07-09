//! Tag types — labels and hierarchical organization.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A tag that can be applied to notes.
///
/// Tags support hierarchy via `/` separators (e.g. "project/agnos/core").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: Uuid,
    pub name: String,
    /// Optional hex color (e.g. "#3b82f6").
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Tag {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            color: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_color(mut self, color: String) -> Self {
        self.color = Some(color);
        self
    }

    /// Returns the parent tag path, if this is a nested tag.
    ///
    /// `"project/agnos/core"` → `Some("project/agnos")`
    /// `"toplevel"` → `None`
    pub fn parent_path(&self) -> Option<&str> {
        self.name.rsplit_once('/').map(|(parent, _)| parent)
    }

    /// Returns all ancestor paths from root to immediate parent.
    ///
    /// `"a/b/c"` → `["a", "a/b"]`
    pub fn ancestor_paths(&self) -> Vec<&str> {
        let mut paths = Vec::new();
        let parts: Vec<&str> = self.name.split('/').collect();
        for i in 1..parts.len() {
            // Find the byte offset of the i-th separator
            let end = parts[..i].iter().map(|p| p.len()).sum::<usize>() + (i - 1); // account for separators
            paths.push(&self.name[..end]);
        }
        paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parent_path_nested() {
        let tag = Tag::new("project/agnos/core".into());
        assert_eq!(tag.parent_path(), Some("project/agnos"));
    }

    #[test]
    fn parent_path_top_level() {
        let tag = Tag::new("toplevel".into());
        assert_eq!(tag.parent_path(), None);
    }

    #[test]
    fn ancestor_paths() {
        let tag = Tag::new("a/b/c".into());
        assert_eq!(tag.ancestor_paths(), vec!["a", "a/b"]);
    }

    #[test]
    fn with_color() {
        let tag = Tag::new("test".into()).with_color("#ff0000".into());
        assert_eq!(tag.color, Some("#ff0000".into()));
    }
}
