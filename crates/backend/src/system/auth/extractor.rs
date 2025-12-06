use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use contracts::system::auth::TokenClaims;

/// Extractor for getting current user from JWT token
/// Usage in handlers: `async fn handler(CurrentUser(claims): CurrentUser) -> Response`
pub struct CurrentUser(pub TokenClaims);

#[async_trait]
impl<S> FromRequestParts<S> for CurrentUser
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract TokenClaims from request extensions (set by middleware)
        parts
            .extensions
            .get::<TokenClaims>()
            .cloned()
            .map(CurrentUser)
            .ok_or(StatusCode::UNAUTHORIZED)
    }
}
