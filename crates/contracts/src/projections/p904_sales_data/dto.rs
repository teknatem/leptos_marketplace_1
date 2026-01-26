use serde::{Deserialize, Serialize};

/// DTO для записи Sales Data (P904)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesDataDto {
    pub id: String,

    // Technical fields
    pub registrator_ref: String,
    pub registrator_type: String,

    // Dimensions
    pub date: String,
    pub connection_mp_ref: String,
    pub nomenclature_ref: String,
    pub marketplace_product_ref: String,

    // Sums
    pub customer_in: f64,
    pub customer_out: f64,
    pub coinvest_in: f64,
    pub commission_out: f64,
    pub acquiring_out: f64,
    pub penalty_out: f64,
    pub logistics_out: f64,
    pub seller_out: f64,
    pub price_full: f64,
    pub price_list: f64,
    pub price_return: f64,
    pub commission_percent: f64,
    pub coinvest_persent: f64,
    pub total: f64,
    pub cost: Option<f64>,

    // Info fields
    pub document_no: String,
    pub article: String,
    pub posted_at: String,

    // Enhanced field from join with connection_mp
    pub connection_mp_name: Option<String>,
}

/// Запрос на получение списка Sales Data с фильтрами
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesDataListRequest {
    #[serde(default)]
    pub date_from: Option<String>,
    #[serde(default)]
    pub date_to: Option<String>,
    #[serde(default)]
    pub connection_mp_ref: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: u64,
}

fn default_limit() -> u64 {
    1000
}

/// Ответ со списком Sales Data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesDataListResponse {
    pub items: Vec<SalesDataDto>,
    pub total_count: i32,
    pub has_more: bool,
}
