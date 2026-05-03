//! Audit logic for the access control system.
//!
//! This module contains pure computation functions — no HTTP/Axum types.
//! HTTP handlers in `system/api/handlers/audit.rs` call into this module.

use contracts::system::{
    access::ScopeDescriptorDto,
    audit::{AuditReport, CoverageStats, RoutePolicyDto, ViolationEntry, ViolationType},
};

use crate::system::access::{
    primary_roles::{ADMIN_GRANTS, MANAGER_GRANTS, OPERATOR_GRANTS, VIEWER_GRANTS},
    route_registry::ROUTE_REGISTRY,
    scope_catalog::{find_scope, SCOPE_CATALOG},
};

/// Serialize the full route registry as DTOs.
pub fn get_route_registry() -> Vec<RoutePolicyDto> {
    ROUTE_REGISTRY
        .iter()
        .map(|p| RoutePolicyDto {
            method: p.method.to_string(),
            path: p.path.to_string(),
            scope_id: p.scope_id.map(|s| s.to_string()),
            mode: p.mode.as_str().to_string(),
            is_violation: p.mode.is_violation(),
        })
        .collect()
}

/// Return all scope descriptors from the catalog.
pub fn get_scope_catalog() -> Vec<ScopeDescriptorDto> {
    SCOPE_CATALOG.iter().map(ScopeDescriptorDto::from).collect()
}

/// Compute the full audit report:
/// - Violations (unscoped routes, orphan scopes, unknown scope IDs)
/// - Coverage statistics per built-in role
pub fn compute_violations() -> AuditReport {
    let catalog_ids: std::collections::HashSet<&str> =
        SCOPE_CATALOG.iter().map(|s| s.scope_id).collect();

    let registry_scope_ids: std::collections::HashSet<&str> =
        ROUTE_REGISTRY.iter().filter_map(|p| p.scope_id).collect();

    let mut violations: Vec<ViolationEntry> = Vec::new();

    // 1. Routes with AuthOnly mode — have no scope
    for p in ROUTE_REGISTRY {
        if p.mode.is_violation() {
            violations.push(ViolationEntry {
                violation_type: ViolationType::Unscoped,
                subject: format!("{} {}", p.method, p.path),
                description: format!(
                    "Route is protected by auth but has no scope_id — \
                     any authenticated user can access it regardless of role"
                ),
            });
        }
    }

    // 2. Routes referencing scope_ids not in the catalog
    for p in ROUTE_REGISTRY {
        if let Some(scope_id) = p.scope_id {
            if !catalog_ids.contains(scope_id) {
                violations.push(ViolationEntry {
                    violation_type: ViolationType::UnknownScopeId,
                    subject: format!("{} {}", p.method, p.path),
                    description: format!(
                        "scope_id '{}' is referenced in ROUTE_REGISTRY but not in SCOPE_CATALOG",
                        scope_id
                    ),
                });
            }
        }
    }

    // 3. Scope catalog entries not covered by any route (orphan)
    for scope in SCOPE_CATALOG {
        if !registry_scope_ids.contains(scope.scope_id) {
            violations.push(ViolationEntry {
                violation_type: ViolationType::OrphanScope,
                subject: scope.scope_id.to_string(),
                description: format!(
                    "Scope '{}' ({}) is in SCOPE_CATALOG but no route uses it",
                    scope.scope_id, scope.label
                ),
            });
        }
    }

    // 4. Coverage statistics — only for scoped catalog entries
    let scoped_catalog_count = SCOPE_CATALOG.len();

    let role_grants: &[(&str, &[(&str, &str)])] = &[
        ("admin", ADMIN_GRANTS),
        ("manager", MANAGER_GRANTS),
        ("operator", OPERATOR_GRANTS),
        ("viewer", VIEWER_GRANTS),
    ];

    let role_coverage: Vec<CoverageStats> = role_grants
        .iter()
        .map(|(role_code, grants)| {
            let granted_scopes: std::collections::HashSet<&str> =
                grants.iter().map(|(s, _)| *s).collect();
            let covered = SCOPE_CATALOG
                .iter()
                .filter(|s| granted_scopes.contains(s.scope_id))
                .count();
            CoverageStats {
                role_code: role_code.to_string(),
                covered,
                total: scoped_catalog_count,
            }
        })
        .collect();

    // Admin gets everything — override to 100%
    let role_coverage: Vec<CoverageStats> = role_coverage
        .into_iter()
        .map(|mut cs| {
            if cs.role_code == "admin" {
                cs.covered = cs.total;
            }
            cs
        })
        .collect();

    let total_routes = ROUTE_REGISTRY.len();
    let scoped_routes = ROUTE_REGISTRY
        .iter()
        .filter(|p| p.scope_id.is_some())
        .count();
    let unscoped_routes = ROUTE_REGISTRY
        .iter()
        .filter(|p| p.scope_id.is_none() && p.mode.is_violation())
        .count();
    let open_routes = ROUTE_REGISTRY
        .iter()
        .filter(|p| matches!(p.mode, contracts::system::access::PolicyMode::Public))
        .count();

    AuditReport {
        violations,
        role_coverage,
        total_routes,
        scoped_routes,
        unscoped_routes,
        open_routes,
    }
}

/// Find a scope descriptor by ID (for use in handlers).
pub fn scope_descriptor(scope_id: &str) -> Option<ScopeDescriptorDto> {
    find_scope(scope_id).map(ScopeDescriptorDto::from)
}
