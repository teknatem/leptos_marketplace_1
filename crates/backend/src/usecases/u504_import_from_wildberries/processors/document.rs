use anyhow::Result;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::a027_wb_documents::aggregate::{
    WbDocument, WbDocumentHeader, WbDocumentSourceMeta, WbWeeklyReportManualData,
};

use crate::domain::a027_wb_documents;

use super::super::wildberries_api_client::WbDocumentListItem;

const WEEKLY_REPORT_CATEGORY: &str = "Еженедельный отчет реализации";

pub async fn process_document_header(
    connection: &ConnectionMP,
    organization_id: &str,
    item: &WbDocumentListItem,
) -> Result<bool> {
    let connection_id = connection.to_string_id();
    let existing = a027_wb_documents::service::get_by_connection_and_service_name(
        &connection_id,
        &item.service_name,
    )
    .await?;

    let header = WbDocumentHeader {
        service_name: item.service_name.clone(),
        name: item.name.clone(),
        category: item.category.clone(),
        extensions: item.extensions.clone(),
        creation_time: item.creation_time.clone(),
        viewed: item.viewed,
        connection_id,
        organization_id: organization_id.to_string(),
        marketplace_id: connection.marketplace_id.clone(),
    };

    let mut source_meta = existing
        .as_ref()
        .map(|document| document.source_meta.clone())
        .unwrap_or_else(default_source_meta);
    source_meta.fetched_at = chrono::Utc::now().to_rfc3339();
    source_meta.locale = "ru".to_string();
    source_meta.document_version = 1;

    let mut document = WbDocument::new_for_insert(header, source_meta);
    document.is_weekly_report = item.category.trim() == WEEKLY_REPORT_CATEGORY;
    document.report_period_from = existing
        .as_ref()
        .and_then(|doc| doc.report_period_from.clone());
    document.report_period_to = existing
        .as_ref()
        .and_then(|doc| doc.report_period_to.clone());
    document.weekly_report_data = existing
        .as_ref()
        .map(|doc| doc.weekly_report_data.clone())
        .unwrap_or_else(WbWeeklyReportManualData::default);

    document.before_write();
    document.validate().map_err(anyhow::Error::msg)?;

    a027_wb_documents::service::upsert_by_service_name(&document).await
}

fn default_source_meta() -> WbDocumentSourceMeta {
    WbDocumentSourceMeta {
        fetched_at: chrono::Utc::now().to_rfc3339(),
        locale: "ru".to_string(),
        document_version: 1,
    }
}
