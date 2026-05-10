use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::shared::analytics::ValueFormat;
use crate::shared::data_view::ViewContext;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiTimelineIndicatorInfo {
    pub id: String,
    pub code: String,
    pub description: String,
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(default)]
    pub view_id: Option<String>,
    #[serde(default)]
    pub metric_id: Option<String>,
    #[serde(default)]
    pub day_dimension: Option<String>,
    pub compatible: bool,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub priority: bool,
    pub format: ValueFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BiTimelineIndicatorsResponse {
    pub indicators: Vec<BiTimelineIndicatorInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiTimelineRequest {
    pub context: ViewContext,
    #[serde(default)]
    pub indicator_ids: Vec<String>,
    #[serde(default)]
    pub indicator_codes: Vec<String>,
    #[serde(default)]
    pub params: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiTimelinePoint {
    pub offset: i64,
    pub label: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiTimelineSeries {
    pub indicator: BiTimelineIndicatorInfo,
    pub period1_label: String,
    pub period2_label: String,
    pub series_p1: Vec<BiTimelinePoint>,
    pub series_p2: Vec<BiTimelinePoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiTimelineError {
    pub indicator_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BiTimelineResponse {
    pub items: Vec<BiTimelineSeries>,
    pub errors: Vec<BiTimelineError>,
}
