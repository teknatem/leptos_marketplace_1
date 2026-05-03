//! Thin HTTP handlers for the access control audit API.
//!
//! All logic lives in `system::audit`. These handlers are just JSON wrappers.

use axum::Json;
use contracts::system::{
    access::ScopeDescriptorDto,
    audit::{AuditReport, RoutePolicyDto},
};

use crate::system::audit as audit_svc;

/// GET /api/system/audit/routes
/// Returns the full route registry with policy metadata.
pub async fn list_routes() -> Json<Vec<RoutePolicyDto>> {
    Json(audit_svc::get_route_registry())
}

/// GET /api/system/audit/violations
/// Computes and returns the full audit report with violations and coverage stats.
pub async fn list_violations() -> Json<AuditReport> {
    Json(audit_svc::compute_violations())
}

/// GET /api/system/scopes
/// Returns all known scopes from the catalog with rich metadata.
pub async fn list_scopes() -> Json<Vec<ScopeDescriptorDto>> {
    Json(audit_svc::get_scope_catalog())
}
