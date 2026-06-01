use serde::{Deserialize, Serialize};

/// DTO строки проекции финансовых оборотов слоя `fina`.
/// Каждая строка зеркалит одну GL-проводку слоя fina, построенную из
/// финансового отчёта МП (p903/p907), и совпадает с ней по сумме, дате
/// транзакции, `turnover_code` и основным измерениям.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpFinanceTurnoverDto {
    pub id: String,
    pub transaction_date: String,
    pub general_ledger_ref: Option<String>,
    pub registrator_type: String,
    pub registrator_ref: String,
    pub connection_mp_ref: String,
    pub nomenclature_ref: Option<String>,
    pub marketplace_product_ref: Option<String>,
    pub turnover_code: String,
    pub order_key: String,
    /// UUID агрегата-заказа (a015_wb_orders / a013_ym_order), если оборот связан
    /// с заказом. Для оборотов без заказа — None.
    pub order_ref: Option<String>,
    /// Тип документа-заказа: a015_wb_orders / a013_ym_order.
    pub order_registrator_type: Option<String>,
    pub event_kind: String,
    /// URL — юрлицо, FIZ — физлицо.
    pub customer_kind: Option<String>,
    /// FBO, FBS, FBW.
    pub fulfillment_type: Option<String>,
    pub layer: String,
    pub amount: f64,
    pub quantity: Option<f64>,
    pub created_at_msk: String,
    pub updated_at_msk: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpFinanceTurnoverListRequest {
    #[serde(default)]
    pub date_from: Option<String>,
    #[serde(default)]
    pub date_to: Option<String>,
    #[serde(default)]
    pub connection_mp_ref: Option<String>,
    #[serde(default)]
    pub registrator_type: Option<String>,
    #[serde(default)]
    pub turnover_code: Option<String>,
    #[serde(default)]
    pub order_key: Option<String>,
    #[serde(default)]
    pub event_kind: Option<String>,
    #[serde(default)]
    pub sort_by: Option<String>,
    #[serde(default)]
    pub sort_desc: Option<bool>,
    #[serde(default)]
    pub offset: Option<u64>,
    #[serde(default = "default_limit")]
    pub limit: u64,
}

fn default_limit() -> u64 {
    1000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpFinanceTurnoverListResponse {
    pub items: Vec<MpFinanceTurnoverDto>,
    pub total_count: i32,
    pub has_more: bool,
}
