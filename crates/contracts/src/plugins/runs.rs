//! DTO наблюдаемости плагинов: журнал запусков, агрегированная статистика и
//! классификация «здоровья» (отклонение от нормы) для UI.

use serde::{Deserialize, Serialize};

/// Светофор состояния плагина по окну запусков.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginHealth {
    /// Запусков в окне нет.
    #[default]
    NoData,
    Ok,
    Warn,
    Crit,
}

/// Срез ошибок по стадии (PluginError.stage).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageCount {
    pub stage: String,
    pub count: i64,
}

/// Одна запись журнала запусков.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRunRecord {
    pub id: String,
    pub method: String,
    pub started_at: String,
    pub duration_ms: i64,
    /// ok | error | timeout
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_stage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row_count: Option<i64>,
}

/// Агрегированная статистика по окну (days дней).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginRunSummary {
    pub days: i64,
    pub total: i64,
    pub errors: i64,
    pub timeouts: i64,
    pub error_rate: f64,
    pub avg_ms: i64,
    pub max_ms: i64,
    #[serde(default)]
    pub by_stage: Vec<StageCount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_run_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_status: Option<String>,
    pub health: PluginHealth,
}

/// Полная статистика для страницы плагина: сводка + последние запуски.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginStats {
    pub summary: PluginRunSummary,
    #[serde(default)]
    pub recent: Vec<PluginRunRecord>,
}

/// Краткая сводка по плагину для реестра (колонка «Здоровье»).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRunBrief {
    pub plugin_id: String,
    pub runs: i64,
    pub error_rate: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_run_at: Option<String>,
    pub health: PluginHealth,
}
