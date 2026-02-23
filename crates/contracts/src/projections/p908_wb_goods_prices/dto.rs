use serde::{Deserialize, Serialize};

/// DTO для строки цен WB товара (проекция p908)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbGoodsPriceDto {
    pub nm_id: i64,
    pub connection_mp_ref: String,
    pub vendor_code: Option<String>,
    pub discount: Option<i32>,
    pub editable_size_price: bool,
    /// Цена первого размера (в рублях, дробная)
    pub price: Option<f64>,
    /// Цена первого размера со скидкой
    pub discounted_price: Option<f64>,
    /// Полный массив размеров в формате JSON
    pub sizes_json: String,
    pub fetched_at: String,
    /// Resolved UUID from a004_nomenclature (base_nomenclature_ref or own id)
    pub ext_nomenklature_ref: Option<String>,
    /// Dealer price from p906_nomenclature_prices
    pub dealer_price_ut: Option<f64>,
    /// Margin: (discounted_price - dealer_price_ut) / dealer_price_ut * 100
    pub margin_pro: Option<f64>,
    /// Nomenclature description from a004_nomenclature (JOIN)
    pub nomenclature_name: Option<String>,
    /// Connection cabinet name from a006_connection_mp (JOIN)
    pub connection_name: Option<String>,
}

/// Запрос на получение списка цен товаров WB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbGoodsPriceListRequest {
    #[serde(default)]
    pub connection_mp_ref: Option<String>,
    #[serde(default)]
    pub vendor_code: Option<String>,
    #[serde(default)]
    pub search: Option<String>,
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
    "nm_id".to_string()
}

fn default_true() -> bool {
    true
}

fn default_limit() -> i32 {
    1000
}

/// Ответ со списком цен товаров WB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbGoodsPriceListResponse {
    pub items: Vec<WbGoodsPriceDto>,
    pub total_count: i32,
    pub has_more: bool,
}
