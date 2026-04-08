//! DTOs для GL-отчёта (сводный отчёт + GL-first детализация через detail projections).

use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Сводный отчёт по GL
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlReportQuery {
    pub date_from: String,
    pub date_to: String,
    /// Фильтр по подключению маркетплейса (connection_mp_ref).
    pub connection_mp_ref: Option<String>,
    /// Фильтр по счёту: отбираются строки, где debit_account = account
    /// ИЛИ credit_account = account. Если None — берутся все строки.
    pub account: Option<String>,
    /// Фильтр по слою: oper / fact / plan.
    pub layer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlReportRow {
    pub turnover_code: String,
    pub turnover_name: String,
    /// Слой учёта (oper / fact / plan).
    pub layer: String,
    /// Сумма по дебету (при фильтре счёта — SUM(amount) WHERE debit_account = account).
    pub debit_amount: f64,
    /// Сумма по кредиту (при фильтре счёта — SUM(amount) WHERE credit_account = account).
    pub credit_amount: f64,
    /// Сальдо = debit_amount - credit_amount.
    pub balance: f64,
    pub entry_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlReportResponse {
    pub rows: Vec<GlReportRow>,
    pub total_debit: f64,
    pub total_credit: f64,
    pub total_balance: f64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Измерения для детализации
// ─────────────────────────────────────────────────────────────────────────────

/// Описание доступного измерения для drilldown конкретного оборота.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlDimensionDef {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlDimensionsResponse {
    pub turnover_code: String,
    pub dimensions: Vec<GlDimensionDef>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Детализация (drilldown из GL в detail projections)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlDrilldownQuery {
    pub turnover_code: String,
    /// ID измерения (из GlDimensionDef.id).
    pub group_by: String,
    pub date_from: String,
    pub date_to: String,
    pub connection_mp_ref: Option<String>,
    #[serde(default)]
    pub connection_mp_refs: Vec<String>,
    pub account: Option<String>,
    pub layer: Option<String>,
    #[serde(default)]
    pub corr_account: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlDrilldownRow {
    pub group_key: String,
    pub group_label: String,
    pub amount: f64,
    pub entry_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlDrilldownResponse {
    pub rows: Vec<GlDrilldownRow>,
    pub group_by_label: String,
    pub turnover_code: String,
    pub turnover_name: String,
    pub total_amount: f64,
    pub total_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlDrilldownSessionCreate {
    pub title: Option<String>,
    pub query: GlDrilldownQuery,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlDrilldownSessionCreateResponse {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlDrilldownSessionRecord {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub use_count: i64,
    pub query: GlDrilldownQuery,
}
