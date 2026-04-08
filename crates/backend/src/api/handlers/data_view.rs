//! API handlers for DataView semantic layer catalog.
//!
//! GET  /api/data-view              → list all DataView metadata
//! GET  /api/data-view/:id          → single DataView metadata
//! GET  /api/data-view/filters      → global filter registry (all FilterDef)
//! GET  /api/data-view/:id/filters  → resolved filters for a specific DataView
//! POST /api/data-view/:id/compute  → compute scalar value directly via DataView
//! POST /api/data-view/:id/drilldown → compute drilldown report via DataView

use axum::{extract::Path, http::StatusCode, response::Json};
use serde::Deserialize;
use serde_json::json;

use crate::data_view::{filters::global_filter_registry, DataViewRegistry};
use contracts::shared::data_view::{FilterDef, ViewContext};
use contracts::shared::drilldown::DrilldownResponse;

/// GET /api/data-view
/// Returns list of all registered DataView metadata.
pub async fn list() -> Json<serde_json::Value> {
    let registry = DataViewRegistry::new();
    let views: Vec<_> = registry.list_meta().into_iter().collect();
    Json(json!({ "views": views }))
}

/// GET /api/data-view/filters
/// Returns the full global filter registry: { filters: [FilterDef] }.
pub async fn list_filters() -> Json<serde_json::Value> {
    let registry = global_filter_registry();
    let mut filters: Vec<FilterDef> = registry.into_values().collect();
    filters.sort_by(|a, b| a.id.cmp(&b.id));
    Json(json!({ "filters": filters }))
}

/// GET /api/data-view/:id/filters
/// Returns resolved FilterDef list for a specific DataView, sorted by order.
pub async fn get_view_filters(
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let registry = DataViewRegistry::new();
    if !registry.has_view(&id) {
        return Err((StatusCode::NOT_FOUND, format!("DataView not found: {}", id)));
    }
    let filters = registry.resolve_filters(&id);
    Ok(Json(json!({ "filters": filters })))
}

/// GET /api/data-view/:id
/// Returns metadata for a single DataView by id.
pub async fn get_by_id(
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let registry = DataViewRegistry::new();
    match registry.get_meta(&id) {
        Some(meta) => Ok(Json(json!(meta))),
        None => Err((StatusCode::NOT_FOUND, format!("DataView not found: {}", id))),
    }
}

#[derive(Deserialize)]
pub struct ComputeParams {
    pub date_from: String,
    pub date_to: String,
    #[serde(default)]
    pub period2_from: Option<String>,
    #[serde(default)]
    pub period2_to: Option<String>,
    /// Comma-separated list of connection_mp UUIDs
    #[serde(default)]
    pub connection_mp_refs: Option<String>,
    /// Metric to compute: revenue | cost | commission | expenses | profit | profit_d
    #[serde(default)]
    pub metric: Option<String>,
}

/// POST /api/data-view/:id/compute
/// Directly compute a scalar value via the DataViewRegistry (no indicator needed).
pub async fn compute(
    Path(id): Path<String>,
    Json(params): Json<ComputeParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let registry = DataViewRegistry::new();

    if !registry.has_view(&id) {
        return Err((StatusCode::NOT_FOUND, format!("DataView not found: {}", id)));
    }

    let connection_mp_refs = params
        .connection_mp_refs
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let mut extra_params = std::collections::HashMap::new();
    if let Some(metric) = params.metric {
        extra_params.insert("metric".to_string(), metric);
    }

    let ctx = ViewContext {
        date_from: params.date_from,
        date_to: params.date_to,
        period2_from: params.period2_from,
        period2_to: params.period2_to,
        connection_mp_refs,
        params: extra_params,
    };

    match registry.compute_scalar(&id, &ctx).await {
        Ok(val) => Ok(Json(json!({
            "value": val.value,
            "previous_value": val.previous_value,
            "change_percent": val.change_percent,
            "status": format!("{:?}", val.status),
            "subtitle": val.subtitle,
            "details": val.details,
            "spark_points": val.spark_points,
        }))),
        Err(e) => {
            tracing::error!("DataView compute error for {}: {}", id, e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

#[derive(Deserialize)]
pub struct DvDrilldownBody {
    pub date_from: String,
    pub date_to: String,
    #[serde(default)]
    pub period2_from: Option<String>,
    #[serde(default)]
    pub period2_to: Option<String>,
    pub group_by: String,
    #[serde(default)]
    pub connection_mp_refs: Vec<String>,
    #[serde(default)]
    pub metric_id: Option<String>,
    #[serde(default)]
    pub params: std::collections::HashMap<String, String>,
    /// Multi-resource режим: список resource/metric id.
    /// Если непуст — возвращается multi-column ответ; metric_id игнорируется.
    #[serde(default)]
    pub metric_ids: Vec<String>,
}

/// POST /api/data-view/:id/drilldown
/// Compute drilldown report for a DataView with the given group_by dimension.
pub async fn drilldown(
    Path(id): Path<String>,
    Json(body): Json<DvDrilldownBody>,
) -> Result<Json<DrilldownResponse>, (StatusCode, String)> {
    let registry = DataViewRegistry::new();

    if !registry.has_view(&id) {
        return Err((StatusCode::NOT_FOUND, format!("DataView not found: {}", id)));
    }

    let mut extra_params = body.params;
    if let Some(metric_id) = body.metric_id.filter(|value| !value.trim().is_empty()) {
        extra_params.insert("metric".to_string(), metric_id);
    }

    let ctx = ViewContext {
        date_from: body.date_from,
        date_to: body.date_to,
        period2_from: body.period2_from,
        period2_to: body.period2_to,
        connection_mp_refs: body.connection_mp_refs,
        params: extra_params,
    };

    match registry
        .compute_drilldown(&id, &ctx, &body.group_by, &body.metric_ids)
        .await
    {
        Ok(resp) => Ok(Json(resp)),
        Err(e) => {
            tracing::error!("DataView drilldown error for {}: {}", id, e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}
