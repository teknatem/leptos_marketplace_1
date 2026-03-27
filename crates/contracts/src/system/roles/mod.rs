use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: String,
    pub code: String,
    pub name: String,
    pub is_system: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRoleDto {
    pub code: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRoleDto {
    pub id: String,
    pub name: String,
}

/// Single scope → access_mode entry for a role's permission set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleScopeAccess {
    pub scope_id: String,
    /// "none" | "read" | "all"
    pub access_mode: String,
}

/// Minimal scope descriptor returned by GET /api/system/scopes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeInfo {
    pub scope_id: String,
}
