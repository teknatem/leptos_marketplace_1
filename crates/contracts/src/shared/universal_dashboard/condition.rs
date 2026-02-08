use serde::{Deserialize, Serialize};

#[allow(deprecated)]
use super::config::FieldFilter;
use super::schema::{FilterOperator, ValueType};

/// Helper function for serde default
fn default_true() -> bool {
    true
}

/// Filter condition with all components needed for UI, SQL generation, and display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterCondition {
    /// Unique identifier for this condition
    pub id: String,

    /// Field ID this condition applies to
    pub field_id: String,

    /// Value type of the field (for reusable conditions)
    pub value_type: ValueType,

    /// Condition definition (for UI and logic)
    pub definition: ConditionDef,

    /// Human-readable display text for users
    pub display_text: String,

    /// Whether this condition is active (applied to query)
    #[serde(default = "default_true")]
    pub active: bool,

    /// SQL fragment (generated on backend, optional on frontend)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sql_fragment: Option<SqlFragment>,
}

impl FilterCondition {
    /// Create a new filter condition
    pub fn new(field_id: String, value_type: ValueType, definition: ConditionDef) -> Self {
        let display_text = definition.generate_display_text("Field");
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            field_id,
            value_type,
            definition,
            display_text,
            active: true,
            sql_fragment: None,
        }
    }

    /// Update display text with field name
    pub fn with_field_name(mut self, field_name: &str) -> Self {
        self.display_text = self.definition.generate_display_text(field_name);
        self
    }

    /// Set SQL fragment
    pub fn with_sql_fragment(mut self, sql_fragment: SqlFragment) -> Self {
        self.sql_fragment = Some(sql_fragment);
        self
    }

    /// Preserve ID and active state from existing condition (for updates)
    pub fn with_preserved_state(mut self, existing: &FilterCondition) -> Self {
        self.id = existing.id.clone();
        self.active = existing.active;
        self
    }
}

/// Condition definition - describes what kind of filter this is
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConditionDef {
    /// Comparison: field operator value (=, <, >, !=, etc.)
    Comparison {
        operator: ComparisonOp,
        value: String,
    },

    /// Range: field BETWEEN from AND to
    Range {
        from: Option<String>,
        to: Option<String>,
    },

    /// Date period with optional presets
    DatePeriod {
        preset: Option<DatePreset>,
        from: Option<String>, // YYYY-MM-DD
        to: Option<String>,   // YYYY-MM-DD
    },

    /// Nullability check (IS NULL / IS NOT NULL)
    Nullability { is_null: bool },

    /// Text contains pattern (LIKE)
    Contains { pattern: String },

    /// In list of values
    InList {
        values: Vec<String>,
        negated: bool, // false = IN, true = NOT IN
    },
}

impl ConditionDef {
    /// Generate human-readable display text
    pub fn generate_display_text(&self, field_name: &str) -> String {
        match self {
            ConditionDef::Comparison { operator, value } => {
                format!("{} {} {}", field_name, operator.symbol(), value)
            }
            ConditionDef::Range { from, to } => match (from, to) {
                (Some(f), Some(t)) => format!("{}: {} — {}", field_name, f, t),
                (Some(f), None) => format!("{} ≥ {}", field_name, f),
                (None, Some(t)) => format!("{} ≤ {}", field_name, t),
                (None, None) => format!("{}: любой диапазон", field_name),
            },
            ConditionDef::DatePeriod { preset, from, to } => {
                if let Some(p) = preset {
                    format!("{}: {}", field_name, p.display_name())
                } else {
                    match (from, to) {
                        (Some(f), Some(t)) => format!("{}: {} — {}", field_name, f, t),
                        (Some(f), None) => format!("{} ≥ {}", field_name, f),
                        (None, Some(t)) => format!("{} ≤ {}", field_name, t),
                        (None, None) => format!("{}: любой период", field_name),
                    }
                }
            }
            ConditionDef::Nullability { is_null } => {
                if *is_null {
                    format!("{} не заполнено", field_name)
                } else {
                    format!("{} заполнено", field_name)
                }
            }
            ConditionDef::Contains { pattern } => {
                format!("{} содержит \"{}\"", field_name, pattern)
            }
            ConditionDef::InList { values, negated } => {
                let prefix = if *negated { "не в" } else { "в" };
                if values.len() <= 3 {
                    format!("{} {} [{}]", field_name, prefix, values.join(", "))
                } else {
                    format!(
                        "{} {} списке ({} значений)",
                        field_name,
                        prefix,
                        values.len()
                    )
                }
            }
        }
    }
}

/// SQL fragment - result of SQL generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlFragment {
    /// SQL condition clause (e.g., "field >= ? AND field <= ?")
    pub sql: String,

    /// Parameter values (in order)
    pub params: Vec<String>,

    /// Additional JOINs needed (for Ref conditions)
    #[serde(default)]
    pub joins: Vec<String>,
}

/// Comparison operator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonOp {
    /// Equal (=)
    Eq,
    /// Not equal (<> or !=)
    NotEq,
    /// Less than (<)
    Lt,
    /// Greater than (>)
    Gt,
    /// Less than or equal (<=)
    LtEq,
    /// Greater than or equal (>=)
    GtEq,
}

