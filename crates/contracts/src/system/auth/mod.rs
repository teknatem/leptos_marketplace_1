use serde::{Deserialize, Serialize};

use crate::shared::access::ScopeAccess;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub user: UserInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshResponse {
    pub access_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub full_name: Option<String>,
    pub email: Option<String>,
    pub is_admin: bool,
    /// Primary role code (e.g. "admin", "manager", "operator", "viewer")
    #[serde(default = "default_viewer_role")]
    pub primary_role: String,
    /// Effective scope access list, resolved from primary + additional roles.
    /// Empty for is_admin=true users (they bypass all scope checks).
    #[serde(default)]
    pub scopes: Vec<ScopeAccess>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    pub sub: String, // user_id
    pub username: String,
    pub is_admin: bool,
    /// Primary role code — carried in JWT to avoid DB lookup on every request.
    /// Defaults to "viewer" for tokens issued before this field was added.
    #[serde(default = "default_viewer_role")]
    pub primary_role: String,
    pub exp: usize, // expiration timestamp
    pub iat: usize, // issued at
}

fn default_viewer_role() -> String {
    "viewer".to_string()
}
