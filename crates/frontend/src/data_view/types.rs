//! Frontend mirror of DataViewMeta structs (from contracts::shared::data_view).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionMeta {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMeta {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataViewMeta {
    pub id: String,
    pub name: String,
    pub category: String,
    pub version: u32,
    pub description: String,
    pub ai_description: String,
    pub data_sources: Vec<String>,
    pub available_dimensions: Vec<DimensionMeta>,
    #[serde(default)]
    pub available_resources: Vec<ResourceMeta>,
    #[serde(default)]
    pub filters: Vec<FilterRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DrilldownDimensionCapability {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub coverage_pct: Option<f64>,
    #[serde(default)]
    pub supported_turnover_codes: Vec<String>,
    #[serde(default)]
    pub missing_turnover_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DrilldownCapabilitiesResponse {
    #[serde(default)]
    pub safe_dimensions: Vec<DrilldownDimensionCapability>,
    #[serde(default)]
    pub partial_dimensions: Vec<DrilldownDimensionCapability>,
}

// ── Filter Registry types ─────────────────────────────────────────────────────

/// Один вариант для Select-фильтра.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

/// Тип фильтра — определяет UI-компонент для рендеринга.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FilterKind {
    /// Парный выбор диапазона дат — рендерится как DateRangePicker.
    DateRange {
        from_id: String,
        to_id: String,
    },
    MultiSelect {
        source: String,
    },
    Select {
        options: Vec<SelectOption>,
    },
    Text,
}

/// Определение фильтра из глобального реестра.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterDef {
    pub id: String,
    pub label: String,
    pub kind: FilterKind,
}

/// Использование фильтра в конкретном DataView.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterRef {
    pub filter_id: String,
    pub required: bool,
    pub order: u32,
    #[serde(default)]
    pub default_value: Option<String>,
    #[serde(default)]
    pub label_override: Option<String>,
}

/// Ответ эндпоинта GET /api/data-view/filters (полный реестр).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalFiltersResponse {
    pub filters: Vec<FilterDef>,
}

/// Ответ эндпоинта GET /api/data-view/:id/filters (фильтры конкретного view).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewFiltersResponse {
    pub filters: Vec<FilterDef>,
}

/// Хелпер: все фильтры как HashMap по id.
pub type FilterRegistry = HashMap<String, FilterDef>;
