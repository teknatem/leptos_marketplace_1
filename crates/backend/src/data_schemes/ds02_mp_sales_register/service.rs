#![allow(deprecated)]

use anyhow::Result;
use contracts::shared::universal_dashboard::{
    CellValue, ColumnHeader, ColumnType, DashboardConfig, DistinctValue, ExecuteDashboardResponse,
    GenerateSqlResponse, SaveDashboardConfigRequest, SavedDashboardConfig,
    SavedDashboardConfigSummary, UpdateDashboardConfigRequest,
};
use sea_orm::{ConnectionTrait, FromQueryResult, Statement};
use std::collections::HashMap;
use uuid::Uuid;

use crate::shared::data::db::get_connection;
use crate::shared::universal_dashboard::{
    get_registry, query_builder::QueryParam, QueryBuilder, RawRow, TreeBuilder,
};

use super::schema::DS02_SCHEMA;

/// Execute a dashboard query
pub async fn execute_dashboard(config: DashboardConfig) -> Result<ExecuteDashboardResponse> {
    // Support DS01 and DS02 schemas, with backward compatibility for old IDs
    let schema = match config.data_source.as_str() {
        "ds02_mp_sales_register" => &DS02_SCHEMA,
        // Backward compatibility
        "p900_sales_register" => &DS02_SCHEMA,
        "ds01_wb_finance_report" | "p903_wb_finance_report" | "s001_wb_finance" => {
            &crate::data_schemes::ds01_wb_finance_report::schema::DS01_SCHEMA
        }
        _ => {
            // Try registry for other data sources
            let registry = get_registry();
            if registry.has_schema(&config.data_source) {
                // Schema found but not yet supported for execution
                return Err(anyhow::anyhow!(
                    "Data source '{}' is registered but not yet supported for execution",
                    config.data_source
                ));
            } else {
                return Err(anyhow::anyhow!(
                    "Unknown data source: {}",
                    config.data_source
                ));
            }
        }
    };

    // Get table name from registry (with backward compatibility mapping)
    let registry = get_registry();
    let data_source_for_table = match config.data_source.as_str() {
        "p900_sales_register" => "ds02_mp_sales_register",
        "p903_wb_finance_report" | "s001_wb_finance" => "ds01_wb_finance_report",
        _ => config.data_source.as_str(),
    };
    let table_name = registry
        .get_table_name(data_source_for_table)
        .ok_or_else(|| {
            anyhow::anyhow!("Table name not found for schema: {}", data_source_for_table)
        })?;

    // Build SQL query
    let query_builder = QueryBuilder::new(schema, &config, table_name);
    let query_result = query_builder
        .build()
        .map_err(|e| anyhow::anyhow!("Query build error: {}", e))?;

    // Execute query
    let db = get_connection();
    let stmt = build_statement(&query_result.sql, &query_result.params);

    // Execute raw query and parse results manually
    let query_results = db.query_all(stmt).await?;

    // Transform results to RawRow format
    let raw_rows: Vec<RawRow> = query_results
        .into_iter()
        .map(|query_result| {
            let mut values = HashMap::new();

            // Parse grouping columns
            for grouping_id in config.groupings.iter() {
                let field = schema.fields.iter().find(|f| f.id == grouping_id).unwrap();

                let value = match field.field_type {
                    contracts::shared::universal_dashboard::FieldType::Text
                    | contracts::shared::universal_dashboard::FieldType::Date => {
                        // For ref fields, try to use _display column first
                        if field.ref_table.is_some() && field.ref_display_column.is_some() {
                            let display_col = format!("{}_display", grouping_id);
                            if let Ok(Some(display)) =
                                query_result.try_get::<Option<String>>("", &display_col)
                            {
                                CellValue::Text(display)
                            } else {
                                query_result
                                    .try_get::<Option<String>>("", grouping_id)
                                    .ok()
                                    .flatten()
                                    .map(CellValue::Text)
                                    .unwrap_or(CellValue::Null)
                            }
                        } else if field.ref_table.is_some() && field.ref_display_column.is_none() {
                            // For dimension fields (ref_table but no ref_display_column)
                            // The column comes from the joined table
                            query_result
                                .try_get::<Option<String>>("", grouping_id)
                                .ok()
                                .flatten()
                                .map(CellValue::Text)
                                .unwrap_or(CellValue::Null)
                        } else {
                            query_result
                                .try_get::<Option<String>>("", grouping_id)
                                .ok()
                                .flatten()
                                .map(CellValue::Text)
                                .unwrap_or(CellValue::Null)
                        }
                    }
                    contracts::shared::universal_dashboard::FieldType::Integer => query_result
                        .try_get::<Option<i64>>("", grouping_id)
                        .ok()
                        .flatten()
                        .map(CellValue::Integer)
                        .unwrap_or(CellValue::Null),
                    contracts::shared::universal_dashboard::FieldType::Numeric => query_result
                        .try_get::<Option<f64>>("", grouping_id)
                        .ok()
                        .flatten()
                        .map(CellValue::Number)
                        .unwrap_or(CellValue::Null),
                };
                values.insert(grouping_id.clone(), value);
            }

            // Parse aggregated columns
            for selected in &config.selected_fields {
                if let Some(_aggregate) = &selected.aggregate {
                    let value = query_result
                        .try_get::<Option<f64>>("", &selected.field_id)
                        .ok()
                        .flatten()
                        .map(CellValue::Number)
                        .unwrap_or(CellValue::Null);
                    values.insert(selected.field_id.clone(), value);
                }
            }

            RawRow { values }
        })
        .collect();

    // Build column headers
    let mut columns = Vec::new();

    // Add grouping columns
    for grouping_id in &config.groupings {
        let field = schema
            .fields
            .iter()
            .find(|f| f.id == grouping_id)
            .ok_or_else(|| anyhow::anyhow!("Field not found: {}", grouping_id))?;

        columns.push(ColumnHeader {
            id: field.id.to_string(),
            name: field.name.to_string(),
            column_type: ColumnType::Grouping,
        });
    }

    // Add aggregated columns
    for selected in &config.selected_fields {
        if let Some(aggregate) = &selected.aggregate {
            let field = schema
                .fields
                .iter()
                .find(|f| f.id == selected.field_id)
                .ok_or_else(|| anyhow::anyhow!("Field not found: {}", selected.field_id))?;

            columns.push(ColumnHeader {
                id: field.id.to_string(),
                name: format!("{} ({})", field.name, aggregate_name(aggregate)),
                column_type: ColumnType::Aggregated,
            });
        }
    }

    // Build pivot tree
    let grouping_columns: Vec<String> = config.groupings.clone();
    let aggregated_columns: Vec<String> = config
        .selected_fields
        .iter()
        .filter(|f| f.aggregate.is_some())
        .map(|f| f.field_id.clone())
        .collect();

    let tree_builder = TreeBuilder::new(grouping_columns, aggregated_columns);
    let rows = tree_builder.build(raw_rows);

    Ok(ExecuteDashboardResponse {
        data_source: config.data_source,
        columns,
        rows,
    })
}

