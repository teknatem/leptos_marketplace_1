use contracts::system::favorites::{FavoriteDto, FavoriteUpdateRequest, FavoriteUpsertRequest};
use gloo_net::http::Request;
use urlencoding::encode;

use crate::shared::api_utils::api_base;
use crate::system::auth::storage;

fn auth_header() -> Result<String, String> {
    storage::get_access_token()
        .map(|token| format!("Bearer {}", token))
        .ok_or_else(|| "Not authenticated".to_string())
}

pub async fn list_favorites() -> Result<Vec<FavoriteDto>, String> {
    let response = Request::get(&format!("{}/api/system/favorites", api_base()))
        .header("Authorization", &auth_header()?)
        .header("Cache-Control", "no-cache")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch favorites: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to fetch favorites: {}", response.status()));
    }

    response
        .json::<Vec<FavoriteDto>>()
        .await
        .map_err(|e| format!("Failed to parse favorites: {}", e))
}

pub async fn get_target_favorite(
    target_kind: &str,
    target_id: &str,
) -> Result<Option<FavoriteDto>, String> {
    let url = format!(
        "{}/api/system/favorites/target?target_kind={}&target_id={}",
        api_base(),
        encode(target_kind),
        encode(target_id)
    );
    let response = Request::get(&url)
        .header("Authorization", &auth_header()?)
        .header("Cache-Control", "no-cache")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch favorite: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to fetch favorite: {}", response.status()));
    }

    response
        .json::<Option<FavoriteDto>>()
        .await
        .map_err(|e| format!("Failed to parse favorite: {}", e))
}

pub async fn upsert_favorite(req: FavoriteUpsertRequest) -> Result<FavoriteDto, String> {
    let response = Request::post(&format!("{}/api/system/favorites", api_base()))
        .header("Authorization", &auth_header()?)
        .json(&req)
        .map_err(|e| format!("Failed to serialize favorite: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to save favorite: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to save favorite: {}", response.status()));
    }

    response
        .json::<FavoriteDto>()
        .await
        .map_err(|e| format!("Failed to parse favorite: {}", e))
}

pub async fn update_favorite(id: &str, req: FavoriteUpdateRequest) -> Result<FavoriteDto, String> {
    let response = Request::put(&format!("{}/api/system/favorites/{}", api_base(), id))
        .header("Authorization", &auth_header()?)
        .json(&req)
        .map_err(|e| format!("Failed to serialize favorite: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to update favorite: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to update favorite: {}", response.status()));
    }

    response
        .json::<FavoriteDto>()
        .await
        .map_err(|e| format!("Failed to parse favorite: {}", e))
}

pub async fn delete_favorite(id: &str) -> Result<(), String> {
    let response = Request::delete(&format!("{}/api/system/favorites/{}", api_base(), id))
        .header("Authorization", &auth_header()?)
        .send()
        .await
        .map_err(|e| format!("Failed to delete favorite: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to delete favorite: {}", response.status()));
    }

    Ok(())
}
