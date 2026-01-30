#![allow(deprecated)]
#![allow(unreachable_patterns)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::condition::FilterCondition;
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
    /// Sorting configuration
    #[serde(default)]
    pub sort: DashboardSort,
    /// Fields that are enabled (checked) in the UI - only these are considered
    #[serde(default)]
    pub enabled_fields: Vec<String>,
}

/// A field selected for display with optional aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedField {
    /// Field identifier
    pub field_id: String,
    /// Aggregation function (for numeric fields)
    pub aggregate: Option<AggregateFunction>,
}

/// Sort configuration for dashboard results
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DashboardSort {
    /// Sorting rules (applied in order)
    #[serde(default)]
    pub rules: Vec<SortRule>,
}

/// Single sorting rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortRule {
    /// Field to sort by
    pub field_id: String,
    /// Sort direction
    pub direction: SortDirection,
}

/// Sort direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortDirection {
    Asc,
    Desc,
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

/// Old filter structure (deprecated, kept for backward compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[deprecated(note = "Use FilterCondition instead")]
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
#[allow(deprecated)]
#[allow(unreachable_patterns)]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DashboardFilters {
    /// Start date filter (YYYY-MM-DD)
    /// @deprecated Use conditions with DatePeriod instead
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[deprecated(note = "Use conditions with ConditionDef::DatePeriod instead")]
    pub date_from: Option<String>,
    /// End date filter (YYYY-MM-DD)
    /// @deprecated Use conditions with DatePeriod instead
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[deprecated(note = "Use conditions with ConditionDef::DatePeriod instead")]
    pub date_to: Option<String>,
    /// Dimension filters: field_id -> list of allowed values (legacy)
    /// @deprecated Use conditions with InList instead
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    #[deprecated(note = "Use conditions with ConditionDef::InList instead")]
    pub dimensions: HashMap<String, Vec<String>>,
    /// Filter conditions (new format)
    #[serde(default, alias = "field_filters")]
    pub conditions: Vec<FilterCondition>,
    /// Old field filters (deprecated, for backward compatibility)
    #[allow(deprecated)]
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    #[deprecated(note = "Use conditions instead")]
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
