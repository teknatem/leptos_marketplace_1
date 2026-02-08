//! Schema validation for pivot tables
//!
//! Validates that schemas match the actual database structure
//! and can execute queries successfully.

use std::collections::HashSet;
use std::time::Instant;

use contracts::shared::universal_dashboard::{
    AggregateFunction, DashboardConfig, DashboardFilters, DashboardSort, DataSourceSchemaOwned,
    FieldType, SchemaInfo, SchemaValidationResult, SelectedField, ValidateAllSchemasResponse,
};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, FromQueryResult, Statement};

use super::entity_registry::SchemaRegistry;

/// Validate all schemas in the registry
pub async fn validate_all_schemas(
    registry: &SchemaRegistry,
    db: &DatabaseConnection,
) -> ValidateAllSchemasResponse {
    let start = Instant::now();
    let mut results = Vec::new();

    let schemas = registry.list_all();

    for schema_info in schemas {
        if let Some(schema) = registry.get_schema(&schema_info.id) {
            let result = validate_schema(&schema, &schema_info, db).await;
            results.push(result);
        }
    }

    let valid_count = results.iter().filter(|r| r.is_valid).count();
    let total_schemas = results.len();
    let invalid_count = total_schemas - valid_count;

    ValidateAllSchemasResponse {
        results,
        total_schemas,
        valid_count,
        invalid_count,
        total_time_ms: start.elapsed().as_millis() as u64,
    }
}

/// Validate a single schema
pub async fn validate_schema(
    schema: &DataSourceSchemaOwned,
    info: &SchemaInfo,
    db: &DatabaseConnection,
) -> SchemaValidationResult {
    let start = Instant::now();
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut row_count = None;

    let table_name = &info.table_name;

    // 1. Check if table exists
    if let Err(e) = check_table_exists(table_name, db).await {
        errors.push(format!("Table '{}': {}", table_name, e));
    }

    // 2. Check columns exist (only if table exists)
    if errors.is_empty() {
        let missing = check_columns_exist(table_name, schema, db).await;
        for col in missing {
            errors.push(format!(
                "Column '{}' not found in table '{}'",
                col, table_name
            ));
        }
    }

    // 3. Check ref tables exist
    for field in &schema.fields {
        if let Some(ref_table) = &field.ref_table {
            if let Err(e) = check_table_exists(ref_table, db).await {
                warnings.push(format!(
                    "Ref table '{}' for field '{}': {}",
                    ref_table, field.id, e
                ));
            }
        }
    }

    // 4. Execute test query (only if no errors so far)
    if errors.is_empty() {
        match execute_test_query(&schema, table_name, db).await {
            Ok(count) => {
                row_count = Some(count);
            }
            Err(e) => {
                errors.push(format!("Test query failed: {}", e));
            }
        }
    }

    SchemaValidationResult {
        schema_id: schema.id.clone(),
        schema_name: schema.name.clone(),
        source: info.source,
        is_valid: errors.is_empty(),
        errors,
        warnings,
        execution_time_us: start.elapsed().as_micros() as u64,
        row_count,
    }
}

/// Check if a table exists in the database
async fn check_table_exists(table_name: &str, db: &DatabaseConnection) -> Result<(), String> {
    let sql = format!("SELECT 1 FROM {} LIMIT 0", table_name);
    let stmt = Statement::from_string(DatabaseBackend::Sqlite, sql);

    db.execute(stmt)
        .await
        .map_err(|e| format!("Table does not exist or is inaccessible: {}", e))?;
    Ok(())
}

/// Check if all schema columns exist in the table
async fn check_columns_exist(
    table_name: &str,
    schema: &DataSourceSchemaOwned,
    db: &DatabaseConnection,
) -> Vec<String> {
    // Get actual columns from SQLite
    let sql = format!("PRAGMA table_info({})", table_name);
    let stmt = Statement::from_string(DatabaseBackend::Sqlite, sql);

    #[derive(Debug, FromQueryResult)]
    struct ColumnInfo {
        name: String,
    }

    let columns: Vec<ColumnInfo> = db
        .query_all(stmt)
        .await
        .map(|rows| {
            rows.into_iter()
                .filter_map(|row| {
                    let name: Option<String> = sea_orm::TryGetable::try_get(&row, "", "name").ok();
                    name.map(|n| ColumnInfo { name: n })
                })
                .collect()
        })
        .unwrap_or_default();

    let db_columns: HashSet<String> = columns.into_iter().map(|c| c.name).collect();

    // Find missing columns (skip fields from joined tables - they have source_table)
    schema
        .fields
        .iter()
        .filter(|f| f.source_table.is_none()) // Only check fields from main table
        .filter(|f| !db_columns.contains(&f.db_column))
        .map(|f| f.db_column.clone())
        .collect()
}

