use axum::{extract::Json, http::StatusCode};
use contracts::system::auth::{LoginRequest, LoginResponse, RefreshRequest, RefreshResponse, UserInfo};

use crate::system::{auth::jwt, users::service as user_service};
use crate::system::auth::extractor::CurrentUser;

/// Login handler
pub async fn login(
    Json(request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    // Verify credentials
    let user = user_service::verify_credentials(&request.username, &request.password)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Generate tokens
    let access_token = jwt::generate_access_token(&user.id, &user.username, user.is_admin)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let refresh_token = jwt::generate_refresh_token();

    // Store refresh token in database
    store_refresh_token(&user.id, &refresh_token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = LoginResponse {
        access_token,
        refresh_token,
        user: UserInfo {
            id: user.id,
            username: user.username,
            full_name: user.full_name,
            email: user.email,
            is_admin: user.is_admin,
        },
    };

    Ok(Json(response))
}

/// Refresh token handler
pub async fn refresh(
    Json(request): Json<RefreshRequest>,
) -> Result<Json<RefreshResponse>, StatusCode> {
    // Validate refresh token from database
    let user_id = validate_refresh_token(&request.refresh_token)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Get user
    let user = user_service::get_by_id(&user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Generate new access token
    let access_token = jwt::generate_access_token(&user.id, &user.username, user.is_admin)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = RefreshResponse { access_token };

    Ok(Json(response))
}

/// Logout handler
pub async fn logout(
    Json(request): Json<RefreshRequest>,
) -> Result<StatusCode, StatusCode> {
    // Revoke refresh token
    revoke_refresh_token(&request.refresh_token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

/// Get current user handler (protected by middleware)
pub async fn current_user(
    CurrentUser(claims): CurrentUser,
) -> Result<Json<UserInfo>, StatusCode> {
    // Get user from database
    let user = user_service::get_by_id(&claims.sub)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let user_info = UserInfo {
        id: user.id,
        username: user.username,
        full_name: user.full_name,
        email: user.email,
        is_admin: user.is_admin,
    };

    Ok(Json(user_info))
}

// Helper functions for refresh tokens

async fn store_refresh_token(user_id: &str, token: &str) -> anyhow::Result<()> {
    use crate::shared::data::db::get_connection;
    use chrono::Utc;
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

    let token_id = uuid::Uuid::new_v4().to_string();
    let token_hash = hash_token(token);
    let expires_at = jwt::calculate_refresh_token_expiration();
    let created_at = Utc::now().to_rfc3339();

    let conn = get_connection();
    conn.execute(Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        "INSERT INTO sys_refresh_tokens (id, user_id, token_hash, expires_at, created_at) 
         VALUES (?, ?, ?, ?, ?)",
        [
            token_id.into(),
            user_id.to_string().into(),
            token_hash.into(),
            expires_at.into(),
            created_at.into(),
        ],
    ))
    .await?;

    Ok(())
}

async fn validate_refresh_token(token: &str) -> anyhow::Result<String> {
    use crate::shared::data::db::get_connection;
    use chrono::Utc;
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

    let token_hash = hash_token(token);
    let now = Utc::now().to_rfc3339();

    let conn = get_connection();
    let result = conn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT user_id FROM sys_refresh_tokens 
             WHERE token_hash = ? AND expires_at > ? AND revoked_at IS NULL",
            [token_hash.into(), now.into()],
        ))
        .await?;

    match result {
        Some(row) => {
            let user_id: String = row.try_get("", "user_id")?;
            Ok(user_id)
        }
        None => Err(anyhow::anyhow!("Invalid or expired refresh token")),
    }
}

async fn revoke_refresh_token(token: &str) -> anyhow::Result<()> {
    use crate::shared::data::db::get_connection;
    use chrono::Utc;
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

    let token_hash = hash_token(token);
    let revoked_at = Utc::now().to_rfc3339();

    let conn = get_connection();
    conn.execute(Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        "UPDATE sys_refresh_tokens SET revoked_at = ? WHERE token_hash = ?",
        [revoked_at.into(), token_hash.into()],
    ))
    .await?;

    Ok(())
}

fn hash_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

