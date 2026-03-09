//! Filters tab — global filters editor (key/label/value table)

use super::super::view_model::BiDashboardDetailsVm;
use crate::data_view::api as dv_api;
use crate::data_view::types::{FilterDef, FilterRef};
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::icons::icon;
use leptos::prelude::*;
use thaw::*;

fn parse_filters(json: &str) -> Vec<FilterRef> {
    serde_json::from_str(json).unwrap_or_default()
}

fn serialize_filters(rows: &[FilterRef]) -> String {
    serde_json::to_string_pretty(rows).unwrap_or_else(|_| "[]".to_string())
}

#[component]
pub fn FiltersTab(vm: BiDashboardDetailsVm) -> impl IntoView {
    let filters_sig = vm.filters_json;
    let registry = RwSignal::new(Vec::<FilterDef>::new());
    let registry_loading = RwSignal::new(true);
    let registry_error = RwSignal::new(None::<String>);

    let rows = RwSignal::new(parse_filters(&filters_sig.get_untracked()));

    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            match dv_api::fetch_global_filters().await {
                Ok(mut defs) => {
                    defs.sort_by(|a, b| a.id.cmp(&b.id));
                    registry.set(defs);
                    registry_error.set(None);
                }
                Err(err) => registry_error.set(Some(err)),
            }
            registry_loading.set(false);
        });
    });

    // Sync rows → JSON
    let sync_to_vm = move || {
        filters_sig.set(serialize_filters(&rows.get_untracked()));
    };

    let on_add = {
        let rows = rows.clone();
        let sync = sync_to_vm.clone();
        move |_| {
            rows.update(|v| {
                v.push(FilterRef {
                    filter_id: String::new(),
                    required: false,
                    order: v.len() as u32,
                    default_value: None,
                    label_override: None,
                })
            });
            sync();
        }
    };

    view! {
        <div class="details-tabs__content">
            <CardAnimated delay_ms=0 nav_id="a025_bi_dashboard_details_filters_main">
                <div class="details-section">
                    <div class="details-section__header">
                        <h4 class="details-section__title">"Глобальные фильтры"</h4>
                        <Button
                            appearance=ButtonAppearance::Secondary
                            size=ButtonSize::Small
                            on_click=on_add
                        >
                            {icon("plus")} " Добавить"
                        </Button>
                    </div>
                    <p class="form__hint">
                        "Дашборд хранит "
                        <code>"Vec<FilterRef>"</code>
                        " и ссылается на глобальный реестр фильтров DataView. "
                        "Здесь задаются порядок, обязательность, default_value и label_override."
                    </p>

                    {move || registry_loading.get().then(|| view! {
                        <div class="form__hint">{icon("loader")} " Загрузка реестра фильтров..."</div>
                    })}
                    {move || registry_error.get().map(|e| view! {
                        <div class="form__hint" style="color: var(--color-error);">
                            {icon("alert-circle")} " " {e}
                        </div>
                    })}

                    <table class="data-table data-table--compact">
                        <thead>
                            <tr>
                                <th>"Фильтр"</th>
                                <th>"Обязателен"</th>
                                <th>"Порядок"</th>
                                <th>"Значение по умолчанию"</th>
                                <th>"Переопред. метки"</th>
                                <th></th>
                            </tr>
                        </thead>
                        <tbody>
                            {move || {
                                let current_rows = rows.get();
                                current_rows.into_iter().enumerate().map(|(i, row)| {
                                    let rows_id = rows.clone();
                                    let rows_required = rows.clone();
                                    let rows_order = rows.clone();
                                    let rows_default = rows.clone();
                                    let rows_label_override = rows.clone();
                                    let rows_del = rows.clone();
                                    let sync_id = sync_to_vm.clone();
                                    let sync_required = sync_to_vm.clone();
                                    let sync_order = sync_to_vm.clone();
                                    let sync_default = sync_to_vm.clone();
                                    let sync_label_override = sync_to_vm.clone();
                                    let sync_del = sync_to_vm.clone();
                                    let registry_defs = registry.get();

                                    view! {
                                        <tr>
                                            <td>
                                                <select
                                                    class="form__select form__select--sm"
                                                    on:change=move |ev| {
                                                        use wasm_bindgen::JsCast;
                                                        let val = ev.target().unwrap()
                                                            .unchecked_into::<web_sys::HtmlSelectElement>()
                                                            .value();
                                                        rows_id.update(|v| {
                                                            if let Some(r) = v.get_mut(i) { r.filter_id = val; }
                                                        });
                                                        sync_id();
                                                    }
                                                >
                                                    <option value="">"— выбрать filter_id —"</option>
                                                    {registry_defs.into_iter().map(|def| {
                                                        let is_selected = row.filter_id == def.id;
                                                        view! {
                                                            <option value=def.id.clone() selected=is_selected>
                                                                {format!("{} — {}", def.id, def.label)}
                                                            </option>
                                                        }
                                                    }).collect_view()}
                                                </select>
                                            </td>
                                            <td>
                                                <input
                                                    type="checkbox"
                                                    prop:checked=row.required
                                                    on:change=move |ev| {
                                                        use wasm_bindgen::JsCast;
                                                        let checked = ev.target().unwrap()
                                                            .unchecked_into::<web_sys::HtmlInputElement>()
                                                            .checked();
                                                        rows_required.update(|v| {
                                                            if let Some(r) = v.get_mut(i) { r.required = checked; }
                                                        });
                                                        sync_required();
                                                    }
                                                />
                                            </td>
                                            <td>
                                                <input
                                                    type="number"
                                                    class="form__input form__input--sm"
                                                    value=row.order.to_string()
                                                    min="0"
                                                    on:input=move |ev| {
                                                        use wasm_bindgen::JsCast;
                                                        let val = ev.target().unwrap()
                                                            .unchecked_into::<web_sys::HtmlInputElement>()
                                                            .value();
                                                        rows_order.update(|v| {
                                                            if let Some(r) = v.get_mut(i) {
                                                                r.order = val.parse::<u32>().unwrap_or(0);
                                                            }
                                                        });
                                                        sync_order();
                                                    }
                                                />
                                            </td>
                                            <td>
                                                <input
                                                    type="text"
                                                    class="form__input form__input--sm"
                                                    value=row.default_value.clone().unwrap_or_default()
                                                    placeholder="Напр. 2025-01-01,2025-01-31"
                                                    on:input=move |ev| {
                                                        use wasm_bindgen::JsCast;
                                                        let val = ev.target().unwrap()
                                                            .unchecked_into::<web_sys::HtmlInputElement>()
                                                            .value();
                                                        rows_default.update(|v| {
                                                            if let Some(r) = v.get_mut(i) {
                                                                r.default_value = if val.trim().is_empty() { None } else { Some(val) };
                                                            }
                                                        });
                                                        sync_default();
                                                    }
                                                />
                                            </td>
                                            <td>
                                                <input
                                                    type="text"
                                                    class="form__input form__input--sm"
                                                    value=row.label_override.clone().unwrap_or_default()
                                                    placeholder="Своя метка"
                                                    on:input=move |ev| {
                                                        use wasm_bindgen::JsCast;
                                                        let val = ev.target().unwrap()
                                                            .unchecked_into::<web_sys::HtmlInputElement>()
                                                            .value();
                                                        rows_label_override.update(|v| {
                                                            if let Some(r) = v.get_mut(i) {
                                                                r.label_override = if val.trim().is_empty() { None } else { Some(val) };
                                                            }
                                                        });
                                                        sync_label_override();
                                                    }
                                                />
                                            </td>
                                            <td>
                                                <Button
                                                    size=ButtonSize::Small
                                                    appearance=ButtonAppearance::Secondary
                                                    on_click=move |_| {
                                                        rows_del.update(|v| { v.remove(i); });
                                                        sync_del();
                                                    }
                                                >
                                                    {icon("trash-2")}
                                                </Button>
                                            </td>
                                        </tr>
                                    }
                                }).collect::<Vec<_>>()
                            }}
                        </tbody>
                    </table>

                    {move || if rows.get().is_empty() {
                        view! {
                            <div class="placeholder placeholder--small">
                                "Нет глобальных фильтров. Нажмите «Добавить» для создания."
                            </div>
                        }.into_any()
                    } else {
                        view! { <></> }.into_any()
                    }}
                </div>
            </CardAnimated>
        </div>
    }
}
