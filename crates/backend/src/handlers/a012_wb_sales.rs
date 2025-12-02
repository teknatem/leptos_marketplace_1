use axum::{extract::Query, Json};
use chrono::NaiveDate;
use contracts::domain::a012_wb_sales::aggregate::WbSales;
use contracts::domain::common::AggregateId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use tokio::join;

use crate::domain::a002_organization;
use crate::domain::a012_wb_sales;
use crate::shared::data::db::get_connection;
use crate::shared::data::raw_storage;
use sea_orm::{ConnectionTrait, Statement};
use std::collections::HashMap;
use std::sync::OnceLock;
use tokio::sync::RwLock;

// Cache for reference data (refreshed periodically)
static ORG_CACHE: OnceLock<RwLock<(HashMap<String, String>, std::time::Instant)>> = OnceLock::new();
static MP_CACHE: OnceLock<RwLock<(HashMap<String, (String, Option<String>)>, std::time::Instant)>> = OnceLock::new();
static NOM_CACHE: OnceLock<RwLock<(HashMap<String, (String, String)>, std::time::Instant)>> = OnceLock::new();

const CACHE_TTL_SECS: u64 = 300; // 5 minutes

async fn get_org_map() -> HashMap<String, String> {
    let cache = ORG_CACHE.get_or_init(|| RwLock::new((HashMap::new(), std::time::Instant::now() - std::time::Duration::from_secs(CACHE_TTL_SECS + 1))));
    
    {
        let read = cache.read().await;
        if read.1.elapsed().as_secs() < CACHE_TTL_SECS {
            return read.0.clone();
        }
    }
    
    // Refresh cache
    let mut write = cache.write().await;
    if write.1.elapsed().as_secs() < CACHE_TTL_SECS {
        return write.0.clone();
    }
    
    if let Ok(orgs) = a002_organization::service::list_all().await {
        write.0 = orgs.into_iter().map(|org| (org.base.id.as_string(), org.base.description.clone())).collect();
        write.1 = std::time::Instant::now();
    }
    write.0.clone()
}

async fn get_mp_map() -> HashMap<String, (String, Option<String>)> {
    let cache = MP_CACHE.get_or_init(|| RwLock::new((HashMap::new(), std::time::Instant::now() - std::time::Duration::from_secs(CACHE_TTL_SECS + 1))));
    
    {
        let read = cache.read().await;
        if read.1.elapsed().as_secs() < CACHE_TTL_SECS {
            return read.0.clone();
        }
    }
    
    let mut write = cache.write().await;
    if write.1.elapsed().as_secs() < CACHE_TTL_SECS {
        return write.0.clone();
    }
    
    if let Ok(mps) = crate::domain::a007_marketplace_product::service::list_all().await {
        write.0 = mps.into_iter().map(|mp| (mp.base.id.as_string(), (mp.article.clone(), mp.nomenclature_ref.clone()))).collect();
        write.1 = std::time::Instant::now();
    }
    write.0.clone()
}

async fn get_nom_map() -> HashMap<String, (String, String)> {
    let cache = NOM_CACHE.get_or_init(|| RwLock::new((HashMap::new(), std::time::Instant::now() - std::time::Duration::from_secs(CACHE_TTL_SECS + 1))));
    
    {
        let read = cache.read().await;
        if read.1.elapsed().as_secs() < CACHE_TTL_SECS {
            return read.0.clone();
        }
    }
    
    let mut write = cache.write().await;
    if write.1.elapsed().as_secs() < CACHE_TTL_SECS {
        return write.0.clone();
    }
    
    if let Ok(noms) = crate::domain::a004_nomenclature::service::list_all().await {
        write.0 = noms.into_iter().map(|nom| (nom.base.id.as_string(), (nom.base.code.clone(), nom.article.clone()))).collect();
        write.1 = std::time::Instant::now();
    }
    write.0.clone()
}

