//! DataSpec tab — DataView source + metric selection + live test

use super::super::view_model::{
    value_format_presets, BiIndicatorDetailsVm, GL_TURNOVER_DATA_VIEW_ID,
};
use crate::data_view::api as dv_api;
use crate::data_view::types::{DataViewMeta, FilterDef};
use crate::data_view::ui::FilterBar;
use crate::general_ledger::api::fetch_general_ledger_turnovers;
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::icons::icon;
use contracts::projections::general_ledger::GeneralLedgerTurnoverDto;
use leptos::prelude::*;
use thaw::*;

#[derive(Clone, Debug, PartialEq, Eq)]
struct GlTurnoverSlot {
    code: String,
    sign: i32,
}

fn empty_gl_turnover_slot() -> GlTurnoverSlot {
    GlTurnoverSlot {
        code: String::new(),
        sign: 1,
    }
}

fn parse_gl_turnover_slots(turnover_items: &str, turnover_code: &str) -> [GlTurnoverSlot; 2] {
    let mut slots: Vec<GlTurnoverSlot> = turnover_items
        .split(|ch| ch == ',' || ch == ';' || ch == '\n' || ch == '\r')
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .filter_map(|token| {
            let (sign, code) = match token.chars().next() {
                Some('-') => (-1, token[1..].trim()),
                Some('+') => (1, token[1..].trim()),
                _ => (1, token),
            };
            (!code.is_empty()).then(|| GlTurnoverSlot {
                code: code.to_string(),
                sign,
            })
        })
        .take(2)
        .collect();

    if slots.is_empty() && !turnover_code.trim().is_empty() {
        slots.push(GlTurnoverSlot {
            code: turnover_code.trim().to_string(),
            sign: 1,
        });
    }

    while slots.len() < 2 {
        slots.push(empty_gl_turnover_slot());
    }

    [slots.remove(0), slots.remove(0)]
}

