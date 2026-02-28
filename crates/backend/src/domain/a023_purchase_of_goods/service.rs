use super::repository;
use anyhow::Result;
use contracts::domain::a023_purchase_of_goods::aggregate::PurchaseOfGoods;
use uuid::Uuid;

pub use repository::{PurchaseOfGoodsListQuery, PurchaseOfGoodsListResult};

/// Сохранить или обновить документ из OData
/// Возвращает (id, is_new)
pub async fn upsert_from_odata(doc: &PurchaseOfGoods) -> Result<(String, bool)> {
    let is_new = repository::upsert_document(doc).await?;
    Ok((doc.to_string_id(), is_new))
}

pub async fn get_by_id(id: Uuid) -> Result<Option<PurchaseOfGoods>> {
    repository::get_by_id(id).await
}

pub async fn list_paginated(query: PurchaseOfGoodsListQuery) -> Result<PurchaseOfGoodsListResult> {
    repository::list_sql(query).await
}
