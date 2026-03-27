//! User access resolver.
//!
//! Computes effective scopes for a user by merging:
//!   1. Built-in grants for the user's `primary_role_code`
//!   2. Additional grants from `sys_user_roles → sys_role_scope_access`
//!
//! Access modes are merged with `all > read` semantics.

use anyhow::Result;
use contracts::shared::access::ScopeAccess;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

use super::primary_roles;

/// Resolve effective scopes for a user.
/// Returns an empty Vec for `is_admin = true` users (they bypass all checks).
pub async fn resolve_user_scopes(user_id: &str) -> Result<Vec<ScopeAccess>> {
    use crate::shared::data::db::get_connection;
    let conn = get_connection();

    // Load primary_role_code and is_admin for this user.
    // Use a query that falls back gracefully if primary_role_code column doesn't exist yet.
    let row_result = conn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT primary_role_code, is_admin FROM sys_users WHERE id = ?",
            [user_id.into()],
        ))
        .await;

    // If the query fails (e.g. column not yet migrated), fall back to checking is_admin only
    let row = match row_result {
        Ok(r) => r,
        Err(_) => {
            // Fallback: query without primary_role_code
            let fb = conn
                .query_one(Statement::from_sql_and_values(
                    DatabaseBackend::Sqlite,
                    "SELECT is_admin FROM sys_users WHERE id = ?",
                    [user_id.into()],
                ))
                .await?;
            if fb
                .as_ref()
                .and_then(|r| r.try_get::<i64>("", "is_admin").ok())
                .map(|v| v != 0)
                .unwrap_or(false)
            {
                return Ok(vec![]); // admin — bypass
            }
            return Ok(vec![]); // no migration yet — deny non-admin gracefully
        }
    };

    let (primary_role_code, is_admin) = match row {
        None => return Ok(vec![]),
        Some(r) => {
            let role: String = r.try_get("", "primary_role_code").unwrap_or_default();
            let admin: bool = r
                .try_get::<i64>("", "is_admin")
                .map(|v| v != 0)
                .unwrap_or(false);
            (role, admin)
        }
    };

    // Admin users bypass all checks → return empty scopes
    if is_admin {
        return Ok(vec![]);
    }

    // Collect grants: scope_id → max mode ("all" > "read")
    let mut grants: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    // 1. Apply primary role grants from code catalog
    for (scope_id, mode) in primary_roles::grants_for_role(&primary_role_code) {
        merge_grant(&mut grants, scope_id, mode);
    }

    // 2. Apply additional DB role grants
    let db_rows = conn
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT rsa.access_scope_id, rsa.access_mode
             FROM sys_user_roles ur
             JOIN sys_role_scope_access rsa ON rsa.role_id = ur.role_id
             WHERE ur.user_id = ?",
            [user_id.into()],
        ))
        .await?;

    for row in db_rows {
        let scope: String = row.try_get("", "access_scope_id")?;
        let mode: String = row.try_get("", "access_mode")?;
        merge_grant(&mut grants, &scope, &mode);
    }

    // Convert to sorted Vec<ScopeAccess>
    let mut result: Vec<ScopeAccess> = grants
        .into_iter()
        .map(|(scope_id, mode)| ScopeAccess { scope_id, mode })
        .collect();
    result.sort_by(|a, b| a.scope_id.cmp(&b.scope_id));

    Ok(result)
}

/// Get the primary_role_code for a user (used during token generation).
/// Returns "viewer" as fallback if migration hasn't been applied yet.
pub async fn get_primary_role_code(user_id: &str) -> Result<String> {
    use crate::shared::data::db::get_connection;
    let conn = get_connection();

    let row = conn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT primary_role_code FROM sys_users WHERE id = ?",
            [user_id.into()],
        ))
        .await;

    // Graceful fallback: column may not exist before migration 0017
    match row {
        Ok(Some(r)) => Ok(r
            .try_get::<String>("", "primary_role_code")
            .unwrap_or_else(|_| "viewer".to_string())),
        _ => Ok("viewer".to_string()),
    }
}

/// Merge a new grant into the map, keeping the highest mode ("all" beats "read").
fn merge_grant(grants: &mut std::collections::HashMap<String, String>, scope_id: &str, mode: &str) {
    let entry = grants
        .entry(scope_id.to_string())
        .or_insert_with(|| mode.to_string());
    // "all" takes precedence over "read"
    if mode == "all" {
        *entry = "all".to_string();
    }
}
