use axum::{extract::Query, Json};
use chrono::NaiveDate;
use contracts::domain::a015_wb_orders::aggregate::WbOrders;
use contracts::domain::common::AggregateId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::a002_organization;
use crate::domain::a015_wb_orders;
use crate::shared::data::raw_storage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbOrdersListItemDto {
    #[serde(flatten)]
    pub order: WbOrders,
    pub organization_name: Option<String>,
    pub marketplace_article: Option<String>,
    pub nomenclature_code: Option<String>,
    pub nomenclature_article: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListOrdersQuery {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub organization_id: Option<String>,
}

/// Handler для получения списка Wildberries Orders
pub async fn list_orders(
    Query(query): Query<ListOrdersQuery>,
) -> Result<Json<Vec<WbOrdersListItemDto>>, axum::http::StatusCode> {
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
        a015_wb_orders::service::list_by_date_range(date_from, date_to)
            .await
            .map_err(|e| {
                tracing::error!("Failed to list Wildberries orders: {}", e);
                axum::http::StatusCode::INTERNAL_SERVER_ERROR
            })?
    } else {
        a015_wb_orders::service::list_all().await.map_err(|e| {
            tracing::error!("Failed to list Wildberries orders: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?
    };

    // Фильтруем по organization_id, если указан
    if let Some(org_id) = query.organization_id {
        items.retain(|order| order.header.organization_id == org_id);
    }

    // Сортируем по дате (новые сначала) перед применением пагинации
    items.sort_by(|a, b| b.state.order_dt.cmp(&a.state.order_dt));

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
    let result: Vec<WbOrdersListItemDto> = items
        .into_iter()
        .map(|order| {
            let organization_name = org_map.get(&order.header.organization_id).cloned();
            
            // Получаем данные из marketplace_product
            let (marketplace_article, nomenclature_ref_from_mp) = order.marketplace_product_ref
                .as_ref()
                .and_then(|mp_ref| mp_map.get(mp_ref).cloned())
                .unwrap_or((String::new(), None));

            // Получаем данные из nomenclature (приоритет отдаем прямой ссылке из order)
            let nom_ref = order.nomenclature_ref.as_ref().or(nomenclature_ref_from_mp.as_ref());
            let (nomenclature_code, nomenclature_article) = nom_ref
                .and_then(|nom_ref| nom_map.get(nom_ref).cloned())
                .unwrap_or((String::new(), String::new()));

            WbOrdersListItemDto {
                order,
                organization_name,
                marketplace_article: Some(marketplace_article),
                nomenclature_code: Some(nomenclature_code),
                nomenclature_article: Some(nomenclature_article),
            }
        })
        .collect();

    Ok(Json(result))
}

/// Handler для получения детальной информации о Wildberries Order
pub async fn get_order_detail(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<WbOrders>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    let item = a015_wb_orders::service::get_by_id(uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get Wildberries order detail: {}", e);
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
) -> Result<Json<Vec<WbOrders>>, axum::http::StatusCode> {
    let items = a015_wb_orders::repository::search_by_document_no(&query.srid)
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

/// Handler для удаления документа
pub async fn delete_order(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    a015_wb_orders::service::delete(uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete order: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(serde_json::json!({"success": true})))
}

/// Handler для проведения документа
pub async fn post_order(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    a015_wb_orders::posting::post_document(uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to post order: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(serde_json::json!({"success": true, "message": "Document posted"})))
}

/// Handler для отмены проведения документа
pub async fn unpost_order(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    a015_wb_orders::posting::unpost_document(uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to unpost order: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(serde_json::json!({"success": true, "message": "Document unposted"})))
}

