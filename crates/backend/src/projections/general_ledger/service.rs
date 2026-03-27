use anyhow::Result;

use super::repository;

pub async fn save_entries(entries: &[repository::Model]) -> Result<()> {
    for entry in entries {
        repository::save_entry(entry).await?;
    }
    Ok(())
}

pub async fn remove_by_registrator_ref(registrator_ref: &str) -> Result<()> {
    repository::delete_by_registrator_ref(registrator_ref).await?;
    Ok(())
}
