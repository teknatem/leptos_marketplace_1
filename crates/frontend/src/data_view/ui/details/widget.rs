//! DataView detail page — метаданные + тест-форма.

use crate::data_view::api::{self, ComputeResult};
use crate::data_view::types::{DataViewMeta, FilterDef, FilterKind};
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::components::date_range_picker::DateRangePicker;
use crate::shared::filters::ConnectionMpMultiSelect;
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use leptos::prelude::*;
use thaw::*;

fn default_date_from() -> String {
    use js_sys::Date;
    let now = Date::new_0();
    let y = now.get_full_year();
    let m = now.get_month(); // 0-based
    if m == 0 {
        format!("{:04}-12-01", y - 1)
    } else {
        format!("{:04}-{:02}-01", y, m)
    }
}

fn default_date_to() -> String {
    use js_sys::Date;
    let now = Date::new_0();
    let y = now.get_full_year();
    let m = now.get_month(); // 0-based, current month
    let last_day = if m == 11 {
        31
    } else {
        let next_m = m + 1;
        Date::new_with_year_month_day(y, next_m as i32, 0).get_date()
    };
    if m == 0 {
        format!("{:04}-12-31", y - 1)
    } else {
        format!("{:04}-{:02}-{:02}", y, m, last_day)
    }
}

