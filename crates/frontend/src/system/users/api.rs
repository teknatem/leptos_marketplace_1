use contracts::system::users::{ChangePasswordDto, CreateUserDto, UpdateUserDto, User};
use gloo_net::http::Request;

use crate::shared::api_utils::api_base;
use crate::system::auth::storage;

fn get_auth_header() -> Option<String> {
    storage::get_access_token().map(|token| format!("Bearer {}", token))
}

/// Fetch all users
pub async fn fetch_users() -> Result<Vec<User>, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::get(&format!("{}/api/system/users", api_base()))
        .header("Authorization", &auth_header)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to fetch users: {}", response.status()));
    }

    response
        .json::<Vec<User>>()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Create new user
pub async fn create_user(dto: CreateUserDto) -> Result<String, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::post(&format!("{}/api/system/users", api_base()))
        .header("Authorization", &auth_header)
        .json(&dto)
        .map_err(|e| format!("Failed to serialize request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to create user: {}", response.status()));
    }

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(result["id"].as_str().unwrap_or("").to_string())
}

/// Update user
pub async fn update_user(dto: UpdateUserDto) -> Result<(), String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::put(&format!("{}/api/system/users/{}", api_base(), dto.id))
        .header("Authorization", &auth_header)
        .json(&dto)
        .map_err(|e| format!("Failed to serialize request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to update user: {}", response.status()));
    }

    Ok(())
}

/// Delete user
pub async fn delete_user(id: &str) -> Result<(), String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::delete(&format!("{}/api/system/users/{}", api_base(), id))
        .header("Authorization", &auth_header)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to delete user: {}", response.status()));
    }

    Ok(())
}

/// Change password
pub async fn change_password(dto: ChangePasswordDto) -> Result<(), String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::post(&format!(
        "{}/api/system/users/{}/change-password",
        api_base(), dto.user_id
    ))
    .header("Authorization", &auth_header)
    .json(&dto)
    .map_err(|e| format!("Failed to serialize request: {}", e))?
    .send()
    .await
    .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to change password: {}", response.status()));
    }

    Ok(())
}
