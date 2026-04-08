use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WbWeeklyReconciliationQuery {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub connection_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbWeeklyReconciliationRow {
    pub document_id: String,
    pub service_name: String,
    pub connection_id: String,
    pub connection_name: Option<String>,
    pub report_period_from: Option<String>,
    pub report_period_to: Option<String>,
    pub realized_goods_total: Option<f64>,
    pub wb_reward_with_vat: Option<f64>,
    pub seller_transfer_total: Option<f64>,
    pub gl_total_balance: Option<f64>,
    pub difference: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WbWeeklyReconciliationResponse {
    pub items: Vec<WbWeeklyReconciliationRow>,
}