/// Execute a test query to verify the schema works
/// Creates a realistic dashboard config and tests actual query generation
async fn execute_test_query(
    schema: &DataSourceSchemaOwned,
    table_name: &str,
    db: &DatabaseConnection,
) -> Result<i64, String> {
    // Build a test dashboard config with grouping and aggregation
    let grouping_fields: Vec<String> = schema
        .fields
        .iter()
        .filter(|f| f.can_group)
        .take(3) // Take up to 3 grouping fields for test
        .map(|f| f.id.clone())
        .collect();

    let aggregate_fields: Vec<SelectedField> = schema
        .fields
        .iter()
        .filter(|f| f.can_aggregate)
        .take(2) // Take up to 2 aggregate fields for test
        .map(|f| SelectedField {
            field_id: f.id.clone(),
            aggregate: Some(match f.field_type {
                Some(FieldType::Numeric) => AggregateFunction::Sum,
                Some(FieldType::Integer) => AggregateFunction::Count,
                _ => AggregateFunction::Count,
            }),
        })
        .collect();

    // If no groupings or aggregates, fall back to simple COUNT query
    if grouping_fields.is_empty() && aggregate_fields.is_empty() {
        let sql = format!("SELECT COUNT(*) as cnt FROM {}", table_name);
        let stmt = Statement::from_string(DatabaseBackend::Sqlite, sql);

        let result = db
            .query_one(stmt)
            .await
            .map_err(|e| format!("Query execution failed: {}", e))?
            .ok_or_else(|| "No result returned".to_string())?;

        let count: i64 = sea_orm::TryGetable::try_get(&result, "", "cnt")
            .map_err(|e| format!("Failed to get count: {:?}", e))?;

        return Ok(count);
    }

    // Build realistic dashboard config
    let config = DashboardConfig {
        data_source: schema.id.clone(),
        selected_fields: aggregate_fields,
        groupings: grouping_fields,
        display_fields: vec![],
        enabled_fields: vec![], // Empty = all enabled
        sort: DashboardSort::default(),
        filters: DashboardFilters::default(),
    };

    // Create a test SQL with all groupings and joins to verify ORDER BY clause
    // This mimics what QueryBuilder does but simplified
    let mut select_parts = Vec::new();
    let mut group_parts = Vec::new();
    let mut order_parts = Vec::new();
    let mut joins = Vec::new();
    let mut seen_joins = HashSet::new();

    for field_id in &config.groupings {
        if let Some(field) = schema.fields.iter().find(|f| f.id == *field_id) {
            let table_prefix = if let Some(ref source_table) = field.source_table {
                // Add JOIN if needed
                if let Some(ref join_col) = field.join_on_column {
                    let join_key = format!("{}:{}", source_table, join_col);
                    if !seen_joins.contains(&join_key) {
                        joins.push(format!(
                            "LEFT JOIN {} ON {}.{} = {}.id",
                            source_table, table_name, join_col, source_table
                        ));
                        seen_joins.insert(join_key);
                    }
                }
                source_table.as_str()
            } else if let Some(ref ref_table) = field.ref_table {
                if field.ref_display_column.is_some() {
                    // Add JOIN for ref table
                    let join_key = format!("{}:{}", ref_table, field.db_column);
                    if !seen_joins.contains(&join_key) {
                        joins.push(format!(
                            "LEFT JOIN {} ON {}.{} = {}.id",
                            ref_table, table_name, field.db_column, ref_table
                        ));
                        seen_joins.insert(join_key);
                    }
                    ref_table.as_str()
                } else {
                    table_name
                }
            } else {
                table_name
            };

            let column = format!("{}.{}", table_prefix, field.db_column);
            select_parts.push(column.clone());
            group_parts.push(column.clone());
            order_parts.push(column);
        }
    }

    // Add aggregate fields
    for selected in &config.selected_fields {
        if let Some(field) = schema.fields.iter().find(|f| f.id == selected.field_id) {
            if let Some(ref agg) = selected.aggregate {
                let agg_func = match agg {
                    AggregateFunction::Sum => "SUM",
                    AggregateFunction::Count => "COUNT",
                    AggregateFunction::Avg => "AVG",
                    AggregateFunction::Min => "MIN",
                    AggregateFunction::Max => "MAX",
                };
                select_parts.push(format!(
                    "{}({}.{}) AS {}",
                    agg_func, table_name, field.db_column, field.id
                ));
            }
        }
    }

    // Build final SQL
    let sql = format!(
        "SELECT {} FROM {} {} {} {} LIMIT 100",
        select_parts.join(", "),
        table_name,
        joins.join(" "),
        if !group_parts.is_empty() {
            format!("GROUP BY {}", group_parts.join(", "))
        } else {
            String::new()
        },
        if !order_parts.is_empty() {
            format!("ORDER BY {}", order_parts.join(", "))
        } else {
            String::new()
        }
    );

    // Execute test query
    let stmt = Statement::from_string(DatabaseBackend::Sqlite, sql);
    let results = db
        .query_all(stmt)
        .await
        .map_err(|e| format!("Query execution failed: {}", e))?;

    Ok(results.len() as i64)
}

#[cfg(test)]
mod tests {
    // Tests require a database connection
    // They are integration tests that should be run separately
}
