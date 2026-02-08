#![allow(deprecated)]

use serde::{Deserialize, Serialize};

/// Source of schema definition
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchemaSource {
    /// Auto-generated from entity metadata
    Auto,
    /// Custom schema defined in code
    Custom,
}

/// Summary information about a schema for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaInfo {
    /// Schema identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Source of the schema
    pub source: SchemaSource,
    /// Database table name
    pub table_name: String,
}

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
#[allow(deprecated)]
#[derive(Debug, Clone)]
pub struct FieldDef {
    /// Unique field identifier (e.g., "retail_amount")
    pub id: &'static str,
    /// Human-readable field name (e.g., "Сумма продаж")
    pub name: &'static str,
    /// Type of the field (DEPRECATED - use get_value_type())
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
    /// Source table for the field (if different from main table)
    /// Example: Some("a004_nomenclature") for dimension fields
    pub source_table: Option<&'static str>,
    /// Column in main table to join on (e.g., "nomenclature_ref")
    /// Used when source_table is specified
    pub join_on_column: Option<&'static str>,
}

#[allow(deprecated)]
impl FieldDef {
    /// Get the ValueType for this field (computed from field_type and ref_table)
    pub fn get_value_type(&self) -> ValueType {
        ValueType::from_field_type(self.field_type, self.ref_table)
    }
}

/// Owned version of FieldDef for API responses
#[allow(deprecated)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefOwned {
    /// Unique field identifier
    pub id: String,
    /// Human-readable field name
    pub name: String,
    /// Type of the field value
    pub value_type: ValueType,
    /// Can this field be used in GROUP BY
    pub can_group: bool,
    /// Can this field be aggregated
    pub can_aggregate: bool,
    /// Actual database column name
    pub db_column: String,
    /// Reference display column (for Ref types)
    pub ref_display_column: Option<String>,
    /// Source table for the field (if different from main table)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_table: Option<String>,
    /// Column in main table to join on
    #[serde(skip_serializing_if = "Option::is_none")]
    pub join_on_column: Option<String>,

    // Deprecated fields (kept for backward compatibility with old APIs)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[allow(deprecated)]
    pub field_type: Option<FieldType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_table: Option<String>,
}

impl FieldDefOwned {
    /// Get the ValueType for this field (from value_type or computed from field_type)
    pub fn get_value_type(&self) -> ValueType {
        self.value_type.clone()
    }
}

#[allow(deprecated)]
impl From<&FieldDef> for FieldDefOwned {
    fn from(field: &FieldDef) -> Self {
        Self {
            id: field.id.to_string(),
            name: field.name.to_string(),
            value_type: field.get_value_type(),
            can_group: field.can_group,
            can_aggregate: field.can_aggregate,
            db_column: field.db_column.to_string(),
            ref_display_column: field.ref_display_column.map(|s| s.to_string()),
            source_table: field.source_table.map(|s| s.to_string()),
            join_on_column: field.join_on_column.map(|s| s.to_string()),
            // For backward compatibility
            field_type: Some(field.field_type),
            ref_table: field.ref_table.map(|s| s.to_string()),
        }
    }
}

/// Value type enumeration (extended type system)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ValueType {
    /// Integer type
    Integer,
    /// Numeric type (floating point)
    Numeric,
    /// Text/string type
    Text,
    /// Date type (stored as TEXT in SQLite)
    Date,
    /// DateTime type (date with time)
    DateTime,
    /// Boolean type
    Boolean,
    /// Reference to another entity/table
    Ref {
        /// Dictionary/table name (e.g., "counterparty", "product")
        dictionary: String,
    },
}

impl ValueType {
    /// Get canonical name for comparison
    pub fn canonical_name(&self) -> String {
        match self {
            ValueType::Integer => "integer".to_string(),
            ValueType::Numeric => "numeric".to_string(),
            ValueType::Text => "text".to_string(),
            ValueType::Date => "date".to_string(),
            ValueType::DateTime => "datetime".to_string(),
            ValueType::Boolean => "boolean".to_string(),
            ValueType::Ref { dictionary } => format!("ref:{}", dictionary),
        }
    }

    /// Check if this type is compatible with another
    pub fn is_compatible_with(&self, other: &ValueType) -> bool {
        match (self, other) {
            // Exact match
            (a, b) if a == b => true,
            // Numeric types are compatible
            (ValueType::Integer, ValueType::Numeric) => true,
            (ValueType::Numeric, ValueType::Integer) => true,
            // Date and DateTime are compatible
            (ValueType::Date, ValueType::DateTime) => true,
            (ValueType::DateTime, ValueType::Date) => true,
            _ => false,
        }
    }

    /// Convert from old FieldType (for backward compatibility)
    #[allow(deprecated)]
    pub fn from_field_type(field_type: FieldType, ref_table: Option<&str>) -> Self {
        if let Some(dict) = ref_table {
            ValueType::Ref {
                dictionary: dict.to_string(),
            }
        } else {
            match field_type {
                FieldType::Integer => ValueType::Integer,
                FieldType::Numeric => ValueType::Numeric,
                FieldType::Text => ValueType::Text,
                FieldType::Date => ValueType::Date,
            }
        }
    }
}

/// Old field type enumeration (deprecated, kept for backward compatibility)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[deprecated(note = "Use ValueType instead")]
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
