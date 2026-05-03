use axum::http::Method;
use axum::response::IntoResponse;
use axum::{
    body::Body, extract::Request, http::StatusCode, middleware::Next, response::Response, Json,
};
use contracts::shared::access::AccessMode;

/// Middleware that requires valid JWT authentication
pub async fn require_auth(mut req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    let token = extract_bearer_token(&req)?;
    let claims = super::jwt::validate_token(&token)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}

/// Middleware that requires admin privileges
pub async fn require_admin(mut req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    let token = extract_bearer_token(&req)?;
    let claims = super::jwt::validate_token(&token)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    if !claims.is_admin {
        return Err(StatusCode::FORBIDDEN);
    }
    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}

/// Check scope access — use this from an inline closure in routes.rs:
/// `.layer(middleware::from_fn(|req, next| async move { check_scope("aXXX", req, next).await }))`
/// GET → Read, POST/PUT/DELETE/PATCH → All.
pub async fn check_scope(
    scope_id: &'static str,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let required_mode = if req.method() == Method::GET {
        AccessMode::Read
    } else {
        AccessMode::All
    };
    check_scope_with_mode(scope_id, required_mode, req, next).await
}

/// Like `check_scope`, but always requires only Read access regardless of HTTP method.
/// Use for POST endpoints that are computationally read-only (e.g. compute-batch).
pub async fn check_scope_read(
    scope_id: &'static str,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    check_scope_with_mode(scope_id, AccessMode::Read, req, next).await
}

async fn check_scope_with_mode(
    scope_id: &'static str,
    required_mode: AccessMode,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = extract_bearer_token(&req)?;
    let claims = super::jwt::validate_token(&token)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    if !claims.is_admin {
        let scopes = crate::system::access::resolver::resolve_user_scopes(&claims.sub)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let granted = scopes.iter().any(|s| {
            s.scope_id == scope_id
                && match required_mode {
                    AccessMode::Read => s.mode == "read" || s.mode == "all",
                    AccessMode::All => s.mode == "all",
                }
        });

        if !granted {
            let required_access = match required_mode {
                AccessMode::Read => "read",
                AccessMode::All => "all",
            };
            return Ok((
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({
                    "error": "access_denied",
                    "scope_id": scope_id,
                    "required_access": required_access
                })),
            )
                .into_response());
        }
    }

    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}

/// Middleware for external API routes: validates the `X-Api-Key` header against
/// the static key configured in `[external_api].api_key` in config.toml.
/// Returns 503 if the external API is not configured, 401 if the key is wrong.
pub async fn check_api_key(req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    let expected =
        crate::shared::config::get_ext_api_key().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let provided = req
        .headers()
        .get("X-Api-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if provided != expected {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(req).await)
}

/// Extract Bearer token string from Authorization header (returns owned String, no borrow issues).
fn extract_bearer_token(req: &Request<Body>) -> Result<String, StatusCode> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    auth_header
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)
        .map(|t| t.to_string())
}
