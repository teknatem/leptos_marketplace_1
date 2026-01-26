use serde::{Deserialize, Serialize};

/// Data source schema definition for a dashboard (static version for backend)
#[derive(Debug, Clone)]
pub struct DataSourceSchema {
    /// Unique identifier for the data source (e.g., "p903_wb_finance_report")
    pub id: &'static str,
    /// Human-readable name (e.g., "WB Finance Report")
    pub name: &'static str,
    /// Available fields in this data source
    pub fields: &'static [FieldDef],
}

/// Owned version of DataSourceSchema for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceSchemaOwned {
    /// Unique identifier for the data source
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Available fields in this data source
    pub fields: Vec<FieldDefOwned>,
}

impl From<&DataSourceSchema> for DataSourceSchemaOwned {
    fn from(schema: &DataSourceSchema) -> Self {
        Self {
            id: schema.id.to_string(),
            name: schema.name.to_string(),
            fields: schema.fields.iter().map(|f| f.into()).collect(),
        }
    }
}

/// Definition of a single field in a data source (static version)
#[derive(Debug, Clone)]
pub struct FieldDef {
    /// Unique field identifier (e.g., "retail_amount")
    pub id: &'static str,
    /// Human-readable field name (e.g., "Сумма продаж")
    pub name: &'static str,
    /// Type of the field
    pub field_type: FieldType,
    /// Can this field be used in GROUP BY
    pub can_group: bool,
    /// Can this field be aggregated (SUM, AVG, etc.)
    pub can_aggregate: bool,
    /// Actual database column name
    pub db_column: &'static str,
    /// Reference table for JOIN (e.g., "a006_connection_mp")
    pub ref_table: Option<&'static str>,
    /// Reference display column (e.g., "description")
    pub ref_display_column: Option<&'static str>,
}

/// Owned version of FieldDef for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefOwned {
    /// Unique field identifier
    pub id: String,
    /// Human-readable field name
    pub name: String,
    /// Type of the field
    pub field_type: FieldType,
    /// Can this field be used in GROUP BY
    pub can_group: bool,
    /// Can this field be aggregated
    pub can_aggregate: bool,
    /// Actual database column name
    pub db_column: String,
    /// Reference table for JOIN
    pub ref_table: Option<String>,
    /// Reference display column
    pub ref_display_column: Option<String>,
}

impl From<&FieldDef> for FieldDefOwned {
    fn from(field: &FieldDef) -> Self {
        Self {
            id: field.id.to_string(),
            name: field.name.to_string(),
            field_type: field.field_type,
            can_group: field.can_group,
            can_aggregate: field.can_aggregate,
            db_column: field.db_column.to_string(),
            ref_table: field.ref_table.map(|s| s.to_string()),
            ref_display_column: field.ref_display_column.map(|s| s.to_string()),
        }
    }
}

/// Field type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldType {
    /// Numeric type (floating point)
    Numeric,
    /// Text/string type
    Text,
    /// Date type (stored as TEXT in SQLite)
    Date,
    /// Integer type
    Integer,
}

/// Aggregate function to apply to a field
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AggregateFunction {
    /// Sum of values
    Sum,
    /// Count of rows
    Count,
    /// Average value
    Avg,
    /// Minimum value
    Min,
    /// Maximum value
    Max,
}

impl AggregateFunction {
    /// Get SQL function name
    pub fn to_sql(&self) -> &'static str {
        match self {
            AggregateFunction::Sum => "SUM",
            AggregateFunction::Count => "COUNT",
            AggregateFunction::Avg => "AVG",
            AggregateFunction::Min => "MIN",
            AggregateFunction::Max => "MAX",
        }
    }
}

/// Filter operator for field conditions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterOperator {
    /// Equal (=)
    Eq,
    /// Not equal (<>)
    NotEq,
    /// Less than (<)
    Lt,
    /// Greater than (>)
    Gt,
    /// Less than or equal (<=)
    LtEq,
    /// Greater than or equal (>=)
    GtEq,
    /// Like pattern matching
    Like,
    /// In list
    In,
    /// Between two values
    Between,
    /// Is NULL
    IsNull,
}

impl FilterOperator {
    /// Get SQL operator string
    pub fn to_sql(&self) -> &'static str {
        match self {
            FilterOperator::Eq => "=",
            FilterOperator::NotEq => "<>",
            FilterOperator::Lt => "<",
            FilterOperator::Gt => ">",
            FilterOperator::LtEq => "<=",
            FilterOperator::GtEq => ">=",
            FilterOperator::Like => "LIKE",
            FilterOperator::In => "IN",
            FilterOperator::Between => "BETWEEN",
            FilterOperator::IsNull => "IS NULL",
        }
    }

    /// Get display label for UI
    pub fn label(&self) -> &'static str {
        match self {
            FilterOperator::Eq => "=",
            FilterOperator::NotEq => "≠",
            FilterOperator::Lt => "<",
            FilterOperator::Gt => ">",
            FilterOperator::LtEq => "≤",
            FilterOperator::GtEq => "≥",
            FilterOperator::Like => "содержит",
            FilterOperator::In => "в списке",
            FilterOperator::Between => "между",
            FilterOperator::IsNull => "пусто",
        }
    }
}
