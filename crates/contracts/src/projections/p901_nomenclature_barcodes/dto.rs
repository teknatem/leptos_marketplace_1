use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// DTO для штрихкода номенклатуры
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NomenclatureBarcodeDto {
    pub barcode: String,
    pub nomenclature_ref: String,  // UUID на a004_nomenclature
    pub nomenclature_name: Option<String>,  // Название номенклатуры
    pub article: Option<String>,
    pub source: String,  // "1C" | "OZON" | "WB" | "YM"
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
}

/// Запрос на получение штрихкода по ID
#[derive(Debug, Deserialize)]
pub struct BarcodeByIdRequest {
    pub barcode: String,
}

/// Ответ на получение штрихкода по ID
#[derive(Debug, Serialize)]
pub struct BarcodeByIdResponse {
    pub barcode: NomenclatureBarcodeDto,
}

/// Запрос на получение штрихкодов по nomenclature_ref
#[derive(Debug, Deserialize)]
pub struct BarcodesByNomenclatureRequest {
    #[serde(default)]
    pub include_inactive: bool,
}

/// Ответ на получение штрихкодов по nomenclature_ref
#[derive(Debug, Serialize)]
pub struct BarcodesByNomenclatureResponse {
    pub nomenclature_ref: String,
    pub barcodes: Vec<NomenclatureBarcodeDto>,
    pub total_count: usize,
}

/// Запрос на получение списка штрихкодов с фильтрами
#[derive(Debug, Deserialize)]
pub struct BarcodeListRequest {
    pub nomenclature_ref: Option<String>,
    pub article: Option<String>,
    pub source: Option<String>,
    #[serde(default = "default_include_inactive")]
    pub include_inactive: bool,
    #[serde(default = "default_limit")]
    pub limit: i32,
    #[serde(default)]
    pub offset: i32,
}

fn default_include_inactive() -> bool {
    false
}

fn default_limit() -> i32 {
    100
}

/// Ответ на получение списка штрихкодов
#[derive(Debug, Serialize)]
pub struct BarcodeListResponse {
    pub barcodes: Vec<NomenclatureBarcodeDto>,
    pub total_count: i32,
    pub limit: i32,
    pub offset: i32,
}
