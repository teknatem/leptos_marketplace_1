use serde::{Deserialize, Serialize};

/// Масштаб агрегации статистики внешнего API.
///
/// Намеренно отдельный от `TaskHistoryScale`: внешний API и регламентные задания —
/// разные предметные области, их контракты вправе разойтись.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtApiScale {
    Day,
    Week,
    Month,
}

/// Метрика для графика внешнего API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtApiMetric {
    /// Количество входящих вызовов.
    RequestCount,
    /// Отданный наружу трафик, байты.
    TrafficBytes,
    /// Среднее время ответа в бакете, мс.
    AvgDurationMs,
    /// Количество ответов со статусом >= 400.
    ErrorCount,
}

/// Запрос временного ряда по внешнему API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtApiHistoryRequest {
    pub scale: ExtApiScale,
    pub metric: ExtApiMetric,
    /// Начало периода в формате YYYY-MM-DD (МСК).
    pub date_from: String,
}

/// Одна точка временного ряда.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtApiHistoryPoint {
    /// Начало бакета, строка вида `YYYY-MM-DDTHH:MM:SS MSK`.
    pub bucket: String,
    pub value: f64,
    /// Позиция бакета: минуты для day, 5-минутные интервалы для week, часы для month.
    pub offset: u32,
}

/// Ответ временного ряда.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtApiHistoryResponse {
    pub points: Vec<ExtApiHistoryPoint>,
    pub bucket_count: u32,
    pub date_from: String,
    /// Итоги за весь период — для карточек над графиком.
    pub totals: ExtApiTotals,
}

/// Сводные итоги за период.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtApiTotals {
    pub req_count: i64,
    pub bytes_out: i64,
    pub error_count: i64,
    pub avg_ms: f64,
}

/// Сырая строка лога вызова внешнего API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtApiLogRow {
    pub id: String,
    pub ts: String,
    pub method: String,
    pub route: String,
    pub path: String,
    pub query: Option<String>,
    pub status: i32,
    pub duration_ms: i64,
    pub bytes_out: i64,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub client_id: Option<String>,
}

/// Строка сводки — по эндпоинту или по потребителю (см. `ExtApiSummaryResponse`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtApiSummaryRow {
    /// Роут (`/api/ext/v1/wb-stocks`) либо потребитель
    /// (`COALESCE(client_id, user_agent, client_ip)`).
    pub key: String,
    pub req_count: i64,
    pub bytes_out: i64,
    pub error_count: i64,
    pub avg_ms: f64,
}

/// Сводка за период: разрезы по эндпоинтам и по потребителям.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtApiSummaryResponse {
    pub by_route: Vec<ExtApiSummaryRow>,
    pub by_client: Vec<ExtApiSummaryRow>,
}

/// Ответ со списком последних вызовов.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtApiLogListResponse {
    pub rows: Vec<ExtApiLogRow>,
}
