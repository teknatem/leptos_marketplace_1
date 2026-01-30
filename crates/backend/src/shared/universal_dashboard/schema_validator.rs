//! Schema validation for pivot tables
//!
//! Validates that schemas match the actual database structure
//! and can execute queries successfully.

use std::collections::HashSet;
use std::time::Instant;

use contracts::shared::universal_dashboard::{
    DataSourceSchemaOwned, SchemaInfo, SchemaValidationResult,
    ValidateAllSchemasResponse,
};
use sea_orm::{ConnectionTrait, DatabaseConnection, DatabaseBackend, FromQueryResult, Statement};

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
        match execute_test_query(table_name, db).await {
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
        execution_time_ms: start.elapsed().as_millis() as u64,
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

    // Find missing columns
    schema
        .fields
        .iter()
        .filter(|f| !db_columns.contains(&f.db_column))
        .map(|f| f.db_column.clone())
        .collect()
}

/// Execute a simple test query to verify the schema works
async fn execute_test_query(table_name: &str, db: &DatabaseConnection) -> Result<i64, String> {
    let sql = format!("SELECT COUNT(*) as cnt FROM {}", table_name);
    let stmt = Statement::from_string(DatabaseBackend::Sqlite, sql);

    let result = db
        .query_one(stmt)
        .await
        .map_err(|e| format!("Query execution failed: {}", e))?
        .ok_or_else(|| "No result returned".to_string())?;

    let count: i64 = sea_orm::TryGetable::try_get(&result, "", "cnt")
        .map_err(|e| format!("Failed to get count: {:?}", e))?;

    Ok(count)
}

#[cfg(test)]
mod tests {
    // Tests require a database connection
    // They are integration tests that should be run separately
}
