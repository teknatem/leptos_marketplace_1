use anyhow::Result;
use contracts::domain::a040_wb_search_analytics_daily::aggregate::WbSearchAnalyticsDaily;
use uuid::Uuid;

use super::repository;
pub use repository::{
    WbSearchAnalyticsListQuery, WbSearchAnalyticsListResult, WbSearchAnalyticsListRow,
};

pub async fn replace_for_period(
    connection_id: &str,
    date_from: &str,
    date_to: &str,
    documents: &[WbSearchAnalyticsDaily],
) -> Result<usize> {
    repository::replace_for_period(connection_id, date_from, date_to, documents).await
}

pub async fn get_by_id(id: Uuid) -> Result<Option<WbSearchAnalyticsDaily>> {
    repository::get_by_id(id).await
}

pub async fn list_paginated(
    query: WbSearchAnalyticsListQuery,
) -> Result<WbSearchAnalyticsListResult> {
    repository::list_sql(query).await
}
