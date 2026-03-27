use crate::shared::analytics::TurnoverLayer;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralLedgerEntryDto {
    pub id: String,
    pub posting_id: String,
    pub entry_date: String,
    pub layer: TurnoverLayer,
    pub registrator_type: String,
    pub registrator_ref: String,
    pub debit_account: String,
    pub credit_account: String,
    pub amount: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qty: Option<f64>,
    pub turnover_code: String,
    pub detail_kind: String,
    pub detail_id: String,
    #[serde(default)]
    pub resource_name: String,
    #[serde(default = "default_resource_sign")]
    pub resource_sign: i32,
    pub created_at: String,
    /// Комментарий из реестра оборотов: смысл, формула, счета Дт/Кт.
    #[serde(default)]
    pub comment: String,
}

fn default_resource_sign() -> i32 {
    1
}
