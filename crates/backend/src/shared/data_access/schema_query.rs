use super::catalog::{DataSourceKind, DataSourceRef};
use super::sql_guard::wrap_limited_sql;
use super::TabularResult;
use crate::shared::data::db::get_connection;
use crate::shared::universal_dashboard::{get_registry, QueryBuilder, QueryParam};
use contracts::shared::universal_dashboard::{
    AggregateFunction, ComparisonOp, ConditionDef, DashboardConfig, DashboardFilters,
    DashboardSort, FilterCondition, SelectedField, SortDirection, SortRule,
};
use sea_orm::{DatabaseBackend, FromQueryResult, Statement, Value as DbValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::time::{Duration, Instant};

const DEFAULT_LIMIT: usize = 50;
const MAX_LIMIT: usize = 200;
const QUERY_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaAggregate {
    Sum,
    Count,
    Avg,
    Min,
    Max,
}

impl From<SchemaAggregate> for AggregateFunction {
    fn from(value: SchemaAggregate) -> Self {
        match value {
            SchemaAggregate::Sum => Self::Sum,
            SchemaAggregate::Count => Self::Count,
            SchemaAggregate::Avg => Self::Avg,
            SchemaAggregate::Min => Self::Min,
            SchemaAggregate::Max => Self::Max,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaMetric {
    pub field_id: String,
    pub aggregate: SchemaAggregate,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaFilterOperator {
    Eq,
    NotEq,
    Lt,
    Lte,
    Gt,
    Gte,
    Between,
    In,
    NotIn,
    Contains,
    IsNull,
    IsNotNull,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaFilter {
    pub field_id: String,
    pub operator: SchemaFilterOperator,
    #[serde(default)]
    pub value: Option<Value>,
    #[serde(default)]
    pub values: Vec<Value>,
    #[serde(default)]
    pub from: Option<Value>,
    #[serde(default)]
    pub to: Option<Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaSortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaSortRule {
    pub field_id: String,
    pub direction: SchemaSortDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaQueryRequest {
    pub schema_id: String,
    #[serde(default)]
    pub fields: Vec<String>,
    #[serde(default)]
    pub group_by: Vec<String>,
    #[serde(default)]
    pub metrics: Vec<SchemaMetric>,
    #[serde(default)]
    pub filters: Vec<SchemaFilter>,
    #[serde(default)]
    pub sort: Vec<SchemaSortRule>,
    #[serde(default)]
    pub limit: Option<usize>,
}

fn scalar_to_string(value: &Value) -> Result<String, String> {
    match value {
        Value::String(value) => Ok(value.clone()),
        Value::Number(value) => Ok(value.to_string()),
        Value::Bool(value) => Ok(value.to_string()),
        _ => Err("Filter values must be scalar strings, numbers, or booleans".to_string()),
    }
}

fn required_value(filter: &SchemaFilter) -> Result<String, String> {
    filter
        .value
        .as_ref()
        .ok_or_else(|| format!("Filter '{}' requires value", filter.field_id))
        .and_then(scalar_to_string)
}

fn filter_definition(filter: &SchemaFilter) -> Result<ConditionDef, String> {
    let comparison = |operator| {
        Ok(ConditionDef::Comparison {
            operator,
            value: required_value(filter)?,
        })
    };
    match filter.operator {
        SchemaFilterOperator::Eq => comparison(ComparisonOp::Eq),
        SchemaFilterOperator::NotEq => comparison(ComparisonOp::NotEq),
        SchemaFilterOperator::Lt => comparison(ComparisonOp::Lt),
        SchemaFilterOperator::Lte => comparison(ComparisonOp::LtEq),
        SchemaFilterOperator::Gt => comparison(ComparisonOp::Gt),
        SchemaFilterOperator::Gte => comparison(ComparisonOp::GtEq),
        SchemaFilterOperator::Between => Ok(ConditionDef::Range {
            from: filter.from.as_ref().map(scalar_to_string).transpose()?,
            to: filter.to.as_ref().map(scalar_to_string).transpose()?,
        }),
        SchemaFilterOperator::In | SchemaFilterOperator::NotIn => {
            if filter.values.is_empty() {
                return Err(format!("Filter '{}' requires values", filter.field_id));
            }
            Ok(ConditionDef::InList {
                values: filter
                    .values
                    .iter()
                    .map(scalar_to_string)
                    .collect::<Result<Vec<_>, _>>()?,
                negated: matches!(filter.operator, SchemaFilterOperator::NotIn),
            })
        }
        SchemaFilterOperator::Contains => Ok(ConditionDef::Contains {
            pattern: required_value(filter)?,
        }),
        SchemaFilterOperator::IsNull | SchemaFilterOperator::IsNotNull => {
            Ok(ConditionDef::Nullability {
                is_null: matches!(filter.operator, SchemaFilterOperator::IsNull),
            })
        }
    }
}

fn query_param_to_db(value: &QueryParam) -> DbValue {
    match value {
        QueryParam::Text(value) => DbValue::String(Some(Box::new(value.clone()))),
        QueryParam::Integer(value) => DbValue::BigInt(Some(*value)),
        QueryParam::Numeric(value) => DbValue::Double(Some(*value)),
    }
}

pub async fn query_schema(request: SchemaQueryRequest) -> Result<TabularResult, String> {
    let started = Instant::now();
    let registry = get_registry();
    let schema = registry
        .get_schema(&request.schema_id)
        .ok_or_else(|| format!("Unknown data schema: {}", request.schema_id))?;
    let table = registry
        .get_table_name(&request.schema_id)
        .ok_or_else(|| format!("No table registered for schema: {}", request.schema_id))?;

    if request.fields.is_empty() && request.group_by.is_empty() && request.metrics.is_empty() {
        return Err("At least one field, group_by, or metric is required".to_string());
    }
    let limit = request.limit.unwrap_or(DEFAULT_LIMIT);
    if !(1..=MAX_LIMIT).contains(&limit) {
        return Err(format!("limit must be between 1 and {MAX_LIMIT}"));
    }

    let grouping_set: HashSet<&str> = request.group_by.iter().map(String::as_str).collect();
    let display_fields = request
        .fields
        .iter()
        .filter(|field| !grouping_set.contains(field.as_str()))
        .cloned()
        .collect();
    let selected_fields = request
        .metrics
        .iter()
        .map(|metric| SelectedField {
            field_id: metric.field_id.clone(),
            aggregate: Some(metric.aggregate.into()),
        })
        .collect();

    let mut conditions = Vec::new();
    for filter in &request.filters {
        let field = schema
            .fields
            .iter()
            .find(|field| field.id == filter.field_id)
            .ok_or_else(|| format!("Unknown field '{}'", filter.field_id))?;
        conditions.push(FilterCondition::new(
            filter.field_id.clone(),
            field.value_type.clone(),
            filter_definition(filter)?,
        ));
    }

    let config = DashboardConfig {
        data_source: request.schema_id.clone(),
        selected_fields,
        groupings: request.group_by,
        display_fields,
        filters: DashboardFilters {
            conditions,
            ..DashboardFilters::default()
        },
        sort: DashboardSort {
            rules: request
                .sort
                .into_iter()
                .map(|rule| SortRule {
                    field_id: rule.field_id,
                    direction: match rule.direction {
                        SchemaSortDirection::Asc => SortDirection::Asc,
                        SchemaSortDirection::Desc => SortDirection::Desc,
                    },
                })
                .collect(),
        },
        enabled_fields: Vec::new(),
    };

    let built = QueryBuilder::new(&schema, &config, table)
        .build()
        .map_err(|error| format!("Schema query validation failed: {error}"))?;
    let sql = wrap_limited_sql(&built.sql, limit + 1, "semantic_limited_result");
    let params: Vec<DbValue> = built.params.iter().map(query_param_to_db).collect();
    let statement = Statement::from_sql_and_values(DatabaseBackend::Sqlite, sql, params);
    let rows_future = Value::find_by_statement(statement).all(get_connection());
    let mut rows = tokio::time::timeout(QUERY_TIMEOUT, rows_future)
        .await
        .map_err(|_| "Schema query timed out after 10 seconds".to_string())?
        .map_err(|error| format!("Schema query failed: {error}"))?;
    let truncated = rows.len() > limit;
    if truncated {
        rows.truncate(limit);
    }
    let columns = rows
        .first()
        .and_then(Value::as_object)
        .map(|row| row.keys().cloned().collect())
        .unwrap_or_default();
    let result = TabularResult {
        source: DataSourceRef {
            kind: DataSourceKind::Base,
            id: schema.id,
        },
        row_count: rows.len(),
        rows,
        columns,
        truncated,
    };
    tracing::info!(
        source_kind = "base",
        source_id = %result.source.id,
        elapsed_ms = started.elapsed().as_millis(),
        row_count = result.row_count,
        truncated = result.truncated,
        "semantic data query completed"
    );
    Ok(result)
}
