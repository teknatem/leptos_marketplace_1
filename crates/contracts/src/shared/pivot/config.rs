use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::schema::{AggregateFunction, FilterOperator};

/// Dashboard configuration selected by the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    /// Data source identifier (e.g., "p903_wb_finance_report")
    pub data_source: String,
    /// Selected fields to display/aggregate
    pub selected_fields: Vec<SelectedField>,
    /// Fields to group by (order matters for hierarchy)
    pub groupings: Vec<String>,
    /// Fields to display without grouping
    #[serde(default)]
    pub display_fields: Vec<String>,
    /// Filters to apply
    pub filters: DashboardFilters,
}

/// A field selected for display with optional aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedField {
    /// Field identifier
    pub field_id: String,
    /// Aggregation function (for numeric fields)
    pub aggregate: Option<AggregateFunction>,
}

/// Role of a field in the dashboard configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldRole {
    /// Field is not used
    None,
    /// Field is used for grouping (GROUP BY)
    Grouping,
    /// Field is aggregated as a measure
    Measure,
    /// Field is displayed without grouping
    Display,
}

/// Filter condition for a specific field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldFilter {
    /// Field identifier
    pub field_id: String,
    /// Filter operator
    pub operator: FilterOperator,
    /// Filter value
    pub value: String,
    /// Second value (for BETWEEN operator)
    pub value2: Option<String>,
}

/// Filters to apply to the dashboard query
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DashboardFilters {
    /// Start date filter (YYYY-MM-DD)
    pub date_from: Option<String>,
    /// End date filter (YYYY-MM-DD)
    pub date_to: Option<String>,
    /// Dimension filters: field_id -> list of allowed values
    pub dimensions: HashMap<String, Vec<String>>,
    /// Field-specific filters with operators
    pub field_filters: Vec<FieldFilter>,
}

/// Saved dashboard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedDashboardConfig {
    /// Unique identifier (UUID)
    pub id: String,
    /// User-defined name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Data source identifier
    pub data_source: String,
    /// Serialized configuration
    pub config: DashboardConfig,
    /// Creation timestamp
    pub created_at: String,
    /// Last update timestamp
    pub updated_at: String,
}

/// Request to execute a dashboard query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteDashboardRequest {
    /// Configuration to execute
    pub config: DashboardConfig,
}

/// Request to save a dashboard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveDashboardConfigRequest {
    /// Configuration name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Configuration to save
    pub config: DashboardConfig,
}

/// Request to update a dashboard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDashboardConfigRequest {
    /// Configuration ID
    pub id: String,
    /// New name
    pub name: String,
    /// New description
    pub description: Option<String>,
    /// Updated configuration
    pub config: DashboardConfig,
}
