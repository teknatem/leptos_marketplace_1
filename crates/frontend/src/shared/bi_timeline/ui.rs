use std::collections::HashMap;

use chrono::{Datelike, Duration, NaiveDate, Utc};
use contracts::shared::analytics::ValueFormat;
use contracts::shared::bi_timeline::{
    BiTimelineIndicatorInfo, BiTimelinePoint, BiTimelineRequest, BiTimelineResponse,
    BiTimelineSeries,
};
use contracts::shared::data_view::ViewContext;
use leptos::prelude::*;
use thaw::*;
use wasm_bindgen_futures::spawn_local;

use crate::data_view::api as dv_api;
use crate::data_view::types::FilterDef;
use crate::data_view::ui::FilterBar;
use crate::layout::global_context::AppGlobalContext;
use crate::layout::tabs::{detail_tab_label, pick_identifier};
use crate::shared::bi_timeline::api;
use crate::shared::components::popover::IndicatorInfoButton;
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;

const BI_TIMELINE_DAY_COUNT: usize = 31;

#[derive(Clone, Debug, Default)]
pub struct BiTimelineInitial {
    pub indicator_id: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub period2_from: Option<String>,
    pub period2_to: Option<String>,
    pub connection_mp_refs: Vec<String>,
}

fn fmt_ymd(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

fn month_bounds(anchor: NaiveDate) -> (NaiveDate, NaiveDate) {
    let start = NaiveDate::from_ymd_opt(anchor.year(), anchor.month(), 1).unwrap_or(anchor);
    let end = if anchor.month() == 12 {
        NaiveDate::from_ymd_opt(anchor.year() + 1, 1, 1)
            .map(|date| date - Duration::days(1))
            .unwrap_or(anchor)
    } else {
        NaiveDate::from_ymd_opt(anchor.year(), anchor.month() + 1, 1)
            .map(|date| date - Duration::days(1))
            .unwrap_or(anchor)
    };
    (start, end)
}

fn default_context(initial: &BiTimelineInitial) -> ViewContext {
    let today = Utc::now().date_naive();
    let (cur_from, cur_to) = month_bounds(today);
    let prev_anchor = cur_from - Duration::days(1);
    let (prev_from, prev_to) = month_bounds(prev_anchor);

    ViewContext {
        date_from: initial
            .date_from
            .clone()
            .unwrap_or_else(|| fmt_ymd(cur_from)),
        date_to: initial.date_to.clone().unwrap_or_else(|| fmt_ymd(cur_to)),
        period2_from: initial
            .period2_from
            .clone()
            .or_else(|| Some(fmt_ymd(prev_from))),
        period2_to: initial
            .period2_to
            .clone()
            .or_else(|| Some(fmt_ymd(prev_to))),
        connection_mp_refs: initial.connection_mp_refs.clone(),
        params: HashMap::new(),
    }
}

fn timeline_filters(all: Vec<FilterDef>) -> Vec<FilterDef> {
    let mut filters: Vec<FilterDef> = all
        .into_iter()
        .filter(|def| {
            def.id == "date_range_1" || def.id == "date_range_2" || def.id == "connection_mp_refs"
        })
        .collect();
    filters.sort_by_key(|def| match def.id.as_str() {
        "date_range_1" => 1,
        "date_range_2" => 2,
        "connection_mp_refs" => 3,
        _ => 99,
    });
    filters
}

fn format_value(value: f64, format: &ValueFormat) -> String {
    let triads = |decimals: usize| format_number_triads(value, decimals);
    match format {
        ValueFormat::Money {
            currency, decimals, ..
        } => {
            format!("{} {}", triads(decimals.unwrap_or(0) as usize), currency)
        }
        ValueFormat::Number { decimals } => triads(*decimals as usize),
        ValueFormat::Percent { decimals } => format!("{}%", triads(*decimals as usize)),
        ValueFormat::Integer => triads(0),
    }
}

fn format_number_triads(value: f64, decimals: usize) -> String {
    let sign = if value < 0.0 { "-" } else { "" };
    let abs_value = value.abs();
    let raw = format!("{:.*}", decimals, abs_value);
    let (whole, fraction) = raw.split_once('.').unwrap_or((raw.as_str(), ""));
    let mut grouped_rev = String::new();
    for (idx, ch) in whole.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            grouped_rev.push(' ');
        }
        grouped_rev.push(ch);
    }
    let whole_grouped: String = grouped_rev.chars().rev().collect();
    if decimals == 0 {
        format!("{sign}{whole_grouped}")
    } else {
        format!("{sign}{whole_grouped}.{fraction}")
    }
}

