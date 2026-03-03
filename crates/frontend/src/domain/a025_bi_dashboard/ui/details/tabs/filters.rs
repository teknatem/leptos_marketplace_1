//! Filters tab — global filters editor (key/label/value table)

use super::super::view_model::BiDashboardDetailsVm;
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::icons::icon;
use leptos::prelude::*;
use thaw::*;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct GlobalFilterRow {
    pub key: String,
    pub label: String,
    pub value: String,
}

fn parse_filters(json: &str) -> Vec<GlobalFilterRow> {
    serde_json::from_str(json).unwrap_or_default()
}

fn serialize_filters(rows: &[GlobalFilterRow]) -> String {
    serde_json::to_string_pretty(rows).unwrap_or_else(|_| "[]".to_string())
}

#[component]
pub fn FiltersTab(vm: BiDashboardDetailsVm) -> impl IntoView {
    let filters_sig = vm.global_filters_json;

    let rows = RwSignal::new(parse_filters(&filters_sig.get_untracked()));

    // Sync rows → JSON
    let sync_to_vm = move || {
        filters_sig.set(serialize_filters(&rows.get_untracked()));
    };

    let on_add = {
        let rows = rows.clone();
        let sync = sync_to_vm.clone();
        move |_| {
            rows.update(|v| v.push(GlobalFilterRow {
                key: String::new(),
                label: String::new(),
                value: String::new(),
            }));
            sync();
        }
    };

    view! {
        <div class="details-tabs__content">
            <CardAnimated delay_ms=0>
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
                        "Ключ фильтра должен совпадать с "
                        <code>"global_filter_key"</code>
                        " в параметрах индикатора (a024). Значение по умолчанию будет применено ко всем индикаторам дашборда."
                    </p>

                    <table class="data-table data-table--compact">
                        <thead>
                            <tr>
                                <th>"Ключ"</th>
                                <th>"Метка"</th>
                                <th>"Значение по умолчанию"</th>
                                <th></th>
                            </tr>
                        </thead>
                        <tbody>
                            {move || {
                                let current_rows = rows.get();
                                current_rows.into_iter().enumerate().map(|(i, row)| {
                                    let rows_key = rows.clone();
                                    let rows_label = rows.clone();
                                    let rows_value = rows.clone();
                                    let rows_del = rows.clone();
                                    let sync_key = sync_to_vm.clone();
                                    let sync_label = sync_to_vm.clone();
                                    let sync_value = sync_to_vm.clone();
                                    let sync_del = sync_to_vm.clone();

                                    view! {
                                        <tr>
                                            <td>
                                                <input
                                                    type="text"
                                                    class="form__input form__input--sm"
                                                    value=row.key.clone()
                                                    placeholder="date_range"
                                                    on:input=move |ev| {
                                                        use wasm_bindgen::JsCast;
                                                        let val = ev.target().unwrap()
                                                            .unchecked_into::<web_sys::HtmlInputElement>()
                                                            .value();
                                                        rows_key.update(|v| {
                                                            if let Some(r) = v.get_mut(i) { r.key = val; }
                                                        });
                                                        sync_key();
                                                    }
                                                />
                                            </td>
                                            <td>
                                                <input
                                                    type="text"
                                                    class="form__input form__input--sm"
                                                    value=row.label.clone()
                                                    placeholder="Период"
                                                    on:input=move |ev| {
                                                        use wasm_bindgen::JsCast;
                                                        let val = ev.target().unwrap()
                                                            .unchecked_into::<web_sys::HtmlInputElement>()
                                                            .value();
                                                        rows_label.update(|v| {
                                                            if let Some(r) = v.get_mut(i) { r.label = val; }
                                                        });
                                                        sync_label();
                                                    }
                                                />
                                            </td>
                                            <td>
                                                <input
                                                    type="text"
                                                    class="form__input form__input--sm"
                                                    value=row.value.clone()
                                                    placeholder="last_30_days"
                                                    on:input=move |ev| {
                                                        use wasm_bindgen::JsCast;
                                                        let val = ev.target().unwrap()
                                                            .unchecked_into::<web_sys::HtmlInputElement>()
                                                            .value();
                                                        rows_value.update(|v| {
                                                            if let Some(r) = v.get_mut(i) { r.value = val; }
                                                        });
                                                        sync_value();
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
                        view! { <div></div> }.into_any()
                    }}
                </div>
            </CardAnimated>
        </div>
    }
}
