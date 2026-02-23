use axum::{http::StatusCode, Json};
use contracts::shared::indicators::*;

use crate::shared::indicators::{metadata, registry::IndicatorRegistry};

/// POST /api/indicators/compute
///
/// Batch-computes a set of indicators for the given context (period, filters).
pub async fn compute_indicators(
    Json(req): Json<ComputeIndicatorsRequest>,
) -> Result<Json<ComputeIndicatorsResponse>, StatusCode> {
    tracing::info!(
        "Indicators: computing {} indicators for period {}..{}",
        req.indicator_ids.len(),
        req.context.date_from,
        req.context.date_to,
    );

    let registry = IndicatorRegistry::new();
    let values = registry.compute(&req.indicator_ids, &req.context).await;

    tracing::info!("Indicators: returning {} values", values.len());
    Ok(Json(ComputeIndicatorsResponse { values }))
}

/// GET /api/indicators/meta
///
/// Returns the full catalogue of available indicators and sets.
pub async fn get_indicator_catalog() -> Json<IndicatorCatalogResponse> {
    Json(metadata::build_catalog())
}
