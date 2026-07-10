use anyhow::Result;

use super::repository;

const KEY_SCHEDULER_ENABLED: &str = "scheduler_enabled";
const KEY_RAW_JSON_CAPTURE_ENABLED: &str = "raw_json_capture_enabled";

pub async fn get_scheduler_enabled() -> Result<bool> {
    let value = repository::get_setting(KEY_SCHEDULER_ENABLED).await?;
    Ok(value.map_or(true, |v| v != "false"))
}

pub async fn set_scheduler_enabled(enabled: bool) -> Result<()> {
    repository::set_setting(
        KEY_SCHEDULER_ENABLED,
        if enabled { "true" } else { "false" },
    )
    .await?;
    Ok(())
}

pub async fn get_raw_json_capture_enabled() -> Result<bool> {
    let value = repository::get_setting(KEY_RAW_JSON_CAPTURE_ENABLED).await?;
    Ok(value.map_or(false, |v| v == "true"))
}

pub async fn set_raw_json_capture_enabled(enabled: bool) -> Result<()> {
    repository::set_setting(
        KEY_RAW_JSON_CAPTURE_ENABLED,
        if enabled { "true" } else { "false" },
    )
    .await?;
    crate::shared::data::raw_storage::set_capture_enabled_cache(enabled);
    Ok(())
}
