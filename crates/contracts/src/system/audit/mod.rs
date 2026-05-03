//! Audit DTOs for the access policy audit API.
//!
//! Mirrors `backend/src/system/audit/` — contains only serializable data transfer types.

use serde::{Deserialize, Serialize};

/// Serializable representation of a single route policy entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutePolicyDto {
    /// HTTP method: "GET", "POST", "PUT", "DELETE", "PATCH", "*"
    pub method: String,
    /// URL path pattern
    pub path: String,
    /// Scope identifier, if any
    pub scope_id: Option<String>,
    /// Policy mode string: "auto", "read_only", "admin_only", "auth_only", "public"
    pub mode: String,
    /// True if this entry represents a policy violation
    pub is_violation: bool,
}

/// Type of policy violation detected by the audit engine.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ViolationType {
    /// Route has `AuthOnly` mode — protected by auth but no scope assigned
    Unscoped,
    /// Route has `Public` mode but is not in the official public whitelist (future use)
    OpenNoAuth,
    /// Scope exists in catalog but no route uses it
    OrphanScope,
    /// Scope ID referenced in a route policy is not in the scope catalog
    UnknownScopeId,
}

impl ViolationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unscoped => "unscoped",
            Self::OpenNoAuth => "open_no_auth",
            Self::OrphanScope => "orphan_scope",
            Self::UnknownScopeId => "unknown_scope_id",
        }
    }
}

/// One violation entry in the audit report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViolationEntry {
    pub violation_type: ViolationType,
    /// The route path or scope_id this violation is about
    pub subject: String,
    /// Human-readable description
    pub description: String,
}

/// Per-role scope coverage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageStats {
    pub role_code: String,
    /// Number of scopes this role has explicit grants for
    pub covered: usize,
    /// Total number of scopes that require grants (non-admin, non-public)
    pub total: usize,
}

/// Full audit report returned by `GET /api/system/audit/violations`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub violations: Vec<ViolationEntry>,
    pub role_coverage: Vec<CoverageStats>,
    pub total_routes: usize,
    pub scoped_routes: usize,
    pub unscoped_routes: usize,
    pub open_routes: usize,
}
