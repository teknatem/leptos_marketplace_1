use serde::{Deserialize, Serialize};

/// DTO для строки отчёта по платежам Yandex Market
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmPaymentReportDto {
    /// Internal stable primary key (real transaction_id or SYNTH_... synthetic key)
    pub record_key: String,

    // Metadata
    pub connection_mp_ref: String,
    pub organization_ref: String,

    // Business info
    pub business_id: Option<i64>,
    pub partner_id: Option<i64>,
    pub shop_name: Option<String>,
    pub inn: Option<String>,
    pub model: Option<String>,

    // Transaction info — transaction_id is nullable (YM may not assign one yet)
    pub transaction_id: Option<String>,
    pub transaction_date: Option<String>,
    pub transaction_type: Option<String>,
    pub transaction_source: Option<String>,
    pub transaction_sum: Option<f64>,
    pub payment_status: Option<String>,

    // Order info
    pub order_id: Option<i64>,
    pub shop_order_id: Option<String>,
    pub order_creation_date: Option<String>,
    pub order_delivery_date: Option<String>,
    pub order_type: Option<String>,

    // Product/service info
    pub shop_sku: Option<String>,
    pub offer_or_service_name: Option<String>,
    pub count: Option<i32>,

    // Bank / Act info
    pub act_id: Option<i64>,
    pub act_date: Option<String>,
    pub bank_order_id: Option<i64>,
    pub bank_order_date: Option<String>,
    pub bank_sum: Option<f64>,

    // Extra
    pub claim_number: Option<String>,
    pub bonus_account_year_month: Option<String>,
    pub comments: Option<String>,

    // Technical fields
    pub loaded_at_utc: String,
    pub payload_version: i32,
}

/// Запрос на получение списка записей отчёта по платежам
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmPaymentReportListRequest {
    #[serde(default)]
    pub date_from: String,
    #[serde(default)]
    pub date_to: String,
    #[serde(default)]
    pub transaction_type: Option<String>,
    #[serde(default)]
    pub payment_status: Option<String>,
    #[serde(default)]
    pub shop_sku: Option<String>,
    #[serde(default)]
    pub order_id: Option<i64>,
    #[serde(default)]
    pub connection_mp_ref: Option<String>,
    #[serde(default)]
    pub organization_ref: Option<String>,
    #[serde(default = "default_sort_by")]
    pub sort_by: String,
    #[serde(default = "default_true")]
    pub sort_desc: bool,
    #[serde(default = "default_limit")]
    pub limit: i32,
    #[serde(default)]
    pub offset: i32,
}

fn default_sort_by() -> String {
    "transaction_date".to_string()
}

fn default_true() -> bool {
    true
}

fn default_limit() -> i32 {
    1000
}

/// Ответ со списком записей отчёта по платежам
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmPaymentReportListResponse {
    pub items: Vec<YmPaymentReportDto>,
    pub total_count: i32,
    pub has_more: bool,
}
