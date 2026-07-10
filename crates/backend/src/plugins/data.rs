use crate::shared::data_access::{
    execute_raw_query, query_schema, run_data_view_drilldown, DataSourceKind, DataSourceRef,
    DataViewContextRequest, DataViewDrilldownRequest, RawQueryRequest, SchemaAggregate,
    SchemaFilter, SchemaFilterOperator, SchemaMetric, SchemaQueryRequest, SchemaSortDirection,
    SchemaSortRule, SqlAccessProfile, TabularResult,
};
use contracts::plugins::{
    PluginDataSource, PluginRunContext, PluginSchemaAggregate, PluginSchemaFilterOperator,
    PluginSchemaSortDirection,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub const CHART_ROW_LIMIT: usize = 200;
pub const TABLE_ROW_LIMIT: usize = 2_000;
pub const SNAPSHOT_MAX_BYTES: usize = 1024 * 1024;

pub fn default_source_context() -> PluginRunContext {
    let today = chrono::Utc::now().date_naive();
    PluginRunContext {
        date_from: Some(
            (today - chrono::Duration::days(29))
                .format("%Y-%m-%d")
                .to_string(),
        ),
        date_to: Some(today.format("%Y-%m-%d").to_string()),
        ..PluginRunContext::default()
    }
}

pub fn effective_source_context(context: Option<&PluginRunContext>) -> PluginRunContext {
    let mut effective = context.cloned().unwrap_or_default();
    let defaults = default_source_context();
    if effective.date_from.is_none() {
        effective.date_from = defaults.date_from;
    }
    if effective.date_to.is_none() {
        effective.date_to = defaults.date_to;
    }
    effective
}

pub fn source_uses_period_context(source: &PluginDataSource) -> bool {
    serde_json::to_string(source)
        .is_ok_and(|json| json.contains("$context.date_from") || json.contains("$context.date_to"))
}

fn resolve_context_value(value: &Value, context: &PluginRunContext) -> Result<Value, String> {
    let Some(binding) = value
        .as_str()
        .and_then(|text| text.strip_prefix("$context."))
    else {
        return Ok(value.clone());
    };
    match binding {
        "date_from" => context
            .date_from
            .clone()
            .map(Value::String)
            .ok_or_else(|| "$context.date_from is not set".to_string()),
        "date_to" => context
            .date_to
            .clone()
            .map(Value::String)
            .ok_or_else(|| "$context.date_to is not set".to_string()),
        key if key.starts_with("params.") => context
            .params
            .get(&key[7..])
            .cloned()
            .map(Value::String)
            .ok_or_else(|| format!("$context.{key} is not set")),
        _ => Err(format!("Unsupported context binding: $context.{binding}")),
    }
}

pub fn infer_columns(rows: &[Value], columns: &[String]) -> Vec<Value> {
    columns
        .iter()
        .map(|name| {
            let values: Vec<&Value> = rows.iter().filter_map(|row| row.get(name)).collect();
            let nullable = values.iter().any(|value| value.is_null()) || values.len() < rows.len();
            let non_null: Vec<&Value> = values
                .into_iter()
                .filter(|value| !value.is_null())
                .collect();
            let value_type = if !non_null.is_empty()
                && non_null.iter().all(|value| {
                    value.is_number()
                        || value
                            .as_str()
                            .and_then(|text| text.trim().parse::<f64>().ok())
                            .is_some_and(f64::is_finite)
                }) {
                "number"
            } else if !non_null.is_empty()
                && non_null.iter().all(|value| {
                    value.as_str().is_some_and(|text| {
                        chrono::NaiveDate::parse_from_str(
                            text.get(..10).unwrap_or(text),
                            "%Y-%m-%d",
                        )
                        .is_ok()
                    })
                })
            {
                "date"
            } else {
                "text"
            };
            json!({ "name": name, "type": value_type, "nullable": nullable })
        })
        .collect()
}

pub fn source_hash(source: &PluginDataSource) -> String {
    let bytes = serde_json::to_vec(source).unwrap_or_default();
    format!("{:x}", Sha256::digest(bytes))
}

pub fn stable_builder_code(
    prefix: &str,
    chat_id: &str,
    title: &str,
    source: &PluginDataSource,
) -> String {
    let seed = format!("{chat_id}\0{title}\0{}", source_hash(source));
    let digest = format!("{:x}", Sha256::digest(seed.as_bytes()));
    format!("{prefix}-{}", &digest[..12]).to_ascii_uppercase()
}

fn schema_aggregate(value: PluginSchemaAggregate) -> SchemaAggregate {
    match value {
        PluginSchemaAggregate::Sum => SchemaAggregate::Sum,
        PluginSchemaAggregate::Count => SchemaAggregate::Count,
        PluginSchemaAggregate::Avg => SchemaAggregate::Avg,
        PluginSchemaAggregate::Min => SchemaAggregate::Min,
        PluginSchemaAggregate::Max => SchemaAggregate::Max,
    }
}

fn schema_filter_operator(value: PluginSchemaFilterOperator) -> SchemaFilterOperator {
    match value {
        PluginSchemaFilterOperator::Eq => SchemaFilterOperator::Eq,
        PluginSchemaFilterOperator::NotEq => SchemaFilterOperator::NotEq,
        PluginSchemaFilterOperator::Lt => SchemaFilterOperator::Lt,
        PluginSchemaFilterOperator::Lte => SchemaFilterOperator::Lte,
        PluginSchemaFilterOperator::Gt => SchemaFilterOperator::Gt,
        PluginSchemaFilterOperator::Gte => SchemaFilterOperator::Gte,
        PluginSchemaFilterOperator::Between => SchemaFilterOperator::Between,
        PluginSchemaFilterOperator::In => SchemaFilterOperator::In,
        PluginSchemaFilterOperator::NotIn => SchemaFilterOperator::NotIn,
        PluginSchemaFilterOperator::Contains => SchemaFilterOperator::Contains,
        PluginSchemaFilterOperator::IsNull => SchemaFilterOperator::IsNull,
        PluginSchemaFilterOperator::IsNotNull => SchemaFilterOperator::IsNotNull,
    }
}

fn schema_sort_direction(value: PluginSchemaSortDirection) -> SchemaSortDirection {
    match value {
        PluginSchemaSortDirection::Asc => SchemaSortDirection::Asc,
        PluginSchemaSortDirection::Desc => SchemaSortDirection::Desc,
    }
}

pub async fn execute_source(
    source: &PluginDataSource,
    limit: usize,
) -> Result<TabularResult, String> {
    execute_source_with_context(source, limit, None).await
}

pub async fn execute_source_with_context(
    source: &PluginDataSource,
    limit: usize,
    context: Option<&PluginRunContext>,
) -> Result<TabularResult, String> {
    let limit = limit.clamp(1, TABLE_ROW_LIMIT);
    let provided_context = context.cloned().unwrap_or_default();
    let run_context = effective_source_context(context);
    let mut result = match source {
        PluginDataSource::Schema {
            schema_id,
            fields,
            group_by,
            metrics,
            filters,
            sort,
        } => {
            query_schema(SchemaQueryRequest {
                schema_id: schema_id.clone(),
                fields: fields.clone(),
                group_by: group_by.clone(),
                metrics: metrics
                    .iter()
                    .map(|metric| SchemaMetric {
                        field_id: metric.field_id.clone(),
                        aggregate: schema_aggregate(metric.aggregate),
                    })
                    .collect(),
                filters: filters
                    .iter()
                    .map(|filter| -> Result<SchemaFilter, String> {
                        Ok(SchemaFilter {
                            field_id: filter.field_id.clone(),
                            operator: schema_filter_operator(filter.operator),
                            value: filter
                                .value
                                .as_ref()
                                .map(|value| resolve_context_value(value, &run_context))
                                .transpose()?,
                            values: filter
                                .values
                                .iter()
                                .map(|value| resolve_context_value(value, &run_context))
                                .collect::<Result<Vec<_>, _>>()?,
                            from: filter
                                .from
                                .as_ref()
                                .map(|value| resolve_context_value(value, &run_context))
                                .transpose()?,
                            to: filter
                                .to
                                .as_ref()
                                .map(|value| resolve_context_value(value, &run_context))
                                .transpose()?,
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                sort: sort
                    .iter()
                    .map(|rule| SchemaSortRule {
                        field_id: rule.field_id.clone(),
                        direction: schema_sort_direction(rule.direction),
                    })
                    .collect(),
                limit: Some(limit),
            })
            .await?
        }
        PluginDataSource::Sql { sql, params } => {
            execute_raw_query(
                RawQueryRequest {
                    sql: sql.clone(),
                    params: params
                        .iter()
                        .map(|value| resolve_context_value(value, &run_context))
                        .collect::<Result<Vec<_>, _>>()?,
                    limit: Some(limit),
                },
                SqlAccessProfile::Analytics,
            )
            .await?
        }
        PluginDataSource::Dataview {
            view_id,
            metric_ids,
            group_by,
            context: source_context,
        } => {
            let response = run_data_view_drilldown(DataViewDrilldownRequest {
                view_id: view_id.clone(),
                group_by: group_by.clone(),
                metric_ids: metric_ids.clone(),
                context: DataViewContextRequest {
                    date_from: provided_context
                        .date_from
                        .clone()
                        .unwrap_or_else(|| source_context.date_from.clone()),
                    date_to: provided_context
                        .date_to
                        .clone()
                        .unwrap_or_else(|| source_context.date_to.clone()),
                    period2_from: source_context.period2_from.clone(),
                    period2_to: source_context.period2_to.clone(),
                    connection_mp_refs: if provided_context.connection_mp_refs.is_empty() {
                        source_context.connection_mp_refs.clone()
                    } else {
                        provided_context.connection_mp_refs.clone()
                    },
                    params: {
                        let mut params = source_context.params.clone();
                        params.extend(provided_context.params.clone());
                        params
                    },
                },
            })
            .await?;
            let truncated = response.rows.len() > limit;
            let mut rows = Vec::with_capacity(response.rows.len().min(limit));
            for row in response.rows.into_iter().take(limit) {
                let mut object = serde_json::Map::new();
                object.insert("group_key".into(), json!(row.group_key));
                object.insert("label".into(), json!(row.label));
                if row.metric_values.is_empty() {
                    object.insert("value1".into(), json!(row.value1));
                    object.insert("value2".into(), json!(row.value2));
                    object.insert("delta_pct".into(), json!(row.delta_pct));
                } else {
                    for (metric, values) in row.metric_values {
                        object.insert(metric.clone(), json!(values.value1));
                        object.insert(format!("{metric}_period2"), json!(values.value2));
                        object.insert(format!("{metric}_delta_pct"), json!(values.delta_pct));
                    }
                }
                rows.push(Value::Object(object));
            }
            let columns = rows
                .first()
                .and_then(Value::as_object)
                .map(|row| row.keys().cloned().collect())
                .unwrap_or_default();
            TabularResult {
                source: DataSourceRef {
                    kind: DataSourceKind::Dataview,
                    id: view_id.clone(),
                },
                row_count: rows.len(),
                rows,
                columns,
                truncated,
                generated_sql: None,
            }
        }
    };
    normalize_tabular(&mut result)?;
    Ok(result)
}

fn normalize_tabular(result: &mut TabularResult) -> Result<(), String> {
    let inferred = infer_columns(&result.rows, &result.columns);
    let numeric: Vec<String> = inferred
        .iter()
        .filter(|column| column.get("type").and_then(Value::as_str) == Some("number"))
        .filter_map(|column| {
            column
                .get("name")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .collect();
    for row in &mut result.rows {
        let Some(object) = row.as_object_mut() else {
            return Err("data source returned a non-object row".to_string());
        };
        for field in &numeric {
            let Some(Value::String(text)) = object.get(field) else {
                continue;
            };
            let parsed = text
                .trim()
                .parse::<f64>()
                .map_err(|_| format!("numeric normalization failed for column '{field}'"))?;
            let number = serde_json::Number::from_f64(parsed)
                .ok_or_else(|| format!("non-finite number in column '{field}'"))?;
            object.insert(field.clone(), Value::Number(number));
        }
    }
    Ok(())
}

pub fn validate_snapshot_payload(
    value: &Value,
    row_limit: usize,
) -> Result<(usize, usize), String> {
    let rows = value
        .as_array()
        .ok_or_else(|| "snapshot payload must be an array of rows".to_string())?;
    if rows.len() > row_limit {
        return Err(format!(
            "snapshot_limit_exceeded: {} rows, limit is {}",
            rows.len(),
            row_limit
        ));
    }
    let size = serde_json::to_vec(value)
        .map_err(|error| format!("snapshot serialization failed: {error}"))?
        .len();
    if size > SNAPSHOT_MAX_BYTES {
        return Err(format!(
            "snapshot_limit_exceeded: {} bytes, limit is {}",
            size, SNAPSHOT_MAX_BYTES
        ));
    }
    Ok((rows.len(), size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_row_limits_are_strict_and_never_truncate() {
        let chart = Value::Array((0..CHART_ROW_LIMIT).map(|i| json!({ "v": i })).collect());
        assert_eq!(
            validate_snapshot_payload(&chart, CHART_ROW_LIMIT)
                .expect("chart boundary")
                .0,
            CHART_ROW_LIMIT
        );
        let too_many = Value::Array((0..=CHART_ROW_LIMIT).map(|i| json!({ "v": i })).collect());
        assert!(validate_snapshot_payload(&too_many, CHART_ROW_LIMIT)
            .unwrap_err()
            .contains("snapshot_limit_exceeded"));

        let table = Value::Array((0..TABLE_ROW_LIMIT).map(|_| json!({})).collect());
        assert_eq!(
            validate_snapshot_payload(&table, TABLE_ROW_LIMIT)
                .expect("table boundary")
                .0,
            TABLE_ROW_LIMIT
        );
    }

    #[test]
    fn snapshot_size_limit_is_strict() {
        let payload = json!([{ "value": "x".repeat(SNAPSHOT_MAX_BYTES) }]);
        let error = validate_snapshot_payload(&payload, CHART_ROW_LIMIT).unwrap_err();
        assert!(error.contains("snapshot_limit_exceeded"));
        assert!(error.contains("bytes"));
    }

    #[test]
    fn source_hash_is_stable_and_sensitive_to_params() {
        let first = PluginDataSource::Sql {
            sql: "SELECT ? AS v".into(),
            params: vec![json!(1)],
        };
        let same = first.clone();
        let changed = PluginDataSource::Sql {
            sql: "SELECT ? AS v".into(),
            params: vec![json!(2)],
        };
        assert_eq!(source_hash(&first), source_hash(&same));
        assert_ne!(source_hash(&first), source_hash(&changed));
    }

    #[test]
    fn resolves_period_and_named_context_bindings_without_string_rewrites() {
        let mut context = PluginRunContext {
            date_from: Some("2026-06-01".into()),
            date_to: Some("2026-06-30".into()),
            ..PluginRunContext::default()
        };
        context.params.insert("channel".into(), "WB".into());
        assert_eq!(
            resolve_context_value(&json!("$context.date_from"), &context).unwrap(),
            json!("2026-06-01")
        );
        assert_eq!(
            resolve_context_value(&json!("$context.params.channel"), &context).unwrap(),
            json!("WB")
        );
        assert_eq!(
            resolve_context_value(&json!(7), &context).unwrap(),
            json!(7)
        );
    }

    #[test]
    fn detects_interactive_period_sources() {
        let source = PluginDataSource::Sql {
            sql: "SELECT 1 WHERE ? <= ?".into(),
            params: vec![json!("$context.date_from"), json!("$context.date_to")],
        };
        assert!(source_uses_period_context(&source));
    }
}
