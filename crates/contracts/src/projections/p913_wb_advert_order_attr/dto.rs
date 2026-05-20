use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbAdvertOrderAttrDto {
    pub id: String,
    pub connection_mp_ref: String,
    pub entry_date: String,
    pub turnover_code: String,
    pub amount: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nomenclature_ref: Option<String>,
    pub wb_advert_campaign_code: String,
    pub order_key: String,
    pub registrator_type: String,
    pub registrator_ref: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub general_ledger_ref: Option<String>,
    pub is_problem: bool,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub sale_amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbAdvertOrderAttrListResponse {
    pub items: Vec<WbAdvertOrderAttrDto>,
    pub total_count: i32,
}