impl ComparisonOp {
    /// Get SQL operator string
    pub fn to_sql(&self) -> &'static str {
        match self {
            ComparisonOp::Eq => "=",
            ComparisonOp::NotEq => "<>",
            ComparisonOp::Lt => "<",
            ComparisonOp::Gt => ">",
            ComparisonOp::LtEq => "<=",
            ComparisonOp::GtEq => ">=",
        }
    }

    /// Get display symbol for UI
    pub fn symbol(&self) -> &'static str {
        match self {
            ComparisonOp::Eq => "=",
            ComparisonOp::NotEq => "≠",
            ComparisonOp::Lt => "<",
            ComparisonOp::Gt => ">",
            ComparisonOp::LtEq => "≤",
            ComparisonOp::GtEq => "≥",
        }
    }

    /// Get display label for UI
    pub fn label(&self) -> &'static str {
        match self {
            ComparisonOp::Eq => "равно",
            ComparisonOp::NotEq => "не равно",
            ComparisonOp::Lt => "меньше",
            ComparisonOp::Gt => "больше",
            ComparisonOp::LtEq => "меньше или равно",
            ComparisonOp::GtEq => "больше или равно",
        }
    }
}

/// Date preset enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DatePreset {
    Today,
    Yesterday,
    ThisWeek,
    LastWeek,
    ThisMonth,
    LastMonth,
    ThisQuarter,
    LastQuarter,
    ThisYear,
    LastYear,
    Last7Days,
    Last30Days,
    Last90Days,
}

impl DatePreset {
    /// Get display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            DatePreset::Today => "Сегодня",
            DatePreset::Yesterday => "Вчера",
            DatePreset::ThisWeek => "Эта неделя",
            DatePreset::LastWeek => "Прошлая неделя",
            DatePreset::ThisMonth => "Этот месяц",
            DatePreset::LastMonth => "Прошлый месяц",
            DatePreset::ThisQuarter => "Этот квартал",
            DatePreset::LastQuarter => "Прошлый квартал",
            DatePreset::ThisYear => "Этот год",
            DatePreset::LastYear => "Прошлый год",
            DatePreset::Last7Days => "Последние 7 дней",
            DatePreset::Last30Days => "Последние 30 дней",
            DatePreset::Last90Days => "Последние 90 дней",
        }
    }

    /// Get all available presets
    pub fn all() -> &'static [DatePreset] {
        &[
            DatePreset::Today,
            DatePreset::Yesterday,
            DatePreset::ThisWeek,
            DatePreset::LastWeek,
            DatePreset::ThisMonth,
            DatePreset::LastMonth,
            DatePreset::ThisQuarter,
            DatePreset::LastQuarter,
            DatePreset::ThisYear,
            DatePreset::LastYear,
            DatePreset::Last7Days,
            DatePreset::Last30Days,
            DatePreset::Last90Days,
        ]
    }
}

/// Migration: Convert old FieldFilter to new FilterCondition
#[allow(deprecated)]
impl From<FieldFilter> for FilterCondition {
    fn from(old: FieldFilter) -> Self {
        let definition = match old.operator {
            FilterOperator::Eq => ConditionDef::Comparison {
                operator: ComparisonOp::Eq,
                value: old.value.clone(),
            },
            FilterOperator::NotEq => ConditionDef::Comparison {
                operator: ComparisonOp::NotEq,
                value: old.value.clone(),
            },
            FilterOperator::Lt => ConditionDef::Comparison {
                operator: ComparisonOp::Lt,
                value: old.value.clone(),
            },
            FilterOperator::Gt => ConditionDef::Comparison {
                operator: ComparisonOp::Gt,
                value: old.value.clone(),
            },
            FilterOperator::LtEq => ConditionDef::Comparison {
                operator: ComparisonOp::LtEq,
                value: old.value.clone(),
            },
            FilterOperator::GtEq => ConditionDef::Comparison {
                operator: ComparisonOp::GtEq,
                value: old.value.clone(),
            },
            FilterOperator::Like => ConditionDef::Contains {
                pattern: old.value.clone(),
            },
            FilterOperator::In => {
                let values: Vec<String> =
                    old.value.split(',').map(|s| s.trim().to_string()).collect();
                ConditionDef::InList {
                    values,
                    negated: false,
                }
            }
            FilterOperator::Between => ConditionDef::Range {
                from: Some(old.value.clone()),
                to: old.value2.clone(),
            },
            FilterOperator::IsNull => ConditionDef::Nullability { is_null: true },
        };

        let display_text = definition.generate_display_text(&old.field_id);

        FilterCondition {
            id: uuid::Uuid::new_v4().to_string(),
            field_id: old.field_id,
            // Default to Text type - should be updated with actual field type
            value_type: ValueType::Text,
            definition,
            display_text,
            active: true,
            sql_fragment: None,
        }
    }
}

/// Helper function to migrate DashboardFilters from old to new format
#[allow(deprecated)]
pub fn migrate_filters_to_conditions(field_filters: &[FieldFilter]) -> Vec<FilterCondition> {
    field_filters
        .iter()
        .map(|f| FilterCondition::from(f.clone()))
        .collect()
}