#[component]
#[allow(non_snake_case)]
pub fn DataViewDetail(view_id: String) -> impl IntoView {
    let (meta, set_meta) = signal::<Option<DataViewMeta>>(None);
    let (loading_meta, set_loading_meta) = signal(true);
    let (meta_error, set_meta_error) = signal::<Option<String>>(None);

    let (view_filters, set_view_filters) = signal::<Vec<FilterDef>>(vec![]);

    let test_date_from = RwSignal::new(default_date_from());
    let test_date_to = RwSignal::new(default_date_to());
    let test_period2_from = RwSignal::new(String::new());
    let test_period2_to = RwSignal::new(String::new());
    let test_connection_ids = RwSignal::new(Vec::<String>::new());
    let test_metric = RwSignal::new("revenue".to_string());
    let test_loading = RwSignal::new(false);
    let test_error: RwSignal<Option<String>> = RwSignal::new(None);
    let test_result: RwSignal<Option<ComputeResult>> = RwSignal::new(None);

    let view_id_clone = view_id.clone();
    Effect::new(move |_| {
        let id = view_id_clone.clone();
        wasm_bindgen_futures::spawn_local(async move {
            set_loading_meta.set(true);
            match api::fetch_by_id(&id).await {
                Ok(m) => {
                    set_meta.set(Some(m));
                    set_loading_meta.set(false);
                }
                Err(e) => {
                    set_meta_error.set(Some(e));
                    set_loading_meta.set(false);
                }
            }
            // Load resolved filters in parallel (ignore errors — non-critical)
            if let Ok(filters) = api::fetch_view_filters(&id).await {
                set_view_filters.set(filters);
            }
        });
    });

    let view_id_stored = StoredValue::new(view_id.clone());
    let run_test = move |_| {
        let id = view_id_stored.get_value();
        let date_from = test_date_from.get();
        let date_to = test_date_to.get();
        let p2_from_raw = test_period2_from.get();
        let p2_to_raw = test_period2_to.get();
        let connection_ids = test_connection_ids.get();
        let metric_raw = test_metric.get();

        let p2_from = if p2_from_raw.trim().is_empty() {
            None
        } else {
            Some(p2_from_raw)
        };
        let p2_to = if p2_to_raw.trim().is_empty() {
            None
        } else {
            Some(p2_to_raw)
        };
        let metric = if metric_raw.trim().is_empty() {
            None
        } else {
            Some(metric_raw)
        };

        test_loading.set(true);
        test_error.set(None);
        test_result.set(None);

        wasm_bindgen_futures::spawn_local(async move {
            match api::compute_view(
                &id,
                &date_from,
                &date_to,
                p2_from.as_deref(),
                p2_to.as_deref(),
                connection_ids,
                metric,
            )
            .await
            {
                Ok(r) => {
                    test_loading.set(false);
                    test_result.set(Some(r));
                }
                Err(e) => {
                    test_loading.set(false);
                    test_error.set(Some(e));
                }
            }
        });
    };

    view! {
        <PageFrame page_id="data_view--detail" category="detail">
            {move || {
                if loading_meta.get() {
                    return view! {
                        <div class="placeholder">"Загрузка метаданных DataView..."</div>
                    }.into_any();
                }
                if let Some(e) = meta_error.get() {
                    return view! {
                        <div class="warning-box warning-box--error">
                            <span class="warning-box__icon">"⚠"</span>
                            <span class="warning-box__text">{e}</span>
                        </div>
                    }.into_any();
                }
                let Some(m) = meta.get() else {
                    return view! { <div class="placeholder">"Нет данных"</div> }.into_any();
                };

                view! {
                    <div>
                        <div class="page__header">
                            <div class="page__header-left">
                                <h1 class="page__title">
                                    {icon("layers")} " " {m.name.clone()}
                                </h1>
                                <span class="text-muted" style="font-family: monospace; font-size: 13px;">
                                    {m.id.clone()} " · v" {m.version}
                                </span>
                            </div>
                        </div>

                        <div class="detail-grid">

                            // ── Левая колонка: мета + измерения + параметры ─────────────────
                            <div class="detail-grid__col">

                                <CardAnimated delay_ms=0>
                                    <h4 class="details-section__title">{icon("info")} " Описание"</h4>
                                    <div class="form__group">
                                        <label class="form__label">"ID"</label>
                                        <code style="font-family: monospace; font-size: 13px; color: var(--color-accent);">
                                            {m.id.clone()}
                                        </code>
                                    </div>
                                    <div class="form__group">
                                        <label class="form__label">"Название"</label>
                                        <span>{m.name.clone()}</span>
                                    </div>
                                    <div class="form__group">
                                        <label class="form__label">"Категория"</label>
                                        <Badge color=BadgeColor::Informative>{m.category.clone()}</Badge>
                                    </div>
                                    <div class="form__group">
                                        <label class="form__label">"Версия"</label>
                                        <span>{m.version}</span>
                                    </div>
                                    <div class="form__group">
                                        <label class="form__label">"Описание"</label>
                                        <p style="margin: 0; color: var(--color-text-secondary);">{m.description.clone()}</p>
                                    </div>
                                    <div class="form__group">
                                        <label class="form__label">"Источники данных"</label>
                                        <div style="display: flex; gap: 6px; flex-wrap: wrap;">
                                            {m.data_sources.iter().map(|s| {
                                                let s = s.clone();
                                                view! { <Badge color=BadgeColor::Subtle>{icon("database")} " " {s}</Badge> }
                                            }).collect_view()}
                                        </div>
                                    </div>
                                </CardAnimated>

                                <CardAnimated delay_ms=100>
                                    <h4 class="details-section__title">{icon("bar-chart-2")} " Доступные измерения (Drilldown)"</h4>
                                    <p class="form__hint">"По каким разрезам можно детализировать значение."</p>
                                    <div style="display: flex; flex-direction: column; gap: 8px; margin-top: 8px;">
                                        {m.available_dimensions.iter().map(|d| {
                                            let label = d.label.clone();
                                            let id_d  = d.id.clone();
                                            let desc  = d.description.clone();
                                            view! {
                                                <div style="display: flex; align-items: baseline; gap: 10px; \
                                                           padding: 6px 10px; border-radius: var(--radius-sm); \
                                                           background: var(--color-surface-2); border: 1px solid var(--color-border);">
                                                    <code style="font-size: 11px; color: var(--color-accent); min-width: 160px; font-family: monospace;">{id_d}</code>
                                                    <span style="font-weight: 600; font-size: 13px;">{label}</span>
                                                    {if !desc.is_empty() {
                                                        view! { <span style="font-size: 12px; color: var(--color-text-secondary);">" — " {desc}</span> }.into_any()
                                                    } else {
                                                        view! { <span /> }.into_any()
                                                    }}
                                                </div>
                                            }
                                        }).collect_view()}
                                    </div>
                                </CardAnimated>

                                <CardAnimated delay_ms=200>
                                    <h4 class="details-section__title">{icon("filter")} " Фильтры"</h4>
                                    <p class="form__hint">"Фильтры, применяемые к этому DataView (из глобального реестра)."</p>
                                    {move || {
                                        let filters = view_filters.get();
                                        if filters.is_empty() {
                                            return view! {
                                                <div class="placeholder" style="font-size: 13px; padding: 8px 0;">"Нет фильтров"</div>
                                            }.into_any();
                                        }
                                        view! {
                                            <div style="display: flex; flex-direction: column; gap: 8px; margin-top: 8px;">
                                                {filters.into_iter().map(|f| {
                                                    let fid   = f.id.clone();
                                                    let label = f.label.clone();
                                                    let (kind_color, kind_label) = match &f.kind {
                                                        FilterKind::DateRange { .. }   => (BadgeColor::Informative, "DateRange"),
                                                        FilterKind::MultiSelect { .. } => (BadgeColor::Success,   "MultiSelect"),
                                                        FilterKind::Select { .. }      => (BadgeColor::Warning,   "Select"),
                                                        FilterKind::Text               => (BadgeColor::Subtle,    "Text"),
                                                    };
                                                    view! {
                                                        <div style="display: flex; align-items: center; gap: 10px; \
                                                                   padding: 6px 10px; border-radius: var(--radius-sm); \
                                                                   background: var(--color-surface-2); border: 1px solid var(--color-border);">
                                                            <code style="font-size: 11px; color: var(--color-accent); min-width: 160px; font-family: monospace;">{fid}</code>
                                                            <span style="font-weight: 600; font-size: 13px; flex: 1;">{label}</span>
                                                            <Badge color=kind_color>{kind_label}</Badge>
                                                        </div>
                                                    }
                                                }).collect_view()}
                                            </div>
                                        }.into_any()
                                    }}
                                </CardAnimated>

                            </div>

                            // ── Правая колонка: AI + ресурсы + тест ─────────────────────────
                            <div class="detail-grid__col">

                                <CardAnimated delay_ms=50>
                                    <h4 class="details-section__title">{icon("bot")} " AI/LLM описание"</h4>
                                    <p class="form__hint">"Используется LLM-агентами для понимания семантики DataView."</p>
                                    <div style="padding: 12px; background: var(--color-surface-2); \
                                               border-radius: var(--radius-sm); border: 1px solid var(--color-border); \
                                               font-size: 13px; line-height: 1.6; white-space: pre-wrap; \
                                               color: var(--color-text-secondary);">
                                        {m.ai_description.clone()}
                                    </div>
                                </CardAnimated>

                                {if !m.available_resources.is_empty() {
                                    let resources = m.available_resources.clone();
                                    view! {
                                        <CardAnimated delay_ms=150>
                                            <h4 class="details-section__title">{icon("zap")} " Доступные ресурсы (метрики)"</h4>
                                            <p class="form__hint">"Какие метрики умеет вычислять DataView (параметр " <code style="font-family: monospace; font-size: 12px;">metric</code> ")."</p>
                                            <div style="display: flex; flex-direction: column; gap: 8px; margin-top: 8px;">
                                                {resources.iter().map(|r| {
                                                    let rid   = r.id.clone();
                                                    let label = r.label.clone();
                                                    let desc  = r.description.clone();
                                                    let unit  = r.unit.clone();
                                                    view! {
                                                        <div style="display: flex; align-items: baseline; gap: 10px; \
                                                                   padding: 6px 10px; border-radius: var(--radius-sm); \
                                                                   background: var(--color-surface-2); border: 1px solid var(--color-border);">
                                                            <code style="font-size: 11px; color: var(--color-accent); min-width: 120px; font-family: monospace;">{rid}</code>
                                                            <span style="font-weight: 600; font-size: 13px;">{label}</span>
                                                            {if !unit.is_empty() {
                                                                view! { <Badge color=BadgeColor::Subtle>{unit}</Badge> }.into_any()
                                                            } else {
                                                                view! { <span /> }.into_any()
                                                            }}
                                                            {if !desc.is_empty() {
                                                                view! { <span style="font-size: 12px; color: var(--color-text-secondary);">" — " {desc}</span> }.into_any()
                                                            } else {
                                                                view! { <span /> }.into_any()
                                                            }}
                                                        </div>
                                                    }
                                                }).collect_view()}
                                            </div>
                                        </CardAnimated>
                                    }.into_any()
                                } else {
                                    view! { <span /> }.into_any()
                                }}

                                <CardAnimated delay_ms=250>
                                    <h4 class="details-section__title">{icon("play")} " Тест на реальных данных"</h4>
                                <p class="form__hint">
                                    "Прямой вызов DataView без привязки к индикатору."
                                </p>

                                <div class="form__group">
                                    <label class="form__label">"Метрика"</label>
                                    <select class="form__input"
                                        style="font-family: monospace; font-size: 13px;"
                                        on:change=move |ev| {
                                            use wasm_bindgen::JsCast;
                                            let val = ev.target()
                                                .and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok())
                                                .map(|el| el.value())
                                                .unwrap_or_default();
                                            test_metric.set(val);
                                        }
                                    >
                                        <option value="revenue" selected=move || test_metric.get() == "revenue">"revenue — Выручка"</option>
                                        <option value="cost" selected=move || test_metric.get() == "cost">"cost — Себестоимость"</option>
                                        <option value="commission" selected=move || test_metric.get() == "commission">"commission — Комиссия МП"</option>
                                        <option value="expenses" selected=move || test_metric.get() == "expenses">"expenses — Расходы"</option>
                                        <option value="profit" selected=move || test_metric.get() == "profit">"profit — Прибыль продавца"</option>
                                        <option value="profit_d" selected=move || test_metric.get() == "profit_d">"profit_d — Прибыль (дилер)"</option>
                                    </select>
                                </div>

                                <div class="form__group">
                                    <label class="form__label">"Кабинет МП"</label>
                                    <ConnectionMpMultiSelect selected=test_connection_ids />
                                </div>

                                <div class="form__group">
                                    <label class="form__label">"Период 1 (основной)"</label>
                                    <DateRangePicker
                                    date_from=Signal::derive(move || test_date_from.get())
                                    date_to=Signal::derive(move || test_date_to.get())
                                    on_change=Callback::new(move |(f, t)| {
                                        test_date_from.set(f);
                                        test_date_to.set(t);
                                    })
                                />
                                </div>

                                <div class="form__group">
                                    <label class="form__label">"Период 2 (дополнительный)"</label>
                                    <DateRangePicker
                                        date_from=Signal::derive(move || test_period2_from.get())
                                        date_to=Signal::derive(move || test_period2_to.get())
                                        on_change=Callback::new(move |(f, t)| {
                                            test_period2_from.set(f);
                                            test_period2_to.set(t);
                                        })
                                />
                                </div>

                                <div style="margin-top: var(--spacing-md); display: flex; gap: var(--spacing-sm); align-items: center;">
                                    <Button appearance=ButtonAppearance::Primary on_click=run_test.clone() disabled=test_loading>
                                        {icon("play")} " Выполнить"
                                    </Button>
                                    {move || test_loading.get().then(|| view! {
                                        <span style="color: var(--color-text-secondary); font-size: 13px;">"Вычисление..."</span>
                                    })}
                                </div>

                                {move || test_error.get().map(|e| view! {
                                    <div class="warning-box"
                                        style="margin-top: var(--spacing-sm); background: var(--color-error-50); border-color: var(--color-error-100);">
                                        <span class="warning-box__icon" style="color: var(--color-error);">"⚠"</span>
                                        <span class="warning-box__text" style="color: var(--color-error);">{e}</span>
                                    </div>
                                })}

                                {move || test_result.get().map(|r| {
                                    let value_str = r.value.map(|v| format!("{v:.2}")).unwrap_or_else(|| "—".into());
                                    let prev_str  = r.previous_value.map(|v| format!("{v:.2}")).unwrap_or_else(|| "—".into());
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

                                    // Compute sparkline SVG polyline points
                                    let spark = r.spark_points.clone();
                                    let spark_count = spark.len();
                                    let spark_svg = if spark.len() >= 2 {
                                        let w = 200.0_f64;
                                        let h = 40.0_f64;
                                        let min = spark.iter().cloned().fold(f64::INFINITY, f64::min);
                                        let max = spark.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                                        let range = (max - min).max(1.0);
                                        let n = spark.len() as f64 - 1.0;
                                        let pts: String = spark.iter().enumerate().map(|(i, &v)| {
                                            let x = (i as f64 / n) * w;
                                            let y = h - ((v - min) / range) * (h - 4.0) - 2.0;
                                            format!("{:.1},{:.1}", x, y)
                                        }).collect::<Vec<_>>().join(" ");
                                        Some(pts)
                                    } else {
                                        None
                                    };

                                    // Auto-computed P2 dates (mirrors backend shift_date logic)
                                    let p2_from_actual = test_period2_from.get();
                                    let p2_to_actual   = test_period2_to.get();
                                    let p2_label = if p2_from_actual.trim().is_empty() {
                                        format!("авто (−1 мес. от {} … {})", test_date_from.get(), test_date_to.get())
                                    } else {
                                        format!("{} … {}", p2_from_actual, p2_to_actual)
                                    };

                                    view! {
                                        <div style="margin-top: var(--spacing-md); padding: var(--spacing-md); \
                                                   border: 1px solid var(--color-border); border-radius: var(--radius-md); \
                                                   background: var(--color-surface-2); display: flex; flex-direction: column; gap: 12px;">

                                            <div style="display: flex; gap: var(--spacing-lg); flex-wrap: wrap; align-items: baseline;">
                                                <div>
                                                    <span style="font-size: 11px; color: var(--color-text-secondary); text-transform: uppercase;">"Период 1 (P1)"</span>
                                                    <div style="font-size: 28px; font-weight: 700; font-family: monospace;">{value_str}</div>
                                                    <span style="font-size: 11px; color: var(--color-text-muted);">
                                                        {test_date_from.get()} " … " {test_date_to.get()}
                                                    </span>
                                                </div>
                                                <div>
                                                    <span style="font-size: 11px; color: var(--color-text-secondary); text-transform: uppercase;">"Период 2 (P2)"</span>
                                                    <div style="font-size: 18px; color: var(--color-text-secondary); font-family: monospace;">{prev_str}</div>
                                                    <span style="font-size: 11px; color: var(--color-text-muted);">{p2_label}</span>
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

                                            <div>
                                                <span style="font-size: 11px; color: var(--color-text-secondary); text-transform: uppercase;">
                                                    "Sparkline (" {spark_count} " точек по дням)"
                                                </span>
                                                {if let Some(pts) = spark_svg {
                                                    view! {
                                                        <div style="margin-top: 6px;">
                                                            <svg width="200" height="40"
                                                                style="display: block; overflow: visible;">
                                                                <polyline
                                                                    points={pts}
                                                                    fill="none"
                                                                    stroke="var(--color-accent)"
                                                                    stroke-width="1.5"
                                                                    stroke-linejoin="round"
                                                                    stroke-linecap="round"
                                                                />
                                                            </svg>
                                                        </div>
                                                    }.into_any()
                                                } else if spark_count == 0 {
                                                    view! {
                                                        <div style="font-size: 12px; color: var(--color-text-muted); margin-top: 4px;">
                                                            "Нет данных за период"
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <div style="font-size: 12px; color: var(--color-text-muted); margin-top: 4px;">
                                                            "Только 1 точка"
                                                        </div>
                                                    }.into_any()
                                                }}
                                            </div>
                                        </div>
                                    }
                                })}
                                </CardAnimated>

                            </div>

                        </div>
                    </div>
                }.into_any()
            }}
        </PageFrame>
    }
}