fn sum_points(points: &[BiTimelinePoint]) -> f64 {
    points.iter().map(|point| point.value).sum()
}

fn context_equals(left: &ViewContext, right: &ViewContext) -> bool {
    left.date_from == right.date_from
        && left.date_to == right.date_to
        && left.period2_from == right.period2_from
        && left.period2_to == right.period2_to
        && left.connection_mp_refs == right.connection_mp_refs
        && left.params == right.params
}

fn chart_path(
    points: &[BiTimelinePoint],
    min: f64,
    max: f64,
    width: f64,
    height: f64,
    day_count: usize,
) -> String {
    if points.is_empty() {
        return String::new();
    }

    let offset_range = day_count.saturating_sub(1).max(1) as f64;
    let value_range = if (max - min).abs() < 1e-9 {
        1.0
    } else {
        max - min
    };

    points
        .iter()
        .enumerate()
        .map(|(idx, point)| {
            let clamped_offset = point.offset.clamp(0, day_count.saturating_sub(1) as i64);
            let x = (clamped_offset as f64 / offset_range) * width;
            let y = height - ((point.value - min) / value_range) * height;
            if idx == 0 {
                format!("M{:.1} {:.1}", x, y)
            } else {
                format!("L{:.1} {:.1}", x, y)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn x_axis_ticks(day_count: usize, width: f64) -> Vec<f64> {
    let offset_range = day_count.saturating_sub(1).max(1) as f64;
    (0..day_count)
        .map(|offset| (offset as f64 / offset_range) * width)
        .collect()
}

#[component]
fn TimelineChart(item: BiTimelineSeries) -> impl IntoView {
    let all_values: Vec<f64> = item
        .series_p1
        .iter()
        .chain(item.series_p2.iter())
        .map(|point| point.value)
        .collect();
    let min = all_values.iter().cloned().fold(0.0, f64::min);
    let max = all_values.iter().cloned().fold(0.0, f64::max);
    let width = 640.0;
    let plot_height = 210.0;
    let svg_height = plot_height + 6.0;
    let p1_path = chart_path(
        &item.series_p1,
        min,
        max,
        width,
        plot_height,
        BI_TIMELINE_DAY_COUNT,
    );
    let p2_path = chart_path(
        &item.series_p2,
        min,
        max,
        width,
        plot_height,
        BI_TIMELINE_DAY_COUNT,
    );
    let axis_ticks = x_axis_ticks(BI_TIMELINE_DAY_COUNT, width);
    let total_p1 = sum_points(&item.series_p1);
    let total_p2 = sum_points(&item.series_p2);
    let delta = if total_p2.abs() < 0.01 {
        None
    } else {
        Some(((total_p1 - total_p2) / total_p2.abs()) * 100.0)
    };
    let delta_label = delta
        .map(|value| {
            if value >= 0.0 {
                format!("+{:.1}%", value)
            } else {
                format!("{:.1}%", value)
            }
        })
        .unwrap_or_else(|| "—".to_string());
    let format_spec = item.indicator.format.clone();

    view! {
        <section class="bi-timeline-card">
            <div class="bi-timeline-card__header">
                <div>
                    <div class="bi-timeline-card__title">{item.indicator.description.clone()}</div>
                    <div class="bi-timeline-card__subtitle">
                        {item.indicator.code.clone()}
                        " · "
                        {item.indicator.view_id.clone().unwrap_or_default()}
                        {item.indicator.metric_id.clone().map(|metric| format!(" · metric: {}", metric)).unwrap_or_default()}
                    </div>
                </div>
                <div class="bi-timeline-card__totals">
                    <span>{format_value(total_p1, &format_spec)}</span>
                    <span class="bi-timeline-card__delta">{delta_label}</span>
                </div>
            </div>
            <div class="bi-timeline-chart">
                <svg viewBox=format!("0 0 {} {}", width, svg_height) preserveAspectRatio="none">
                    <path class="bi-timeline-chart__grid" d=format!("M0 {:.1} L{:.1} {:.1}", plot_height, width, plot_height) />
                    <path class="bi-timeline-chart__line bi-timeline-chart__line--p1" d=p1_path />
                    <path class="bi-timeline-chart__line bi-timeline-chart__line--p2" d=p2_path />
                    {axis_ticks.into_iter().map(|x| view! {
                        <g class="bi-timeline-chart__tick">
                            <line x1=format!("{:.1}", x) y1=format!("{:.1}", plot_height) x2=format!("{:.1}", x) y2=format!("{:.1}", plot_height + 5.0) />
                        </g>
                    }).collect_view()}
                </svg>
                <div class="bi-timeline-chart__axis">
                    {(1..=BI_TIMELINE_DAY_COUNT).map(|day| view! {
                        <span>{day}</span>
                    }).collect_view()}
                </div>
            </div>
            <div class="bi-timeline-card__legend">
                <span><i class="bi-timeline-card__dot bi-timeline-card__dot--p1"></i>{item.period1_label}</span>
                <span><i class="bi-timeline-card__dot bi-timeline-card__dot--p2"></i>{item.period2_label}</span>
            </div>
        </section>
    }
}

#[component]
pub fn BiTimelinePage(initial: BiTimelineInitial) -> impl IntoView {
    let tabs_store = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let ctx = RwSignal::new(default_context(&initial));
    let filters = RwSignal::new(Vec::<FilterDef>::new());
    let indicators = RwSignal::new(Vec::<BiTimelineIndicatorInfo>::new());
    let selected_ids = RwSignal::new(Vec::<String>::new());
    let response = RwSignal::new(None::<BiTimelineResponse>);
    let applied_ctx = RwSignal::new(ctx.get_untracked());
    let applied_selected_ids = RwSignal::new(Vec::<String>::new());
    let last_auto_refresh_selection = RwSignal::new(Vec::<String>::new());
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let is_filter_expanded = RwSignal::new(true);
    let has_pending_changes =
        Signal::derive(move || !context_equals(&ctx.get(), &applied_ctx.get()));

    let initial_indicator_id = initial.indicator_id.clone();

    Effect::new(move |_| {
        let initial_indicator_id = initial_indicator_id.clone();
        spawn_local(async move {
            match dv_api::fetch_global_filters().await {
                Ok(defs) => {
                    let defs = timeline_filters(defs);
                    filters.set(defs);
                }
                Err(err) => error.set(Some(format!("Фильтры BI Timeline: {}", err))),
            }

            match api::fetch_indicators().await {
                Ok(resp) => {
                    let mut ids = Vec::new();
                    if let Some(initial_id) = initial_indicator_id.clone() {
                        ids.push(initial_id);
                    } else {
                        ids = resp
                            .indicators
                            .iter()
                            .filter(|item| item.priority)
                            .take(3)
                            .map(|item| item.id.clone())
                            .collect();
                        if ids.is_empty() {
                            ids = resp
                                .indicators
                                .iter()
                                .take(3)
                                .map(|item| item.id.clone())
                                .collect();
                        }
                    }
                    indicators.set(resp.indicators);
                    selected_ids.set(ids);
                    let selected = selected_ids.get_untracked();
                    if !selected.is_empty() {
                        loading.set(true);
                        let request_ctx = ctx.get_untracked();
                        let request = BiTimelineRequest {
                            context: request_ctx.clone(),
                            indicator_ids: selected.clone(),
                            indicator_codes: vec![],
                            params: HashMap::new(),
                        };
                        match api::fetch_series(&request).await {
                            Ok(resp) => {
                                applied_ctx.set(request_ctx);
                                applied_selected_ids.set(selected);
                                response.set(Some(resp));
                            }
                            Err(err) => error.set(Some(err)),
                        }
                        loading.set(false);
                    }
                }
                Err(err) => error.set(Some(format!("Индикаторы BI Timeline: {}", err))),
            }
        });
    });

    let run_query = Callback::new(move |_: ()| {
        loading.set(true);
        error.set(None);
        let request_ctx = ctx.get_untracked();
        let selected = selected_ids.get_untracked();
        last_auto_refresh_selection.set(selected.clone());
        let request = BiTimelineRequest {
            context: request_ctx.clone(),
            indicator_ids: selected.clone(),
            indicator_codes: vec![],
            params: HashMap::new(),
        };
        spawn_local(async move {
            match api::fetch_series(&request).await {
                Ok(resp) => {
                    applied_ctx.set(request_ctx);
                    applied_selected_ids.set(selected);
                    response.set(Some(resp));
                }
                Err(err) => error.set(Some(err)),
            }
            loading.set(false);
        });
    });

    Effect::new(move |_| {
        let selected = selected_ids.get();
        if selected == applied_selected_ids.get() || selected == last_auto_refresh_selection.get() {
            return;
        }
        if response.get_untracked().is_none() || loading.get_untracked() {
            return;
        }

        error.set(None);
        last_auto_refresh_selection.set(selected.clone());

        if selected.is_empty() {
            applied_ctx.set(ctx.get_untracked());
            applied_selected_ids.set(selected);
            response.set(Some(BiTimelineResponse {
                items: vec![],
                errors: vec![],
            }));
            return;
        }

        loading.set(true);
        let request_ctx = ctx.get_untracked();
        let request = BiTimelineRequest {
            context: request_ctx.clone(),
            indicator_ids: selected.clone(),
            indicator_codes: vec![],
            params: HashMap::new(),
        };
        spawn_local(async move {
            match api::fetch_series(&request).await {
                Ok(resp) => {
                    applied_ctx.set(request_ctx);
                    applied_selected_ids.set(selected);
                    response.set(Some(resp));
                }
                Err(err) => error.set(Some(err)),
            }
            loading.set(false);
        });
    });

    view! {
        <PageFrame page_id="bi_timeline" category="dashboard">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"BI Timeline"</h1>
                    <div class="page__subtitle">
                        "Просмотр динамики BI-индикаторов"
                    </div>
                </div>
                <div class="page__header-right">
                    <Button
                        appearance=ButtonAppearance::Primary
                        disabled=Signal::derive(move || loading.get())
                        on_click=move |_| run_query.run(())
                    >
                        {move || if loading.get() { "Строим..." } else if has_pending_changes.get() { "Обновить графики" } else { "Построить графики" }}
                    </Button>
                </div>
            </div>

            <div class="page__content bi-timeline">
                {move || error.get().map(|message| view! {
                    <div class="form__error">{message}</div>
                })}

                <div class="filter-panel">
                    <div class="filter-panel-header">
                        <div
                            class="filter-panel-header__left"
                            on:click=move |_| is_filter_expanded.update(|expanded| *expanded = !*expanded)
                        >
                            <svg
                                width="16"
                                height="16"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                class=move || if is_filter_expanded.get() {
                                    "filter-panel__chevron filter-panel__chevron--expanded"
                                } else {
                                    "filter-panel__chevron"
                                }
                            >
                                <polyline points="6 9 12 15 18 9"></polyline>
                            </svg>
                            {icon("filter")}
                            <span class="filter-panel__title">"Фильтр"</span>
                        </div>
                        <div class="filter-panel-header__right" />
                    </div>
                    <Show when=move || is_filter_expanded.get()>
                        <div class="filter-panel-content">
                            {move || {
                                let defs = filters.get();
                                if defs.is_empty() {
                                    view! { <div class="form__hint">"Загрузка фильтров..."</div> }.into_any()
                                } else {
                                    view! { <FilterBar filters=defs ctx=ctx /> }.into_any()
                                }
                            }}
                        </div>
                    </Show>
                </div>

                <div class="bi-timeline__layout">
                    <section class="bi-timeline__indicators">
                        <div class="bi-timeline__panel-title">{icon("activity")} " Индикаторы"</div>
                        {move || {
                            let selected = selected_ids.get();
                            let list = indicators.get();
                            if list.is_empty() {
                                view! { <div class="placeholder placeholder--small">"Индикаторы не найдены."</div> }.into_any()
                            } else {
                                list.into_iter().map(|indicator| {
                                    let id = indicator.id.clone();
                                    let checked = selected.contains(&id);
                                    let row_class = if checked {
                                        "bi-timeline-indicator bi-timeline-indicator--selected"
                                    } else {
                                        "bi-timeline-indicator"
                                    };
                                    let indicator_id_nav = id.clone();
                                    let code_nav = indicator.code.clone();
                                    let desc_nav = indicator.description.clone();
                                    view! {
                                        <label class=row_class>
                                            <input
                                                type="checkbox"
                                                prop:checked=checked
                                                on:change=move |ev| {
                                                    let is_checked = event_target_checked(&ev);
                                                    let id = id.clone();
                                                    selected_ids.update(|items| {
                                                        if is_checked {
                                                            if !items.contains(&id) {
                                                                items.push(id);
                                                            }
                                                        } else {
                                                            items.retain(|item| item != &id);
                                                        }
                                                    });
                                                }
                                            />
                                            <span>
                                                <strong>{indicator.description.clone()}</strong>
                                            </span>
                                            <IndicatorInfoButton
                                                title=indicator.description.clone()
                                                comment=indicator.comment.clone()
                                                on_open=Callback::new(move |_| {
                                                    use contracts::domain::a024_bi_indicator::ENTITY_METADATA as A024;
                                                    let identifier = pick_identifier(None, Some(&code_nav), Some(&desc_nav), &indicator_id_nav);
                                                    let title = detail_tab_label(A024.ui.element_name, &identifier);
                                                    tabs_store.open_tab(&format!("a024_bi_indicator_details_{}", indicator_id_nav), &title);
                                                })
                                            />
                                        </label>
                                    }
                                }).collect_view().into_any()
                            }
                        }}
                    </section>

                    <section class="bi-timeline__charts">
                        {move || if let Some(resp) = response.get() {
                            if resp.items.is_empty() {
                                view! {
                                    <div class="placeholder">
                                        "Нет данных для выбранных индикаторов и периода."
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <>
                                        {resp.items.into_iter().map(|item| view! {
                                            <TimelineChart item=item />
                                        }).collect_view()}
                                        {loading.get().then(|| view! {
                                            <div class="form__hint">"Обновляем дневные ряды..."</div>
                                        })}
                                        {(!resp.errors.is_empty()).then(|| view! {
                                            <div class="form__hint">
                                                {format!("{} индикатор(ов) не удалось построить.", resp.errors.len())}
                                            </div>
                                        })}
                                    </>
                                }.into_any()
                            }
                        } else if loading.get() {
                            view! { <div class="placeholder">"Загрузка дневных рядов..."</div> }.into_any()
                        } else {
                            view! {
                                <div class="placeholder">
                                    "Выберите индикаторы и нажмите «Построить графики»."
                                </div>
                            }.into_any()
                        }}
                    </section>
                </div>
            </div>
        </PageFrame>
    }
}
