use axum::{extract::{Path, Query}, Json};
use axum::http::StatusCode;
use contracts::projections::p901_nomenclature_barcodes::{
    BarcodeByIdResponse, BarcodeListRequest, BarcodeListResponse,
    BarcodesByNomenclatureResponse,
};

use crate::projections::p901_nomenclature_barcodes::{repository, service};

/// Handler для получения номенклатуры по штрихкоду
pub async fn get_by_barcode(
    Path(barcode): Path<String>,
) -> Result<Json<BarcodeByIdResponse>, StatusCode> {
    let model = repository::get_by_barcode(&barcode)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get barcode {}: {}", barcode, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let dto = service::model_to_dto(&model);

    Ok(Json(BarcodeByIdResponse { barcode: dto }))
}

/// Handler для получения всех штрихкодов по nomenclature_ref
pub async fn get_barcodes_by_nomenclature(
    Path(nomenclature_ref): Path<String>,
    Query(req): Query<contracts::projections::p901_nomenclature_barcodes::BarcodesByNomenclatureRequest>,
) -> Result<Json<BarcodesByNomenclatureResponse>, StatusCode> {
    let models = repository::get_by_nomenclature_ref(&nomenclature_ref, req.include_inactive)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get barcodes for nomenclature {}: {}", nomenclature_ref, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let dtos = service::models_to_dtos(models);
    let total_count = dtos.len();

    Ok(Json(BarcodesByNomenclatureResponse {
        nomenclature_ref,
        barcodes: dtos,
        total_count,
    }))
}

/// Handler для получения списка штрихкодов с фильтрами
pub async fn list_barcodes(
    Query(req): Query<BarcodeListRequest>,
) -> Result<Json<BarcodeListResponse>, StatusCode> {
    let (models, total_count) = repository::list_with_filters(
        req.nomenclature_ref,
        req.article,
        req.source,
        req.include_inactive,
        req.limit,
        req.offset,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to list barcodes: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let dtos = service::barcodes_with_nomenclature_to_dtos(models);

    Ok(Json(BarcodeListResponse {
        barcodes: dtos,
        total_count,
        limit: req.limit,
        offset: req.offset,
    }))
}
