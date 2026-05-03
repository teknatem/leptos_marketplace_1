use axum::{
    body::Body,
    extract::{Path, Query},
    http::{header, StatusCode},
    response::Response,
    Json,
};
use contracts::domain::a026_wb_advert_daily::aggregate::{
    WbAdvertDaily, WbAdvertDailyLine, WbAdvertDailyMetrics,
};
use contracts::domain::common::AggregateId;
use contracts::general_ledger::GeneralLedgerEntryDto;
use contracts::shared::analytics::TurnoverLayer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::domain::a026_wb_advert_daily;
use crate::domain::a026_wb_advert_daily::repository::{
    self as a026_repo, WbAdvertDailyListQuery, WbAdvertDailyListRow, WbAdvertDailyReportQuery,
};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub connection_id: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub search_query: Option<String>,
    pub sort_by: Option<String>,
    pub sort_desc: Option<bool>,
}

/// Query для `GET .../report.csv` (фильтры как у списка + опционально позиции в строках).
#[derive(Debug, Deserialize)]
pub struct ReportCsvQuery {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub connection_id: Option<String>,
    pub search_query: Option<String>,
    /// Фильтр по строкам: `nm_id`, название WB, артикул/наименование номенклатуры (подстрока; для `nm_id` — также точное совпадение числа).
    pub position_query: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WbAdvertDailyListItemDto {
    pub id: String,
    pub document_no: String,
    pub document_date: String,
    pub advert_id: i64,
    pub lines_count: i32,
    pub total_views: i64,
    pub total_clicks: i64,
    pub total_orders: i64,
    pub total_sum: f64,
    pub total_sum_price: f64,
    pub connection_id: String,
    pub connection_name: Option<String>,
    pub organization_name: Option<String>,
    pub fetched_at: String,
    pub is_posted: bool,
}

impl From<WbAdvertDailyListRow> for WbAdvertDailyListItemDto {
    fn from(row: WbAdvertDailyListRow) -> Self {
        Self {
            id: row.id,
            document_no: row.document_no,
            document_date: row.document_date,
            advert_id: row.advert_id,
            lines_count: row.lines_count,
            total_views: row.total_views,
            total_clicks: row.total_clicks,
            total_orders: row.total_orders,
            total_sum: row.total_sum,
            total_sum_price: row.total_sum_price,
            connection_id: row.connection_id,
            connection_name: row.connection_name,
            organization_name: row.organization_name,
            fetched_at: row.fetched_at,
            is_posted: row.is_posted,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse {
    pub items: Vec<WbAdvertDailyListItemDto>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct WbAdvertDailyLineDetailsDto {
    pub nm_id: i64,
    pub wb_name: String,
    pub nomenclature_ref: Option<String>,
    pub nomenclature_article: Option<String>,
    pub nomenclature_name: Option<String>,
    pub advert_ids: Vec<i64>,
    pub app_types: Vec<i32>,
    pub placements: Vec<String>,
    pub metrics: WbAdvertDailyMetrics,
}

#[derive(Debug, Clone, Serialize)]
pub struct WbAdvertDailyDetailsDto {
    pub id: String,
    pub document_no: String,
    pub document_date: String,
    pub advert_id: i64,
    pub connection_id: String,
    pub connection_name: Option<String>,
    pub organization_id: String,
    pub organization_name: Option<String>,
    pub marketplace_id: String,
    pub marketplace_name: Option<String>,
    pub totals: WbAdvertDailyMetrics,
    pub unattributed_totals: WbAdvertDailyMetrics,
    pub source: String,
    pub fetched_at: String,
    pub created_at: String,
    pub updated_at: String,
    pub is_posted: bool,
    pub lines: Vec<WbAdvertDailyLineDetailsDto>,
}

pub async fn list_paginated(
    Query(query): Query<ListQuery>,
) -> Result<Json<PaginatedResponse>, axum::http::StatusCode> {
    let page_size = query.limit.unwrap_or(100);
    let offset = query.offset.unwrap_or(0);
    let page = if page_size > 0 { offset / page_size } else { 0 };

    let list_query = WbAdvertDailyListQuery {
        date_from: query.date_from,
        date_to: query.date_to,
        connection_id: query.connection_id,
        search_query: query.search_query,
        sort_by: query.sort_by.unwrap_or_else(|| "document_date".to_string()),
        sort_desc: query.sort_desc.unwrap_or(true),
        limit: page_size,
        offset,
    };

    match a026_wb_advert_daily::service::list_paginated(list_query).await {
        Ok(result) => {
            let total_pages = if page_size > 0 {
                (result.total + page_size - 1) / page_size
            } else {
                1
            };
            Ok(Json(PaginatedResponse {
                items: result.items.into_iter().map(Into::into).collect(),
                total: result.total,
                page,
                page_size,
                total_pages,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to list WB advert daily documents: {}", e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

fn csv_plain_error(status: StatusCode, message: impl Into<String>) -> Response {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from(message.into()))
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

/// Line-level CSV: одна строка = одна позиция документа `a026`. Разделитель `;`, UTF-8 BOM.
pub async fn report_csv(Query(q): Query<ReportCsvQuery>) -> Response {
    let report_query = WbAdvertDailyReportQuery {
        date_from: q.date_from.clone(),
        date_to: q.date_to.clone(),
        connection_id: q.connection_id.clone(),
        search_query: q.search_query.clone(),
    };

    let (doc_count, sum_lines) = match a026_repo::report_preflight(&report_query).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("a026 report_preflight: {}", e);
            return csv_plain_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Не удалось подготовить отчёт",
            );
        }
    };

    if doc_count > a026_repo::A026_REPORT_MAX_DOCUMENTS {
        return csv_plain_error(
            StatusCode::PAYLOAD_TOO_LARGE,
            format!(
                "Слишком много документов для одного отчёта ({} > {}). Сузьте период, кабинет или поиск.",
                doc_count,
                a026_repo::A026_REPORT_MAX_DOCUMENTS
            ),
        );
    }

    let position_trim = q
        .position_query
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    if position_trim.is_none() && sum_lines > a026_repo::A026_REPORT_MAX_LINE_ROWS {
        return csv_plain_error(
            StatusCode::PAYLOAD_TOO_LARGE,
            format!(
                "Слишком много строк позиций (оценка {} > {}). Сузьте период или кабинет, либо задайте фильтр по позиции.",
                sum_lines,
                a026_repo::A026_REPORT_MAX_LINE_ROWS
            ),
        );
    }

    let docs = match a026_repo::list_documents_for_report(&report_query).await {
        Ok(d) => d,
        Err(e) => {
            tracing::error!("a026 list_documents_for_report: {}", e);
            return csv_plain_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Не удалось загрузить документы для отчёта",
            );
        }
    };

    let position_q = position_trim.unwrap_or("");

    const HDR: &[&str] = &[
        "Номер документа",
        "Дата",
        "advert_id",
        "Кабинет",
        "Организация",
        "Маркетплейс",
        "Источник",
        "Получено",
        "Проведён",
        "nm_id",
        "Наименование WB",
        "Артикул",
        "Наименование",
        "Кампании",
        "Типы app",
        "Места размещения",
        "Показы",
        "Клики",
        "CTR",
        "CPC",
        "В корзину",
        "Заказы",
        "Штрихкоды",
        "Расход",
        "Сумма заказов",
        "CR",
        "Отменено",
    ];

    if docs.is_empty() {
        let mut buffer: Vec<u8> = Vec::new();
        buffer.extend_from_slice("\u{FEFF}".as_bytes());
        let mut wtr = csv::WriterBuilder::new()
            .delimiter(b';')
            .from_writer(&mut buffer);
        if wtr.write_record(HDR).is_err() || wtr.flush().is_err() {
            return csv_plain_error(StatusCode::INTERNAL_SERVER_ERROR, "Ошибка формирования CSV");
        }
        drop(wtr);
        let filename = "wb_advert_daily_empty.csv";
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
            .header(
                header::CONTENT_DISPOSITION,
                format!(r#"attachment; filename="{filename}""#),
            )
            .body(Body::from(buffer))
            .unwrap_or_else(|_| Response::new(Body::empty()));
    }

    let nom_cache = match nomenclature_cache_for_documents(&docs).await {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("a026 report nomenclature cache: {}", e);
            return csv_plain_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Не удалось загрузить номенклатуру для отчёта",
            );
        }
    };

    let max_lines = a026_repo::A026_REPORT_MAX_LINE_ROWS as usize;
    let export_line_count = count_export_lines(&docs, &nom_cache, position_q);
    if export_line_count > max_lines {
        return csv_plain_error(
            StatusCode::PAYLOAD_TOO_LARGE,
            format!(
                "Слишком много строк после фильтра ({export_line_count} > {max_lines}). Сузьте фильтры."
            ),
        );
    }

    let mut buffer: Vec<u8> = Vec::new();
    buffer.extend_from_slice("\u{FEFF}".as_bytes());

    let mut wtr = csv::WriterBuilder::new()
        .delimiter(b';')
        .from_writer(&mut buffer);

    if let Err(e) = wtr.write_record(HDR) {
        tracing::error!("a026 report csv header: {}", e);
        return csv_plain_error(StatusCode::INTERNAL_SERVER_ERROR, "Ошибка формирования CSV");
    }

    for doc in &docs {
        let ctx = match build_doc_csv_row_prefix(doc).await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("a026 report doc context: {}", e);
                return csv_plain_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Ошибка обогащения документа для отчёта",
                );
            }
        };

        for line in &doc.lines {
            let dto = line_to_details_dto(line, &nom_cache);
            if !line_matches_position(&dto, position_q) {
                continue;
            }
            let m = &dto.metrics;
            let row = vec![
                ctx.document_no.clone(),
                ctx.document_date.clone(),
                ctx.advert_id.clone(),
                ctx.connection_name.clone(),
                ctx.organization_name.clone(),
                ctx.marketplace_name.clone(),
                ctx.source.clone(),
                ctx.fetched_at.clone(),
                ctx.is_posted.clone(),
                dto.nm_id.to_string(),
                dto.wb_name.clone(),
                dto.nomenclature_article.clone().unwrap_or_default(),
                dto.nomenclature_name.clone().unwrap_or_default(),
                join_i64_slice(&dto.advert_ids),
                join_i32_slice(&dto.app_types),
                dto.placements.join("|"),
                m.views.to_string(),
                m.clicks.to_string(),
                format_metric(m.ctr),
                format_metric(m.cpc),
                m.atbs.to_string(),
                m.orders.to_string(),
                m.shks.to_string(),
                format_metric(m.sum),
                format_metric(m.sum_price),
                format_metric(m.cr),
                m.canceled.to_string(),
            ];
            if let Err(e) = wtr.write_record(&row) {
                tracing::error!("a026 report csv row: {}", e);
                return csv_plain_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Ошибка записи строки CSV",
                );
            }
        }
    }

    if let Err(e) = wtr.flush() {
        tracing::error!("a026 report csv flush: {}", e);
        return csv_plain_error(StatusCode::INTERNAL_SERVER_ERROR, "Ошибка завершения CSV");
    }

    drop(wtr);

    let df = q.date_from.as_deref().unwrap_or("all");
    let dt = q.date_to.as_deref().unwrap_or("all");
    let safe_df: String = df
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();
    let safe_dt: String = dt
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();
    let filename = format!("wb_advert_daily_{safe_df}_{safe_dt}.csv");

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            format!(r#"attachment; filename="{filename}""#),
        )
        .body(Body::from(buffer))
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

fn join_i64_slice(v: &[i64]) -> String {
    v.iter()
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join("|")
}

fn join_i32_slice(v: &[i32]) -> String {
    v.iter()
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join("|")
}

fn format_metric(v: f64) -> String {
    // Запятая как разделитель дробной части для Excel/1C в русской локали.
    format!("{v:.6}").replace('.', ",")
}

fn line_matches_position(line: &WbAdvertDailyLineDetailsDto, q: &str) -> bool {
    let q = q.trim();
    if q.is_empty() {
        return true;
    }
    let q_lower = q.to_lowercase();
    if let Ok(n) = q.parse::<i64>() {
        if line.nm_id == n {
            return true;
        }
    }
    if line.wb_name.to_lowercase().contains(&q_lower) {
        return true;
    }
    line.nomenclature_article
        .as_deref()
        .map(|s| s.to_lowercase().contains(&q_lower))
        .unwrap_or(false)
        || line
            .nomenclature_name
            .as_deref()
            .map(|s| s.to_lowercase().contains(&q_lower))
            .unwrap_or(false)
}

fn line_to_details_dto(
    line: &WbAdvertDailyLine,
    nomenclature_cache: &HashMap<String, (Option<String>, Option<String>)>,
) -> WbAdvertDailyLineDetailsDto {
    let (article, name) = line
        .nomenclature_ref
        .as_ref()
        .and_then(|nom_ref| nomenclature_cache.get(nom_ref).cloned())
        .unwrap_or((None, None));

    WbAdvertDailyLineDetailsDto {
        nm_id: line.nm_id,
        wb_name: line.nm_name.clone(),
        nomenclature_ref: line.nomenclature_ref.clone(),
        nomenclature_article: article,
        nomenclature_name: name,
        advert_ids: line.advert_ids.clone(),
        app_types: line.app_types.clone(),
        placements: line.placements.clone(),
        metrics: line.metrics.clone(),
    }
}

async fn nomenclature_cache_for_documents(
    docs: &[WbAdvertDaily],
) -> anyhow::Result<HashMap<String, (Option<String>, Option<String>)>> {
    let mut refs = std::collections::HashSet::new();
    for doc in docs {
        for line in &doc.lines {
            if let Some(r) = &line.nomenclature_ref {
                refs.insert(r.clone());
            }
        }
    }
    let mut map = HashMap::new();
    for nom_ref in refs {
        if let Some(uuid) = parse_uuid(&nom_ref) {
            let nom = crate::domain::a004_nomenclature::service::get_by_id(uuid).await?;
            map.insert(
                nom_ref,
                nom.map_or((None, None), |n| {
                    (Some(n.article), Some(n.base.description))
                }),
            );
        } else {
            map.insert(nom_ref, (None, None));
        }
    }
    Ok(map)
}

fn count_export_lines(
    docs: &[WbAdvertDaily],
    nom_cache: &HashMap<String, (Option<String>, Option<String>)>,
    position_q: &str,
) -> usize {
    let mut n = 0usize;
    for doc in docs {
        for line in &doc.lines {
            let dto = line_to_details_dto(line, nom_cache);
            if line_matches_position(&dto, position_q) {
                n += 1;
            }
        }
    }
    n
}

struct DocCsvRowPrefix {
    document_no: String,
    document_date: String,
    advert_id: String,
    connection_name: String,
    organization_name: String,
    marketplace_name: String,
    source: String,
    fetched_at: String,
    is_posted: String,
}

async fn build_doc_csv_row_prefix(doc: &WbAdvertDaily) -> anyhow::Result<DocCsvRowPrefix> {
    let connection_name = resolve_connection_name(&doc.header.connection_id)
        .await?
        .unwrap_or_default();
    let organization_name = resolve_organization_name(&doc.header.organization_id)
        .await?
        .unwrap_or_default();
    let marketplace_name = resolve_marketplace_name(&doc.header.marketplace_id)
        .await?
        .unwrap_or_default();

    Ok(DocCsvRowPrefix {
        document_no: doc.header.document_no.clone(),
        document_date: doc.header.document_date.clone(),
        advert_id: doc.header.advert_id.to_string(),
        connection_name,
        organization_name,
        marketplace_name,
        source: doc.source_meta.source.clone(),
        fetched_at: doc.source_meta.fetched_at.clone(),
        is_posted: (doc.is_posted || doc.base.metadata.is_posted).to_string(),
    })
}

pub async fn get_by_id(
    Path(id): Path<String>,
) -> Result<Json<WbAdvertDailyDetailsDto>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    let doc = match a026_wb_advert_daily::service::get_by_id(uuid).await {
        Ok(Some(doc)) => doc,
        Ok(None) => return Err(axum::http::StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to get WB advert daily document {}: {}", id, e);
            return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    match build_details_dto(doc).await {
        Ok(dto) => Ok(Json(dto)),
        Err(e) => {
            tracing::error!("Failed to enrich WB advert daily document {}: {}", id, e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn build_details_dto(doc: WbAdvertDaily) -> anyhow::Result<WbAdvertDailyDetailsDto> {
    let connection_name = resolve_connection_name(&doc.header.connection_id).await?;
    let organization_name = resolve_organization_name(&doc.header.organization_id).await?;
    let marketplace_name = resolve_marketplace_name(&doc.header.marketplace_id).await?;

    let mut nomenclature_cache: HashMap<String, (Option<String>, Option<String>)> = HashMap::new();
    for line in &doc.lines {
        let Some(nom_ref) = line.nomenclature_ref.as_ref() else {
            continue;
        };
        if nomenclature_cache.contains_key(nom_ref) {
            continue;
        }

        let Some(uuid) = parse_uuid(nom_ref) else {
            nomenclature_cache.insert(nom_ref.clone(), (None, None));
            continue;
        };

        let nomenclature = crate::domain::a004_nomenclature::service::get_by_id(uuid).await?;
        let cached = nomenclature.map_or((None, None), |nom| {
            (Some(nom.article), Some(nom.base.description))
        });
        nomenclature_cache.insert(nom_ref.clone(), cached);
    }

    let lines = doc
        .lines
        .iter()
        .map(|line| {
            let (article, name) = line
                .nomenclature_ref
                .as_ref()
                .and_then(|nom_ref| nomenclature_cache.get(nom_ref).cloned())
                .unwrap_or((None, None));

            WbAdvertDailyLineDetailsDto {
                nm_id: line.nm_id,
                wb_name: line.nm_name.clone(),
                nomenclature_ref: line.nomenclature_ref.clone(),
                nomenclature_article: article,
                nomenclature_name: name,
                advert_ids: line.advert_ids.clone(),
                app_types: line.app_types.clone(),
                placements: line.placements.clone(),
                metrics: line.metrics.clone(),
            }
        })
        .collect();

    Ok(WbAdvertDailyDetailsDto {
        id: doc.base.id.as_string(),
        document_no: doc.header.document_no.clone(),
        document_date: doc.header.document_date.clone(),
        advert_id: doc.header.advert_id,
        connection_id: doc.header.connection_id.clone(),
        connection_name,
        organization_id: doc.header.organization_id.clone(),
        organization_name,
        marketplace_id: doc.header.marketplace_id.clone(),
        marketplace_name,
        totals: doc.totals.clone(),
        unattributed_totals: doc.unattributed_totals.clone(),
        source: doc.source_meta.source.clone(),
        fetched_at: doc.source_meta.fetched_at.clone(),
        created_at: doc.base.metadata.created_at.to_rfc3339(),
        updated_at: doc.base.metadata.updated_at.to_rfc3339(),
        is_posted: doc.is_posted || doc.base.metadata.is_posted,
        lines,
    })
}

async fn resolve_connection_name(connection_id: &str) -> anyhow::Result<Option<String>> {
    let Some(uuid) = parse_uuid(connection_id) else {
        return Ok(None);
    };
    let connection = crate::domain::a006_connection_mp::service::get_by_id(uuid).await?;
    Ok(connection.map(|item| item.base.description))
}

async fn resolve_organization_name(organization_id: &str) -> anyhow::Result<Option<String>> {
    let Some(uuid) = parse_uuid(organization_id) else {
        return Ok(None);
    };
    let organization = crate::domain::a002_organization::service::get_by_id(uuid).await?;
    Ok(organization.map(|item| item.base.description))
}

async fn resolve_marketplace_name(marketplace_id: &str) -> anyhow::Result<Option<String>> {
    let Some(uuid) = parse_uuid(marketplace_id) else {
        return Ok(None);
    };
    let marketplace = crate::domain::a005_marketplace::service::get_by_id(uuid).await?;
    Ok(marketplace.map(|item| item.base.description))
}

fn parse_uuid(value: &str) -> Option<Uuid> {
    Uuid::parse_str(value).ok()
}

pub async fn post_document(
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    a026_wb_advert_daily::posting::post_document(uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to post WB advert daily document {}: {}", id, e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(serde_json::json!({"success": true})))
}

pub async fn unpost_document(
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    a026_wb_advert_daily::posting::unpost_document(uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to unpost WB advert daily document {}: {}", id, e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(serde_json::json!({"success": true})))
}

pub async fn get_projections(
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let p911_items = crate::projections::p911_wb_advert_by_items::service::list_by_registrator_ref(
        &format!("a026:{}", id),
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to get p911 projections for {}: {}", id, e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(serde_json::json!({
        "p911_wb_advert_by_items": p911_items
    })))
}

pub async fn get_general_ledger_entries(
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let rows = crate::general_ledger::repository::list_by_registrator("a026_wb_advert_daily", &id)
        .await
        .map_err(|e| {
            tracing::error!(
                "Failed to get general ledger entries for a026 {}: {}",
                id,
                e
            );
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let general_ledger_entries = rows.into_iter().map(to_journal_dto).collect::<Vec<_>>();

    Ok(Json(
        serde_json::json!({ "general_ledger_entries": general_ledger_entries }),
    ))
}

fn to_journal_dto(row: crate::general_ledger::repository::Model) -> GeneralLedgerEntryDto {
    let comment = crate::general_ledger::turnover_registry::get_turnover_class(&row.turnover_code)
        .map(|c| c.journal_comment.to_string())
        .unwrap_or_default();

    GeneralLedgerEntryDto {
        id: row.id,
        entry_date: row.entry_date,
        layer: TurnoverLayer::from_str(&row.layer).unwrap_or(TurnoverLayer::Oper),
        connection_mp_ref: row.connection_mp_ref,
        registrator_type: row.registrator_type,
        registrator_ref: row.registrator_ref,
        order_id: row.order_id,
        debit_account: row.debit_account,
        credit_account: row.credit_account,
        amount: row.amount,
        qty: row.qty,
        turnover_code: row.turnover_code,
        resource_table: row.resource_table,
        resource_field: row.resource_field,
        resource_sign: row.resource_sign,
        created_at: row.created_at,
        comment,
    }
}
