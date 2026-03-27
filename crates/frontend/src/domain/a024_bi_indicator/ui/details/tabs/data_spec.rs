//! DataSpec tab — DataView source + metric selection + live test

use super::super::view_model::{value_format_presets, BiIndicatorDetailsVm};
use crate::data_view::api as dv_api;
use crate::data_view::types::{DataViewMeta, FilterDef};
use crate::data_view::ui::FilterBar;
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::icons::icon;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn DataSpecTab(vm: BiIndicatorDetailsVm) -> impl IntoView {
    let vm_test = vm.clone();
    let run_test = move |_| vm_test.run_test();

    let test_loading = vm.test_loading;
    let test_error = vm.test_error;
    let test_result = vm.test_result;
    let dsc_view_id = vm.dsc_view_id;
    let dsc_metric_id = vm.dsc_metric_id;
    let metric_value = Signal::derive(move || dsc_metric_id.get());
    let vm_format_value = vm.clone();
    let vm_for_formats = vm.clone();
    let format_preset_value = Signal::derive(move || vm_format_value.current_format_preset_key());

    let dv_list: RwSignal<Vec<DataViewMeta>> = RwSignal::new(vec![]);
    let dv_loading: RwSignal<bool> = RwSignal::new(true);
    let dv_error: RwSignal<Option<String>> = RwSignal::new(None);
    let dv_search: RwSignal<String> = RwSignal::new(String::new());
    let drawer_open: RwSignal<bool> = RwSignal::new(false);
    let selected_view_meta: RwSignal<Option<DataViewMeta>> = RwSignal::new(None);
    let test_filter_defs: RwSignal<Vec<FilterDef>> = RwSignal::new(vec![]);

    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            match dv_api::fetch_list().await {
                Ok(list) => {
                    dv_list.set(list);
                    dv_error.set(None);
                }
                Err(e) => dv_error.set(Some(e)),
            }
            dv_loading.set(false);
        });
    });

    Effect::new(move |_| {
        let view_id = dsc_view_id.get();
        if view_id.trim().is_empty() {
            selected_view_meta.set(None);
            test_filter_defs.set(vec![]);
            dsc_metric_id.set(String::new());
            return;
        }

        let metric_signal = dsc_metric_id;
        leptos::task::spawn_local(async move {
            match dv_api::fetch_by_id(&view_id).await {
                Ok(meta) => {
                    let current_metric = metric_signal.get_untracked();
                    let next_metric = if meta.available_resources.is_empty() {
                        String::new()
                    } else if meta
                        .available_resources
                        .iter()
                        .any(|resource| resource.id == current_metric)
                    {
                        current_metric
                    } else {
                        meta.available_resources[0].id.clone()
                    };
                    metric_signal.set(next_metric);
                    selected_view_meta.set(Some(meta));
                }
                Err(_) => {
                    selected_view_meta.set(None);
                    metric_signal.set(String::new());
                }
            }

            match dv_api::fetch_view_filters(&view_id).await {
                Ok(filters) => test_filter_defs.set(filters),
                Err(_) => test_filter_defs.set(vec![]),
            }
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

    let selected_metric_label = Signal::derive(move || {
        let metric_id = dsc_metric_id.get();
        selected_view_meta
            .get()
            .and_then(|meta| {
                meta.available_resources
                    .into_iter()
                    .find(|resource| resource.id == metric_id)
            })
            .map(|resource| resource.label)
            .unwrap_or_default()
    });

    view! {
        <CardAnimated delay_ms=0 nav_id="a024_bi_indicator_details_data_spec_main">
            <h4 class="details-section__title">
                {icon("database")} " DataView, метрика и тест расчёта"
            </h4>
            <p class="form__hint">
                "Индикатор работает через DataView. Здесь выбираются источник, ресурс "
                "(метрика), доступные фильтры и выполняется проверка на реальных данных."
            </p>

            <div class="bi-indicator-action__section">
                <div class="bi-indicator-action__section-header">
                    <h5 class="bi-indicator-action__section-title">"Источник данных"</h5>
                    <Button
                        appearance=ButtonAppearance::Primary
                        on_click=move |_| drawer_open.set(true)
                    >
                        {icon("layers")} " Выбрать DataView"
                    </Button>
                </div>

                {move || {
                    let view_id = dsc_view_id.get();
                    if view_id.trim().is_empty() {
                        view! {
                            <div class="dv-status-banner dv-status-banner--empty">
                                {icon("alert-circle")}
                                " DataView не выбран. Без него индикатор не сможет вычисляться."
                            </div>
                        }
                            .into_any()
                    } else {
                        let metric_label = selected_metric_label.get();
                        view! {
                            <div class="dv-status-banner dv-status-banner--active">
                                {icon("check-circle")}
                                " Активен: "
                                <code>{view_id}</code>
                                {if metric_label.is_empty() {
                                    view! { <span /> }.into_any()
                                } else {
                                    view! {
                                        <span class="bi-indicator-dataspec__metric-pill">
                                            {metric_label}
                                        </span>
                                    }
                                        .into_any()
                                }}
                                <button
                                    class="dv-status-banner__clear"
                                    title="Очистить"
                                    on:click=move |_| {
                                        dsc_view_id.set(String::new());
                                        dsc_metric_id.set(String::new());
                                    }
                                >
                                    "×"
                                </button>
                            </div>
                        }
                            .into_any()
                    }
                }}

                {move || match selected_view_meta.get() {
                    Some(meta) => {
                        let resources = meta.available_resources.clone();
                        let dimensions = meta.available_dimensions.clone();
                        let data_sources = meta.data_sources.clone();
                        let vm_format_change = vm_for_formats.clone();
                        let format_preset_signal = format_preset_value;
                        view! {
                            <div class="bi-indicator-dataspec__summary">
                                <div class="bi-indicator-dataspec__summary-main">
                                    <div class="bi-indicator-dataspec__title-row">
                                        <strong>{meta.name}</strong>
                                        <span class="bi-indicator-dataspec__meta-code">{meta.id}</span>
                                        <span class="bi-indicator-dataspec__meta-code">
                                            "v"{meta.version}
                                        </span>
                                    </div>
                                    <p class="bi-indicator-dataspec__description">
                                        {meta.description}
                                    </p>
                                </div>

                                <div class="bi-indicator-dataspec__stats">
                                    <span class="bi-indicator-dataspec__meta-code">
                                        "Источники: " {data_sources.join(", ")}
                                    </span>
                                    <span class="bi-indicator-dataspec__meta-code">
                                        "Измерения: " {dimensions.len().to_string()}
                                    </span>
                                    <span class="bi-indicator-dataspec__meta-code">
                                        "Фильтры: " {test_filter_defs.get().len().to_string()}
                                    </span>
                                </div>

                                {if resources.is_empty() {
                                    view! {
                                        <div class="placeholder">
                                            "У выбранного DataView нет `available_resources`. "
                                            "Будет использоваться поведение по умолчанию."
                                        </div>
                                    }
                                        .into_any()
                                } else {
                                    view! {
                                        <div class="bi-indicator-dataspec__field-grid">
                                            <div class="form__group bi-indicator-dataspec__field">
                                                <label class="form__label">"Метрика / ресурс"</label>
                                                <select
                                                    class="form__select"
                                                    prop:value=move || metric_value.get()
                                                    on:change=move |ev| {
                                                        dsc_metric_id.set(event_target_value(&ev));
                                                    }
                                                >
                                                    {resources.into_iter().map(|resource| {
                                                        let description = if resource.description.trim().is_empty() {
                                                            resource.label.clone()
                                                        } else {
                                                            format!("{} — {}", resource.id, resource.label)
                                                        };
                                                        view! {
                                                            <option value=resource.id>{description}</option>
                                                        }
                                                    }).collect_view()}
                                                </select>
                                            </div>
                                            <div class="form__group bi-indicator-dataspec__field">
                                                <label class="form__label">"Формат значения"</label>
                                                <select
                                                    class="form__select"
                                                    prop:value=move || format_preset_signal.get()
                                                    on:change=move |ev| {
                                                        vm_format_change
                                                            .apply_format_preset(&event_target_value(&ev));
                                                    }
                                                >
                                                    {value_format_presets().iter().map(|preset| {
                                                        view! {
                                                            <option value=preset.key>{preset.label}</option>
                                                        }
                                                    }).collect_view()}
                                                </select>
                                                <p class="form__hint bi-indicator-dataspec__field-hint">
                                                    "Быстрый выбор для "
                                                    <code>"view_spec.format"</code>
                                                    ". Для нестандартного JSON используйте вкладку ViewSpec."
                                                </p>
                                            </div>
                                        </div>
                                    }
                                        .into_any()
                                }}

                                {if test_filter_defs.get().is_empty() {
                                    view! {
                                        <div class="placeholder">
                                            "У выбранного DataView нет зарегистрированных фильтров."
                                        </div>
                                    }
                                        .into_any()
                                } else {
                                    view! {
                                        <div class="bi-indicator-filter-list">
                                            {test_filter_defs.get().into_iter().map(|filter| {
                                                view! {
                                                    <div class="bi-indicator-filter-list__item">
                                                        <code class="bi-indicator-filter-list__id">
                                                            {filter.id}
                                                        </code>
                                                        <span class="bi-indicator-filter-list__label">
                                                            {filter.label}
                                                        </span>
                                                    </div>
                                                }
                                            }).collect_view()}
                                        </div>
                                    }
                                        .into_any()
                                }}
                            </div>
                        }
                            .into_any()
                    }
                    None => view! {
                        <div class="placeholder">
                            "Выберите DataView, чтобы увидеть доступные ресурсы, измерения и фильтры."
                        </div>
                    }
                        .into_any(),
                }}
            </div>

            <div class="bi-indicator-action__section">
                <div class="bi-indicator-action__section-header">
                    <h5 class="bi-indicator-action__section-title">"Тестовый контекст"</h5>
                </div>

                {move || {
                    if dsc_view_id.get().trim().is_empty() {
                        view! {
                            <div class="placeholder">
                                "Сначала выберите DataView, чтобы заполнить фильтры для теста."
                            </div>
                        }
                            .into_any()
                    } else {
                        view! {
                            <FilterBar filters=test_filter_defs.get() ctx=vm.test_ctx />
                        }
                            .into_any()
                    }
                }}

                <div class="bi-indicator-action__actions">
                    <Button
                        appearance=ButtonAppearance::Primary
                        on_click=run_test
                        disabled=test_loading
                    >
                        {icon("play")} " Выполнить тест"
                    </Button>
                    <span class="form__hint">
                        "Тест использует сохранённый индикатор. После изменения DataView или метрики сначала сохраните запись."
                    </span>
                </div>
            </div>

            <div class="bi-indicator-action__section">
                <div class="bi-indicator-action__section-header">
                    <h5 class="bi-indicator-action__section-title">"Результат"</h5>
                </div>

                {move || {
                    if test_loading.get() {
                        view! {
                            <div class="placeholder">"Вычисление..."</div>
                        }
                            .into_any()
                    } else if let Some(err) = test_error.get() {
                        view! {
                            <div class="warning-box warning-box--error">
                                <span class="warning-box__icon">"⚠"</span>
                                <span class="warning-box__text">{err}</span>
                            </div>
                        }
                            .into_any()
                    } else if let Some(result) = test_result.get() {
                        let value_str = result
                            .value
                            .map(|value| format!("{value:.4}"))
                            .unwrap_or_else(|| "—".to_string());
                        let previous_value = result
                            .previous_value
                            .map(|value| format!("{value:.4}"))
                            .unwrap_or_else(|| "—".to_string());
                        let delta_str = result
                            .change_percent
                            .map(|value| {
                                let sign = if value >= 0.0 { "+" } else { "" };
                                format!("{sign}{value:.2}%")
                            })
                            .unwrap_or_else(|| "—".to_string());
                        let status_class = match result.status.as_str() {
                            "Good" => "bi-indicator-test-result__value--good",
                            "Bad" => "bi-indicator-test-result__value--bad",
                            "Warning" => "bi-indicator-test-result__value--warn",
                            _ => "bi-indicator-test-result__value--neutral",
                        };

                        view! {
                            <div class="bi-indicator-test-result">
                                <div class="bi-indicator-test-result__item">
                                    <span class="bi-indicator-test-result__label">"Значение"</span>
                                    <strong class="bi-indicator-test-result__value">
                                        {value_str}
                                    </strong>
                                </div>
                                <div class="bi-indicator-test-result__item">
                                    <span class="bi-indicator-test-result__label">"Предыдущий период"</span>
                                    <strong class="bi-indicator-test-result__value bi-indicator-test-result__value--neutral">
                                        {previous_value}
                                    </strong>
                                </div>
                                <div class="bi-indicator-test-result__item">
                                    <span class="bi-indicator-test-result__label">"Δ %"</span>
                                    <strong class="bi-indicator-test-result__value">
                                        {delta_str}
                                    </strong>
                                </div>
                                <div class="bi-indicator-test-result__item">
                                    <span class="bi-indicator-test-result__label">"Статус"</span>
                                    <strong class=format!("bi-indicator-test-result__value {status_class}")>
                                        {result.status}
                                    </strong>
                                </div>
                            </div>
                        }
                            .into_any()
                    } else {
                        view! {
                            <div class="placeholder">
                                "После выполнения теста здесь появятся значение, сравнение с предыдущим периодом и статус."
                            </div>
                        }
                            .into_any()
                    }
                }}
            </div>
        </CardAnimated>

        <OverlayDrawer
            open=drawer_open
            position=DrawerPosition::Right
            size=DrawerSize::Medium
            close_on_esc=true
        >
            <DrawerHeader>
                <DrawerHeaderTitle>"Выбор DataView"</DrawerHeaderTitle>
            </DrawerHeader>
            <DrawerBody native_scrollbar=true>
                <DvPickerBody
                    dv_loading=dv_loading
                    dv_error=dv_error
                    dv_search=dv_search
                    filtered_list=filtered_list
                    dsc_view_id=dsc_view_id
                    dsc_metric_id=dsc_metric_id
                    drawer_open=drawer_open
                />
            </DrawerBody>
        </OverlayDrawer>
    }
}

#[component]
fn DvPickerBody(
    dv_loading: RwSignal<bool>,
    dv_error: RwSignal<Option<String>>,
    dv_search: RwSignal<String>,
    filtered_list: Signal<Vec<DataViewMeta>>,
    dsc_view_id: RwSignal<String>,
    dsc_metric_id: RwSignal<String>,
    drawer_open: RwSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="dv-drawer">
            <div class="dv-drawer__search">
                <input
                    type="text"
                    class="form__input"
                    placeholder="Поиск по id, названию, категории…"
                    prop:value=move || dv_search.get()
                    on:input=move |ev| {
                        use wasm_bindgen::JsCast;
                        let value = ev
                            .target()
                            .unwrap()
                            .unchecked_into::<web_sys::HtmlInputElement>()
                            .value();
                        dv_search.set(value);
                    }
                />
            </div>

            {move || {
                if dv_loading.get() {
                    view! { <div class="placeholder">"Загрузка DataView..."</div> }.into_any()
                } else if let Some(err) = dv_error.get() {
                    view! {
                        <div class="warning-box warning-box--error">
                            <span class="warning-box__icon">"⚠"</span>
                            <span class="warning-box__text">{err}</span>
                        </div>
                    }
                        .into_any()
                } else {
                    let list = filtered_list.get();
                    if list.is_empty() {
                        view! {
                            <div class="placeholder">"Нет доступных DataView"</div>
                        }
                            .into_any()
                    } else {
                        view! {
                            <div class="dv-picker-grid">
                                {list.into_iter().map(|dv| {
                                    let id = dv.id.clone();
                                    let click_id = id.clone();
                                    view! {
                                        <button
                                            class=move || {
                                                let base = "dv-picker-card";
                                                if dsc_view_id.get() == id {
                                                    format!("{base} dv-picker-card--active")
                                                } else {
                                                    base.to_string()
                                                }
                                            }
                                            on:click=move |_| {
                                                dsc_view_id.set(click_id.clone());
                                                dsc_metric_id.set(String::new());
                                                drawer_open.set(false);
                                            }
                                        >
                                            <div class="dv-picker-card__name">{dv.name.clone()}</div>
                                            <div class="dv-picker-card__id">{dv.id.clone()}</div>
                                            <div class="dv-picker-card__meta">
                                                <span class="dv-picker-card__category">{dv.category.clone()}</span>
                                                <span class="dv-picker-card__version">"v"{dv.version}</span>
                                            </div>
                                            {if dv.description.trim().is_empty() {
                                                view! { <span /> }.into_any()
                                            } else {
                                                view! {
                                                    <div class="dv-picker-card__desc">{dv.description.clone()}</div>
                                                }
                                                    .into_any()
                                            }}
                                        </button>
                                    }
                                }).collect_view()}
                            </div>
                        }
                            .into_any()
                    }
                }
            }}

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
