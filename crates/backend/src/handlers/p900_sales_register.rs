use axum::{extract::Query, Json};
use contracts::projections::p900_mp_sales_register::{
    DailyStat, MarketplaceStat, SalesRegisterDetailDto, SalesRegisterDto,
    SalesRegisterListRequest, SalesRegisterListResponse, SalesRegisterStatsByDateRequest,
    SalesRegisterStatsByDateResponse, SalesRegisterStatsByMarketplaceResponse,
};

use crate::projections::p900_mp_sales_register::{backfill, repository};

/// Handler для получения списка продаж с фильтрами
pub async fn list_sales(
    Query(req): Query<SalesRegisterListRequest>,
) -> Result<Json<SalesRegisterListResponse>, axum::http::StatusCode> {
    let (items, total) = repository::list_with_filters(
        &req.date_from,
        &req.date_to,
        req.marketplace,
        req.organization_ref,
        req.connection_mp_ref,
        req.status_norm,
        req.seller_sku,
        req.limit,
        req.offset,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to list sales: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let dtos: Vec<SalesRegisterDto> = items.into_iter().map(model_to_dto).collect();

    let has_more = total > (req.offset + dtos.len() as i32);

    Ok(Json(SalesRegisterListResponse {
        items: dtos,
        total_count: total,
        has_more,
    }))
}

/// Handler для получения детальной информации о продаже
pub async fn get_sale_detail(
    axum::extract::Path((marketplace, document_no, line_id)): axum::extract::Path<(
        String,
        String,
        String,
    )>,
) -> Result<Json<SalesRegisterDetailDto>, axum::http::StatusCode> {
    let item = repository::get_by_id(&marketplace, &document_no, &line_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get sale detail: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;

    // TODO: получить связанные данные (organization_name, connection_mp_name, etc.)
    let dto = SalesRegisterDetailDto {
        sale: model_to_dto(item),
        organization_name: None,
        connection_mp_name: None,
        marketplace_product_name: None,
    };

    Ok(Json(dto))
}

/// Handler для статистики по датам
pub async fn get_stats_by_date(
    Query(req): Query<SalesRegisterStatsByDateRequest>,
) -> Result<Json<SalesRegisterStatsByDateResponse>, axum::http::StatusCode> {
    let stats = repository::get_stats_by_date(&req.date_from, &req.date_to, req.marketplace)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get stats by date: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let dtos: Vec<DailyStat> = stats
        .into_iter()
        .map(|s| DailyStat {
            date: s.date,
            sales_count: s.sales_count,
            total_qty: s.total_qty,
            total_revenue: s.total_revenue,
        })
        .collect();

    Ok(Json(SalesRegisterStatsByDateResponse { data: dtos }))
}

/// Handler для статистики по маркетплейсам
pub async fn get_stats_by_marketplace(
    Query(req): Query<SalesRegisterStatsByDateRequest>,
) -> Result<Json<SalesRegisterStatsByMarketplaceResponse>, axum::http::StatusCode> {
    let stats = repository::get_stats_by_marketplace(&req.date_from, &req.date_to)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get stats by marketplace: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let dtos: Vec<MarketplaceStat> = stats
        .into_iter()
        .map(|s| MarketplaceStat {
            marketplace: s.marketplace,
            sales_count: s.sales_count,
            total_qty: s.total_qty,
            total_revenue: s.total_revenue,
        })
        .collect();

    Ok(Json(SalesRegisterStatsByMarketplaceResponse { data: dtos }))
}

/// Handler для запуска backfill marketplace_product_ref
pub async fn backfill_product_refs() -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    tracing::info!("Starting backfill of marketplace_product_ref");

    let stats = backfill::backfill_marketplace_product_refs()
        .await
        .map_err(|e| {
            tracing::error!("Backfill failed: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!(
        "Backfill completed: total={}, updated={}, skipped={}, failed={}",
        stats.total_records,
        stats.records_updated,
        stats.records_skipped,
        stats.records_failed
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "total_records": stats.total_records,
        "records_updated": stats.records_updated,
        "records_skipped": stats.records_skipped,
        "records_failed": stats.records_failed,
    })))
}

/// Преобразование Model в DTO
fn model_to_dto(model: repository::Model) -> SalesRegisterDto {
    SalesRegisterDto {
        marketplace: model.marketplace,
        document_no: model.document_no,
        line_id: model.line_id,
        scheme: model.scheme,
        document_type: model.document_type,
        document_version: model.document_version,
        connection_mp_ref: model.connection_mp_ref,
        organization_ref: model.organization_ref,
        marketplace_product_ref: model.marketplace_product_ref,
        nomenclature_ref: model.nomenclature_ref,
        registrator_ref: model.registrator_ref,
        event_time_source: model.event_time_source,
        sale_date: model.sale_date,
        source_updated_at: model.source_updated_at,
        status_source: model.status_source,
        status_norm: model.status_norm,
        seller_sku: model.seller_sku,
        mp_item_id: model.mp_item_id,
        barcode: model.barcode,
        title: model.title,
        qty: model.qty,
        price_list: model.price_list,
        discount_total: model.discount_total,
        price_effective: model.price_effective,
        amount_line: model.amount_line,
        currency_code: model.currency_code,
        loaded_at_utc: model.loaded_at_utc,
        payload_version: model.payload_version,
        extra: model.extra,
    }
}

/// Handler для получения проекций по registrator_ref
pub async fn get_by_registrator(
    axum::extract::Path(registrator_ref): axum::extract::Path<String>,
) -> Result<Json<Vec<SalesRegisterDto>>, axum::http::StatusCode> {
    let items = repository::get_by_registrator(&registrator_ref)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get projections by registrator: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let dtos: Vec<SalesRegisterDto> = items.into_iter().map(model_to_dto).collect();

    Ok(Json(dtos))
}

