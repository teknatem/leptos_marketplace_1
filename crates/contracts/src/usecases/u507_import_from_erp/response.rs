use serde::{Deserialize, Serialize};

/// Ответ на запрос запуска импорта из ERP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResponse {
    pub session_id: String,
    pub status: ImportStartStatus,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportStartStatus {
    Started,
    Failed,
}
