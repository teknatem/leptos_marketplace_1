use anyhow::{anyhow, Result};
use chrono::Utc;
use contracts::system::favorites::{
    is_supported_favorite_color, is_supported_favorite_target_kind, FavoriteDto,
    FavoriteUpdateRequest, FavoriteUpsertRequest, FAVORITE_COLOR_YELLOW,
};
use uuid::Uuid;

use super::repository;

fn normalize_comment(value: Option<String>) -> Option<String> {
    value.and_then(|s| {
        let trimmed = s.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn validate_target(kind: &str, id: &str, title: Option<&str>, tab_key: Option<&str>) -> Result<()> {
    if !is_supported_favorite_target_kind(kind) {
        return Err(anyhow!("Unsupported favorite target kind"));
    }
    if id.trim().is_empty() {
        return Err(anyhow!("Favorite target id is required"));
    }
    if title.map(str::trim).unwrap_or("x").is_empty() {
        return Err(anyhow!("Favorite target title is required"));
    }
    if tab_key.map(str::trim).unwrap_or("x").is_empty() {
        return Err(anyhow!("Favorite tab key is required"));
    }
    Ok(())
}

fn validate_color(color: &str) -> Result<()> {
    if is_supported_favorite_color(color) {
        Ok(())
    } else {
        Err(anyhow!("Unsupported favorite color"))
    }
}

pub async fn list_visible(user_id: &str) -> Result<Vec<FavoriteDto>> {
    Ok(repository::list_visible(user_id).await?)
}

pub async fn get_personal_target(
    user_id: &str,
    target_kind: &str,
    target_id: &str,
) -> Result<Option<FavoriteDto>> {
    validate_target(target_kind, target_id, None, None)?;
    Ok(repository::find_personal_target(user_id, target_kind, target_id).await?)
}

pub async fn upsert(user_id: &str, req: FavoriteUpsertRequest) -> Result<FavoriteDto> {
    validate_target(
        &req.target_kind,
        &req.target_id,
        Some(&req.target_title),
        Some(&req.tab_key),
    )?;
    validate_color(&req.color)?;

    let now = Utc::now().to_rfc3339();
    let comment = normalize_comment(req.comment);
    let existing =
        repository::find_personal_target(user_id, &req.target_kind, &req.target_id).await?;

    if let Some(existing) = existing {
        let model = repository::Model {
            id: existing.id,
            owner_user_id: existing.owner_user_id,
            target_kind: req.target_kind,
            target_id: req.target_id,
            target_title: req.target_title.trim().to_string(),
            tab_key: req.tab_key.trim().to_string(),
            color: req.color,
            comment,
            is_global: req.is_global,
            created_at: existing.created_at,
            updated_at: now,
        };
        repository::update(model).await.map_err(Into::into)
    } else {
        let model = repository::Model {
            id: Uuid::new_v4().to_string(),
            owner_user_id: user_id.to_string(),
            target_kind: req.target_kind,
            target_id: req.target_id,
            target_title: req.target_title.trim().to_string(),
            tab_key: req.tab_key.trim().to_string(),
            color: if req.color.trim().is_empty() {
                FAVORITE_COLOR_YELLOW.to_string()
            } else {
                req.color
            },
            comment,
            is_global: req.is_global,
            created_at: now.clone(),
            updated_at: now,
        };
        repository::create(model).await.map_err(Into::into)
    }
}

pub async fn update(
    requester_user_id: &str,
    requester_is_admin: bool,
    id: &str,
    req: FavoriteUpdateRequest,
) -> Result<FavoriteDto> {
    validate_color(&req.color)?;
    let existing = repository::find_by_id(id)
        .await?
        .ok_or_else(|| anyhow!("Favorite not found"))?;
    if existing.owner_user_id != requester_user_id && !requester_is_admin {
        return Err(anyhow!("Forbidden"));
    }

    let model = repository::Model {
        id: existing.id,
        owner_user_id: existing.owner_user_id,
        target_kind: existing.target_kind,
        target_id: existing.target_id,
        target_title: existing.target_title,
        tab_key: existing.tab_key,
        color: req.color,
        comment: normalize_comment(req.comment),
        is_global: req.is_global,
        created_at: existing.created_at,
        updated_at: Utc::now().to_rfc3339(),
    };

    repository::update(model).await.map_err(Into::into)
}

pub async fn delete(requester_user_id: &str, requester_is_admin: bool, id: &str) -> Result<()> {
    let existing = repository::find_by_id(id)
        .await?
        .ok_or_else(|| anyhow!("Favorite not found"))?;
    if existing.owner_user_id != requester_user_id && !requester_is_admin {
        return Err(anyhow!("Forbidden"));
    }
    repository::delete(id).await?;
    Ok(())
}