fn compose_turnover_items(slots: &[GlTurnoverSlot; 2]) -> String {
    slots
        .iter()
        .filter_map(|slot| {
            let code = slot.code.trim();
            if code.is_empty() {
                None
            } else if slot.sign < 0 {
                Some(format!("-{code}"))
            } else {
                Some(code.to_string())
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn gl_formula_preview(slots: &[GlTurnoverSlot; 2]) -> String {
    let mut parts = Vec::new();
    for (index, slot) in slots.iter().enumerate() {
        let code = slot.code.trim();
        if code.is_empty() {
            continue;
        }
        if index == 0 {
            if slot.sign < 0 {
                parts.push(format!("- {code}"));
            } else {
                parts.push(code.to_string());
            }
        } else if slot.sign < 0 {
            parts.push(format!("- {code}"));
        } else {
            parts.push(format!("+ {code}"));
        }
    }

    if parts.is_empty() {
        "Обороты GL не выбраны".to_string()
    } else {
        parts.join(" ")
    }
}

fn sync_gl_turnover_params(
    vm: &BiIndicatorDetailsVm,
    slot_index: usize,
    next_code: Option<&str>,
    next_sign: Option<i32>,
) {
    let current_items = vm.get_param_default_value("turnover_items");
    let current_code = vm.get_param_default_value("turnover_code");
    let mut slots = parse_gl_turnover_slots(&current_items, &current_code);

    if slot_index >= slots.len() {
        return;
    }

    if let Some(code) = next_code {
        slots[slot_index].code = code.trim().to_string();
    }
    if let Some(sign) = next_sign {
        slots[slot_index].sign = if sign < 0 { -1 } else { 1 };
    }

    let fallback_code = slots
        .iter()
        .find_map(|slot| {
            let code = slot.code.trim();
            (!code.is_empty()).then(|| code.to_string())
        })
        .unwrap_or_default();
    let turnover_items = compose_turnover_items(&slots);

    vm.set_param_default_value("turnover_items", "string", "GL turnovers", &turnover_items);
    vm.set_param_default_value("turnover_code", "string", "Turnover code", &fallback_code);
}

#[component]
fn GlTurnoverConfigSection(
    vm: BiIndicatorDetailsVm,
    gl_turnovers: RwSignal<Vec<GeneralLedgerTurnoverDto>>,
    gl_turnovers_loading: RwSignal<bool>,
    gl_turnovers_error: RwSignal<Option<String>>,
) -> impl IntoView {
    let vm_slots = vm.clone();
    let turnover_slots = Signal::derive(move || {
        parse_gl_turnover_slots(
            &vm_slots.get_param_default_value("turnover_items"),
            &vm_slots.get_param_default_value("turnover_code"),
        )
    });
    let vm_layer = vm.clone();
    let layer_value = Signal::derive(move || vm_layer.get_param_default_value("layer"));
    let formula_preview = Signal::derive(move || gl_formula_preview(&turnover_slots.get()));
    let turnover_items_preview =
        Signal::derive(move || compose_turnover_items(&turnover_slots.get()));

    view! {
        <div class="bi-indicator-action__section">
            <div class="bi-indicator-action__section-header">
                <h5 class="bi-indicator-action__section-title">"Конфигурация оборотов GL"</h5>
            </div>
            <p class="form__hint">
                "Для dv004 можно задать один или два GL-оборота. Каждый оборот хранится со знаком, а итоговая сумма уходит в "
                <code>"turnover_items"</code>
                ". Для совместимости первый код также дублируется в "
                <code>"turnover_code"</code>
                "."
            </p>

            <div class="bi-indicator-dataspec__formula-box">
                <span class="bi-indicator-dataspec__overview-label">"Формула расчёта"</span>
                <strong class="bi-indicator-dataspec__formula-value">{move || formula_preview.get()}</strong>
                <code class="bi-indicator-dataspec__formula-raw">
                    {move || {
                        let raw = turnover_items_preview.get();
                        if raw.trim().is_empty() {
                            "turnover_items = ∅".to_string()
                        } else {
                            format!("turnover_items = {raw}")
                        }
                    }}
                </code>
            </div>

            <div class="bi-indicator-dataspec__gl-grid">
                <div class="bi-indicator-dataspec__gl-row">
                    <div class="form__group bi-indicator-dataspec__field">
                        <label class="form__label">"Оборот 1: знак"</label>
                        <select
                            class="form__select"
                            prop:value=move || {
                                if turnover_slots.get()[0].sign < 0 { "-".to_string() } else { "+".to_string() }
                            }
                            on:change={
                                let vm = vm.clone();
                                move |ev| {
                                    let sign = if event_target_value(&ev) == "-" { -1 } else { 1 };
                                    sync_gl_turnover_params(&vm, 0, None, Some(sign));
                                }
                            }
                        >
                            <option value="+">"+ (прибавить)"</option>
                            <option value="-">"- (вычесть)"</option>
                        </select>
                    </div>
                    <div class="form__group bi-indicator-dataspec__field">
                        <label class="form__label">"Оборот 1: код"</label>
                        <select
                            class="form__select"
                            prop:value=move || turnover_slots.get()[0].code.clone()
                            on:change={
                                let vm = vm.clone();
                                move |ev| {
                                    sync_gl_turnover_params(
                                        &vm,
                                        0,
                                        Some(&event_target_value(&ev)),
                                        None,
                                    );
                                }
                            }
                        >
                            <option value="">"Не выбран"</option>
                            {move || {
                                gl_turnovers.get().into_iter().map(|item| {
                                    let label = format!("{} - {}", item.code, item.name);
                                    view! {
                                        <option value=item.code>{label}</option>
                                    }
                                }).collect_view()
                            }}
                        </select>
                    </div>
                </div>

                <div class="bi-indicator-dataspec__gl-row">
                    <div class="form__group bi-indicator-dataspec__field">
                        <label class="form__label">"Оборот 2: знак"</label>
                        <select
                            class="form__select"
                            prop:value=move || {
                                if turnover_slots.get()[1].sign < 0 { "-".to_string() } else { "+".to_string() }
                            }
                            on:change={
                                let vm = vm.clone();
                                move |ev| {
                                    let sign = if event_target_value(&ev) == "-" { -1 } else { 1 };
                                    sync_gl_turnover_params(&vm, 1, None, Some(sign));
                                }
                            }
                        >
                            <option value="+">"+ (прибавить)"</option>
                            <option value="-">"- (вычесть)"</option>
                        </select>
                    </div>
                    <div class="form__group bi-indicator-dataspec__field">
                        <label class="form__label">"Оборот 2: код"</label>
                        <select
                            class="form__select"
                            prop:value=move || turnover_slots.get()[1].code.clone()
                            on:change={
                                let vm = vm.clone();
                                move |ev| {
                                    sync_gl_turnover_params(
                                        &vm,
                                        1,
                                        Some(&event_target_value(&ev)),
                                        None,
                                    );
                                }
                            }
                        >
                            <option value="">"Не выбран"</option>
                            {move || {
                                gl_turnovers.get().into_iter().map(|item| {
                                    let label = format!("{} - {}", item.code, item.name);
                                    view! {
                                        <option value=item.code>{label}</option>
                                    }
                                }).collect_view()
                            }}
                        </select>
                    </div>
                </div>

                <div class="form__group bi-indicator-dataspec__field">
                    <label class="form__label">"Слой"</label>
                    <select
                        class="form__select"
                        prop:value=move || layer_value.get()
                        on:change={
                            let vm = vm.clone();
                            move |ev| {
                                vm.set_param_default_value(
                                    "layer",
                                    "string",
                                    "Layer",
                                    &event_target_value(&ev),
                                );
                            }
                        }
                    >
                        <option value="">"Не выбран"</option>
                        <option value="oper">"oper"</option>
                        <option value="fact">"fact"</option>
                        <option value="plan">"plan"</option>
                    </select>
                </div>
            </div>

            {move || {
                if gl_turnovers_loading.get() {
                    view! { <p class="form__hint">"Загрузка оборотов GL..."</p> }.into_any()
                } else if let Some(error) = gl_turnovers_error.get() {
                    view! { <p class="form__hint" style="color: var(--color-danger);">{error}</p> }.into_any()
                } else {
                    view! {
                        <p class="form__hint">
                            "Для COST используйте "
                            <code>"item_cost"</code>
                            " и "
                            <code>"item_cost_storno"</code>
                            " со знаком "
                            <code>"+"</code>
                            " на слое "
                            <code>"oper"</code>
                            "."
                        </p>
                    }.into_any()
                }
            }}
        </div>
    }
}

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
    let vm_gl_picker = vm.clone();
    let format_preset_value = Signal::derive(move || vm_format_value.current_format_preset_key());

    let dv_list: RwSignal<Vec<DataViewMeta>> = RwSignal::new(vec![]);
    let dv_loading: RwSignal<bool> = RwSignal::new(true);
    let dv_error: RwSignal<Option<String>> = RwSignal::new(None);
    let dv_search: RwSignal<String> = RwSignal::new(String::new());
    let drawer_open: RwSignal<bool> = RwSignal::new(false);
    let selected_view_meta: RwSignal<Option<DataViewMeta>> = RwSignal::new(None);
    let test_filter_defs: RwSignal<Vec<FilterDef>> = RwSignal::new(vec![]);
    let gl_turnovers: RwSignal<Vec<GeneralLedgerTurnoverDto>> = RwSignal::new(vec![]);
    let gl_turnovers_loading: RwSignal<bool> = RwSignal::new(false);
    let gl_turnovers_error: RwSignal<Option<String>> = RwSignal::new(None);

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

    Effect::new(move |_| {
        let view_id = dsc_view_id.get();
        if view_id != GL_TURNOVER_DATA_VIEW_ID
            || gl_turnovers_loading.get_untracked()
            || !gl_turnovers.get_untracked().is_empty()
        {
            return;
        }

        gl_turnovers_loading.set(true);
        leptos::task::spawn_local(async move {
            match fetch_general_ledger_turnovers().await {
                Ok(response) => {
                    gl_turnovers.set(response.items);
                    gl_turnovers_error.set(None);
                }
                Err(error) => gl_turnovers_error.set(Some(error)),
            }
            gl_turnovers_loading.set(false);
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
                        let meta_id = meta.id.clone();
                        let is_gl_turnover_view = meta_id == GL_TURNOVER_DATA_VIEW_ID;
                        let resources = meta.available_resources.clone();
                        let dimensions = meta.available_dimensions.clone();
                        let data_sources = meta.data_sources.clone();
                        let current_metric_id = metric_value.get();
                        let current_metric_label = selected_metric_label.get();
                        let vm_format_change = vm_for_formats.clone();
                        let format_preset_signal = format_preset_value;
                        view! {
                            <div class="bi-indicator-dataspec__summary">
                                <div class="bi-indicator-dataspec__summary-main">
                                    <div class="bi-indicator-dataspec__title-row">
                                        <strong>{meta.name.clone()}</strong>
                                        <span class="bi-indicator-dataspec__meta-code">{meta_id.clone()}</span>
                                        <span class="bi-indicator-dataspec__meta-code">
                                            "v"{meta.version}
                                        </span>
                                    </div>
                                    <p class="bi-indicator-dataspec__description">
                                        {meta.description}
                                    </p>
                                </div>

                                <div class="bi-indicator-dataspec__overview-grid">
                                    <div class="bi-indicator-dataspec__overview-card">
                                        <span class="bi-indicator-dataspec__overview-label">"DataView"</span>
                                        <strong class="bi-indicator-dataspec__overview-value">
                                            {meta.name.clone()}
                                        </strong>
                                        <code class="bi-indicator-dataspec__overview-code">{meta_id.clone()}</code>
                                    </div>
                                    <div class="bi-indicator-dataspec__overview-card">
                                        <span class="bi-indicator-dataspec__overview-label">"Метрика"</span>
                                        <strong class="bi-indicator-dataspec__overview-value">
                                            {if current_metric_label.trim().is_empty() {
                                                "Не выбрана".to_string()
                                            } else {
                                                current_metric_label.clone()
                                            }}
                                        </strong>
                                        <code class="bi-indicator-dataspec__overview-code">
                                            {if current_metric_id.trim().is_empty() {
                                                "metric_id = ∅".to_string()
                                            } else {
                                                format!("metric_id = {}", current_metric_id)
                                            }}
                                        </code>
                                    </div>
                                    <div class="bi-indicator-dataspec__overview-card">
                                        <span class="bi-indicator-dataspec__overview-label">"Источники данных"</span>
                                        <strong class="bi-indicator-dataspec__overview-value">
                                            {data_sources.len().to_string()}
                                        </strong>
                                        <span class="bi-indicator-dataspec__overview-meta">
                                            {if data_sources.is_empty() {
                                                "Не указаны".to_string()
                                            } else {
                                                data_sources.join(", ")
                                            }}
                                        </span>
                                    </div>
                                    <div class="bi-indicator-dataspec__overview-card">
                                        <span class="bi-indicator-dataspec__overview-label">"Структура"</span>
                                        <strong class="bi-indicator-dataspec__overview-value">
                                            {format!("{} измерений", dimensions.len())}
                                        </strong>
                                        <span class="bi-indicator-dataspec__overview-meta">
                                            {format!("{} фильтров для теста", test_filter_defs.get().len())}
                                        </span>
                                    </div>
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

                                {if is_gl_turnover_view {
                                    view! {
                                        <GlTurnoverConfigSection
                                            vm=vm_gl_picker.clone()
                                            gl_turnovers=gl_turnovers
                                            gl_turnovers_loading=gl_turnovers_loading
                                            gl_turnovers_error=gl_turnovers_error
                                        />
                                    }
                                        .into_any()
                                } else {
                                    view! { <span /> }.into_any()
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
