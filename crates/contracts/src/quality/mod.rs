//! # Контракты подсистемы контроля качества данных
//!
//! Типы, разделяемые между бэкендом и фронтендом:
//! - [`QualityCheckInfo`] — метаданные проверки (id, название, описание, категория)
//! - [`CheckMetric`] — одна метрика: популяция (знаменатель) + нарушения (числитель) + единица
//! - [`ViolationItem`] — конкретный пример нарушения с ссылками на GL/проекцию
//! - [`CheckResult`] — полный результат запуска проверки
//! - [`BreakdownRow`] / [`CheckBreakdown`] — разрез метрики по измерению (кабинет, исправимость…)
//! - [`CheckDetails`] — полный пакет для страницы детализации (info + result + разрезы)
//! - [`QualityCheckSource`] — один источник для детализации (проекционная таблица)
//! - [`NipRegistratorGroup`] — группа регистратора в drill-down проверки номенклатуры
//! - [`NipGroupsResponse`] — страница списка групп
//! - [`NipProjectionRow`] — строка проекции с отсутствующей номенклатурой
//! - [`NipRepostRequest`] / [`NipRepostResult`] — запрос и итог массового перепроведения
//! - [`NipCleanupRequest`] / [`NipCleanupResult`] — запрос и итог очистки осиротевших строк проекций

use serde::{Deserialize, Serialize};

/// Описание одной проверки качества данных (метаданные из реестра).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityCheckInfo {
    pub code: String,
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
}

/// Метрика по одному источнику данных.
///
/// Ключевой принцип подсистемы: метрика бессмысленна без знаменателя. Каждая
/// метрика несёт **популяцию** (`population` — сколько всего ситуаций попадает
/// под правило) и **нарушения** (`violations` — сколько из них не соответствует).
/// Доля соответствия и число соответствующих строк вычисляются из этой пары.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckMetric {
    pub label: String,
    /// Сколько всего ситуаций попадает под правило (знаменатель).
    pub population: i64,
    /// Сколько из них не соответствует правилу (числитель).
    pub violations: i64,
    /// Что именно считаем: `"строк"`, `"GL-записей"`, `"документов"` и т.п.
    #[serde(default)]
    pub unit: String,
}

impl CheckMetric {
    /// Число соответствующих правилу ситуаций (`population - violations`, не ниже 0).
    pub fn compliant(&self) -> i64 {
        (self.population - self.violations).max(0)
    }

    /// Доля соответствия `[0.0; 1.0]`. Пустая популяция трактуется как 100%.
    pub fn compliance_rate(&self) -> f64 {
        if self.population <= 0 {
            1.0
        } else {
            self.compliant() as f64 / self.population as f64
        }
    }
}

/// Конкретный пример нарушения целостности данных.
///
/// Содержит ссылки на GL-запись и/или строку проекции для перехода
/// в детализацию непосредственно из UI контроля качества.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViolationItem {
    /// Тип нарушения: `"orphan_gl"` | `"orphan_projection"` | `"amount_mismatch"`
    pub violation_type: String,
    /// ID GL-записи (`sys_general_ledger.id`), если применимо.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gl_id: Option<String>,
    /// ID строки проекции (e.g. `p909_mp_order_line_turnovers.id`), если применимо.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_id: Option<String>,
    /// Имя таблицы проекции (e.g. `"p909_mp_order_line_turnovers"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_table: Option<String>,
    /// Дополнительный контекст: дельта суммы, сэмпл ссылки и т.п.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Результат запуска одной проверки.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub check_id: String,
    pub run_at: chrono::DateTime<chrono::Utc>,
    /// Суммарная популяция по всем метрикам (общий знаменатель).
    #[serde(default)]
    pub population_total: i64,
    /// Суммарное число нарушений (0 = данные в порядке).
    #[serde(alias = "total_missing")]
    pub violations_total: i64,
    /// Детализированные метрики по проекциям / категориям.
    pub metrics: Vec<CheckMetric>,
    /// Примеры конкретных нарушений (до 20 штук) для дрилл-дауна в UI.
    #[serde(default)]
    pub violations: Vec<ViolationItem>,
}

