use anyhow::Result;
use contracts::domain::a037_wb_product_snapshot::aggregate::WbProductSnapshot;
use uuid::Uuid;

use super::repository;
pub use repository::{
    WbProductSnapshotListQuery, WbProductSnapshotListResult, WbProductSnapshotListRow,
    WbProductSnapshotSeriesPoint,
};

pub async fn replace_for_period(
    connection_id: &str,
    date_from: &str,
    date_to: &str,
    documents: &[WbProductSnapshot],
) -> Result<usize> {
    repository::replace_for_period(connection_id, date_from, date_to, documents).await
}

pub async fn get_by_id(id: Uuid) -> Result<Option<WbProductSnapshot>> {
    repository::get_by_id(id).await
}

pub async fn list_paginated(
    query: WbProductSnapshotListQuery,
) -> Result<WbProductSnapshotListResult> {
    repository::list_sql(query).await
}

pub async fn series_for_nm(
    connection_id: &str,
    nm_id: i64,
    date_from: &str,
    date_to: &str,
) -> Result<Vec<WbProductSnapshotSeriesPoint>> {
    repository::series_for_nm(connection_id, nm_id, date_from, date_to).await
}

pub async fn previous_before(connection_id: &str, date: &str) -> Result<Option<WbProductSnapshot>> {
    repository::previous_before(connection_id, date).await
}