/// Save a dashboard configuration
pub async fn save_dashboard_config(
    request: SaveDashboardConfigRequest,
) -> Result<SavedDashboardConfig> {
    let db = get_connection();
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let config_json = serde_json::to_string(&request.config)?;

    let sql = r#"
        INSERT INTO sys_dashboard_configs (id, name, description, data_source, config_json, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
    "#;

    let stmt = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Sqlite,
        sql,
        vec![
            id.clone().into(),
            request.name.clone().into(),
            request.description.clone().into(),
            request.config.data_source.clone().into(),
            config_json.clone().into(),
            now.clone().into(),
            now.clone().into(),
        ],
    );

    db.execute(stmt).await?;

    Ok(SavedDashboardConfig {
        id,
        name: request.name,
        description: request.description,
        data_source: request.config.data_source.clone(),
        config: request.config,
        created_at: now.clone(),
        updated_at: now,
    })
}

/// Update a dashboard configuration
pub async fn update_dashboard_config(
    request: UpdateDashboardConfigRequest,
) -> Result<SavedDashboardConfig> {
    let db = get_connection();
    let now = chrono::Utc::now().to_rfc3339();
    let config_json = serde_json::to_string(&request.config)?;

    let sql = r#"
        UPDATE sys_dashboard_configs
        SET name = ?, description = ?, config_json = ?, updated_at = ?
        WHERE id = ?
    "#;

    let stmt = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Sqlite,
        sql,
        vec![
            request.name.clone().into(),
            request.description.clone().into(),
            config_json.clone().into(),
            now.clone().into(),
            request.id.clone().into(),
        ],
    );

    db.execute(stmt).await?;

    Ok(SavedDashboardConfig {
        id: request.id,
        name: request.name,
        description: request.description,
        data_source: request.config.data_source.clone(),
        config: request.config,
        created_at: String::new(), // Not updated
        updated_at: now,
    })
}

