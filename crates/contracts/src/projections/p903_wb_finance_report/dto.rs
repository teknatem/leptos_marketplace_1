use serde::{Deserialize, Serialize};

/// DTO для строки финансового отчета Wildberries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbFinanceReportDto {
    // Composite Key
    pub rr_dt: String,
    pub rrd_id: i64,

    // Metadata
    pub connection_mp_ref: String,
    pub organization_ref: String,

    // Main Fields (22 specified fields)
    pub acquiring_fee: Option<f64>,
    pub acquiring_percent: Option<f64>,
    pub additional_payment: Option<f64>,
    pub bonus_type_name: Option<String>,
    pub commission_percent: Option<f64>,
    pub delivery_amount: Option<f64>,
    pub delivery_rub: Option<f64>,
    pub nm_id: Option<i64>,
    pub penalty: Option<f64>,
    pub ppvz_vw: Option<f64>,
    pub ppvz_vw_nds: Option<f64>,
    pub ppvz_sales_commission: Option<f64>,
    pub quantity: Option<i32>,
    pub rebill_logistic_cost: Option<f64>,
    pub retail_amount: Option<f64>,
    pub retail_price: Option<f64>,
    pub retail_price_withdisc_rub: Option<f64>,
    pub return_amount: Option<f64>,
    pub sa_name: Option<String>,
    pub storage_fee: Option<f64>,
    pub subject_name: Option<String>,
    pub supplier_oper_name: Option<String>,
    pub cashback_amount: Option<f64>,
    pub ppvz_for_pay: Option<f64>,
    pub ppvz_kvw_prc: Option<f64>,
    pub ppvz_kvw_prc_base: Option<f64>,
    pub srv_dbs: Option<i32>,

    // Technical fields
    pub loaded_at_utc: String,
    pub payload_version: i32,
    pub extra: Option<String>,
}

/// Запрос на получение списка финансовых отчетов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbFinanceReportListRequest {
    pub date_from: String,
    pub date_to: String,
    #[serde(default)]
    pub nm_id: Option<i64>,
    #[serde(default)]
    pub sa_name: Option<String>,
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
    "rr_dt".to_string()
}

fn default_true() -> bool {
    true
}

fn default_limit() -> i32 {
    1000
}

/// Ответ со списком финансовых отчетов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbFinanceReportListResponse {
    pub items: Vec<WbFinanceReportDto>,
    pub total_count: i32,
    pub has_more: bool,
}

/// Ответ с деталями по одной записи
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbFinanceReportDetailResponse {
    pub item: WbFinanceReportDto,
}

