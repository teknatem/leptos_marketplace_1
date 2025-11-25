use serde::{Deserialize, Serialize};

/// DTO для записи истории комиссий Wildberries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommissionHistoryDto {
    pub id: String,
    pub date: String,
    pub subject_id: i32,
    pub subject_name: String,
    pub parent_id: i32,
    pub parent_name: String,
    pub kgvp_booking: f64,
    pub kgvp_marketplace: f64,
    pub kgvp_pickup: f64,
    pub kgvp_supplier: f64,
    pub kgvp_supplier_express: f64,
    pub paid_storage_kgvp: f64,
    pub raw_json: String,
    pub loaded_at_utc: String,
    pub payload_version: i32,
}

/// Запрос на получение списка комиссий
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommissionListRequest {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub subject_id: Option<i32>,
    pub sort_by: Option<String>,
    pub sort_desc: Option<bool>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

/// Ответ со списком комиссий
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommissionListResponse {
    pub items: Vec<CommissionHistoryDto>,
    pub total_count: u64,
}

/// Запрос на создание/обновление записи комиссии
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommissionSaveRequest {
    pub id: Option<String>, // None для создания новой записи
    pub date: String,
    pub subject_id: i32,
    pub subject_name: String,
    pub parent_id: i32,
    pub parent_name: String,
    pub kgvp_booking: f64,
    pub kgvp_marketplace: f64,
    pub kgvp_pickup: f64,
    pub kgvp_supplier: f64,
    pub kgvp_supplier_express: f64,
    pub paid_storage_kgvp: f64,
    pub raw_json: Option<String>, // Опционально, если редактируется через UI
}

/// Ответ после сохранения записи
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommissionSaveResponse {
    pub id: String,
    pub message: String,
}

/// Запрос на синхронизацию с API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommissionSyncRequest {
    // Пустой, можно добавить параметры в будущем
}

/// Ответ после синхронизации
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommissionSyncResponse {
    pub status: String, // "success" | "failed"
    pub message: String,
    pub new_records_count: usize,
    pub updated_count: usize,
    pub skipped_count: usize,
}

/// Запрос на удаление записи
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommissionDeleteRequest {
    pub id: String,
}

/// Ответ после удаления
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommissionDeleteResponse {
    pub success: bool,
    pub message: String,
}
