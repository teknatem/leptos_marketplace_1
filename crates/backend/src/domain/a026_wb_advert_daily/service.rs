use anyhow::Result;
use contracts::domain::a026_wb_advert_daily::aggregate::WbAdvertDaily;
use uuid::Uuid;

use super::repository;
pub use repository::{
    list_documents_for_report, report_preflight, WbAdvertDailyListQuery, WbAdvertDailyListResult,
    WbAdvertDailyListRow, WbAdvertDailyReportQuery, A026_REPORT_MAX_DOCUMENTS,
    A026_REPORT_MAX_LINE_ROWS,
};

pub async fn replace_for_period(
    connection_id: &str,
    date_from: &str,
    date_to: &str,
    documents: &[WbAdvertDaily],
) -> Result<usize> {
    repository::replace_for_period(connection_id, date_from, date_to, documents).await
}

pub async fn replace_for_period_advert_ids(
    connection_id: &str,
    date_from: &str,
    date_to: &str,
    advert_ids: &[i64],
    documents: &[WbAdvertDaily],
) -> Result<usize> {
    repository::replace_for_period_advert_ids(
        connection_id,
        date_from,
        date_to,
        advert_ids,
        documents,
    )
    .await
}

pub async fn upsert_document(document: &WbAdvertDaily) -> Result<()> {
    repository::upsert_document(document).await
}

pub async fn get_by_id(id: Uuid) -> Result<Option<WbAdvertDaily>> {
    repository::get_by_id(id).await
}

pub async fn list_by_advert_id(connection_id: &str, advert_id: i64) -> Result<Vec<WbAdvertDaily>> {
    repository::list_by_advert_id(connection_id, advert_id).await
}

pub async fn list_paginated(query: WbAdvertDailyListQuery) -> Result<WbAdvertDailyListResult> {
    repository::list_sql(query).await
}
