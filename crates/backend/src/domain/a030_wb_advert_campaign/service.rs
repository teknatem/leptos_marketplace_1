use anyhow::Result;
use contracts::domain::a030_wb_advert_campaign::aggregate::WbAdvertCampaign;
use std::collections::HashMap;
use uuid::Uuid;

use super::repository;
pub use repository::CampaignInfoSnapshot;

/// Returns `(new_count, total_count)`.
pub async fn upsert_many(items: &[WbAdvertCampaign]) -> Result<(usize, usize)> {
    repository::upsert_many(items).await
}

pub async fn list_by_connection(connection_id: &str) -> Result<Vec<WbAdvertCampaign>> {
    repository::list_by_connection(connection_id).await
}

pub async fn list_all() -> Result<Vec<WbAdvertCampaign>> {
    repository::list_all().await
}

pub async fn get_by_id(id: Uuid) -> Result<Option<WbAdvertCampaign>> {
    repository::get_by_id(id).await
}

pub async fn list_advert_ids_by_connection(connection_id: &str) -> Result<Vec<i64>> {
    repository::list_advert_ids_by_connection(connection_id).await
}

pub async fn list_advert_ids_for_period(connection_id: &str, date_from: &str) -> Result<Vec<i64>> {
    repository::list_advert_ids_for_period(connection_id, date_from).await
}

pub async fn list_info_snapshot(connection_id: &str) -> Result<HashMap<i64, CampaignInfoSnapshot>> {
    repository::list_info_snapshot(connection_id).await
}
