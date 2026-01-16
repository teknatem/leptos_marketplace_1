use serde::{Deserialize, Serialize};

/// DTO для записи Sales Register
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesRegisterDto {
    // NK (Natural Key)
    pub marketplace: String,
    pub document_no: String,
    pub line_id: String,

    // Metadata
    pub scheme: Option<String>,
    pub document_type: String,
    pub document_version: i32,

    // References to aggregates (UUID as String)
    pub connection_mp_ref: String,
    pub organization_ref: String,
    pub marketplace_product_ref: Option<String>,
    pub nomenclature_ref: Option<String>,
    pub registrator_ref: String,

    // Timestamps and status
    pub event_time_source: String,
    pub sale_date: String,
    pub source_updated_at: Option<String>,
    pub status_source: String,
    pub status_norm: String,

    // Product identification
    pub seller_sku: Option<String>,
    pub mp_item_id: String,
    pub barcode: Option<String>,
    pub title: Option<String>,

    // Quantities and money
    pub qty: f64,
    pub price_list: Option<f64>,
    pub discount_total: Option<f64>,
    pub price_effective: Option<f64>,
    pub amount_line: Option<f64>,
    /// Плановая себестоимость (из p906_nomenclature_prices)
    pub cost: Option<f64>,
    pub currency_code: Option<String>,

    // Technical fields
    pub loaded_at_utc: String,
    pub payload_version: i32,
    pub extra: Option<String>,
}

/// Request для получения списка продаж с фильтрами
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesRegisterListRequest {
    pub date_from: String, // NaiveDate as string "YYYY-MM-DD"
    pub date_to: String,
    #[serde(default)]
    pub marketplace: Option<String>,
    #[serde(default)]
    pub organization_ref: Option<String>,
    #[serde(default)]
    pub connection_mp_ref: Option<String>,
    #[serde(default)]
    pub status_norm: Option<String>,
    #[serde(default)]
    pub seller_sku: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i32,
    #[serde(default)]
    pub offset: i32,
}

fn default_limit() -> i32 {
    50
}

/// Response для списка продаж
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesRegisterListResponse {
    pub items: Vec<SalesRegisterDto>,
    pub total_count: i32,
    pub has_more: bool,
}

/// Request для статистики по датам
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesRegisterStatsByDateRequest {
    pub date_from: String,
    pub date_to: String,
    #[serde(default)]
    pub marketplace: Option<String>,
}

/// Статистика за один день
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyStat {
    pub date: String,
    pub sales_count: i32,
    pub total_qty: f64,
    pub total_revenue: f64,
}

/// Response для статистики по датам
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesRegisterStatsByDateResponse {
    pub data: Vec<DailyStat>,
}

/// Статистика по маркетплейсам
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceStat {
    pub marketplace: String,
    pub sales_count: i32,
    pub total_qty: f64,
    pub total_revenue: f64,
}

/// Response для статистики по маркетплейсам
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesRegisterStatsByMarketplaceResponse {
    pub data: Vec<MarketplaceStat>,
}

/// Детальная информация о продаже с ссылками на связанные объекты
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesRegisterDetailDto {
    pub sale: SalesRegisterDto,
    
    // Дополнительные данные для отображения
    pub organization_name: Option<String>,
    pub connection_mp_name: Option<String>,
    pub marketplace_product_name: Option<String>,
}

