use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateScheduledTaskDto {
    pub code: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    pub task_type: String,
    pub schedule_cron: Option<String>,
    pub is_enabled: bool,
    pub config_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateScheduledTaskDto {
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub task_type: String,
    pub schedule_cron: Option<String>,
    pub is_enabled: bool,
    pub config_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToggleScheduledTaskEnabledDto {
    pub is_enabled: bool,
}

/// Установить или сбросить watermark (last_successful_run_at) задачи.
/// `date` — дата в формате "YYYY-MM-DD", null — полный сброс (будет загружено с work_start_date).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetWatermarkDto {
    /// "YYYY-MM-DD" или null для полного сброса
    pub date: Option<String>,
}

/// Статус глобального планировщика задач (включён / выключен).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerStatusDto {
    pub enabled: bool,
    /// Значение `[scheduled_tasks].enabled` из config.toml.
    /// Если в конфиге выключено — фоновый воркер вообще не запускается,
    /// и runtime-переключатель не имеет эффекта. Используется фронтендом
    /// для предупреждающей полосы. По умолчанию `true` (для обратной
    /// совместимости при разборе POST-запроса от клиента).
    #[serde(default = "default_config_enabled")]
    pub config_enabled: bool,
}

fn default_config_enabled() -> bool {
    true
}
