//! Global Filter Registry
//!
//! Единый реестр всех допустимых фильтров системы.
//! FilterDef описывает тип фильтра и UI-компонент — без бизнес-контекста.
//! DataView ссылается на фильтры через FilterRef (filter_id + required + order).
//!
//! Конвенция именования ID:
//!   date_range_1  — период 1 (DateRange: from=date_from, to=date_to)
//!   date_range_2  — период 2 (DateRange: from=period2_from, to=period2_to)
//!   connection_mp_refs — кабинеты МП

use std::collections::HashMap;

use contracts::shared::data_view::{FilterDef, FilterKind};

/// Возвращает полный глобальный реестр фильтров: id → FilterDef.
pub fn global_filter_registry() -> HashMap<String, FilterDef> {
    let defs: Vec<FilterDef> = vec![
        // ── Период 1 ─────────────────────────────────────────────────────────
        FilterDef {
            id: "date_range_1".into(),
            label: "Период 1".into(),
            kind: FilterKind::DateRange {
                from_id: "date_range_1_from".into(),
                to_id: "date_range_1_to".into(),
            },
        },
        // ── Период 2 ─────────────────────────────────────────────────────────
        FilterDef {
            id: "date_range_2".into(),
            label: "Период 2 (сравнение)".into(),
            kind: FilterKind::DateRange {
                from_id: "date_range_2_from".into(),
                to_id: "date_range_2_to".into(),
            },
        },
        // ── Кабинеты МП ──────────────────────────────────────────────────────
        FilterDef {
            id: "connection_mp_refs".into(),
            label: "Кабинет МП".into(),
            kind: FilterKind::MultiSelect {
                source: "connection_mp".into(),
            },
        },
        // Метрика (resource selector) не входит в реестр фильтров —
        // она строится из DataViewMeta.available_resources на стороне UI.
    ];

    defs.into_iter().map(|f| (f.id.clone(), f)).collect()
}
