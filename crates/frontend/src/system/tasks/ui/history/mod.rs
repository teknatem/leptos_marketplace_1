use contracts::system::tasks::history::{
    TaskHistoryMetric, TaskHistoryPoint, TaskHistoryResponse, TaskHistoryScale,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;

use crate::shared::date_utils::format_bytes_compact;
use crate::shared::time_bar_chart::{
    default_date_for_scale, format_number_triads, ChartPoint, ChartScale, PeriodNav, ScaleSelector,
    TimeBarChart,
};
use crate::system::tasks::api;

fn from_chart_scale(scale: ChartScale) -> TaskHistoryScale {
    match scale {
        ChartScale::Day => TaskHistoryScale::Day,
        ChartScale::Week => TaskHistoryScale::Week,
        ChartScale::Month => TaskHistoryScale::Month,
    }
}

/// Метрики описывают обмен заданий с API маркетплейсов (WB, YM): запросы инициируем мы,
/// но трафик считается в обе стороны и на практике почти целиком состоит из ответов.
/// Вызовы внешних потребителей к нам — на вкладке «Внешний API».
fn metric_label(metric: TaskHistoryMetric) -> &'static str {
    match metric {
        // Бэкенд прибавляет 1.0 в каждый бакет, который занял запуск (без деления),
        // поэтому столбик — конкурентность, а не количество запусков.
        TaskHistoryMetric::TaskCount => "Активных заданий одновременно",
        TaskHistoryMetric::RequestCount => "Запросы к API маркетплейсов",
        // `TrafficBytes` = http_bytes_sent + http_bytes_received, т.е. итог обмена.
        TaskHistoryMetric::TrafficBytes => "Трафик с маркетплейсами (вх. + исх.)",
    }
}

/// Итог в углу графика.
///
/// Для запросов и трафика значение запуска поделено между его бакетами, поэтому сумма
/// по периоду — корректный итог. Для конкурентности деления нет, и сумма дала бы
/// «задание-минуты» — величину без смысла; показываем пик.
fn period_summary(metric: TaskHistoryMetric, points: &[TaskHistoryPoint]) -> String {
    match metric {
        TaskHistoryMetric::TaskCount => {
            let peak = points.iter().map(|p| p.value).fold(0.0, f64::max);
            format!("пик: {}", format_number_triads(peak, 0))
        }
        _ => {
            let total: f64 = points.iter().map(|p| p.value).sum();
            format_metric_value(metric, total)
        }
    }
}

fn format_metric_value(metric: TaskHistoryMetric, value: f64) -> String {
    match metric {
        TaskHistoryMetric::TrafficBytes => format_bytes_compact(value.max(0.0) as u64),
        TaskHistoryMetric::TaskCount | TaskHistoryMetric::RequestCount => {
            format_number_triads(value, 0)
        }
    }
}

#[component]
pub fn TaskHistoryView() -> impl IntoView {
    let scale = RwSignal::new(ChartScale::Day);
    let metric = RwSignal::new(TaskHistoryMetric::TaskCount);
    let date_from = RwSignal::new(default_date_for_scale(ChartScale::Day));
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let response = RwSignal::new(None::<TaskHistoryResponse>);

    let load_history = move || {
        let scale_value = from_chart_scale(scale.get_untracked());
        let metric_value = metric.get_untracked();
        let date_value = date_from.get_untracked();
        spawn_local(async move {
            loading.set(true);
            error.set(None);
            match api::fetch_history(scale_value, metric_value, &date_value).await {
                Ok(resp) => response.set(Some(resp)),
                Err(err) => error.set(Some(err)),
            }
            loading.set(false);
        });
    };

    Effect::new(move |_| {
        let _ = (scale.get(), metric.get(), date_from.get());
        load_history();
    });

    let set_scale = Callback::new(move |(next,): (ChartScale,)| {
        scale.set(next);
        date_from.set(default_date_for_scale(next));
    });

    view! {
        <div style="display:flex;flex-direction:column;gap:14px;">
            <div style="padding:10px 12px;font-size:13px;line-height:1.45;color:var(--color-text-secondary);background:var(--colorNeutralBackground2);border:1px solid var(--color-border);border-radius:var(--radius-md);">
                "Агрегированная история по `sys_task_runs`: обмен заданий с API маркетплейсов "
                "(WB, YM). «Трафик» — итог в обе стороны (отправлено + получено); на практике это "
                "почти целиком ответы маркетплейсов, отправляем мы доли процента. Запросы и трафик "
                "запуска поделены между бакетами, которые он занял, поэтому итог в углу — сумма за "
                "период. «Активных заданий одновременно» устроено иначе: столбик показывает, "
                "сколько запусков перекрывали этот бакет, и в углу — пик, а не сумма. "
                "Масштаб определяет размер бакета: день — минута, неделя — 5 минут, месяц — час. "
                "Входящие вызовы внешних систем к нам — на вкладке «Внешний API»."
            </div>

            <div style="display:flex;gap:12px;align-items:end;flex-wrap:wrap;">
                <ScaleSelector scale=scale on_change=set_scale />

                <div style="display:flex;flex-direction:column;gap:6px;">
                    <label style="font-size:12px;color:var(--color-text-secondary);">"Метрика"</label>
                    <select
                        style="min-width:280px;padding:6px 10px;border:1px solid var(--color-border);border-radius:var(--radius-md);background:var(--colorNeutralBackground1);color:var(--color-text-primary);font-size:13px;cursor:pointer;"
                        prop:value=move || match metric.get() {
                            TaskHistoryMetric::TaskCount => "task_count".to_string(),
                            TaskHistoryMetric::RequestCount => "request_count".to_string(),
                            TaskHistoryMetric::TrafficBytes => "traffic_bytes".to_string(),
                        }
                        on:change=move |ev| {
                            let next = match event_target_value(&ev).as_str() {
                                "request_count" => TaskHistoryMetric::RequestCount,
                                "traffic_bytes" => TaskHistoryMetric::TrafficBytes,
                                _ => TaskHistoryMetric::TaskCount,
                            };
                            metric.set(next);
                        }
                    >
                        <option value="task_count">{metric_label(TaskHistoryMetric::TaskCount)}</option>
                        <option value="request_count">{metric_label(TaskHistoryMetric::RequestCount)}</option>
                        <option value="traffic_bytes">{metric_label(TaskHistoryMetric::TrafficBytes)}</option>
                    </select>
                </div>

                <PeriodNav date_from=date_from scale=scale />
            </div>

            {move || error.get().map(|err| view! {
                <div style="padding:12px;border:1px solid var(--color-error-100);border-radius:var(--radius-md);background:var(--color-error-50);color:var(--color-error);">
                    {err}
                </div>
            })}

            {move || if loading.get() {
                view! {
                    <Flex justify=FlexJustify::Center align=FlexAlign::Center style="padding:32px;">
                        <Spinner />" Загрузка истории..."
                    </Flex>
                }.into_any()
            } else if let Some(resp) = response.get() {
                if resp.points.is_empty() {
                    view! {
                        <div style="padding:24px;color:var(--color-text-secondary);font-size:13px;">
                            "За выбранный период данных нет."
                        </div>
                    }.into_any()
                } else {
                    let metric_value = metric.get();
                    let summary_label = period_summary(metric_value, &resp.points);
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
        </div>
    }
}
