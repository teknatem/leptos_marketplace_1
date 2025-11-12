use axum::{extract::Query, Json};
use chrono::NaiveDate;
use contracts::domain::a013_ym_order::aggregate::YmOrder;
use contracts::domain::common::AggregateId;
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::a013_ym_order;
use crate::shared::data::raw_storage;

/// Handler для получения списка Yandex Market Orders
pub async fn list_orders() -> Result<Json<Vec<YmOrder>>, axum::http::StatusCode> {
    let items = a013_ym_order::service::list_all().await.map_err(|e| {
        tracing::error!("Failed to list Yandex Market orders: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(items))
}

/// Handler для получения детальной информации о Yandex Market Order
pub async fn get_order_detail(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<YmOrder>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    let item = a013_ym_order::service::get_by_id(uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get Yandex Market order detail: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;

    Ok(Json(item))
}

/// Handler для получения raw JSON от Yandex Market API по raw_payload_ref
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

    let json_value: serde_json::Value = serde_json::from_str(&raw_json_str)
        .map_err(|e| {
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

    a013_ym_order::posting::post_document(uuid)
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

    a013_ym_order::posting::unpost_document(uuid)
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

/// Handler для проведения документов за период
pub async fn post_period(
    Query(req): Query<PostPeriodRequest>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let from = NaiveDate::parse_from_str(&req.from, "%Y-%m-%d")
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    let to = NaiveDate::parse_from_str(&req.to, "%Y-%m-%d")
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    let documents = a013_ym_order::service::list_all()
        .await
        .map_err(|e| {
            tracing::error!("Failed to list documents: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let mut posted_count = 0;
    let mut failed_count = 0;

    for doc in documents {
        let doc_date = doc.source_meta.fetched_at.date_naive();
        if doc_date >= from && doc_date <= to {
            match a013_ym_order::posting::post_document(doc.base.id.value()).await {
                Ok(_) => {
                    posted_count += 1;
                    tracing::info!("Posted document: {}", doc.base.id.as_string());
                }
                Err(e) => {
                    failed_count += 1;
                    tracing::error!(
                        "Failed to post document {}: {}",
                        doc.base.id.as_string(),
                        e
                    );
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

