//! Vault sharing — multi-user access and conflict resolution.
//!
//! Tracks vault sharing state, user permissions, and provides
//! conflict detection for concurrent edits.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A user with access to a shared vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultUser {
    pub id: Uuid,
    pub name: String,
    pub role: UserRole,
    pub joined_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
}

/// User permission level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    Owner,
    Editor,
    Viewer,
}

impl UserRole {
    pub fn can_edit(&self) -> bool {
        matches!(self, UserRole::Owner | UserRole::Editor)
    }

    pub fn can_manage(&self) -> bool {
        matches!(self, UserRole::Owner)
    }
}

/// Sharing configuration for a vault.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SharingConfig {
    pub enabled: bool,
    pub users: Vec<VaultUser>,
    pub allow_anonymous_read: bool,
}

impl SharingConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable sharing for this vault.
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Add a user to the vault.
    pub fn add_user(&mut self, name: &str, role: UserRole) -> VaultUser {
        let now = Utc::now();
        let user = VaultUser {
            id: Uuid::new_v4(),
            name: name.to_string(),
            role,
            joined_at: now,
            last_active: now,
        };
        self.users.push(user.clone());
        user
    }

    /// Remove a user by ID.
    pub fn remove_user(&mut self, user_id: Uuid) -> bool {
        let before = self.users.len();
        self.users.retain(|u| u.id != user_id);
        self.users.len() < before
    }

    /// Get a user by ID.
    pub fn get_user(&self, user_id: Uuid) -> Option<&VaultUser> {
        self.users.iter().find(|u| u.id == user_id)
    }

    /// Check if a user can edit.
    pub fn can_edit(&self, user_id: Uuid) -> bool {
        self.users
            .iter()
            .any(|u| u.id == user_id && u.role.can_edit())
    }

    /// List all editors and owner.
    pub fn editors(&self) -> Vec<&VaultUser> {
        self.users.iter().filter(|u| u.role.can_edit()).collect()
    }
}

/// A conflict detected during concurrent edits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditConflict {
    pub note_id: Uuid,
    pub user_a: String,
    pub user_b: String,
    pub base_hash: String,
    pub hash_a: String,
    pub hash_b: String,
    pub detected_at: DateTime<Utc>,
}

/// Conflict resolution strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictStrategy {
    /// Last write wins
    LastWrite,
    /// Keep both versions (create a conflict note)
    KeepBoth,
    /// Manual resolution required
    Manual,
}

/// Detect a conflict between two edits.
pub fn detect_conflict(
    note_id: Uuid,
    base_hash: &str,
    edit_a: (&str, &str), // (user_name, new_hash)
    edit_b: (&str, &str),
) -> Option<EditConflict> {
    if edit_a.1 != edit_b.1 {
        Some(EditConflict {
            note_id,
            user_a: edit_a.0.to_string(),
            user_b: edit_b.0.to_string(),
            base_hash: base_hash.to_string(),
            hash_a: edit_a.1.to_string(),
            hash_b: edit_b.1.to_string(),
            detected_at: Utc::now(),
        })
    } else {
        None // Same result, no conflict
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_list_users() {
        let mut config = SharingConfig::new();
        config.enable();
        config.add_user("Alice", UserRole::Owner);
        config.add_user("Bob", UserRole::Editor);
        config.add_user("Charlie", UserRole::Viewer);

        assert!(config.enabled);
        assert_eq!(config.users.len(), 3);
        assert_eq!(config.editors().len(), 2); // Alice + Bob
    }

    #[test]
    fn remove_user() {
        let mut config = SharingConfig::new();
        let user = config.add_user("Alice", UserRole::Editor);
        assert!(config.remove_user(user.id));
        assert!(!config.remove_user(Uuid::new_v4())); // nonexistent
    }

    #[test]
    fn role_permissions() {
        assert!(UserRole::Owner.can_edit());
        assert!(UserRole::Owner.can_manage());
        assert!(UserRole::Editor.can_edit());
        assert!(!UserRole::Editor.can_manage());
        assert!(!UserRole::Viewer.can_edit());
        assert!(!UserRole::Viewer.can_manage());
    }

    #[test]
    fn can_edit_check() {
        let mut config = SharingConfig::new();
        let editor = config.add_user("Editor", UserRole::Editor);
        let viewer = config.add_user("Viewer", UserRole::Viewer);

        assert!(config.can_edit(editor.id));
        assert!(!config.can_edit(viewer.id));
        assert!(!config.can_edit(Uuid::new_v4()));
    }

    #[test]
    fn detect_conflict_different_hashes() {
        let note_id = Uuid::new_v4();
        let conflict = detect_conflict(note_id, "base", ("Alice", "hash_a"), ("Bob", "hash_b"));
        assert!(conflict.is_some());
        let c = conflict.unwrap();
        assert_eq!(c.user_a, "Alice");
        assert_eq!(c.user_b, "Bob");
    }

    #[test]
    fn no_conflict_same_hash() {
        let note_id = Uuid::new_v4();
        let conflict = detect_conflict(note_id, "base", ("Alice", "same"), ("Bob", "same"));
        assert!(conflict.is_none());
    }

    #[test]
    fn sharing_serialization() {
        let mut config = SharingConfig::new();
        config.enable();
        config.add_user("Test", UserRole::Editor);
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("editor"));
    }

    #[test]
    fn get_user_by_id() {
        let mut config = SharingConfig::new();
        let user = config.add_user("Alice", UserRole::Editor);
        assert!(config.get_user(user.id).is_some());
        assert_eq!(config.get_user(user.id).unwrap().name, "Alice");
        assert!(config.get_user(Uuid::new_v4()).is_none());
    }
}
