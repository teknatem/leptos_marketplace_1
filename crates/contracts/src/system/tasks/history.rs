use serde::{Deserialize, Serialize};

/// Масштаб агрегации истории запусков регламентных заданий.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskHistoryScale {
    Day,
    Week,
    Month,
}

/// Метрика для графика истории регламентных заданий.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskHistoryMetric {
    TaskCount,
    RequestCount,
    TrafficBytes,
}

/// Запрос истории запусков.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskHistoryRequest {
    pub scale: TaskHistoryScale,
    pub metric: TaskHistoryMetric,
    /// Начало периода в формате YYYY-MM-DD.
    pub date_from: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_ids: Option<Vec<String>>,
}

/// Одна точка временного ряда.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskHistoryPoint {
    /// Начало бакета в UTC, ISO-like строка.
    pub bucket: String,
    pub value: f64,
    /// Позиция бакета: минуты для day, 5-минутные интервалы для week, часы для month.
    pub offset: u32,
}

/// Ответ истории запусков.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskHistoryResponse {
    pub points: Vec<TaskHistoryPoint>,
    pub bucket_count: u32,
    pub date_from: String,
}
