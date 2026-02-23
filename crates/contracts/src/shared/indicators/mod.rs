use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Indicator identity & display metadata
// ---------------------------------------------------------------------------

/// Unique indicator identifier, used as key in registry and API requests.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IndicatorId(pub String);

impl IndicatorId {
    pub fn new(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// How to format the numeric value on the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ValueFormat {
    Money { currency: String },
    Number { decimals: u8 },
    Percent { decimals: u8 },
    Integer,
}

/// Visual status of the indicator (drives colour).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndicatorStatus {
    Good,
    Bad,
    Neutral,
    Warning,
}

/// Static metadata describing one indicator (label, format, icon, ...).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorMeta {
    pub id: IndicatorId,
    pub label: String,
    pub short_label: Option<String>,
    pub icon: String,
    pub format: ValueFormat,
    pub description: Option<String>,
}

// ---------------------------------------------------------------------------
// Indicator sets
// ---------------------------------------------------------------------------

/// Unique set identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IndicatorSetId(pub String);

impl IndicatorSetId {
    pub fn new(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Metadata for a group of indicators rendered together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorSetMeta {
    pub id: IndicatorSetId,
    pub label: String,
    pub indicators: Vec<IndicatorId>,
    /// Number of columns in the card grid (2, 3, 4).
    pub columns: u8,
}

// ---------------------------------------------------------------------------
// Computed values
// ---------------------------------------------------------------------------

/// A single computed indicator result returned by the backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorValue {
    pub id: IndicatorId,
    /// Primary numeric value (`None` when data is unavailable).
    pub value: Option<f64>,
    /// Value for the previous comparable period.
    pub previous_value: Option<f64>,
    /// Change relative to previous period, expressed as a percentage.
    pub change_percent: Option<f64>,
    pub status: IndicatorStatus,
    /// Optional secondary text displayed below the value.
    pub subtitle: Option<String>,
}

// ---------------------------------------------------------------------------
// API request / response
// ---------------------------------------------------------------------------

/// Context passed by the dashboard to narrow the computation scope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorContext {
    pub date_from: String,
    pub date_to: String,
    #[serde(default)]
    pub organization_ref: Option<String>,
    #[serde(default)]
    pub marketplace: Option<String>,
    /// Filter by specific marketplace cabinet (connection_mp) IDs.
    /// Empty vec means "all cabinets".
    #[serde(default)]
    pub connection_mp_refs: Vec<String>,
    #[serde(default)]
    pub extra: HashMap<String, String>,
}

/// Batch request: compute several indicators in one round-trip.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeIndicatorsRequest {
    pub indicator_ids: Vec<IndicatorId>,
    pub context: IndicatorContext,
}

/// Batch response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeIndicatorsResponse {
    pub values: Vec<IndicatorValue>,
}

/// Full catalogue returned by the metadata endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorCatalogResponse {
    pub indicators: Vec<IndicatorMeta>,
    pub sets: Vec<IndicatorSetMeta>,
}
