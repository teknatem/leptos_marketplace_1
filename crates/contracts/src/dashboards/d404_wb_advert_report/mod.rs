use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WbAdvertReportRequest {
    #[serde(default)]
    pub date_from: Option<String>,
    #[serde(default)]
    pub date_to: Option<String>,
    #[serde(default)]
    pub wb_advert_campaign_code: Option<String>,
    #[serde(default)]
    pub connection_mp_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WbAdvertReportTotals {
    pub accrued: f64,
    pub expensed: f64,
    pub balance: f64,
    pub expense_no_order: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbAdvertReportLink {
    pub label: String,
    pub tab_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbAdvertReportNode {
    pub level: String,
    pub id: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wb_advert_campaign_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nomenclature_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_key: Option<String>,
    pub accrued: f64,
    pub expensed: f64,
    pub balance: f64,
    pub expense_no_order: f64,
    pub links: Vec<WbAdvertReportLink>,
    pub children: Vec<WbAdvertReportNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbAdvertReportResponse {
    pub filters: WbAdvertReportRequest,
    pub totals: WbAdvertReportTotals,
    pub campaigns: Vec<WbAdvertReportNode>,
}
