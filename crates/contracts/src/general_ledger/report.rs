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
    /// Фильтр по субъекту учёта (ym / wb / ozon / san / sts / upr).
    pub entity: Option<String>,
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
    /// Субъект учёта (ym / wb / ozon / san / sts / upr), если задан.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity: Option<String>,
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
    pub code: String,
    pub code_main: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_suffix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    /// Квалифицированное имя поля в БД: таблица.колонка (напр. sys_gl.entry_date).
    pub db_field: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlDimensionsResponse {
    pub turnover_code: String,
    pub dimensions: Vec<GlDimensionDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlDimensionUsageRef {
    pub turnover_code: String,
    pub turnover_name: String,
    pub report_group: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlDimensionCatalogItem {
    pub id: String,
    pub label: String,
    pub code: String,
    pub code_main: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_suffix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    pub root_id: String,
    pub depth: usize,
    pub sort_order: usize,
    #[serde(default)]
    pub path_ids: Vec<String>,
    #[serde(default)]
    pub path_codes: Vec<String>,
    #[serde(default)]
    pub turnover_count: usize,
    #[serde(default)]
    pub used_by_turnovers: Vec<GlDimensionUsageRef>,
    /// Квалифицированное имя поля в БД: таблица.колонка.
    pub db_field: String,
    /// Человекочитаемое описание измерения (абзац для UI-каталога).
    #[serde(default)]
    pub description: String,
    /// Системное (структурное) измерение GL: оборот/счета проводки. Только ярлык
    /// для UI — на доступность для drilldown не влияет.
    #[serde(default)]
    pub is_system: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlDimensionsCatalogResponse {
    pub items: Vec<GlDimensionCatalogItem>,
    pub total: usize,
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
    pub entity: Option<String>,
    #[serde(default)]
    pub corr_account: Option<String>,
}

/// Человекочитаемое представление агрегата (документа-регистратора и т.п.):
/// наименование объекта + дата + id/номер. Строится на бэкенде модулем агрегата.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AggregateRepresentation {
    /// Наименование объекта (description / code / специфичное имя).
    pub title: String,
    /// Основная дата, нормализована до YYYY-MM-DD.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    /// Человекочитаемый номер / короткий id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlDrilldownRow {
    pub group_key: String,
    pub group_label: String,
    pub amount: f64,
    pub entry_count: i64,
    /// Реальное представление документа-регистратора (заполняется только при
    /// group_by = "registrator_ref"); None — UI делает фолбэк на синтетику.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub representation: Option<AggregateRepresentation>,
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

// ─────────────────────────────────────────────────────────────────────────────
// Матрица Слой / Оборот (обзор доступности измерений по слоям)
// ─────────────────────────────────────────────────────────────────────────────

/// Колонка матрицы — слой учёта.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlMatrixLayer {
    pub code: String,
    pub name: String,
    pub color_key: String,
    pub sort_order: usize,
}

/// Строка матрицы — вид оборота.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlMatrixTurnover {
    pub code: String,
    pub name: String,
    pub report_group: String,
}

/// Измерение ячейки с пометкой уровня и списком источников данных.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlMatrixDimension {
    pub def: GlDimensionDef,
    /// Измерение 1-го уровня (без родителя) — учитывается в кратком счётчике.
    pub is_top_level: bool,
    /// Источники, где доступен этот разрез: «GL», «p903», «p914», … Отвечает на
    /// «где взять данные в этом разрезе» для конкретной пары (оборот, слой).
    pub sources: Vec<String>,
}

/// Проекция-зеркало, через которую строятся измерения ячейки.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlMatrixProjection {
    pub resource_table: String,
    pub label: String,
    /// Способ связи: projection_linked / external_linked / gl.
    pub kind: String,
}

/// Ячейка матрицы — пересечение (оборот, слой).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlMatrixCell {
    pub turnover_code: String,
    pub layer: String,
    /// Кол-во измерений 1-го уровня (краткий показатель ячейки).
    pub top_level_count: usize,
    /// Кол-во реальных GL-проводок (0 — комбинация существует лишь теоретически).
    pub entry_count: i64,
    pub dimensions: Vec<GlMatrixDimension>,
    pub projections: Vec<GlMatrixProjection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlLayerTurnoverMatrixResponse {
    pub layers: Vec<GlMatrixLayer>,
    pub turnovers: Vec<GlMatrixTurnover>,
    pub cells: Vec<GlMatrixCell>,
    /// Уникальные измерения по всем ячейкам — для фильтра по измерению в шапке.
    pub filter_dimensions: Vec<GlDimensionDef>,
}
