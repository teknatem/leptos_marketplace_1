use crate::data_view::DataViewRegistry;
use chrono::NaiveDate;
use contracts::shared::analytics::IndicatorValue;
use contracts::shared::data_view::ViewContext;
use contracts::shared::drilldown::DrilldownResponse;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DataViewContextRequest {
    pub date_from: String,
    pub date_to: String,
    #[serde(default)]
    pub period2_from: Option<String>,
    #[serde(default)]
    pub period2_to: Option<String>,
    #[serde(default)]
    pub connection_mp_refs: Vec<String>,
    #[serde(default)]
    pub params: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataViewScalarRequest {
    pub view_id: String,
    pub metric_id: String,
    #[serde(flatten)]
    pub context: DataViewContextRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataViewDrilldownRequest {
    pub view_id: String,
    pub group_by: String,
    pub metric_ids: Vec<String>,
    #[serde(flatten)]
    pub context: DataViewContextRequest,
}

fn parse_date(label: &str, value: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| format!("{label} must use YYYY-MM-DD format"))
}

fn build_context(mut request: DataViewContextRequest) -> Result<ViewContext, String> {
    let date_from = parse_date("date_from", &request.date_from)?;
    let date_to = parse_date("date_to", &request.date_to)?;
    if date_from > date_to {
        return Err("date_from must not be later than date_to".to_string());
    }
    match (&request.period2_from, &request.period2_to) {
        (Some(from), Some(to)) => {
            let from = parse_date("period2_from", from)?;
            let to = parse_date("period2_to", to)?;
            if from > to {
                return Err("period2_from must not be later than period2_to".to_string());
            }
        }
        (None, None) => {}
        _ => return Err("period2_from and period2_to must be provided together".to_string()),
    }
    request
        .connection_mp_refs
        .retain(|value| !value.trim().is_empty());
    Ok(ViewContext {
        date_from: request.date_from,
        date_to: request.date_to,
        period2_from: request.period2_from,
        period2_to: request.period2_to,
        connection_mp_refs: request.connection_mp_refs,
        params: request.params,
    })
}

fn validate_metric(
    meta: &contracts::shared::data_view::DataViewMeta,
    metric_id: &str,
) -> Result<(), String> {
    if meta
        .available_resources
        .iter()
        .any(|resource| resource.id == metric_id)
    {
        Ok(())
    } else {
        Err(format!(
            "Unknown metric '{}' for DataView '{}'. Available: {}",
            metric_id,
            meta.id,
            meta.available_resources
                .iter()
                .map(|resource| resource.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

pub async fn run_data_view_scalar(
    request: DataViewScalarRequest,
) -> Result<IndicatorValue, String> {
    let started = Instant::now();
    let registry = DataViewRegistry::new();
    let meta = registry
        .get_meta(&request.view_id)
        .ok_or_else(|| format!("Unknown DataView: {}", request.view_id))?;
    validate_metric(meta, &request.metric_id)?;
    let mut context = build_context(request.context)?;
    context
        .params
        .insert("metric".to_string(), request.metric_id.clone());
    let result = registry
        .compute_scalar(&request.view_id, &context)
        .await
        .map_err(|error| format!("DataView execution failed: {error}"))?;
    tracing::info!(
        source_kind = "dataview",
        source_id = %request.view_id,
        method = "scalar",
        elapsed_ms = started.elapsed().as_millis(),
        "semantic data query completed"
    );
    Ok(result)
}

pub async fn run_data_view_drilldown(
    request: DataViewDrilldownRequest,
) -> Result<DrilldownResponse, String> {
    let started = Instant::now();
    let registry = DataViewRegistry::new();
    let meta = registry
        .get_meta(&request.view_id)
        .ok_or_else(|| format!("Unknown DataView: {}", request.view_id))?;
    if !meta
        .available_dimensions
        .iter()
        .any(|dimension| dimension.id == request.group_by)
    {
        return Err(format!(
            "Unknown dimension '{}' for DataView '{}'",
            request.group_by, request.view_id
        ));
    }
    if request.metric_ids.is_empty() {
        return Err("metric_ids must contain at least one metric".to_string());
    }
    for metric_id in &request.metric_ids {
        validate_metric(meta, metric_id)?;
    }
    let context = build_context(request.context)?;
    let result = registry
        .compute_drilldown(
            &request.view_id,
            &context,
            &request.group_by,
            &request.metric_ids,
        )
        .await
        .map_err(|error| format!("DataView drilldown failed: {error}"))?;
    tracing::info!(
        source_kind = "dataview",
        source_id = %request.view_id,
        method = "drilldown",
        elapsed_ms = started.elapsed().as_millis(),
        row_count = result.rows.len(),
        "semantic data query completed"
    );
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_rejects_invalid_periods() {
        let context = DataViewContextRequest {
            date_from: "2026-02-01".to_string(),
            date_to: "2026-01-01".to_string(),
            ..DataViewContextRequest::default()
        };
        assert!(build_context(context).is_err());
    }

    #[test]
    fn metric_validation_uses_view_metadata() {
        let registry = DataViewRegistry::new();
        let meta = registry.get_meta("dv001_revenue").unwrap();
        assert!(validate_metric(meta, "revenue").is_ok());
        assert!(validate_metric(meta, "made_up_metric").is_err());
    }
}
