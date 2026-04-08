use crate::projections::general_ledger::GeneralLedgerEntryDto;
use crate::shared::analytics::{AggKind, ReportGroup, SelectionRule, TurnoverLayer, ValueKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbAdvertByItemDto {
    pub id: String,
    pub connection_mp_ref: String,
    pub entry_date: String,
    pub layer: TurnoverLayer,
    pub turnover_code: String,
    pub value_kind: ValueKind,
    pub agg_kind: AggKind,
    pub amount: f64,
    pub nomenclature_ref: Option<String>,
    pub registrator_type: String,
    pub registrator_ref: String,
    pub general_ledger_ref: Option<String>,
    pub is_problem: bool,
    pub created_at: String,
    pub updated_at: String,
    pub turnover_name: String,
    pub turnover_description: String,
    pub turnover_llm_description: String,
    pub selection_rule: SelectionRule,
    pub report_group: ReportGroup,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbAdvertByItemListResponse {
    pub items: Vec<WbAdvertByItemDto>,
    pub total_count: i32,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbAdvertByItemDetailDto {
    pub general_ledger_ref: String,
    pub general_ledger_entry: Option<GeneralLedgerEntryDto>,
    pub items: Vec<WbAdvertByItemDto>,
    pub total_amount: f64,
}
