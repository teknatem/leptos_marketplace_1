use anyhow::Result;
use contracts::domain::a027_wb_documents::aggregate::{WbDocument, WbWeeklyReportManualData};
use uuid::Uuid;

use super::change_token;
use super::repository;
pub use repository::{WbDocumentsListQuery, WbDocumentsListResult, WbDocumentsListRow};

pub async fn upsert_by_service_name(document: &WbDocument) -> Result<bool> {
    let result = repository::upsert_by_service_name(document).await?;
    change_token::TOKEN.bump();
    Ok(result)
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
    comment: Option<Option<String>>,
) -> Result<Option<WbDocument>> {
    let Some(mut document) = repository::get_by_id(id).await? else {
        return Ok(None);
    };

    document.is_weekly_report = is_weekly_report;
    document.report_period_from = report_period_from;
    document.report_period_to = report_period_to;
    document.weekly_report_data = weekly_report_data;
    // `None` оставляет комментарий без изменений (например, при извлечении из PDF),
    // `Some(value)` — перезаписывает его значением из формы проверки.
    if let Some(comment) = comment {
        document.base.set_comment(comment);
    }
    document.before_write();

    upsert_by_service_name(&document).await?;
    repository::get_by_id(id).await
}

pub async fn store_max_deviation(
    id: Uuid,
    max_deviation: Option<f64>,
) -> Result<Option<WbDocument>> {
    let Some(mut document) = repository::get_by_id(id).await? else {
        return Ok(None);
    };

    document.max_deviation = max_deviation;

    upsert_by_service_name(&document).await?;
    repository::get_by_id(id).await
}
