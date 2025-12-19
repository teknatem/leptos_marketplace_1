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
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
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
        Ok(id) => Ok(Json(json!({"id": id}))),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
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
