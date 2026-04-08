use anyhow::Result;
use contracts::domain::a027_wb_documents::aggregate::{WbDocument, WbWeeklyReportManualData};
use uuid::Uuid;

use super::repository;
pub use repository::{WbDocumentsListQuery, WbDocumentsListResult, WbDocumentsListRow};

pub async fn upsert_by_service_name(document: &WbDocument) -> Result<bool> {
    repository::upsert_by_service_name(document).await
}

pub async fn get_by_id(id: Uuid) -> Result<Option<WbDocument>> {
    repository::get_by_id(id).await
}

pub async fn get_by_connection_and_service_name(
    connection_id: &str,
    service_name: &str,
) -> Result<Option<WbDocument>> {
    repository::get_by_connection_and_service_name(connection_id, service_name).await
}

pub async fn list_paginated(query: WbDocumentsListQuery) -> Result<WbDocumentsListResult> {
    repository::list_sql(query).await
}

pub async fn update_manual_fields(
    id: Uuid,
    is_weekly_report: bool,
    report_period_from: Option<String>,
    report_period_to: Option<String>,
    weekly_report_data: WbWeeklyReportManualData,
) -> Result<Option<WbDocument>> {
    let Some(mut document) = repository::get_by_id(id).await? else {
        return Ok(None);
    };

    document.is_weekly_report = is_weekly_report;
    document.report_period_from = report_period_from;
    document.report_period_to = report_period_to;
    document.weekly_report_data = weekly_report_data;
    document.before_write();

    repository::upsert_by_service_name(&document).await?;
    repository::get_by_id(id).await
}
