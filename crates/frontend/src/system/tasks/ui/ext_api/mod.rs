//! Вкладка «Внешний API» страницы sys_tasks.
//!
//! Статистика по **входящим** вызовам `/api/ext/v1/*` — сервису, который система
//! предоставляет наружу (1С, Power BI). Не путать с вкладкой «История», где
//! **исходящий** трафик заданий к API маркетплейсов.

use contracts::system::ext_api_log::{
    ExtApiHistoryResponse, ExtApiLogRow, ExtApiMetric, ExtApiScale, ExtApiSummaryResponse,
    ExtApiSummaryRow, ExtApiTotals,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;

use crate::shared::date_utils::{format_bytes_compact, format_datetime};
use crate::shared::time_bar_chart::{
    default_date_for_scale, format_number_triads, ChartPoint, ChartScale, PeriodNav, ScaleSelector,
    TimeBarChart,
};
use crate::system::tasks::api;

const RECENT_LIMIT: u32 = 100;

fn to_api_scale(scale: ChartScale) -> ExtApiScale {
    match scale {
        ChartScale::Day => ExtApiScale::Day,
        ChartScale::Week => ExtApiScale::Week,
        ChartScale::Month => ExtApiScale::Month,
    }
}

fn metric_label(metric: ExtApiMetric) -> &'static str {
    match metric {
        ExtApiMetric::RequestCount => "Входящие запросы",
        ExtApiMetric::TrafficBytes => "Отданный трафик",
        ExtApiMetric::AvgDurationMs => "Среднее время ответа",
        ExtApiMetric::ErrorCount => "Ошибки (4xx/5xx)",
    }
}

fn metric_param(metric: ExtApiMetric) -> &'static str {
    match metric {
        ExtApiMetric::RequestCount => "request_count",
        ExtApiMetric::TrafficBytes => "traffic_bytes",
        ExtApiMetric::AvgDurationMs => "avg_duration_ms",
        ExtApiMetric::ErrorCount => "error_count",
    }
}

fn format_metric_value(metric: ExtApiMetric, value: f64) -> String {
    match metric {
        ExtApiMetric::TrafficBytes => format_bytes_compact(value.max(0.0) as u64),
        ExtApiMetric::AvgDurationMs => format!("{} мс", format_number_triads(value, 0)),
        ExtApiMetric::RequestCount | ExtApiMetric::ErrorCount => format_number_triads(value, 0),
    }
}

/// Итог за период. Для среднего времени сумма бессмысленна — берём среднее по периоду.
fn period_summary(metric: ExtApiMetric, totals: &ExtApiTotals) -> String {
    match metric {
        ExtApiMetric::RequestCount => format_number_triads(totals.req_count as f64, 0),
        ExtApiMetric::TrafficBytes => format_bytes_compact(totals.bytes_out.max(0) as u64),
        ExtApiMetric::AvgDurationMs => format!("{} мс", format_number_triads(totals.avg_ms, 0)),
        ExtApiMetric::ErrorCount => format_number_triads(totals.error_count as f64, 0),
    }
}

fn status_class(status: i32) -> &'static str {
    if status >= 500 {
        "ext-api__status ext-api__status--server-error"
    } else if status >= 400 {
        "ext-api__status ext-api__status--client-error"
    } else {
        "ext-api__status ext-api__status--ok"
    }
}

#[component]
fn StatCard(label: &'static str, value: String, accent: bool) -> impl IntoView {
    view! {
        <div class=move || if accent { "ext-api__stat ext-api__stat--accent" } else { "ext-api__stat" }>
            <div class="ext-api__stat-label">{label}</div>
            <div class="ext-api__stat-value">{value}</div>
        </div>
    }
}

#[component]
fn SummaryTable(
    title: &'static str,
    empty: &'static str,
    rows: Vec<ExtApiSummaryRow>,
) -> impl IntoView {
    view! {
        <div class="ext-api__panel">
            <div class="ext-api__panel-title">{title}</div>
            {if rows.is_empty() {
                view! { <div class="ext-api__empty">{empty}</div> }.into_any()
            } else {
                view! {
                    <div class="ext-api__table-wrap">
                        <table class="ext-api__table">
                            <thead>
                                <tr>
                                    <th>"Ключ"</th>
                                    <th class="ext-api__num">"Запросов"</th>
                                    <th class="ext-api__num">"Трафик"</th>
                                    <th class="ext-api__num">"Ср. время"</th>
                                    <th class="ext-api__num">"Ошибок"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {rows.into_iter().map(|r| {
                                    let has_errors = r.error_count > 0;
                                    let key_title = r.key.clone();
                                    let error_class = if has_errors {
                                        "ext-api__num ext-api__num--error"
                                    } else {
                                        "ext-api__num"
                                    };
                                    view! {
                                        <tr>
                                            <td class="ext-api__key" title=key_title>{r.key}</td>
                                            <td class="ext-api__num">{format_number_triads(r.req_count as f64, 0)}</td>
                                            <td class="ext-api__num">{format_bytes_compact(r.bytes_out.max(0) as u64)}</td>
                                            <td class="ext-api__num">{format!("{} мс", format_number_triads(r.avg_ms, 0))}</td>
                                            <td class=error_class>
                                                {format_number_triads(r.error_count as f64, 0)}
                                            </td>
                                        </tr>
                                    }
                                }).collect_view()}
                            </tbody>
                        </table>
                    </div>
                }.into_any()
            }}
        </div>
    }
}

