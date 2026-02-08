use contracts::system::auth::{
    LoginRequest, LoginResponse, RefreshRequest, RefreshResponse, UserInfo,
};
use gloo_net::http::Request;

use crate::shared::api_utils::api_base;

/// Login with username and password
pub async fn login(username: String, password: String) -> Result<LoginResponse, String> {
    let request = LoginRequest { username, password };

    let response = Request::post(&format!("{}/api/system/auth/login", api_base()))
        .json(&request)
        .map_err(|e| format!("Failed to serialize request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!("Login failed: {}", response.status()));
    }

    response
        .json::<LoginResponse>()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Refresh access token using refresh token
pub async fn refresh_token(refresh_token: String) -> Result<RefreshResponse, String> {
    let request = RefreshRequest { refresh_token };

    let response = Request::post(&format!("{}/api/system/auth/refresh", api_base()))
        .json(&request)
        .map_err(|e| format!("Failed to serialize request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!("Refresh failed: {}", response.status()));
    }

    response
        .json::<RefreshResponse>()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Logout (revoke refresh token)
pub async fn logout(refresh_token: String) -> Result<(), String> {
    let request = RefreshRequest { refresh_token };

    let response = Request::post(&format!("{}/api/system/auth/logout", api_base()))
        .json(&request)
        .map_err(|e| format!("Failed to serialize request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!("Logout failed: {}", response.status()));
    }

    Ok(())
}

/// Get current user info
pub async fn get_current_user(access_token: &str) -> Result<UserInfo, String> {
    let response = Request::get(&format!("{}/api/system/auth/me", api_base()))
        .header("Authorization", &format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!("Get current user failed: {}", response.status()));
    }

    response
        .json::<UserInfo>()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Fetch with authentication (helper function)
pub async fn fetch_with_auth<T>(url: &str, access_token: &str) -> Result<T, String>
where
    T: for<'de> serde::Deserialize<'de>,
{
    let response = Request::get(&format!("{}{}", api_base(), url))
        .header("Authorization", &format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!("Request failed: {}", response.status()));
    }

    response
        .json::<T>()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}
