use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::schema::{DataSourceSchemaOwned, SchemaInfo, SchemaSource};

/// Response from executing a dashboard query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteDashboardResponse {
    /// Data source that was queried
    pub data_source: String,
    /// Column headers (grouping fields + aggregated fields)
    pub columns: Vec<ColumnHeader>,
    /// Hierarchical data rows
    pub rows: Vec<PivotRow>,
}

/// Column header information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnHeader {
    /// Column identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Column type
    pub column_type: ColumnType,
}

/// Type of column in the pivot table
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColumnType {
    /// Grouping column
    Grouping,
    /// Aggregated numeric column
    Aggregated,
}

/// A single row in the pivot table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PivotRow {
    /// Hierarchy level (0 = grand total, 1+ = grouped levels)
    pub level: usize,
    /// Values by column ID
    pub values: HashMap<String, CellValue>,
    /// Whether this row is a subtotal/total row
    pub is_total: bool,
    /// Child rows (for hierarchical display)
    pub children: Vec<PivotRow>,
}

/// Value in a pivot table cell
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CellValue {
    /// Text value
    Text(String),
    /// Numeric value
    Number(f64),
    /// Integer value
    Integer(i64),
    /// Null value
    Null,
}

/// Response listing available data source schemas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListSchemasResponse {
    /// Available schemas
    pub schemas: Vec<SchemaInfo>,
}

/// Response with full schema details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetSchemaResponse {
    /// Full schema definition
    pub schema: DataSourceSchemaOwned,
}

/// Response listing saved dashboard configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListDashboardConfigsResponse {
    /// List of saved configurations
    pub configs: Vec<SavedDashboardConfigSummary>,
}

/// Summary of a saved dashboard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedDashboardConfigSummary {
    /// Configuration ID
    pub id: String,
    /// Configuration name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Data source identifier
    pub data_source: String,
    /// Creation timestamp
    pub created_at: String,
    /// Last update timestamp
    pub updated_at: String,
}

/// Response after saving a configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveDashboardConfigResponse {
    /// Saved configuration ID
    pub id: String,
    /// Success message
    pub message: String,
}

/// Response after deleting a configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteDashboardConfigResponse {
    /// Success flag
    pub success: bool,
    /// Response message
    pub message: String,
}

/// A distinct value with display representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistinctValue {
    /// Actual value (UUID for refs, or the value itself)
    pub value: String,
    /// Display representation (description for refs, or same as value)
    pub display: String,
}

/// Response with distinct values for a field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistinctValuesResponse {
    /// Field identifier
    pub field_id: String,
    /// List of distinct values with display
    pub values: Vec<DistinctValue>,
}

/// Response with generated SQL query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateSqlResponse {
    /// Generated SQL query
    pub sql: String,
    /// Query parameters preview (for display)
    pub params: Vec<String>,
}

// ============================================================================
// Schema Validation Types
// ============================================================================

/// Result of validating a single schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaValidationResult {
    /// Schema identifier
    pub schema_id: String,
    /// Schema display name
    pub schema_name: String,
    /// Source of the schema (Auto/Custom)
    pub source: SchemaSource,
    /// Whether the schema is valid
    pub is_valid: bool,
    /// List of errors (empty if valid)
    pub errors: Vec<String>,
    /// List of warnings (non-blocking issues)
    pub warnings: Vec<String>,
    /// Time taken to validate in microseconds (for precision)
    pub execution_time_us: u64,
    /// Number of rows in the table (from test query)
    pub row_count: Option<i64>,
}

/// Response from validating all schemas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateAllSchemasResponse {
    /// Individual validation results
    pub results: Vec<SchemaValidationResult>,
    /// Total number of schemas validated
    pub total_schemas: usize,
    /// Number of valid schemas
    pub valid_count: usize,
    /// Number of invalid schemas
    pub invalid_count: usize,
    /// Total validation time in milliseconds
    pub total_time_ms: u64,
}
