//! DataSpec tab — schema_id, sql_artifact_id, query_config JSON + live compute test

use super::super::model::{self, IndicatorSchemaMeta};
use super::super::view_model::BiIndicatorDetailsVm;
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::icons::icon;
use leptos::prelude::*;
use thaw::*;
use wasm_bindgen::JsCast;

#[component]
pub fn DataSpecTab(vm: BiIndicatorDetailsVm) -> impl IntoView {
    // Load indicator catalog on mount
    let catalog: RwSignal<Vec<IndicatorSchemaMeta>> = RwSignal::new(vec![]);
    leptos::task::spawn_local(async move {
        if let Ok(items) = model::fetch_indicator_catalog().await {
            catalog.set(items);
        }
    });

    // Derived: description of currently selected schema_id
    let schema_id_sig = vm.data_spec_schema_id;
    let schema_desc = Signal::derive(move || {
        let id = schema_id_sig.get();
        catalog
            .get()
            .into_iter()
            .find(|m| m.id == id)
            .and_then(|m| m.description)
            .unwrap_or_default()
    });

    let vm_test = vm.clone();
    let run_test = move |_| vm_test.run_test();

    let test_loading = vm.test_loading;
    let test_error = vm.test_error;
    let test_result = vm.test_result;

    view! {
        <div class="detail-grid">

            // ── Card 1: source identity ────────────────────────────────────
            <CardAnimated delay_ms=0>
                <h4 class="details-section__title">
                    {icon("database")} " Источник данных"
                </h4>
                <div class="details-grid--2col">
                    <div class="form__group" style="grid-column: 1 / 2;">
                        <label class="form__label">"Schema ID"</label>

                        // datalist for autocomplete
                        <datalist id="schema-ids-list">
                            {move || catalog.get().into_iter().map(|m| {
                                let label = format!("{} — {}", m.id, m.label);
                                view! { <option value=m.id>{label}</option> }
                            }).collect_view()}
                        </datalist>

                        // Native input with datalist — gives autocomplete from catalog
                        <input
                            type="text"
                            class="form__input"
                            list="schema-ids-list"
                            placeholder="sales_revenue"
                            prop:value=move || schema_id_sig.get()
                            on:input=move |ev| {
                                let input: web_sys::HtmlInputElement =
                                    ev.target().unwrap().unchecked_into();
                                schema_id_sig.set(input.value());
                            }
                        />

                        // Schema description hint
                        {move || {
                            let desc = schema_desc.get();
                            if desc.is_empty() {
                                view! {
                                    <span class="form__hint">
                                        "Введите ID или выберите из списка. "
                                        "Примеры: " <code>"sales_revenue"</code>
                                        ", " <code>"sales_order_count"</code>
                                    </span>
                                }.into_any()
                            } else {
                                view! {
                                    <span class="form__hint" style="color: var(--color-text-primary);">
                                        {icon("info")} " " {desc}
                                    </span>
                                }.into_any()
                            }
                        }}

                        // Catalog quick-select chips (shown when catalog loaded)
                        {move || {
                            let items = catalog.get();
                            if items.is_empty() {
                                return None;
                            }
                            Some(view! {
                                <div style="margin-top: 6px; display: flex; gap: 6px; flex-wrap: wrap;">
                                    {items.into_iter().map(|m| {
                                        let id_clone = m.id.clone();
                                        let label = m.label.clone();
                                        let is_selected = Signal::derive(move || schema_id_sig.get() == id_clone);
                                        let id_for_click = m.id.clone();
                                        view! {
                                            <button
                                                class="schema-chip"
                                                class:schema-chip--active=is_selected
                                                on:click=move |_| schema_id_sig.set(id_for_click.clone())
                                                type="button"
                                            >
                                                {label}
                                            </button>
                                        }
                                    }).collect_view()}
                                </div>
                            })
                        }}
                    </div>

                    <div class="form__group">
                        <label class="form__label">"SQL артефакт (ID)"</label>
                        <Input
                            value=vm.data_spec_sql_artifact_id
                            placeholder="UUID артефакта a019 (необязательно)"
                        />
                        <span class="form__hint">"Ссылка на SQL-артефакт из a019_llm_artifact"</span>
                    </div>
                </div>
            </CardAnimated>

            // ── Card 2: Query Config JSON ──────────────────────────────────
            <CardAnimated delay_ms=50>
                <h4 class="details-section__title">
                    {icon("code")} " Query Config (JSON)"
                </h4>
                <div class="form__group">
                    <label class="form__label">"Конфигурация запроса (DashboardConfig)"</label>
                    <Textarea
                        value=vm.data_spec_query_config_json
                        placeholder="{}"
                        attr:rows=22
                        attr:style="font-family: 'JetBrains Mono', 'Fira Code', monospace; \
                                    font-size: 12px; width: 100%; min-height: 420px; \
                                    resize: vertical; line-height: 1.5;"
                    />
                    <span class="form__hint">
                        "Поля: " <code>"data_source"</code> ", "
                        <code>"selected_fields"</code> ", "
                        <code>"groupings"</code> ", "
                        <code>"filters"</code> ", "
                        <code>"sort"</code>
                    </span>
                </div>
            </CardAnimated>

            // ── Card 3: Live test ──────────────────────────────────────────
            <CardAnimated delay_ms=100>
                <h4 class="details-section__title">
                    {icon("play")} " Тест на реальных данных"
                </h4>

                <div class="details-grid--3col">
                    <div class="form__group">
                        <label class="form__label">"Дата с"</label>
                        <input
                            type="date"
                            class="form__input"
                            prop:value=move || vm.test_date_from.get()
                            on:input=move |ev| {
                                let input: web_sys::HtmlInputElement =
                                    ev.target().unwrap().unchecked_into();
                                vm.test_date_from.set(input.value());
                            }
                        />
                    </div>

                    <div class="form__group">
                        <label class="form__label">"Дата по"</label>
                        <input
                            type="date"
                            class="form__input"
                            prop:value=move || vm.test_date_to.get()
                            on:input=move |ev| {
                                let input: web_sys::HtmlInputElement =
                                    ev.target().unwrap().unchecked_into();
                                vm.test_date_to.set(input.value());
                            }
                        />
                    </div>

                    <div class="form__group">
                        <label class="form__label">"Кабинеты (через запятую или перенос)"</label>
                        <Textarea
                            value=vm.test_connection_ids
                            placeholder="Пусто = все кабинеты"
                            attr:rows=2
                            attr:style="font-family: monospace; font-size: 12px; width: 100%;"
                        />
                    </div>
                </div>

                <div style="margin-top: var(--spacing-md); display: flex; gap: var(--spacing-sm); align-items: center;">
                    <Button
                        appearance=ButtonAppearance::Primary
                        on_click=run_test
                        disabled=test_loading
                    >
                        {icon("play")} " Выполнить"
                    </Button>

                    {move || test_loading.get().then(|| view! {
                        <span style="color: var(--color-text-secondary); font-size: 13px;">
                            "Вычисление..."
                        </span>
                    })}
                </div>

                // Error
                {move || test_error.get().map(|e| view! {
                    <div
                        class="warning-box"
                        style="margin-top: var(--spacing-sm); \
                               background: var(--color-error-50); \
                               border-color: var(--color-error-100);"
                    >
                        <span class="warning-box__icon" style="color: var(--color-error);">"⚠"</span>
                        <span class="warning-box__text" style="color: var(--color-error);">{e}</span>
                    </div>
                })}

                // Result
                {move || test_result.get().map(|r| {
                    let value_str = r.value.map(|v| format!("{v:.4}")).unwrap_or_else(|| "—".into());
                    let prev_str = r.previous_value.map(|v| format!("{v:.4}")).unwrap_or_else(|| "—".into());
                    let delta_str = r.change_percent
                        .map(|p| {
                            let sign = if p >= 0.0 { "+" } else { "" };
                            format!("{sign}{p:.2}%")
                        })
                        .unwrap_or_else(|| "—".into());
                    let status_str = r.status.clone();
                    let subtitle = r.subtitle.unwrap_or_default();

                    let status_color = match status_str.as_str() {
                        "Good" => "var(--color-success)",
                        "Bad" => "var(--color-error)",
                        "Warning" => "var(--color-warning)",
                        _ => "var(--color-text-secondary)",
                    };

                    view! {
                        <div
                            class="data-spec-result"
                            style="margin-top: var(--spacing-md); \
                                   padding: var(--spacing-md); \
                                   border: 1px solid var(--color-border); \
                                   border-radius: var(--radius-md); \
                                   background: var(--color-surface-2);"
                        >
                            <div style="display: flex; gap: var(--spacing-lg); flex-wrap: wrap; align-items: baseline;">
                                <div>
                                    <span style="font-size: 11px; color: var(--color-text-secondary); text-transform: uppercase; letter-spacing: .05em;">"Значение"</span>
                                    <div style="font-size: 28px; font-weight: 700; font-family: monospace;">
                                        {value_str}
                                    </div>
                                </div>
                                <div>
                                    <span style="font-size: 11px; color: var(--color-text-secondary); text-transform: uppercase; letter-spacing: .05em;">"Предыдущий"</span>
                                    <div style="font-size: 18px; color: var(--color-text-secondary); font-family: monospace;">
                                        {prev_str}
                                    </div>
                                </div>
                                <div>
                                    <span style="font-size: 11px; color: var(--color-text-secondary); text-transform: uppercase; letter-spacing: .05em;">"Δ %"</span>
                                    <div style="font-size: 18px; font-weight: 600; font-family: monospace;">
                                        {delta_str}
                                    </div>
                                </div>
                                <div>
                                    <span style="font-size: 11px; color: var(--color-text-secondary); text-transform: uppercase; letter-spacing: .05em;">"Статус"</span>
                                    <div
                                        style=format!(
                                            "font-size: 15px; font-weight: 600; color: {status_color};"
                                        )
                                    >
                                        {status_str}
                                    </div>
                                </div>
                            </div>

                            {(!subtitle.is_empty()).then(|| view! {
                                <div style="margin-top: var(--spacing-sm); \
                                            font-size: 13px; \
                                            color: var(--color-text-secondary);">
                                    {subtitle}
                                </div>
                            })}

                        </div>
                    }
                })}
            </CardAnimated>
        </div>
    }
}
