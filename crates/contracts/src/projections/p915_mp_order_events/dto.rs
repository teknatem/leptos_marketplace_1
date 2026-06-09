//! DTO проекции `p915_mp_order_events`.

use serde::{Deserialize, Serialize};

/// Одно событие таймлайна заказа (плоское зеркало строки БД).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpOrderEventDto {
    pub id: String,
    pub order_id: String,
    pub marketplace_product: Option<String>,
    pub event_date: String,
    pub event_type: String,
    pub layer: String,
    pub amount: Option<f64>,
    pub registrator_type: String,
    pub registrator_ref: String,
    pub connection_mp_ref: String,
    pub created_at_msk: String,
    pub updated_at_msk: String,
}

/// Запрос списка событий с опциональными фильтрами.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MpOrderEventListRequest {
    #[serde(default)]
    pub date_from: Option<String>,
    #[serde(default)]
    pub date_to: Option<String>,
    #[serde(default)]
    pub connection_mp_ref: Option<String>,
    #[serde(default)]
    pub order_id: Option<String>,
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub registrator_type: Option<String>,
    #[serde(default)]
    pub layer: Option<String>,
    #[serde(default)]
    pub sort_by: Option<String>,
    #[serde(default)]
    pub sort_desc: Option<bool>,
    #[serde(default)]
    pub offset: Option<u64>,
    #[serde(default)]
    pub limit: Option<u64>,
}

/// Ответ списка событий.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpOrderEventListResponse {
    pub items: Vec<MpOrderEventDto>,
    pub total_count: i32,
    pub has_more: bool,
}
