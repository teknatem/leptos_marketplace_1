use anyhow::{anyhow, Result};
use chrono::Utc;
use contracts::system::history::{
    PageHistoryDto, PageHistoryRecordRequest, PAGE_HISTORY_DEFAULT_LIMIT, PAGE_HISTORY_MAX_ROWS,
};
use uuid::Uuid;

use super::repository;

pub async fn list_recent(user_id: &str, limit: Option<u64>) -> Result<Vec<PageHistoryDto>> {
    let limit = limit
        .unwrap_or(PAGE_HISTORY_DEFAULT_LIMIT)
        .clamp(1, PAGE_HISTORY_MAX_ROWS);
    Ok(repository::list_recent(user_id, limit).await?)
}

pub async fn record(user_id: &str, req: PageHistoryRecordRequest) -> Result<PageHistoryDto> {
    let tab_key = req.tab_key.trim().to_string();
    if tab_key.is_empty() {
        return Err(anyhow!("Page history tab key is required"));
    }
    let title = {
        let t = req.title.trim();
        if t.is_empty() {
            tab_key.clone()
        } else {
            t.to_string()
        }
    };
    let now = Utc::now().to_rfc3339();

    // Collapse consecutive duplicates: re-opening the same page just refreshes
    // the existing latest row instead of piling up identical entries.
    if let Some(latest) = repository::find_latest(user_id).await? {
        if latest.tab_key == tab_key {
            repository::touch(&latest.id, &title, &now).await?;
            return Ok(PageHistoryDto {
                id: latest.id,
                owner_user_id: user_id.to_string(),
                tab_key,
                title,
                opened_at: now,
            });
        }
    }

    let model = repository::Model {
        id: Uuid::new_v4().to_string(),
        owner_user_id: user_id.to_string(),
        tab_key,
        title,
        opened_at: now,
    };
    let dto = repository::create(model).await?;
    // Cap history size per user.
    repository::prune(user_id, PAGE_HISTORY_MAX_ROWS).await?;
    Ok(dto)
}

pub async fn clear(user_id: &str) -> Result<()> {
    repository::clear(user_id).await?;
    Ok(())
}
