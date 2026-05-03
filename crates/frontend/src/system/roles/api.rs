use contracts::system::access::ScopeDescriptorDto;
use contracts::system::roles::{CreateRoleDto, Role, RoleScopeAccess, UpdateRoleDto};
use gloo_net::http::Request;

use crate::shared::api_utils::api_base;
use crate::system::auth::storage;

fn get_auth_header() -> Option<String> {
    storage::get_access_token().map(|token| format!("Bearer {}", token))
}

pub async fn fetch_roles() -> Result<Vec<Role>, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::get(&format!("{}/api/system/roles", api_base()))
        .header("Authorization", &auth_header)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to fetch roles: {}", response.status()));
    }

    response
        .json::<Vec<Role>>()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

pub async fn create_role(dto: CreateRoleDto) -> Result<String, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::post(&format!("{}/api/system/roles", api_base()))
        .header("Authorization", &auth_header)
        .json(&dto)
        .map_err(|e| format!("Failed to serialize request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to create role: {}", response.status()));
    }

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(result["id"].as_str().unwrap_or("").to_string())
}

pub async fn update_role(dto: UpdateRoleDto) -> Result<(), String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::put(&format!("{}/api/system/roles/{}", api_base(), dto.id))
        .header("Authorization", &auth_header)
        .json(&dto)
        .map_err(|e| format!("Failed to serialize request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to update role: {}", response.status()));
    }

    Ok(())
}

pub async fn delete_role(id: &str) -> Result<(), String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::delete(&format!("{}/api/system/roles/{}", api_base(), id))
        .header("Authorization", &auth_header)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to delete role: {}", response.status()));
    }

    Ok(())
}

pub async fn fetch_role_permissions(role_id: &str) -> Result<Vec<RoleScopeAccess>, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::get(&format!(
        "{}/api/system/roles/{}/permissions",
        api_base(),
        role_id
    ))
    .header("Authorization", &auth_header)
    .send()
    .await
    .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!(
            "Failed to fetch permissions: {}",
            response.status()
        ));
    }

    response
        .json::<Vec<RoleScopeAccess>>()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

pub async fn fetch_scopes() -> Result<Vec<ScopeDescriptorDto>, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::get(&format!("{}/api/system/scopes", api_base()))
        .header("Authorization", &auth_header)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to fetch scopes: {}", response.status()));
    }

    response
        .json::<Vec<ScopeDescriptorDto>>()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

pub async fn update_role_permissions(
    role_id: &str,
    grants: Vec<RoleScopeAccess>,
) -> Result<(), String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::put(&format!(
        "{}/api/system/roles/{}/permissions",
        api_base(),
        role_id
    ))
    .header("Authorization", &auth_header)
    .json(&grants)
    .map_err(|e| format!("Failed to serialize request: {}", e))?
    .send()
    .await
    .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!(
            "Failed to update permissions: {}",
            response.status()
        ));
    }

    Ok(())
}
