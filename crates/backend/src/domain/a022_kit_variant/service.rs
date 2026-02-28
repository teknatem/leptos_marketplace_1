use super::repository;
use anyhow::Result;
use contracts::domain::a022_kit_variant::aggregate::KitVariant;
use uuid::Uuid;

pub use repository::{KitVariantListQuery, KitVariantListResult};

/// Сохранить или обновить вариант комплектации из OData
pub async fn upsert_from_odata(item: &KitVariant) -> Result<(String, bool)> {
    let is_new = repository::upsert(item).await?;
    Ok((item.to_string_id(), is_new))
}

pub async fn get_by_id(id: Uuid) -> Result<Option<KitVariant>> {
    repository::get_by_id(id).await
}

pub async fn list_paginated(query: KitVariantListQuery) -> Result<KitVariantListResult> {
    repository::list_sql(query).await
}
