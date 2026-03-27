//! DataView contracts
//!
//! ViewContext — универсальный вход для всех DataView.
//! DataViewMeta — метаданные семантического слоя для каждого DataView.
//! FilterDef / FilterRef — глобальный реестр фильтров и их использование в DataView.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::analytics::IndicatorContext;

// ── Filter Registry types ─────────────────────────────────────────────────────

/// Один вариант для Select-фильтра.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

/// Тип фильтра — определяет UI-компонент для рендеринга.
///
/// Однотипные фильтры именуются с общим префиксом в filter ID:
/// - `date_range_1` / `date_range_2` для периодов (DateRange)
/// - `connection_mp_refs` для кабинетов МП
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FilterKind {
    /// Парный выбор диапазона дат — рендерится как DateRangePicker.
    /// `from_id` / `to_id` — имена полей в ViewContext.
    DateRange { from_id: String, to_id: String },
    /// Множественный выбор из справочника (source = "connection_mp" | ...)
    MultiSelect { source: String },
    /// Выбор из фиксированного списка вариантов
    Select { options: Vec<SelectOption> },
    /// Текстовый поиск
    Text,
}

/// Глобальное определение типа фильтра — неизменяемый контракт.
///
/// Хранится в глобальном реестре на бэкенде.
/// Описывает только тип и UI-компонент, без бизнес-контекста конкретного DataView.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterDef {
    /// Уникальный ID: "date_range_1_from", "connection_mp_refs", "metric_revenue"
    pub id: String,
    /// Метка по умолчанию: "Период 1, дата с"
    pub label: String,
    /// Тип — диктует UI компонент
    pub kind: FilterKind,
}

/// Использование фильтра в конкретном DataView.
///
/// Связывает FilterDef с конкретным DataView, добавляя контекстные свойства:
/// обязательность, порядок отображения, значение по умолчанию.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterRef {
    /// Ссылка на FilterDef.id в глобальном реестре
    pub filter_id: String,
    /// Обязателен ли этот фильтр в данном DataView
    pub required: bool,
    /// Порядок отображения в UI
    pub order: u32,
    /// Значение по умолчанию (строка, интерпретируется по FilterKind)
    #[serde(default)]
    pub default_value: Option<String>,
    /// Переопределение метки (если None — используется FilterDef.label)
    #[serde(default)]
    pub label_override: Option<String>,
}

// ── Metadata structs ─────────────────────────────────────────────────────────

/// Описание одного измерения (группировки) доступного для drilldown.
///
/// SQL-поля (db_column, ref_table и др.) — опциональны. Позволяют DataView
/// быть самодостаточным: каждый dvNNN описывает свои измерения полностью
/// и не зависит от глобального SchemaRegistry.
///
/// Если `db_column` отсутствует — при выполнении используется `id` как имя колонки.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionMeta {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub description: String,
    /// Реальное имя колонки в таблице. Если None — совпадает с id.
    #[serde(default)]
    pub db_column: Option<String>,
    /// JOIN со справочной таблицей для отображения метки
    /// (напр. "a006_connection_mp" для connection_mp_ref).
    #[serde(default)]
    pub ref_table: Option<String>,
    /// Колонка в ref_table для отображаемого имени (напр. "description").
    #[serde(default)]
    pub ref_display_column: Option<String>,
    /// Для косвенных JOIN: таблица-источник измерения
    /// (напр. "a004_nomenclature" для dim1-dim6).
    #[serde(default)]
    pub source_table: Option<String>,
    /// Колонка связи в основной таблице для косвенного JOIN
    /// (напр. "nomenclature_ref").
    #[serde(default)]
    pub join_on_column: Option<String>,
}

/// Описание одного ресурса (метрики), которую умеет вычислять DataView.
///
/// Ресурс — конкретное бизнес-вычисление: выручка, себестоимость и т.д.
/// Передаётся в `params["metric"]` при вызове DataView.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMeta {
    /// Значение параметра `metric` при вызове: "revenue" | "cost" | ...
    pub id: String,
    /// Человекочитаемое название: "Выручка"
    pub label: String,
    /// Описание формулы и семантики
    #[serde(default)]
    pub description: String,
    /// Единица измерения: "currency" | "count" | "percent" | ""
    #[serde(default)]
    pub unit: String,
}

/// Метаданные DataView — каталог семантического слоя.
///
/// Описывает именованное бизнес-вычисление: что считает, из каких источников,
/// по каким измерениям можно детализировать. Используется в UI и LLM-контексте.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataViewMeta {
    /// Уникальный идентификатор: "dv001_revenue"
    pub id: String,
    /// Человекочитаемое название: "Выручка (2 периода)"
    pub name: String,
    /// Категория для группировки: "revenue" | "orders" | "costs" | ...
    pub category: String,
    /// Версия реализации
    pub version: u32,
    /// Краткое описание для UI
    pub description: String,
    /// Развёрнутое описание для LLM-агентов и семантического каталога
    pub ai_description: String,
    /// Таблицы / схемы из которых читаются данные
    pub data_sources: Vec<String>,
    /// Доступные измерения для drilldown
    pub available_dimensions: Vec<DimensionMeta>,
    /// Доступные ресурсы (метрики), которые умеет вычислять DataView
    #[serde(default)]
    pub available_resources: Vec<ResourceMeta>,
    /// Фильтры DataView — ссылки на глобальный реестр FilterDef.
    /// Содержат контекстные свойства: required, order, default_value.
    #[serde(default)]
    pub filters: Vec<FilterRef>,
}

/// Универсальный контекст запроса для DataView.
///
/// Содержит два периода явно (не в `extra`), фильтры и произвольные параметры.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ViewContext {
    /// Период 1: начало (YYYY-MM-DD)
    pub date_from: String,
    /// Период 1: конец (YYYY-MM-DD)
    pub date_to: String,
    /// Период 2: начало (если None — view вычисляет сам, обычно -1 месяц)
    #[serde(default)]
    pub period2_from: Option<String>,
    /// Период 2: конец (если None — view вычисляет сам)
    #[serde(default)]
    pub period2_to: Option<String>,
    /// Фильтр по кабинетам МП (пустой = все)
    #[serde(default)]
    pub connection_mp_refs: Vec<String>,
    /// Произвольные параметры, специфичные для конкретного view
    #[serde(default)]
    pub params: HashMap<String, String>,
}

impl From<&IndicatorContext> for ViewContext {
    fn from(ctx: &IndicatorContext) -> Self {
        Self {
            date_from: ctx.date_from.clone(),
            date_to: ctx.date_to.clone(),
            period2_from: ctx.extra.get("period2_from").cloned(),
            period2_to: ctx.extra.get("period2_to").cloned(),
            connection_mp_refs: ctx.connection_mp_refs.clone(),
            params: ctx.extra.clone(),
        }
    }
}
