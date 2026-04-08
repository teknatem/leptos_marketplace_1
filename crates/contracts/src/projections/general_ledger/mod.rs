use crate::general_ledger::GlDimensionDef;
use crate::shared::analytics::{
    AggKind, ReportGroup, SelectionRule, SignPolicy, TurnoverLayer, TurnoverScope, ValueKind,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralLedgerEntryDto {
    pub id: String,
    pub entry_date: String,
    pub layer: TurnoverLayer,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_mp_ref: Option<String>,
    pub registrator_type: String,
    pub registrator_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    pub debit_account: String,
    pub credit_account: String,
    pub amount: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qty: Option<f64>,
    pub turnover_code: String,
    #[serde(default)]
    pub resource_table: String,
    #[serde(default = "default_resource_field")]
    pub resource_field: String,
    #[serde(default = "default_resource_sign")]
    pub resource_sign: i32,
    pub created_at: String,
    /// Комментарий из реестра оборотов: смысл, формула, счета Дт/Кт.
    #[serde(default)]
    pub comment: String,
}

fn default_resource_field() -> String {
    "amount".to_string()
}

fn default_resource_sign() -> i32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralLedgerTurnoverDto {
    pub code: String,
    pub name: String,
    pub description: String,
    pub llm_description: String,
    pub scope: TurnoverScope,
    pub value_kind: ValueKind,
    pub agg_kind: AggKind,
    pub selection_rule: SelectionRule,
    pub sign_policy: SignPolicy,
    pub report_group: ReportGroup,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub source_examples: Vec<String>,
    pub formula_hint: String,
    pub notes: String,
    pub debit_account: String,
    pub credit_account: String,
    pub generates_journal_entry: bool,
    pub journal_comment: String,
    #[serde(default)]
    pub gl_entries_count: usize,
    #[serde(default)]
    pub available_dimensions: Vec<GlDimensionDef>,
}
