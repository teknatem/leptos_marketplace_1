use anyhow::Result;
use contracts::system::roles::{CreateRoleDto, Role, RoleScopeAccess, UpdateRoleDto};

use super::repository;
use crate::system::access::primary_roles;

pub async fn list_all() -> Result<Vec<Role>> {
    repository::list_all().await
}

pub async fn get_by_id(id: &str) -> Result<Option<Role>> {
    repository::get_by_id(id).await
}

pub async fn create(dto: CreateRoleDto) -> Result<String> {
    if dto.code.trim().is_empty() {
        return Err(anyhow::anyhow!("Role code cannot be empty"));
    }
    if dto.name.trim().is_empty() {
        return Err(anyhow::anyhow!("Role name cannot be empty"));
    }

    if let Some(_) = repository::get_by_code(&dto.code).await? {
        return Err(anyhow::anyhow!("Role code already exists"));
    }

    repository::create(&dto).await
}

pub async fn update(dto: UpdateRoleDto) -> Result<()> {
    let role = repository::get_by_id(&dto.id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Role not found"))?;

    if role.is_system {
        return Err(anyhow::anyhow!("Cannot edit a system role"));
    }

    if dto.name.trim().is_empty() {
        return Err(anyhow::anyhow!("Role name cannot be empty"));
    }

    repository::update(&dto).await
}

pub async fn delete(id: &str) -> Result<bool> {
    let role = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Role not found"))?;

    if role.is_system {
        return Err(anyhow::anyhow!("Cannot delete a system role"));
    }

    repository::delete(id).await
}

/// Returns effective permissions for a role.
/// For system roles the grants come from code (primary_roles module).
/// For custom DB roles the grants come from sys_role_scope_access.
pub async fn get_permissions(role_id: &str) -> Result<Vec<RoleScopeAccess>> {
    let role = repository::get_by_id(role_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Role not found"))?;

    if role.is_system {
        let grants = primary_roles::grants_for_role(&role.code);
        let result = grants
            .iter()
            .map(|(scope_id, mode)| RoleScopeAccess {
                scope_id: scope_id.to_string(),
                access_mode: mode.to_string(),
            })
            .collect();
        return Ok(result);
    }

    repository::get_scope_access(role_id).await
}