impl CheckResult {
    /// Доля соответствия по всей проверке `[0.0; 1.0]`.
    pub fn compliance_rate(&self) -> f64 {
        if self.population_total <= 0 {
            1.0
        } else {
            (self.population_total - self.violations_total).max(0) as f64
                / self.population_total as f64
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Разрезы метрик и пакет детализации (страница quality_check_details)
// ─────────────────────────────────────────────────────────────────────────────

/// Одна строка разреза: значение измерения + пара (популяция, нарушения).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakdownRow {
    pub label: String,
    pub population: i64,
    pub violations: i64,
}

impl BreakdownRow {
    pub fn compliant(&self) -> i64 {
        (self.population - self.violations).max(0)
    }

    pub fn compliance_rate(&self) -> f64 {
        if self.population <= 0 {
            1.0
        } else {
            self.compliant() as f64 / self.population as f64
        }
    }
}

/// Разрез метрики по одному измерению (по кабинету, по исправимости и т.п.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckBreakdown {
    /// Машинный ключ измерения: `"by_connection"`, `"fixability"`, …
    pub key: String,
    /// Заголовок разреза для UI, напр. `"По кабинету маркетплейса"`.
    pub title: String,
    /// Подпись колонки значения измерения, напр. `"Кабинет"`.
    pub dimension_label: String,
    /// `false` — строки это срезы популяции (показываем популяцию/нарушения/долю
    /// соответствия). `true` — строки это **разбиение самих нарушений** на категории
    /// (в `population` лежит счётчик категории; UI показывает счётчик и долю от итога).
    #[serde(default)]
    pub is_partition: bool,
    pub rows: Vec<BreakdownRow>,
}

/// Полный пакет данных для страницы `quality_check_details`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckDetails {
    pub info: QualityCheckInfo,
    pub result: CheckResult,
    /// Разрезы метрик по измерениям (могут быть пустыми, если у правила их нет).
    #[serde(default)]
    pub breakdowns: Vec<CheckBreakdown>,
    /// Источники для drill-down (проекционные таблицы), если поддерживаются.
    #[serde(default)]
    pub sources: Vec<QualityCheckSource>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Drill-down: Nomenclature In Projections (NIP)
// ─────────────────────────────────────────────────────────────────────────────

/// Один источник для детализации — проекционная таблица с меткой.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityCheckSource {
    /// Имя таблицы, напр. `"p909_mp_order_line_turnovers"`.
    pub projection_table: String,
    /// Читаемый заголовок, напр. `"p909 — Обороты строк заказов МП"`.
    pub label: String,
}

/// Одна колонка из карточки/списка исходного документа-регистратора.
///
/// Позволяет drill-down показывать строки регистраторов в том же виде, что и
/// исходный список агрегата (напр. для `a026`: Дата, Документ, Кампания, Кабинет,
/// Расход). Набор колонок задаётся бэкендом по типу регистратора.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceColumn {
    /// Заголовок колонки, напр. `"Кампания"`.
    pub label: String,
    /// Отображаемое значение.
    pub value: String,
    /// `true` — выравнивать по правому краю (числовые колонки).
    #[serde(default)]
    pub align_right: bool,
}

/// Группа регистратора: все строки с пустым `nomenclature_ref` для одного документа-источника.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NipRegistratorGroup {
    pub projection_table: String,
    pub registrator_type: String,
    pub registrator_ref: String,
    /// Читаемое имя типа регистратора, напр. `"Реклама WB (день)"`.
    pub registrator_type_label: String,
    /// Краткое отображение ID (первые 8 символов UUID + "…").
    pub display_short: String,
    pub min_entry_date: Option<String>,
    pub max_entry_date: Option<String>,
    pub missing_count: i64,
    /// `true` — для этого типа регистратора доступно перепроведение.
    pub can_post: bool,
    /// `true` — группу можно удалить из проекции как осиротевшую.
    #[serde(default)]
    pub can_cleanup: bool,
    /// Префикс tab-ключа для открытия карточки документа во фронтенде,
    /// напр. `"a026_wb_advert_daily_details"`.
    pub tab_key_prefix: Option<String>,
    /// Колонки исходного документа (как в списке агрегата). Пусто, если для типа
    /// регистратора шаблон не определён — тогда UI показывает базовые колонки.
    #[serde(default)]
    pub source_columns: Vec<SourceColumn>,
}

/// Страница списка групп регистраторов.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NipGroupsResponse {
    pub items: Vec<NipRegistratorGroup>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

/// Строка проекции с отсутствующей номенклатурой (детали регистратора).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NipProjectionRow {
    pub id: String,
    pub entry_date: String,
    pub turnover_code: String,
    pub amount: f64,
    pub connection_mp_ref: String,
    /// Метка дополнительного поля контекста (напр. `"Заказ"`, `"Кампания"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_label: Option<String>,
    /// Значение дополнительного поля контекста.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_value: Option<String>,
}

/// Запрос массового перепроведения регистраторов.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NipRepostRequest {
    pub projection_table: String,
    pub registrator_type: String,
    pub registrator_refs: Vec<String>,
}

/// Результат массового перепроведения.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NipRepostResult {
    pub requested: usize,
    pub reposted: usize,
    pub errors: Vec<String>,
}

/// Запрос очистки осиротевших строк проекций.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NipCleanupRequest {
    pub projection_table: String,
    pub registrator_refs: Vec<String>,
}

/// Результат очистки осиротевших строк проекций.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NipCleanupResult {
    pub requested: usize,
    pub deleted_rows: usize,
    pub errors: Vec<String>,
}
