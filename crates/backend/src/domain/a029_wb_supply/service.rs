use super::repository;
use anyhow::Result;
use contracts::domain::a029_wb_supply::aggregate::WbSupply;
use uuid::Uuid;

pub async fn store_document_with_raw(mut document: WbSupply, raw_json: &str) -> Result<Uuid> {
    let raw_ref = crate::shared::data::raw_storage::save_raw_json(
        "WB",
        "WB_Supply",
        &document.header.supply_id,
        raw_json,
        document.source_meta.fetched_at,
    )
    .await?;

    document.source_meta.raw_payload_ref = raw_ref;

    document
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;
    document.before_write();

    let id = repository::upsert_document(&document).await?;
    Ok(id)
}

pub async fn get_by_id(id: Uuid) -> Result<Option<WbSupply>> {
    repository::get_by_id(id).await
}

pub async fn get_by_supply_id(supply_id: &str) -> Result<Option<WbSupply>> {
    repository::get_by_supply_id(supply_id).await
}

pub async fn delete(id: Uuid) -> Result<bool> {
    repository::soft_delete(id).await
}
