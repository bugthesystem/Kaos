//! Role-based access control.

use crate::console::types::ApiKeyScope;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// User or API key identity.
#[derive(Debug, Clone)]
pub enum Identity {
    /// Authenticated user.
    User {
        id: Uuid,
        username: String,
        role: Role,
    },
    /// API key.
    ApiKey {
        id: Uuid,
        name: String,
        scopes: ApiKeyScope,
    },
}

impl Identity {
    /// Check if identity has permission.
    pub fn has_permission(&self, permission: Permission) -> bool {
        match self {
            Identity::User { role, .. } => role.has_permission(permission),
            Identity::ApiKey { scopes, .. } => scopes.has_permission(permission),
        }
    }

    /// Get display name.
    pub fn name(&self) -> &str {
        match self {
            Identity::User { username, .. } => username,
            Identity::ApiKey { name, .. } => name,
        }
    }
}

/// User roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Full access to everything.
    Admin,
    /// Read + write access, no account management.
    Developer,
    /// Read-only access.
    Viewer,
}

impl Role {
    /// Check if role has permission.
    pub fn has_permission(&self, permission: Permission) -> bool {
        match permission {
            Permission::ViewStatus => true,
            Permission::ViewSessions => true,
            Permission::ViewRooms => true,
            Permission::ViewLua => true,
            Permission::ViewConfig => true,
            Permission::KickSession => matches!(self, Role::Admin | Role::Developer),
            Permission::TerminateRoom => matches!(self, Role::Admin | Role::Developer),
            Permission::ExecuteRpc => matches!(self, Role::Admin | Role::Developer),
            Permission::ManageAccounts => matches!(self, Role::Admin),
            Permission::ManageApiKeys => matches!(self, Role::Admin),
        }
    }

    /// Convert to string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::Developer => "developer",
            Role::Viewer => "viewer",
        }
    }

    /// Parse from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "admin" => Some(Role::Admin),
            "developer" => Some(Role::Developer),
            "viewer" => Some(Role::Viewer),
            _ => None,
        }
    }
}

/// Permissions for actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    ViewStatus,
    ViewSessions,
    ViewRooms,
    ViewLua,
    ViewConfig,
    KickSession,
    TerminateRoom,
    ExecuteRpc,
    ManageAccounts,
    ManageApiKeys,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_permissions() {
        let role = Role::Admin;
        assert!(role.has_permission(Permission::ViewStatus));
        assert!(role.has_permission(Permission::KickSession));
        assert!(role.has_permission(Permission::ManageAccounts));
        assert!(role.has_permission(Permission::ManageApiKeys));
    }

    #[test]
    fn test_developer_permissions() {
        let role = Role::Developer;
        assert!(role.has_permission(Permission::ViewStatus));
        assert!(role.has_permission(Permission::KickSession));
        assert!(!role.has_permission(Permission::ManageAccounts));
        assert!(!role.has_permission(Permission::ManageApiKeys));
    }

    #[test]
    fn test_viewer_permissions() {
        let role = Role::Viewer;
        assert!(role.has_permission(Permission::ViewStatus));
        assert!(!role.has_permission(Permission::KickSession));
        assert!(!role.has_permission(Permission::ManageAccounts));
    }

    #[test]
    fn test_role_serialization() {
        assert_eq!(Role::Admin.as_str(), "admin");
        assert_eq!(Role::from_str("admin"), Some(Role::Admin));
        assert_eq!(Role::from_str("ADMIN"), Some(Role::Admin));
        assert_eq!(Role::from_str("invalid"), None);
    }
}
