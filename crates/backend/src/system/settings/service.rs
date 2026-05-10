use anyhow::Result;

use super::repository;

const KEY_SCHEDULER_ENABLED: &str = "scheduler_enabled";

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
