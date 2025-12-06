use anyhow::{Context, Result};
use chrono::Utc;
use contracts::system::auth::TokenClaims;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand::Rng;

const ACCESS_TOKEN_LIFETIME_HOURS: i64 = 24; // 24 hours for long lifetime
const REFRESH_TOKEN_LIFETIME_DAYS: i64 = 90; // 90 days

/// Generate JWT access token with 24 hours lifetime
pub async fn generate_access_token(user_id: &str, username: &str, is_admin: bool) -> Result<String> {
    let now = Utc::now();
    let exp = (now + chrono::Duration::hours(ACCESS_TOKEN_LIFETIME_HOURS)).timestamp() as usize;
    let iat = now.timestamp() as usize;

    let claims = TokenClaims {
        sub: user_id.to_string(),
        username: username.to_string(),
        is_admin,
        exp,
        iat,
    };

    let secret = get_jwt_secret().await?;
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .context("Failed to encode JWT token")?;

    Ok(token)
}

/// Validate JWT token and extract claims
pub async fn validate_token(token: &str) -> Result<TokenClaims> {
    let secret = get_jwt_secret().await?;

    let token_data = decode::<TokenClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .context("Failed to decode JWT token")?;

    Ok(token_data.claims)
}

/// Generate refresh token (UUID-based)
pub fn generate_refresh_token() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Get or create JWT secret from database
pub async fn get_jwt_secret() -> Result<String> {
    // Try to get from sys_settings table
    match get_jwt_secret_from_db().await {
        Ok(Some(secret)) => Ok(secret),
        Ok(None) | Err(_) => {
            // Generate new secret and save to DB
            let secret = generate_jwt_secret();
            let _ = save_jwt_secret_to_db(&secret).await;
            Ok(secret)
        }
    }
}

/// Generate a cryptographically secure JWT secret (256 bits)
fn generate_jwt_secret() -> String {
    use base64::{engine::general_purpose, Engine as _};
    let mut rng = rand::thread_rng();
    let random_bytes: Vec<u8> = (0..32).map(|_| rng.gen::<u8>()).collect();
    general_purpose::STANDARD.encode(&random_bytes)
}

/// Get JWT secret from sys_settings table
async fn get_jwt_secret_from_db() -> Result<Option<String>> {
    use crate::shared::data::db::get_connection;
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

    let conn = get_connection();
    
    let result = conn.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        "SELECT value FROM sys_settings WHERE key = ?",
        ["jwt_secret".into()],
    ))
    .await?;
    
    match result {
        Some(row) => {
            let secret: String = row.try_get("", "value")?;
            Ok(Some(secret))
        }
        None => Ok(None),
    }
}

/// Save JWT secret to sys_settings table
async fn save_jwt_secret_to_db(secret: &str) -> Result<()> {
    use crate::shared::data::db::get_connection;
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

    let conn = get_connection();
    let now = Utc::now().to_rfc3339();

    conn.execute(Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        "INSERT OR REPLACE INTO sys_settings (key, value, description, created_at, updated_at) 
         VALUES (?, ?, ?, ?, ?)",
        [
            "jwt_secret".into(),
            secret.to_string().into(),
            "Auto-generated JWT secret for authentication".into(),
            now.clone().into(),
            now.into(),
        ],
    ))
    .await?;

    Ok(())
}

/// Calculate refresh token expiration timestamp
pub fn calculate_refresh_token_expiration() -> String {
    let exp = Utc::now() + chrono::Duration::days(REFRESH_TOKEN_LIFETIME_DAYS);
    exp.to_rfc3339()
}