/// Get a saved dashboard configuration
pub async fn get_dashboard_config(id: &str) -> Result<SavedDashboardConfig> {
    let db = get_connection();

    let sql = r#"
        SELECT id, name, description, data_source, config_json, created_at, updated_at
        FROM sys_dashboard_configs
        WHERE id = ?
    "#;

    let stmt =
        Statement::from_sql_and_values(sea_orm::DatabaseBackend::Sqlite, sql, vec![id.into()]);

    let row: SavedConfigRow = SavedConfigRow::find_by_statement(stmt)
        .one(db)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Config not found"))?;

    let config: DashboardConfig = serde_json::from_str(&row.config_json)?;

    Ok(SavedDashboardConfig {
        id: row.id,
        name: row.name,
        description: row.description,
        data_source: row.data_source,
        config,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

/// List all saved dashboard configurations
pub async fn list_dashboard_configs(
    data_source: Option<&str>,
) -> Result<Vec<SavedDashboardConfigSummary>> {
    let db = get_connection();

    let (sql, params) = if let Some(ds) = data_source {
        (
            r#"
                SELECT id, name, description, data_source, created_at, updated_at
                FROM sys_dashboard_configs
                WHERE data_source = ?
                ORDER BY updated_at DESC
            "#,
            vec![ds.into()],
        )
    } else {
        (
            r#"
                SELECT id, name, description, data_source, created_at, updated_at
                FROM sys_dashboard_configs
                ORDER BY updated_at DESC
            "#,
            vec![],
        )
    };

    let stmt = Statement::from_sql_and_values(sea_orm::DatabaseBackend::Sqlite, sql, params);

    let rows: Vec<SavedConfigSummaryRow> = SavedConfigSummaryRow::find_by_statement(stmt)
        .all(db)
        .await?;

    Ok(rows
        .into_iter()
        .map(|row| SavedDashboardConfigSummary {
            id: row.id,
            name: row.name,
            description: row.description,
            data_source: row.data_source,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
        .collect())
}

/// Delete a dashboard configuration
pub async fn delete_dashboard_config(id: &str) -> Result<()> {
    let db = get_connection();

    let sql = r#"
        DELETE FROM sys_dashboard_configs
        WHERE id = ?
    "#;

    let stmt =
        Statement::from_sql_and_values(sea_orm::DatabaseBackend::Sqlite, sql, vec![id.into()]);

    db.execute(stmt).await?;
    Ok(())
}

/// Generate SQL query without executing
pub async fn generate_sql(config: DashboardConfig) -> Result<GenerateSqlResponse> {
    // Support DS01 and DS02 schemas, with backward compatibility
    let schema = match config.data_source.as_str() {
        "ds02_mp_sales_register" => &DS02_SCHEMA,
        // Backward compatibility
        "p900_sales_register" => &DS02_SCHEMA,
        "ds01_wb_finance_report" | "p903_wb_finance_report" | "s001_wb_finance" => {
            &crate::data_schemes::ds01_wb_finance_report::schema::DS01_SCHEMA
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported data source: {}",
                config.data_source
            ));
        }
    };

    // Get table name from registry (with backward compatibility mapping)
    let registry = get_registry();
    let data_source_for_table = match config.data_source.as_str() {
        "p900_sales_register" => "ds02_mp_sales_register",
        "p903_wb_finance_report" | "s001_wb_finance" => "ds01_wb_finance_report",
        _ => config.data_source.as_str(),
    };
    let table_name = registry
        .get_table_name(data_source_for_table)
        .ok_or_else(|| {
            anyhow::anyhow!("Table name not found for schema: {}", data_source_for_table)
        })?;

    // Build SQL query
    let query_builder = QueryBuilder::new(schema, &config, table_name);
    let result = query_builder
        .build()
        .map_err(|e| anyhow::anyhow!("Query build error: {}", e))?;

    // Convert parameters to string representation for display
    let params: Vec<String> = result
        .params
        .iter()
        .map(|p| match p {
            QueryParam::Text(s) => format!("'{}'", s),
            QueryParam::Integer(i) => i.to_string(),
            QueryParam::Numeric(n) => n.to_string(),
        })
        .collect();

    Ok(GenerateSqlResponse {
        sql: result.sql,
        params,
    })
}

/// Get distinct values for a field
pub async fn get_distinct_values(
    schema_id: &str,
    field_id: &str,
    limit: Option<usize>,
) -> Result<Vec<DistinctValue>> {
    // Get schema and table name from registry
    let registry = get_registry();
    let schema = registry
        .get_schema(schema_id)
        .ok_or_else(|| anyhow::anyhow!("Schema not found: {}", schema_id))?;
    let main_table = registry
        .get_table_name(schema_id)
        .ok_or_else(|| anyhow::anyhow!("Table not found for schema: {}", schema_id))?;

    // Find field definition
    let field = schema
        .fields
        .iter()
        .find(|f| f.id == field_id)
        .ok_or_else(|| anyhow::anyhow!("Field not found: {}", field_id))?;

    let db = get_connection();
    let limit_clause = limit.map(|l| format!(" LIMIT {}", l)).unwrap_or_default();

    // Build query based on whether field has ref_table
    let sql = if let Some(ref_table) = &field.ref_table {
        // Field with reference - JOIN with ref table
        let ref_display_col = field.ref_display_column.as_deref().unwrap_or("description");
        format!(
            "SELECT DISTINCT {}.{} as value, {}.{} as display \
             FROM {} \
             LEFT JOIN {} ON {}.{} = {}.id \
             WHERE {}.{} IS NOT NULL \
             ORDER BY {}.{} {}",
            main_table,
            field.db_column,
            ref_table,
            ref_display_col,
            main_table,
            ref_table,
            main_table,
            field.db_column,
            ref_table,
            main_table,
            field.db_column,
            ref_table,
            ref_display_col,
            limit_clause
        )
    } else {
        // Regular field - value and display are the same
        format!(
            "SELECT DISTINCT {} as value, {} as display \
             FROM {} \
             WHERE {} IS NOT NULL \
             ORDER BY {} {}",
            field.db_column,
            field.db_column,
            main_table,
            field.db_column,
            field.db_column,
            limit_clause
        )
    };

    let stmt = Statement::from_string(sea_orm::DatabaseBackend::Sqlite, sql);
    let results = db.query_all(stmt).await?;

    // Parse results
    let values: Vec<DistinctValue> = results
        .into_iter()
        .filter_map(|row| {
            let value = row.try_get_by_index::<String>(0).ok()?;
            let display = row.try_get_by_index::<String>(1).ok()?;
            Some(DistinctValue { value, display })
        })
        .collect();

    Ok(values)
}

// Helper structures

#[derive(Debug, Clone, FromQueryResult)]
struct SavedConfigRow {
    id: String,
    name: String,
    description: Option<String>,
    data_source: String,
    config_json: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, FromQueryResult)]
struct SavedConfigSummaryRow {
    id: String,
    name: String,
    description: Option<String>,
    data_source: String,
    created_at: String,
    updated_at: String,
}

/// Build a Statement with parameters
fn build_statement(sql: &str, params: &[QueryParam]) -> Statement {
    let values: Vec<sea_orm::Value> = params
        .iter()
        .map(|p| match p {
            QueryParam::Text(s) => s.clone().into(),
            QueryParam::Integer(i) => (*i).into(),
            QueryParam::Numeric(n) => (*n).into(),
        })
        .collect();

    Statement::from_sql_and_values(sea_orm::DatabaseBackend::Sqlite, sql, values)
}

/// Get human-readable aggregate name
fn aggregate_name(agg: &contracts::shared::universal_dashboard::AggregateFunction) -> &'static str {
    match agg {
        contracts::shared::universal_dashboard::AggregateFunction::Sum => "Сумма",
        contracts::shared::universal_dashboard::AggregateFunction::Count => "Кол-во",
        contracts::shared::universal_dashboard::AggregateFunction::Avg => "Среднее",
        contracts::shared::universal_dashboard::AggregateFunction::Min => "Мин",
        contracts::shared::universal_dashboard::AggregateFunction::Max => "Макс",
    }
}
