//! FilterBar — автоматически строит панель фильтров из Vec<FilterDef>.
//!
//! Каждый FilterDef рендерится в UI-компонент по своему FilterKind.
//! Компонент управляет двумя сигналами:
//!   - view_context: RwSignal<ViewContext> — для DateRange/MultiSelect
//!   - extra_params:  RwSignal<HashMap<String, String>> — для Select/Text (→ params)

use leptos::prelude::*;
use thaw::*;

use crate::data_view::types::{FilterDef, FilterKind};
use crate::shared::components::date_range_picker_smart::DateRangePickerSmart;
use crate::shared::filters::ConnectionMpMultiSelect;
use contracts::shared::data_view::ViewContext;

// ── FilterBar component ───────────────────────────────────────────────────────

/// Автоматически строит панель фильтров по Vec<FilterDef>.
///
/// Состояние фильтров хранится в `ctx` (ViewContext).
/// Фильтры типа Select/Text пишутся в `ctx.params` по ключу `filter.id`.
/// Специальные фильтры DateFrom/DateTo/MultiSelect пишутся напрямую в поля ViewContext.
///
/// Маппинг filter_id → поле ViewContext:
///   date_range_1_from  → ctx.date_from
///   date_range_1_to    → ctx.date_to
///   date_range_2_from  → ctx.period2_from
///   date_range_2_to    → ctx.period2_to
///   connection_mp_refs → ctx.connection_mp_refs (comma-separated)
///   всё остальное      → ctx.params[filter.id]
#[component]
#[allow(non_snake_case)]
pub fn FilterBar(
    filters: Vec<FilterDef>,
    ctx: RwSignal<ViewContext>,
    /// Render filters in a single column (e.g. inside a narrow drawer).
    #[prop(optional)]
    single_col: bool,
) -> impl IntoView {
    let cls = if single_col {
        "filter-bar filter-bar--single-col"
    } else {
        "filter-bar"
    };
    view! {
        <div class=cls>
            {filters.into_iter().map(|def| render_filter(def, ctx)).collect_view()}
        </div>
    }
}

