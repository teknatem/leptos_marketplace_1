use contracts::system::audit::{AuditReport, RoutePolicyDto};
use gloo_net::http::Request;

use crate::shared::api_utils::api_base;
use crate::system::auth::storage;

fn get_auth_header() -> Option<String> {
    storage::get_access_token().map(|token| format!("Bearer {}", token))
}

pub async fn fetch_routes() -> Result<Vec<RoutePolicyDto>, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::get(&format!("{}/api/system/audit/routes", api_base()))
        .header("Authorization", &auth_header)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to fetch routes: {}", response.status()));
    }

    response
        .json::<Vec<RoutePolicyDto>>()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

pub async fn fetch_violations() -> Result<AuditReport, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::get(&format!("{}/api/system/audit/violations", api_base()))
        .header("Authorization", &auth_header)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to fetch violations: {}", response.status()));
    }

    response
        .json::<AuditReport>()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}
