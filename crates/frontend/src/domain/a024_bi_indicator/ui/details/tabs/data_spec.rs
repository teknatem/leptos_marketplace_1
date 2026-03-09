//! DataSpec tab — DataView config + live test

use super::super::view_model::BiIndicatorDetailsVm;
use crate::data_view::api as dv_api;
use crate::data_view::types::DataViewMeta;
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::icons::icon;
use leptos::prelude::*;
use thaw::*;

fn input_date_value(ev: &leptos::ev::Event) -> String {
    use wasm_bindgen::JsCast;
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|el| el.value())
        .unwrap_or_default()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Main tab component
// ═══════════════════════════════════════════════════════════════════════════════

#[component]
pub fn DataSpecTab(vm: BiIndicatorDetailsVm) -> impl IntoView {
    let vm_test = vm.clone();
    let run_test = move |_| vm_test.run_test();
    let test_loading = vm.test_loading;
    let test_error = vm.test_error;
    let test_result = vm.test_result;
    let dsc_view_id = vm.dsc_view_id;

    // ── DataView list state ─────────────────────────────────────────────────
    let dv_list: RwSignal<Vec<DataViewMeta>> = RwSignal::new(vec![]);
    let dv_loading: RwSignal<bool> = RwSignal::new(true);
    let dv_error: RwSignal<Option<String>> = RwSignal::new(None);
    let dv_search: RwSignal<String> = RwSignal::new(String::new());
    let drawer_open: RwSignal<bool> = RwSignal::new(false);

    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            match dv_api::fetch_list().await {
                Ok(list) => { dv_list.set(list); }
                Err(e)   => { dv_error.set(Some(e)); }
            }
            dv_loading.set(false);
        });
    });

    let filtered_list = Signal::derive(move || {
        let q = dv_search.get().trim().to_lowercase();
        dv_list.with(|list| {
            if q.is_empty() {
                list.clone()
            } else {
                list.iter()
                    .filter(|dv| {
                        dv.id.to_lowercase().contains(&q)
                            || dv.name.to_lowercase().contains(&q)
                            || dv.category.to_lowercase().contains(&q)
                    })
                    .cloned()
                    .collect()
            }
        })
    });

    view! {
        <div class="detail-grid">

            // ── Card: DataView ─────────────────────────────────────────────────
            <CardAnimated delay_ms=0>
                <h4 class="details-section__title">
                    {icon("zap")} " DataView — Семантический слой"
                </h4>
                <p class="form__hint" style="margin-bottom: 12px;">
                    "Выберите именованное вычисление из DataViewRegistry. "
                    "Определяет логику получения значения индикатора и drill-down."
                </p>

                // ── Active banner ──────────────────────────────────────────────
                {move || {
                    let vid = dsc_view_id.get();
                    if vid.trim().is_empty() {
                        view! {
                            <div class="dv-status-banner dv-status-banner--empty">
                                {icon("alert-circle")}
                                " DataView не задан — индикатор не вычисляется."
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="dv-status-banner dv-status-banner--active">
                                {icon("check-circle")}
                                " Активен: "
                                <code>{vid}</code>
                                <button
                                    class="dv-status-banner__clear"
                                    title="Очистить"
                                    on:click=move |_| dsc_view_id.set(String::new())
                                >"×"</button>
                            </div>
                        }.into_any()
                    }
                }}

                <div style="margin-top: 14px;">
                    <Button
                        appearance=ButtonAppearance::Primary
                        on_click=move |_| drawer_open.set(true)
                    >
                        {icon("layers")} " Выбрать DataView"
                    </Button>
                </div>
            </CardAnimated>

            // ── Card: Тест ─────────────────────────────────────────────────────
            <CardAnimated delay_ms=50>
                <h4 class="details-section__title">
                    {icon("play")} " Тест на реальных данных"
                </h4>
                <p class="form__hint" style="margin-bottom: 12px;">
                    "Вычисляет индикатор через DataView с указанными параметрами. "
                    "Индикатор должен быть сохранён."
                </p>

                <p style="font-size: 12px; font-weight: 600; color: var(--color-text); margin: 0 0 6px;">"Период 1 (текущий)"</p>
                <div class="details-grid--3col">
                    <div class="form__group">
                        <label class="form__label">"Дата с"</label>
                        <input
                            type="date"
                            class="form__input"
                            prop:value=move || vm.test_date_from.get()
                            on:input=move |ev| { vm.test_date_from.set(input_date_value(&ev)); }
                        />
                    </div>
                    <div class="form__group">
                        <label class="form__label">"Дата по"</label>
                        <input
                            type="date"
                            class="form__input"
                            prop:value=move || vm.test_date_to.get()
                            on:input=move |ev| { vm.test_date_to.set(input_date_value(&ev)); }
                        />
                    </div>
                    <div class="form__group">
                        <label class="form__label">"Кабинеты (через запятую)"</label>
                        <Textarea
                            value=vm.test_connection_ids
                            placeholder="Пусто = все кабинеты"
                            attr:rows=2
                            attr:style="font-family: monospace; font-size: 12px; width: 100%;"
                        />
                    </div>
                </div>

                <p style="font-size: 12px; font-weight: 600; color: var(--color-text); margin: 14px 0 6px;">"Период 2 (для сравнения)"</p>
                <div class="details-grid--3col">
                    <div class="form__group">
                        <label class="form__label">"Дата с"</label>
                        <input
                            type="date"
                            class="form__input"
                            prop:value=move || vm.test_period2_from.get()
                            on:input=move |ev| { vm.test_period2_from.set(input_date_value(&ev)); }
                        />
                    </div>
                    <div class="form__group">
                        <label class="form__label">"Дата по"</label>
                        <input
                            type="date"
                            class="form__input"
                            prop:value=move || vm.test_period2_to.get()
                            on:input=move |ev| { vm.test_period2_to.set(input_date_value(&ev)); }
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
                        <span style="color: var(--color-text-secondary); font-size: 13px;">"Вычисление..."</span>
                    })}
                </div>

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

                {move || test_result.get().map(|r| {
                    let value_str = r.value.map(|v| format!("{v:.4}")).unwrap_or_else(|| "—".into());
                    let prev_str  = r.previous_value.map(|v| format!("{v:.4}")).unwrap_or_else(|| "—".into());
                    let delta_str = r.change_percent
                        .map(|p| { let sign = if p >= 0.0 { "+" } else { "" }; format!("{sign}{p:.2}%") })
                        .unwrap_or_else(|| "—".into());
                    let status_str = r.status.clone();
                    let status_color = match status_str.as_str() {
                        "Good"    => "var(--color-success)",
                        "Bad"     => "var(--color-error)",
                        "Warning" => "var(--color-warning)",
                        _         => "var(--color-text-secondary)",
                    };
                    view! {
                        <div
                            style="margin-top: var(--spacing-md); padding: var(--spacing-md); \
                                   border: 1px solid var(--color-border); \
                                   border-radius: var(--radius-md); \
                                   background: var(--color-surface-2);"
                        >
                            <div style="display: flex; gap: var(--spacing-lg); flex-wrap: wrap; align-items: baseline;">
                                <div>
                                    <span style="font-size: 11px; color: var(--color-text-secondary); text-transform: uppercase;">"Значение"</span>
                                    <div style="font-size: 28px; font-weight: 700; font-family: monospace;">{value_str}</div>
                                </div>
                                <div>
                                    <span style="font-size: 11px; color: var(--color-text-secondary); text-transform: uppercase;">"Предыдущий"</span>
                                    <div style="font-size: 18px; color: var(--color-text-secondary); font-family: monospace;">{prev_str}</div>
                                </div>
                                <div>
                                    <span style="font-size: 11px; color: var(--color-text-secondary); text-transform: uppercase;">"Δ %"</span>
                                    <div style="font-size: 18px; font-weight: 600; font-family: monospace;">{delta_str}</div>
                                </div>
                                <div>
                                    <span style="font-size: 11px; color: var(--color-text-secondary); text-transform: uppercase;">"Статус"</span>
                                    <div style=format!("font-size: 15px; font-weight: 600; color: {status_color};")>{status_str}</div>
                                </div>
                            </div>
                        </div>
                    }
                })}
            </CardAnimated>
        </div>

        // ── DataView Picker Drawer ─────────────────────────────────────────────
        <OverlayDrawer
            open=drawer_open
            position=DrawerPosition::Right
            size=DrawerSize::Medium
            close_on_esc=true
        >
            <DrawerHeader>
                <DrawerHeaderTitle>
                    "Выбор DataView"
                </DrawerHeaderTitle>
            </DrawerHeader>
            <DrawerBody native_scrollbar=true>
                <DvPickerBody
                    dv_loading=dv_loading
                    dv_error=dv_error
                    dv_search=dv_search
                    filtered_list=filtered_list
                    dsc_view_id=dsc_view_id
                    drawer_open=drawer_open
                />
            </DrawerBody>
        </OverlayDrawer>
    }
}