/// Получить минимальные даты операций (rr_dt) из P903 по SRID
/// Возвращает две HashMap: (sales_dates, return_dates)
async fn get_operation_dates_from_p903(
) -> Result<(HashMap<String, String>, HashMap<String, String>), anyhow::Error> {
    let db = get_connection();

    // SQL запрос для операций "Продажа"
    let sql_sales = r#"
        SELECT srid, MIN(rr_dt) as min_rr_dt
        FROM p903_wb_finance_report
        WHERE srid IS NOT NULL AND srid != ''
          AND supplier_oper_name = 'Продажа'
        GROUP BY srid
    "#;

    // SQL запрос для операций "Возврат"
    let sql_returns = r#"
        SELECT srid, MIN(rr_dt) as min_rr_dt
        FROM p903_wb_finance_report
        WHERE srid IS NOT NULL AND srid != ''
          AND supplier_oper_name = 'Возврат'
        GROUP BY srid
    "#;

    // Выполняем запрос для продаж
    let stmt_sales =
        Statement::from_string(sea_orm::DatabaseBackend::Sqlite, sql_sales.to_string());
    let sales_result = db.query_all(stmt_sales).await?;

    let mut sales_dates = HashMap::new();
    for row in sales_result {
        if let (Ok(srid), Ok(rr_dt)) = (
            row.try_get::<String>("", "srid"),
            row.try_get::<String>("", "min_rr_dt"),
        ) {
            sales_dates.insert(srid, rr_dt);
        }
    }

    // Выполняем запрос для возвратов
    let stmt_returns =
        Statement::from_string(sea_orm::DatabaseBackend::Sqlite, sql_returns.to_string());
    let returns_result = db.query_all(stmt_returns).await?;

    let mut return_dates = HashMap::new();
    for row in returns_result {
        if let (Ok(srid), Ok(rr_dt)) = (
            row.try_get::<String>("", "srid"),
            row.try_get::<String>("", "min_rr_dt"),
        ) {
            return_dates.insert(srid, rr_dt);
        }
    }

    Ok((sales_dates, return_dates))
}

/// Compact DTO for list view (only essential fields)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbSalesListItemDto {
    pub id: String,
    pub document_no: String,
    pub sale_date: String,
    pub supplier_article: String,
    pub name: String,
    pub qty: f64,
    pub amount_line: Option<f64>,
    pub total_price: Option<f64>,
    pub finished_price: Option<f64>,
    pub event_type: String,
    pub organization_name: Option<String>,
    pub marketplace_article: Option<String>,
    pub nomenclature_code: Option<String>,
    pub nomenclature_article: Option<String>,
    pub operation_date: Option<String>,
}

impl WbSalesListItemDto {
    pub fn from_wb_sales(
        sale: &WbSales,
        organization_name: Option<String>,
        marketplace_article: Option<String>,
        nomenclature_code: Option<String>,
        nomenclature_article: Option<String>,
        operation_date: Option<String>,
    ) -> Self {
        Self {
            id: sale.base.id.as_string(),
            document_no: sale.header.document_no.clone(),
            sale_date: sale.state.sale_dt.to_rfc3339(),
            supplier_article: sale.line.supplier_article.clone(),
            name: sale.line.name.clone(),
            qty: sale.line.qty,
            amount_line: sale.line.amount_line,
            total_price: sale.line.total_price,
            finished_price: sale.line.finished_price,
            event_type: sale.state.event_type.clone(),
            organization_name,
            marketplace_article,
            nomenclature_code,
            nomenclature_article,
            operation_date,
        }
    }
}

