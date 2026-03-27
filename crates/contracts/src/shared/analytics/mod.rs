use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod turnover;

pub use turnover::*;

/// Unique scalar/metric identifier used by BI/DataView computations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IndicatorId(pub String);

impl IndicatorId {
    pub fn new(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// How to format the numeric value on the frontend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MoneyScale {
    Thousand,
    Million,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ValueFormat {
    Money {
        currency: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        scale: Option<MoneyScale>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        decimals: Option<u8>,
    },
    Number {
        decimals: u8,
    },
    Percent {
        decimals: u8,
    },
    Integer,
}

/// Visual status of the computed scalar value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndicatorStatus {
    Good,
    Bad,
    Neutral,
    Warning,
}

/// A single computed scalar result returned by BI/DataView backends.
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
    /// Daily values for period 1, sorted ascending by date (used for sparkline).
    /// Empty when the data source does not provide daily breakdown.
    #[serde(default)]
    pub spark_points: Vec<f64>,
}

/// Shared compute context used by BI/DataView scalar and drilldown calculations.
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
