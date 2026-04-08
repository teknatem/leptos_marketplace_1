//! DTOs для GL-ведомости по счёту (account_view).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlAccountViewQuery {
    pub account: String,
    pub date_from: String,
    pub date_to: String,
    pub connection_mp_ref: Option<String>,
    pub layer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlAccountViewRow {
    pub turnover_code: String,
    pub turnover_name: String,
    pub corr_account: String,
    pub layer: String,
    pub debit_amount: f64,
    pub credit_amount: f64,
    pub balance: f64,
    pub entry_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlAccountViewResponse {
    pub account: String,
    /// Строки из списка настроенных оборотов — полностью суммируются.
    pub main_rows: Vec<GlAccountViewRow>,
    /// Все остальные строки — для информации и сверки, в итоги не входят.
    pub info_rows: Vec<GlAccountViewRow>,
    /// Итоги только по main_rows
    pub total_debit: f64,
    pub total_credit: f64,
    pub total_balance: f64,
}
