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

/// Проведение документа: пересобрать его движения воронки p916 (стадия 1).
pub async fn post_document(id: Uuid) -> Result<()> {
    repository::post_document(id).await
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

/// Разовый бэкфилл стадии 1 воронки p916 из сохранённых документов a036.
pub async fn backfill_stage1_funnel() -> Result<usize> {
    repository::backfill_stage1_funnel().await
}

/// Пересобрать стадию 1 воронки p916 за период из сохранённых документов a036.
pub async fn rebuild_stage1_for_period(
    connection_ids: &[String],
    date_from: &str,
    date_to: &str,
) -> Result<usize> {
    repository::rebuild_stage1_for_period(connection_ids, date_from, date_to).await
}
