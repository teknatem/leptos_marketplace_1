use axum::{
    extract::{Path, Query},
    Json,
};
use serde::Deserialize;
use serde_json::json;

use crate::domain::a004_nomenclature;

#[derive(Deserialize)]
pub struct SearchNomenclatureQuery {
    pub article: String,
}

#[derive(Deserialize)]
pub struct NomenclatureListParams {
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub sort_by: Option<String>,
    pub sort_desc: Option<bool>,
    pub q: Option<String>,
    pub only_mp: Option<bool>,
}

#[derive(serde::Serialize)]
pub struct NomenclaturePaginatedResponse {
    pub items: Vec<contracts::domain::a004_nomenclature::aggregate::Nomenclature>,
    pub total: u64,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

/// GET /api/nomenclature
pub async fn list_all() -> Result<
    Json<Vec<contracts::domain::a004_nomenclature::aggregate::Nomenclature>>,
    axum::http::StatusCode,
> {
    match a004_nomenclature::service::list_all().await {
        Ok(v) => Ok(Json(v)),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /api/a004/nomenclature?limit=&offset=&sort_by=&sort_desc=&q=&only_mp=
pub async fn list_paginated(
    Query(params): Query<NomenclatureListParams>,
) -> Result<Json<NomenclaturePaginatedResponse>, axum::http::StatusCode> {
    let limit = params.limit.unwrap_or(100).clamp(10, 1000);
    let offset = params.offset.unwrap_or(0);
    let sort_by = params.sort_by.as_deref().unwrap_or("article");
    let sort_desc = params.sort_desc.unwrap_or(false);
    let q = params.q.unwrap_or_default();
    let only_mp = params.only_mp.unwrap_or(false);

    match a004_nomenclature::service::list_paginated(limit, offset, sort_by, sort_desc, &q, only_mp)
        .await
    {
        Ok((items, total)) => {
            let page_size = limit as usize;
            let page = (offset as usize) / page_size;
            let total_pages = ((total as usize) + page_size - 1) / page_size;

            Ok(Json(NomenclaturePaginatedResponse {
                items,
                total,
                page,
                page_size,
                total_pages,
            }))
        }
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /api/nomenclature/:id
pub async fn get_by_id(
    Path(id): Path<String>,
) -> Result<
    Json<contracts::domain::a004_nomenclature::aggregate::Nomenclature>,
    axum::http::StatusCode,
> {
    let uuid = match uuid::Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => return Err(axum::http::StatusCode::BAD_REQUEST),
    };
    match a004_nomenclature::service::get_by_id(uuid).await {
        Ok(Some(v)) => Ok(Json(v)),
        Ok(None) => Err(axum::http::StatusCode::NOT_FOUND),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /api/nomenclature
pub async fn upsert(
    Json(dto): Json<contracts::domain::a004_nomenclature::aggregate::NomenclatureDto>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    tracing::debug!(
        "Received nomenclature upsert: id={:?}, description={}",
        dto.id,
        dto.description
    );

    let result = if dto.id.is_some() {
        a004_nomenclature::service::update(dto)
            .await
            .map(|_| uuid::Uuid::nil().to_string())
    } else {
        a004_nomenclature::service::create(dto)
            .await
            .map(|id| id.to_string())
    };
    match result {
        Ok(id) => {
            tracing::debug!("Nomenclature saved successfully: {}", id);
            Ok(Json(json!({"id": id})))
        }
        Err(e) => {
            let error_msg = format!("{}", e);
            tracing::error!("Failed to save nomenclature: {}", error_msg);
            Err((
                axum::http::StatusCode::BAD_REQUEST,
                Json(json!({"error": error_msg})),
            ))
        }
    }
}

/// DELETE /api/nomenclature/:id
pub async fn delete(Path(id): Path<String>) -> Result<(), axum::http::StatusCode> {
    let uuid = match uuid::Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => return Err(axum::http::StatusCode::BAD_REQUEST),
    };
    match a004_nomenclature::service::delete(uuid).await {
        Ok(true) => Ok(()),
        Ok(false) => Err(axum::http::StatusCode::NOT_FOUND),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /api/nomenclature/import-excel
pub async fn import_excel(
    Json(excel_data): Json<a004_nomenclature::excel_import::ExcelData>,
) -> Result<Json<contracts::domain::a004_nomenclature::ImportResult>, axum::http::StatusCode> {
    tracing::info!(
        "Received Excel import request with {} rows",
        excel_data.metadata.row_count
    );

    // Импортируем данные из ExcelData (backend делает маппинг полей)
    let result = match a004_nomenclature::excel_import::import_nomenclature_from_excel_data(
        excel_data,
    )
    .await
    {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("Excel import error: {}", e);
            return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    Ok(Json(result))
}

/// GET /api/nomenclature/dimensions
pub async fn get_dimensions(
) -> Result<Json<a004_nomenclature::repository::DimensionValues>, axum::http::StatusCode> {
    match a004_nomenclature::repository::get_distinct_dimension_values().await {
        Ok(values) => Ok(Json(values)),
        Err(e) => {
            tracing::error!("Failed to get dimension values: {}", e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/nomenclature/search
pub async fn search_by_article(
    Query(query): Query<SearchNomenclatureQuery>,
) -> Result<
    Json<Vec<contracts::domain::a004_nomenclature::aggregate::Nomenclature>>,
    axum::http::StatusCode,
> {
    match a004_nomenclature::repository::find_by_article(query.article.trim()).await {
        Ok(items) => Ok(Json(items)),
        Err(e) => {
            tracing::error!("Failed to search nomenclature by article: {}", e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
