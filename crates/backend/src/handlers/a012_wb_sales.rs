use axum::{extract::Query, Json};
use chrono::NaiveDate;
use contracts::domain::a012_wb_sales::aggregate::WbSales;
use contracts::domain::common::AggregateId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::a002_organization;
use crate::domain::a012_wb_sales;
use crate::shared::data::raw_storage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbSalesListItemDto {
    #[serde(flatten)]
    pub sales: WbSales,
    pub organization_name: Option<String>,
    pub marketplace_article: Option<String>,
    pub nomenclature_code: Option<String>,
    pub nomenclature_article: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListSalesQuery {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Handler для получения списка Wildberries Sales
pub async fn list_sales(
    Query(query): Query<ListSalesQuery>,
) -> Result<Json<Vec<WbSalesListItemDto>>, axum::http::StatusCode> {
    // Парсим даты
    let date_from = query
        .date_from
        .as_ref()
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

    let date_to = query
        .date_to
        .as_ref()
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

    let limit = query.limit.unwrap_or(20000); // По умолчанию максимум 20000 записей
    let offset = query.offset.unwrap_or(0);

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

    // Сортируем по дате (новые сначала) перед применением пагинации
    items.sort_by(|a, b| b.state.sale_dt.cmp(&a.state.sale_dt));

    let total_count = items.len();

    // Применяем пагинацию
    let items: Vec<_> = items.into_iter().skip(offset).take(limit).collect();

    // ОПТИМИЗАЦИЯ: Загружаем все организации одним запросом
    let organizations = a002_organization::service::list_all().await.map_err(|e| {
        tracing::error!("Failed to load organizations: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Создаем map для быстрого поиска
    let org_map: std::collections::HashMap<String, String> = organizations
        .into_iter()
        .map(|org| (org.base.id.as_string(), org.base.description.clone()))
        .collect();

    // Загружаем все товары маркетплейса
    let marketplace_products = crate::domain::a007_marketplace_product::service::list_all()
        .await
        .map_err(|e| {
            tracing::error!("Failed to load marketplace products: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let mp_map: std::collections::HashMap<String, (String, Option<String>)> = marketplace_products
        .into_iter()
        .map(|mp| (mp.base.id.as_string(), (mp.article.clone(), mp.nomenclature_ref.clone())))
        .collect();

    // Загружаем всю номенклатуру
    let nomenclature_items = crate::domain::a004_nomenclature::service::list_all()
        .await
        .map_err(|e| {
            tracing::error!("Failed to load nomenclature: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let nom_map: std::collections::HashMap<String, (String, String)> = nomenclature_items
        .into_iter()
        .map(|nom| (nom.base.id.as_string(), (nom.base.code.clone(), nom.article.clone())))
        .collect();

    // Формируем результат с дополнительными полями
    let result: Vec<WbSalesListItemDto> = items
        .into_iter()
        .map(|sale| {
            let organization_name = org_map.get(&sale.header.organization_id).cloned();
            
            // Получаем данные из marketplace_product
            let (marketplace_article, nomenclature_ref_from_mp) = sale.marketplace_product_ref
                .as_ref()
                .and_then(|mp_ref| mp_map.get(mp_ref).cloned())
                .unwrap_or((String::new(), None));

            // Получаем данные из nomenclature (приоритет отдаем прямой ссылке из sale)
            let nom_ref = sale.nomenclature_ref.as_ref().or(nomenclature_ref_from_mp.as_ref());
            let (nomenclature_code, nomenclature_article) = nom_ref
                .and_then(|nom_ref| nom_map.get(nom_ref).cloned())
                .unwrap_or((String::new(), String::new()));

            WbSalesListItemDto {
                sales: sale,
                organization_name,
                marketplace_article: Some(marketplace_article),
                nomenclature_code: Some(nomenclature_code),
                nomenclature_article: Some(nomenclature_article),
            }
        })
        .collect();

    Ok(Json(result))
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