#[component]
fn RecentTable(rows: Vec<ExtApiLogRow>) -> impl IntoView {
    view! {
        <div class="ext-api__panel">
            <div class="ext-api__panel-title">
                {format!("Последние вызовы (до {RECENT_LIMIT})")}
            </div>
            {if rows.is_empty() {
                view! { <div class="ext-api__empty">"Вызовов пока не было."</div> }.into_any()
            } else {
                view! {
                    <div class="ext-api__table-wrap">
                        <table class="ext-api__table">
                            <thead>
                                <tr>
                                    <th>"Время"</th>
                                    <th>"Метод"</th>
                                    <th>"Эндпоинт"</th>
                                    <th>"Параметры"</th>
                                    <th class="ext-api__num">"Статус"</th>
                                    <th class="ext-api__num">"Время"</th>
                                    <th class="ext-api__num">"Размер"</th>
                                    <th>"Потребитель"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {rows.into_iter().map(|r| {
                                    let consumer = r
                                        .client_id
                                        .clone()
                                        .or_else(|| r.user_agent.clone())
                                        .or_else(|| r.client_ip.clone())
                                        .unwrap_or_else(|| "—".to_string());
                                    let consumer_title = format!(
                                        "IP: {} · UA: {}",
                                        r.client_ip.clone().unwrap_or_else(|| "—".into()),
                                        r.user_agent.clone().unwrap_or_else(|| "—".into())
                                    );
                                    let query = r.query.clone().unwrap_or_else(|| "—".to_string());
                                    let query_title = query.clone();
                                    let route_title = r.route.clone();
                                    let ts = format_datetime(&r.ts);
                                    view! {
                                        <tr>
                                            <td class="ext-api__mono">{ts}</td>
                                            <td>{r.method}</td>
                                            <td class="ext-api__key" title=route_title>{r.route}</td>
                                            <td class="ext-api__query" title=query_title>{query}</td>
                                            <td class="ext-api__num">
                                                <span class=status_class(r.status)>{r.status}</span>
                                            </td>
                                            <td class="ext-api__num">{format!("{} мс", r.duration_ms)}</td>
                                            <td class="ext-api__num">{format_bytes_compact(r.bytes_out.max(0) as u64)}</td>
                                            <td class="ext-api__key" title=consumer_title>{consumer}</td>
                                        </tr>
                                    }
                                }).collect_view()}
                            </tbody>
                        </table>
                    </div>
                }.into_any()
            }}
        </div>
    }
}

