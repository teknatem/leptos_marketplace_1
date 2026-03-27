use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Multi-metric types ────────────────────────────────────────────────────────

/// Описание одной метрики-колонки в multi-resource drilldown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricColumnDef {
    pub id: String,
    pub label: String,
}

/// Значения одной метрики для одной строки (П1 + П2 + Δ%).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetricValues {
    pub value1: f64,
    pub value2: f64,
    pub delta_pct: Option<f64>,
}

/// Универсальный запрос детализации (drilldown) по схеме данных
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrilldownRequest {
    /// ID схемы данных (ds03_p904_sales, ds01_wb_finance_report, ...)
    pub schema_id: String,
    /// Field ID из схемы для группировки (date, article, connection_mp_ref, ...)
    pub group_by: String,
    /// Период 1: начало
    pub date_from: String,
    /// Период 1: конец
    pub date_to: String,
    /// Период 2: начало (если None — авто-сдвиг на 1 месяц назад)
    #[serde(default)]
    pub period2_from: Option<String>,
    /// Период 2: конец (если None — авто-сдвиг на 1 месяц назад)
    #[serde(default)]
    pub period2_to: Option<String>,
    /// Фильтр по кабинетам МП (пустой = все)
    #[serde(default)]
    pub connection_mp_refs: Vec<String>,
    /// Дополнительные фильтры (ключ = field_id схемы, значение = строка)
    #[serde(default)]
    pub extra_filters: HashMap<String, String>,
    /// Метрика агрегации (customer_in, seller_out, order_count, ...)
    /// По умолчанию — customer_in
    #[serde(default = "default_metric")]
    pub metric_column: String,
}

fn default_metric() -> String {
    "customer_in".to_string()
}

/// Строка детализации — одна группа с двумя периодами
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrilldownRow {
    /// Ключ группировки (raw DB value)
    pub group_key: String,
    /// Человекочитаемое название группы
    pub label: String,
    /// Значение за период 1 (single-metric режим)
    pub value1: f64,
    /// Значение за период 2 (single-metric режим)
    pub value2: f64,
    /// Изменение в процентах (None если period2 = 0)
    pub delta_pct: Option<f64>,
    /// Значения по каждой метрике (multi-resource режим).
    /// Ключ = resource id, пуст в single-metric режиме.
    #[serde(default)]
    pub metric_values: HashMap<String, MetricValues>,
}

/// Ответ на запрос детализации
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrilldownResponse {
    pub rows: Vec<DrilldownRow>,
    /// Заголовок колонки группировки
    pub group_by_label: String,
    /// Метка периода 1 (например, "янв 2026")
    pub period1_label: String,
    /// Метка периода 2 (например, "дек 2025")
    pub period2_label: String,
    /// Метка метрики (single-metric режим)
    pub metric_label: String,
    /// Упорядоченный список метрик-колонок (multi-resource режим).
    /// Пуст в single-metric режиме.
    #[serde(default)]
    pub metric_columns: Vec<MetricColumnDef>,
}
