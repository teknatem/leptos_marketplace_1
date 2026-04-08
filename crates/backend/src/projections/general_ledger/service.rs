use anyhow::Result;

use super::repository;

pub async fn save_entries(entries: &[repository::Model]) -> Result<()> {
    for entry in entries {
        repository::save_entry(entry).await?;
    }
    Ok(())
}

/// Batch INSERT свежих GL-записей без SELECT на каждую строку.
/// Используется после delete_by_registrator_ref, когда все записи заведомо новые.
pub async fn insert_fresh_entries(entries: &[repository::Model]) -> Result<()> {
    repository::insert_entries_bulk(entries).await
}

pub async fn remove_by_registrator_ref(registrator_ref: &str) -> Result<()> {
    repository::delete_by_registrator_ref(registrator_ref).await?;
    Ok(())
}

pub async fn remove_by_registrator(registrator_type: &str, registrator_ref: &str) -> Result<()> {
    repository::delete_by_registrator(registrator_type, registrator_ref).await?;
    Ok(())
}