#[component]
pub fn ExtApiView() -> impl IntoView {
    let scale = RwSignal::new(ChartScale::Day);
    let metric = RwSignal::new(ExtApiMetric::RequestCount);
    let date_from = RwSignal::new(default_date_for_scale(ChartScale::Day));
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let history = RwSignal::new(None::<ExtApiHistoryResponse>);
    let summary = RwSignal::new(None::<ExtApiSummaryResponse>);
    let recent = RwSignal::new(Vec::<ExtApiLogRow>::new());

    let load = move || {
        let scale_value = to_api_scale(scale.get_untracked());
        let metric_value = metric.get_untracked();
        let date_value = date_from.get_untracked();
        spawn_local(async move {
            loading.set(true);
            error.set(None);

            match api::fetch_ext_api_history(scale_value, metric_value, &date_value).await {
                Ok(resp) => history.set(Some(resp)),
                Err(err) => error.set(Some(err)),
            }
            match api::fetch_ext_api_summary(scale_value, &date_value).await {
                Ok(resp) => summary.set(Some(resp)),
                Err(err) => error.set(Some(err)),
            }
            match api::fetch_ext_api_recent(RECENT_LIMIT).await {
                Ok(resp) => recent.set(resp.rows),
                Err(err) => error.set(Some(err)),
            }

            loading.set(false);
        });
    };

    Effect::new(move |_| {
        let _ = (scale.get(), metric.get(), date_from.get());
        load();
    });

    let set_scale = Callback::new(move |(next,): (ChartScale,)| {
        scale.set(next);
        date_from.set(default_date_for_scale(next));
    });

    view! {
        <div class="ext-api">
            <div class="ext-api__note">
                "Входящие вызовы внешнего API (" <code>"/api/ext/v1/*"</code> ") — сервис, который "
                "система предоставляет наружу (1С, Power BI). Пишется строка на каждый вызов, "
                "включая отказы: 401 (неверный ключ) и 400 (некорректные параметры) видны здесь же. "
                "Внутренние вызовы интерфейса не учитываются. Хранение — 90 дней. "
                "Исходящие обращения заданий к маркетплейсам — на вкладке «История»."
            </div>

            <div class="ext-api__controls">
                <ScaleSelector scale=scale on_change=set_scale />

                <div class="ext-api__control">
                    <label class="ext-api__control-label">"Метрика"</label>
                    <select
                        class="ext-api__select"
                        prop:value=move || metric_param(metric.get()).to_string()
                        on:change=move |ev| {
                            let next = match event_target_value(&ev).as_str() {
                                "traffic_bytes" => ExtApiMetric::TrafficBytes,
                                "avg_duration_ms" => ExtApiMetric::AvgDurationMs,
                                "error_count" => ExtApiMetric::ErrorCount,
                                _ => ExtApiMetric::RequestCount,
                            };
                            metric.set(next);
                        }
                    >
                        <option value="request_count">{metric_label(ExtApiMetric::RequestCount)}</option>
                        <option value="traffic_bytes">{metric_label(ExtApiMetric::TrafficBytes)}</option>
                        <option value="avg_duration_ms">{metric_label(ExtApiMetric::AvgDurationMs)}</option>
                        <option value="error_count">{metric_label(ExtApiMetric::ErrorCount)}</option>
                    </select>
                </div>

                <PeriodNav date_from=date_from scale=scale />

                <div class="ext-api__control">
                    <label class="ext-api__control-label">"\u{00a0}"</label>
                    <Button appearance=ButtonAppearance::Secondary on_click=move |_| load() disabled=loading>
                        "Обновить"
                    </Button>
                </div>
            </div>

            {move || error.get().map(|err| view! {
                <MessageBar intent=MessageBarIntent::Error>{err}</MessageBar>
            })}

            {move || history.get().map(|resp| {
                let t = resp.totals.clone();
                let req_value = format_number_triads(t.req_count as f64, 0);
                let bytes_value = format_bytes_compact(t.bytes_out.max(0) as u64);
                let avg_value = format!("{} мс", format_number_triads(t.avg_ms, 0));
                let err_value = format_number_triads(t.error_count as f64, 0);
                let has_errors = t.error_count > 0;
                view! {
                    <div class="ext-api__stats">
                        <StatCard label="Всего запросов" value=req_value accent=false />
                        <StatCard label="Отданный трафик" value=bytes_value accent=false />
                        <StatCard label="Среднее время ответа" value=avg_value accent=false />
                        <StatCard label="Ошибки (4xx/5xx)" value=err_value accent=has_errors />
                    </div>
                }
            })}

            {move || if loading.get() {
                view! {
                    <Flex justify=FlexJustify::Center align=FlexAlign::Center style="padding:32px;">
                        <Spinner />" Загрузка статистики..."
                    </Flex>
                }.into_any()
            } else if let Some(resp) = history.get() {
                if resp.points.is_empty() {
                    view! {
                        <div class="ext-api__empty ext-api__empty--chart">
                            "За выбранный период внешних вызовов не было."
                        </div>
                    }.into_any()
                } else {
                    let metric_value = metric.get();
                    let summary_label = period_summary(metric_value, &resp.totals);
                    let points = resp
                        .points
                        .iter()
                        .map(|p| ChartPoint {
                            bucket: p.bucket.clone(),
                            value: p.value,
                            offset: p.offset,
                        })
                        .collect::<Vec<_>>();
                    view! {
                        <TimeBarChart
                            points=points
                            bucket_count=resp.bucket_count
                            date_from=resp.date_from.clone()
                            scale=scale.get()
                            title=metric_label(metric_value).to_string()
                            summary=summary_label
                            format_value=Callback::new(move |(v,): (f64,)| format_metric_value(metric_value, v))
                        />
                    }.into_any()
                }
            } else {
                view! { <></> }.into_any()
            }}

            {move || summary.get().map(|s| view! {
                <div class="ext-api__panels">
                    <SummaryTable
                        title="По эндпоинтам"
                        empty="За выбранный период вызовов не было."
                        rows=s.by_route
                    />
                    <SummaryTable
                        title="По потребителям"
                        empty="За выбранный период вызовов не было."
                        rows=s.by_client
                    />
                </div>
            })}

            {move || view! { <RecentTable rows=recent.get() /> }}
        </div>
    }
}
