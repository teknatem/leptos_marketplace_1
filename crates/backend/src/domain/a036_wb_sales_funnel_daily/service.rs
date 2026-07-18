use anyhow::Result;
use contracts::domain::a036_wb_sales_funnel_daily::aggregate::WbSalesFunnelDaily;
use uuid::Uuid;

use super::repository;
pub use repository::{
    FunnelProductMetrics, WbSalesFunnelDailyListQuery, WbSalesFunnelDailyListResult,
    WbSalesFunnelDailyListRow, WbSalesFunnelExportRow,
};

pub async fn replace_for_period(
    connection_id: &str,
    date_from: &str,
    date_to: &str,
    documents: &[WbSalesFunnelDaily],
) -> Result<usize> {
    repository::replace_for_period(connection_id, date_from, date_to, documents).await
}

pub async fn get_by_id(id: Uuid) -> Result<Option<WbSalesFunnelDaily>> {
    repository::get_by_id(id).await
}

pub async fn list_paginated(
    query: WbSalesFunnelDailyListQuery,
) -> Result<WbSalesFunnelDailyListResult> {
    repository::list_sql(query).await
}

pub async fn export_rows(
    query: WbSalesFunnelDailyListQuery,
) -> Result<Vec<WbSalesFunnelExportRow>> {
    repository::export_rows(query).await
}

pub async fn product_metrics_sum(
    connection_id: &str,
    date_from: &str,
    date_to: &str,
) -> Result<Vec<FunnelProductMetrics>> {
    repository::product_metrics_sum(connection_id, date_from, date_to).await
}