// ── DvPickerBody ──────────────────────────────────────────────────────────────

#[component]
fn DvPickerBody(
    dv_loading: RwSignal<bool>,
    dv_error: RwSignal<Option<String>>,
    dv_search: RwSignal<String>,
    filtered_list: Signal<Vec<DataViewMeta>>,
    dsc_view_id: RwSignal<String>,
    drawer_open: RwSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="dv-drawer">
            // Search
            <div class="dv-drawer__search">
                <input
                    type="text"
                    class="form__input"
                    placeholder="Поиск по id, названию, категории…"
                    prop:value=move || dv_search.get()
                    on:input=move |ev| {
                        use wasm_bindgen::JsCast;
                        let val = ev.target().unwrap()
                            .unchecked_into::<web_sys::HtmlInputElement>()
                            .value();
                        dv_search.set(val);
                    }
                />
            </div>

            // Loading / error
            {move || dv_loading.get().then(|| view! {
                <div class="form__hint">{icon("loader")} " Загрузка..."</div>
            })}
            {move || dv_error.get().map(|e| view! {
                <div class="form__hint" style="color: var(--color-error);">
                    {icon("alert-circle")} " " {e}
                </div>
            })}

            // List
            <div class="dv-drawer__list">
                {move || {
                    let list = filtered_list.get();
                    if list.is_empty() && !dv_loading.get_untracked() {
                        return view! {
                            <div class="form__hint">"Нет доступных DataView"</div>
                        }.into_any();
                    }
                    list.into_iter().map(|dv| {
                        let id = dv.id.clone();
                        let id_click = id.clone();
                        view! {
                            <button
                                class=move || {
                                    let base = "dv-picker-card";
                                    if dsc_view_id.get() == id { format!("{base} dv-picker-card--active") }
                                    else { base.to_string() }
                                }
                                on:click=move |_| {
                                    dsc_view_id.set(id_click.clone());
                                    drawer_open.set(false);
                                }
                            >
                                <div class="dv-picker-card__name">{dv.name.clone()}</div>
                                <div class="dv-picker-card__id">{dv.id.clone()}</div>
                                <div class="dv-picker-card__meta">
                                    <span class="dv-picker-card__category">{dv.category.clone()}</span>
                                    <span class="dv-picker-card__version">"v"{dv.version}</span>
                                </div>
                                {if !dv.description.is_empty() {
                                    view! {
                                        <div class="dv-picker-card__desc">{dv.description.clone()}</div>
                                    }.into_any()
                                } else {
                                    view! { <span></span> }.into_any()
                                }}
                            </button>
                        }
                    }).collect::<Vec<_>>().into_any()
                }}
            </div>

            // Footer
            <div class="dv-drawer__footer">
                <Button
                    appearance=ButtonAppearance::Secondary
                    on_click=move |_| drawer_open.set(false)
                >
                    "Закрыть"
                </Button>
            </div>
        </div>
    }
}
