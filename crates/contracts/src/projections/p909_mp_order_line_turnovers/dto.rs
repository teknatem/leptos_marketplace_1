use crate::shared::analytics::{
    AggKind, EventKind, ReportGroup, SelectionRule, TurnoverLayer, ValueKind,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpOrderLineTurnoverDto {
    pub id: String,
    pub connection_mp_ref: String,
    pub order_key: String,
    pub line_key: String,
    pub line_event_key: String,
    pub event_kind: EventKind,
    pub entry_date: String,
    pub layer: TurnoverLayer,
    pub turnover_code: String,
    pub value_kind: ValueKind,
    pub agg_kind: AggKind,
    pub amount: f64,
    pub nomenclature_ref: Option<String>,
    pub marketplace_product_ref: Option<String>,
    pub registrator_type: String,
    pub registrator_ref: String,
    pub link_status: String,
    pub general_ledger_ref: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub turnover_name: String,
    pub turnover_description: String,
    pub turnover_llm_description: String,
    pub selection_rule: SelectionRule,
    pub report_group: ReportGroup,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpOrderLineTurnoverListRequest {
    #[serde(default)]
    pub date_from: Option<String>,
    #[serde(default)]
    pub date_to: Option<String>,
    #[serde(default)]
    pub connection_mp_ref: Option<String>,
    #[serde(default)]
    pub order_key: Option<String>,
    #[serde(default)]
    pub line_key: Option<String>,
    #[serde(default)]
    pub layer: Option<String>,
    #[serde(default)]
    pub turnover_code: Option<String>,
    #[serde(default)]
    pub link_status: Option<String>,
    #[serde(default)]
    pub sort_by: Option<String>,
    #[serde(default)]
    pub sort_desc: Option<bool>,
    #[serde(default)]
    pub offset: Option<u64>,
    #[serde(default = "default_limit")]
    pub limit: u64,
}

fn default_limit() -> u64 {
    1000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpOrderLineTurnoverListResponse {
    pub items: Vec<MpOrderLineTurnoverDto>,
    pub total_count: i32,
    pub has_more: bool,
}