fn render_filter(def: FilterDef, ctx: RwSignal<ViewContext>) -> AnyView {
    let label = def.label.clone();
    let id = def.id.clone();

    match def.kind {
        // ── DateRange ────────────────────────────────────────────────────────
        FilterKind::DateRange { from_id, to_id } => {
            let f_id = from_id.clone();
            let t_id = to_id.clone();

            let date_from_sig = Signal::derive(move || {
                let c = ctx.get();
                match f_id.as_str() {
                    "date_range_1_from" => c.date_from,
                    "date_range_2_from" => c.period2_from.unwrap_or_default(),
                    _ => c.params.get(&f_id).cloned().unwrap_or_default(),
                }
            });

            let date_to_sig = Signal::derive(move || {
                let c = ctx.get();
                match t_id.as_str() {
                    "date_range_1_to" => c.date_to,
                    "date_range_2_to" => c.period2_to.unwrap_or_default(),
                    _ => c.params.get(&t_id).cloned().unwrap_or_default(),
                }
            });

            let f_id2 = from_id.clone();
            let t_id2 = to_id.clone();
            let on_change = Callback::new(move |(f, t): (String, String)| {
                ctx.update(|c| {
                    match f_id2.as_str() {
                        "date_range_1_from" => c.date_from = f,
                        "date_range_2_from" => {
                            c.period2_from = if f.is_empty() { None } else { Some(f) }
                        }
                        _ => {
                            c.params.insert(f_id2.clone(), f);
                        }
                    }
                    match t_id2.as_str() {
                        "date_range_1_to" => c.date_to = t,
                        "date_range_2_to" => {
                            c.period2_to = if t.is_empty() { None } else { Some(t) }
                        }
                        _ => {
                            c.params.insert(t_id2.clone(), t);
                        }
                    }
                });
            });

            view! {
                <div class="filter-bar__field filter-bar__field--date-range">
                    <DateRangePickerSmart
                        date_from=date_from_sig
                        date_to=date_to_sig
                        on_change=on_change
                        label=label
                    />
                </div>
            }
            .into_any()
        }

        // ── MultiSelect ──────────────────────────────────────────────────────
        FilterKind::MultiSelect { source } if source == "connection_mp" => {
            // Use the dedicated ConnectionMpMultiSelect component that loads
            // options from the backend and renders a collapsible checkbox panel.
            let local_sel = RwSignal::new(ctx.get_untracked().connection_mp_refs.clone());
            // Sync selection changes back into the shared ViewContext.
            Effect::new(move |_| {
                let ids = local_sel.get();
                ctx.update(|c| c.connection_mp_refs = ids);
            });
            view! {
                <div class="filter-bar__field">
                    <label class="filter-bar__label">{label}</label>
                    <ConnectionMpMultiSelect selected=local_sel />
                </div>
            }
            .into_any()
        }

        FilterKind::MultiSelect { source: _ } => {
            // Generic fallback: comma/newline-separated text input.
            let id_clone = id.clone();
            let value_getter = move || ctx.get().params.get(&id_clone).cloned().unwrap_or_default();
            let id_clone2 = id.clone();
            let text_signal = RwSignal::new(value_getter());
            Effect::new(move |_| text_signal.set(value_getter()));
            let on_change = move |_| {
                let val = text_signal.get();
                ctx.update(|c| {
                    c.params.insert(id_clone2.clone(), val.clone());
                });
            };
            let control_id = format!("filter-{id}");
            view! {
                <div class="filter-bar__field">
                    <label class="filter-bar__label" for=control_id.clone()>{label}</label>
                    <div class="filter-bar__description">
                        "Через запятую или с новой строки. Пусто = все значения."
                    </div>
                    <Textarea
                        value=text_signal
                        placeholder="Через запятую, пусто = все"
                        attr:id=control_id
                        attr:rows=2
                        attr:class="filter-bar__textarea"
                        on_blur=on_change
                    />
                </div>
            }
            .into_any()
        }

        // ── Select ───────────────────────────────────────────────────────────
        FilterKind::Select { options } => {
            let id_for_init = id.clone();
            let id_for_effect = id.clone();
            let local_val = RwSignal::new(
                ctx.get_untracked()
                    .params
                    .get(&id_for_init)
                    .cloned()
                    .unwrap_or_default(),
            );
            Effect::new(move |_| {
                let val = local_val.get();
                ctx.update(|c| {
                    c.params.insert(id_for_effect.clone(), val);
                });
            });
            let control_id = format!("filter-{id}");

            view! {
                <div class="filter-bar__field">
                    <label class="filter-bar__label" for=control_id>{label}</label>
                    <Select value=local_val>
                        <option value="">"— не выбрано —"</option>
                        {options.into_iter().map(|opt| {
                            view! { <option value=opt.value>{opt.label}</option> }
                        }).collect_view()}
                    </Select>
                </div>
            }
            .into_any()
        }

        // ── Text ─────────────────────────────────────────────────────────────
        FilterKind::Text => {
            let id_clone = id.clone();
            let value_getter = move || ctx.get().params.get(&id_clone).cloned().unwrap_or_default();
            let text_signal = RwSignal::new(value_getter());
            Effect::new(move |_| text_signal.set(value_getter()));
            let id_clone2 = id.clone();
            let on_change = move |_| {
                let val = text_signal.get();
                ctx.update(|c| {
                    c.params.insert(id_clone2.clone(), val.clone());
                });
            };
            let control_id = format!("filter-{id}");

            view! {
                <div class="filter-bar__field">
                    <label class="filter-bar__label" for=control_id.clone()>{label}</label>
                    <Input
                        value=text_signal
                        placeholder="Поиск..."
                        attr:id=control_id
                        class="filter-bar__control"
                        on_blur=on_change
                    />
                </div>
            }
            .into_any()
        }
    }
}

/// Применить значение по умолчанию из FilterRef к ViewContext.
///
/// Для `DateRange` фильтров `default_value` задаётся как `"from,to"` (например
/// `"2025-01-01,2025-01-31"`). Остальные типы — строка напрямую.
pub fn apply_defaults(ctx: &mut ViewContext, filter_id: &str, default_value: &str) {
    match filter_id {
        "date_range_1" => {
            let mut parts = default_value.splitn(2, ',');
            if let Some(f) = parts.next() {
                ctx.date_from = f.trim().to_string();
            }
            if let Some(t) = parts.next() {
                ctx.date_to = t.trim().to_string();
            }
        }
        "date_range_2" => {
            let mut parts = default_value.splitn(2, ',');
            if let Some(f) = parts.next() {
                ctx.period2_from = Some(f.trim().to_string());
            }
            if let Some(t) = parts.next() {
                ctx.period2_to = Some(t.trim().to_string());
            }
        }
        "connection_mp_refs" => {
            ctx.connection_mp_refs = default_value
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
        other => {
            ctx.params
                .insert(other.to_string(), default_value.to_string());
        }
    }
}
