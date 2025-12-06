use axum::{body::Body, extract::Request, http::StatusCode, middleware::Next, response::Response};

/// Middleware that requires valid JWT authentication
pub async fn require_auth(mut req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    // Extract Authorization header
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Check Bearer prefix
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Validate token
    let claims = super::jwt::validate_token(token)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Add claims to request extensions for use in handlers
    req.extensions_mut().insert(claims);

    Ok(next.run(req).await)
}

/// Middleware that requires admin privileges
pub async fn require_admin(mut req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    // Extract Authorization header
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Check Bearer prefix
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Validate token
    let claims = super::jwt::validate_token(token)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Check admin flag
    if !claims.is_admin {
        return Err(StatusCode::FORBIDDEN);
    }

    // Add claims to request extensions for use in handlers
    req.extensions_mut().insert(claims);

    Ok(next.run(req).await)
}