/// Paginated response for WB Sales list
#[derive(Debug, Clone, Serialize)]
pub struct PaginatedWbSalesResponse {
    pub items: Vec<WbSalesListItemDto>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

#[derive(Debug, Deserialize)]
pub struct ListSalesQuery {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub organization_id: Option<String>,
    pub sort_by: Option<String>,
    pub sort_desc: Option<bool>,
}

/// Handler для получения списка Wildberries Sales с пагинацией
pub async fn list_sales(
    Query(query): Query<ListSalesQuery>,
) -> Result<Json<PaginatedWbSalesResponse>, axum::http::StatusCode> {
    // Парсим даты
    let date_from = query
        .date_from
        .as_ref()
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

    let date_to = query
        .date_to
        .as_ref()
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

    let page_size = query.limit.unwrap_or(100); // По умолчанию 100 записей на страницу
    let offset = query.offset.unwrap_or(0);
    let page = if page_size > 0 { offset / page_size } else { 0 };
    let sort_by = query.sort_by.clone().unwrap_or_else(|| "sale_date".to_string());
    let sort_desc = query.sort_desc.unwrap_or(true);

    let mut items = if date_from.is_some() || date_to.is_some() {
        a012_wb_sales::service::list_by_date_range(date_from, date_to)
            .await
            .map_err(|e| {
                tracing::error!("Failed to list Wildberries sales: {}", e);
                axum::http::StatusCode::INTERNAL_SERVER_ERROR
            })?
    } else {
        a012_wb_sales::service::list_all().await.map_err(|e| {
            tracing::error!("Failed to list Wildberries sales: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?
    };

    // Фильтруем по organization_id, если указан
    if let Some(ref org_id) = query.organization_id {
        items.retain(|sale| sale.header.organization_id == *org_id);
    }

    // Серверная сортировка по выбранному полю
    items.sort_by(|a, b| {
        let cmp = match sort_by.as_str() {
            "document_no" => a.header.document_no.cmp(&b.header.document_no),
            "sale_date" => a.state.sale_dt.cmp(&b.state.sale_dt),
            "supplier_article" => a.line.supplier_article.cmp(&b.line.supplier_article),
            "name" => a.line.name.cmp(&b.line.name),
            "qty" => a.line.qty.partial_cmp(&b.line.qty).unwrap_or(std::cmp::Ordering::Equal),
            "amount_line" => a.line.amount_line.partial_cmp(&b.line.amount_line).unwrap_or(std::cmp::Ordering::Equal),
            "total_price" => a.line.total_price.partial_cmp(&b.line.total_price).unwrap_or(std::cmp::Ordering::Equal),
            "finished_price" => a.line.finished_price.partial_cmp(&b.line.finished_price).unwrap_or(std::cmp::Ordering::Equal),
            _ => a.state.sale_dt.cmp(&b.state.sale_dt), // По умолчанию по дате
        };
        if sort_desc { cmp.reverse() } else { cmp }
    });

    // Сохраняем общее количество ДО пагинации
    let total = items.len();
    let total_pages = if page_size > 0 { (total + page_size - 1) / page_size } else { 0 };

    // Применяем пагинацию
    let items: Vec<_> = items.into_iter().skip(offset).take(page_size).collect();

    // ОПТИМИЗАЦИЯ: Загружаем справочники параллельно из кэша
    let (org_map, mp_map, nom_map, operation_dates) = join!(
        get_org_map(),
        get_mp_map(),
        get_nom_map(),
        get_operation_dates_from_p903()
    );
    
    let (sales_dates, return_dates) = operation_dates.unwrap_or_default();

    // Формируем результат с дополнительными полями
    let result: Vec<WbSalesListItemDto> = items
        .into_iter()
        .map(|sale| {
            let organization_name = org_map.get(&sale.header.organization_id).cloned();

            // Получаем данные из marketplace_product
            let (marketplace_article, nomenclature_ref_from_mp) = sale
                .marketplace_product_ref
                .as_ref()
                .and_then(|mp_ref| mp_map.get(mp_ref).cloned())
                .unwrap_or((String::new(), None));

            // Получаем данные из nomenclature (приоритет отдаем прямой ссылке из sale)
            let nom_ref = sale
                .nomenclature_ref
                .as_ref()
                .or(nomenclature_ref_from_mp.as_ref());
            let (nomenclature_code, nomenclature_article) = nom_ref
                .and_then(|nom_ref| nom_map.get(nom_ref).cloned())
                .unwrap_or((String::new(), String::new()));

            // Получаем дату операции из P903 по document_no (SRID)
            // Выбираем дату в зависимости от знака finished_price
            let operation_date = match sale.line.finished_price {
                Some(price) if price > 0.0 => {
                    // Положительная цена - ищем среди продаж
                    sales_dates.get(&sale.header.document_no).cloned()
                }
                Some(price) if price < 0.0 => {
                    // Отрицательная цена - ищем среди возвратов
                    return_dates.get(&sale.header.document_no).cloned()
                }
                _ => {
                    // None или 0 - не устанавливаем дату
                    None
                }
            };

            // Use compact DTO with only essential fields
            WbSalesListItemDto::from_wb_sales(
                &sale,
                organization_name,
                if marketplace_article.is_empty() { None } else { Some(marketplace_article) },
                if nomenclature_code.is_empty() { None } else { Some(nomenclature_code) },
                if nomenclature_article.is_empty() { None } else { Some(nomenclature_article) },
                operation_date,
            )
        })
        .collect();

    Ok(Json(PaginatedWbSalesResponse {
        items: result,
        total,
        page,
        page_size,
        total_pages,
    }))
}

/// Handler для получения детальной информации о Wildberries Sale
pub async fn get_sale_detail(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<WbSales>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    let item = a012_wb_sales::service::get_by_id(uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get Wildberries sale detail: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;

    Ok(Json(item))
}

/// Handler для поиска документов по srid (document_no)
#[derive(Debug, Deserialize)]
pub struct SearchBySridQuery {
    pub srid: String,
}

pub async fn search_by_srid(
    Query(query): Query<SearchBySridQuery>,
) -> Result<Json<Vec<WbSales>>, axum::http::StatusCode> {
    let items = a012_wb_sales::repository::search_by_document_no(&query.srid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to search by srid: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(items))
}

/// Handler для получения raw JSON от WB API по raw_payload_ref
pub async fn get_raw_json(
    axum::extract::Path(ref_id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let raw_json_str = raw_storage::get_by_ref(&ref_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get raw JSON: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;

    let json_value: serde_json::Value = serde_json::from_str(&raw_json_str).map_err(|e| {
        tracing::error!("Failed to parse raw JSON: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(json_value))
}

/// Handler для проведения документа
pub async fn post_document(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    a012_wb_sales::posting::post_document(uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to post document: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(serde_json::json!({"success": true})))
}

/// Handler для отмены проведения документа
pub async fn unpost_document(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    a012_wb_sales::posting::unpost_document(uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to unpost document: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(serde_json::json!({"success": true})))
}

#[derive(Deserialize)]
pub struct PostPeriodRequest {
    pub from: String,
    pub to: String,
}

#[derive(Deserialize)]
pub struct BatchOperationRequest {
    pub ids: Vec<String>,
}

/// Handler для проведения документов за период
pub async fn post_period(
    Query(req): Query<PostPeriodRequest>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let from = NaiveDate::parse_from_str(&req.from, "%Y-%m-%d")
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    let to = NaiveDate::parse_from_str(&req.to, "%Y-%m-%d")
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    let documents = a012_wb_sales::service::list_all().await.map_err(|e| {
        tracing::error!("Failed to list documents: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut posted_count = 0;
    let mut failed_count = 0;

    for doc in documents {
        let doc_date = doc.source_meta.fetched_at.date_naive();
        if doc_date >= from && doc_date <= to {
            match a012_wb_sales::posting::post_document(doc.base.id.value()).await {
                Ok(_) => {
                    posted_count += 1;
                }
                Err(e) => {
                    failed_count += 1;
                    tracing::error!("Failed to post document {}: {}", doc.base.id.as_string(), e);
                }
            }
        }
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "posted_count": posted_count,
        "failed_count": failed_count
    })))
}

/// Handler для пакетного проведения документов (до 100 документов за раз)
pub async fn batch_post_documents(
    Json(req): Json<BatchOperationRequest>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let total = req.ids.len();
    let mut succeeded = 0;
    let mut failed = 0;

    for id_str in req.ids {
        let uuid = match Uuid::parse_str(&id_str) {
            Ok(uuid) => uuid,
            Err(_) => {
                failed += 1;
                continue;
            }
        };

        match a012_wb_sales::posting::post_document(uuid).await {
            Ok(_) => succeeded += 1,
            Err(_) => failed += 1,
        }
    }

    tracing::info!(
        "Batch posted {} documents (succeeded: {}, failed: {})",
        total,
        succeeded,
        failed
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "succeeded": succeeded,
        "failed": failed,
        "total": total
    })))
}

/// Handler для пакетной отмены проведения документов (до 100 документов за раз)
pub async fn batch_unpost_documents(
    Json(req): Json<BatchOperationRequest>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let total = req.ids.len();
    let mut succeeded = 0;
    let mut failed = 0;

    for id_str in req.ids {
        let uuid = match Uuid::parse_str(&id_str) {
            Ok(uuid) => uuid,
            Err(_) => {
                failed += 1;
                continue;
            }
        };

        match a012_wb_sales::posting::unpost_document(uuid).await {
            Ok(_) => succeeded += 1,
            Err(_) => failed += 1,
        }
    }

    tracing::info!(
        "Batch unposted {} documents (succeeded: {}, failed: {})",
        total,
        succeeded,
        failed
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "succeeded": succeeded,
        "failed": failed,
        "total": total
    })))
}

/// Handler для получения проекций по registrator_ref
pub async fn get_projections(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    // Получаем данные из проекций p900 и p904 (WB Sales использует только эти)
    let p900_items = crate::projections::p900_mp_sales_register::repository::get_by_registrator(&id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get p900 projections: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let p904_items = crate::projections::p904_sales_data::repository::get_by_registrator(&id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get p904 projections: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Объединяем результаты
    let result = serde_json::json!({
        "p900_sales_register": p900_items,
        "p904_sales_data": p904_items,
    });

    Ok(Json(result))
}
