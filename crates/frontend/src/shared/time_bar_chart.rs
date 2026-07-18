//! Столбчатый график по временным бакетам + общие элементы управления периодом.
//!
//! Извлечён из вкладки «История» регламентных заданий, чтобы вкладка «Внешний API»
//! выглядела в том же стиле без копипасты. Компонент ничего не знает о предметной
//! области: вызывающий передаёт точки, подпись и форматтер значений.

use chrono::{Datelike, Duration, NaiveDate, Utc};
use leptos::prelude::*;

use crate::shared::date_utils::format_datetime;
use crate::shared::icons::icon;

/// Масштаб периода. Определяет размер бакета и разметку оси X.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartScale {
    /// Сутки, бакет — минута.
    Day,
    /// Неделя, бакет — 5 минут.
    Week,
    /// Месяц, бакет — час.
    Month,
}

impl ChartScale {
    pub fn label(self) -> &'static str {
        match self {
            ChartScale::Day => "День",
            ChartScale::Week => "Неделя",
            ChartScale::Month => "Месяц",
        }
    }

    pub const ALL: [ChartScale; 3] = [ChartScale::Day, ChartScale::Week, ChartScale::Month];
}

/// Точка временного ряда, независимая от источника данных.
#[derive(Debug, Clone)]
pub struct ChartPoint {
    /// Начало бакета — строка для тултипа.
    pub bucket: String,
    pub value: f64,
    /// Индекс бакета внутри периода.
    pub offset: u32,
}

pub fn today_msk() -> NaiveDate {
    (Utc::now().naive_utc() + Duration::hours(3)).date()
}

fn fmt_ymd(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

/// Начало периода по умолчанию: сегодня / понедельник недели / первое число месяца.
pub fn default_date_for_scale(scale: ChartScale) -> String {
    let today = today_msk();
    match scale {
        ChartScale::Day => fmt_ymd(today),
        ChartScale::Week => {
            let monday_offset = today.weekday().num_days_from_monday() as i64;
            fmt_ymd(today - Duration::days(monday_offset))
        }
        ChartScale::Month => {
            fmt_ymd(NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today))
        }
    }
}

/// Сдвиг начала периода на шаг вперёд/назад в единицах масштаба.
pub fn shift_date_for_scale(date: &str, scale: ChartScale, direction: i32) -> String {
    let parsed = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap_or_else(|_| today_msk());
    let shifted = match scale {
        ChartScale::Day => parsed + Duration::days(direction as i64),
        ChartScale::Week => parsed + Duration::days((direction * 7) as i64),
        ChartScale::Month => shift_month(parsed, direction),
    };
    fmt_ymd(shifted)
}

fn shift_month(date: NaiveDate, direction: i32) -> NaiveDate {
    let month_index = date.year() * 12 + date.month() as i32 - 1 + direction;
    let year = month_index.div_euclid(12);
    let month = month_index.rem_euclid(12) as u32 + 1;
    let last_day = last_day_of_month(year, month);
    NaiveDate::from_ymd_opt(year, month, date.day().min(last_day)).unwrap_or(date)
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
    let next_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    };
    next_month
        .map(|date| (date - Duration::days(1)).day())
        .unwrap_or(31)
}

/// Число с разделением на триады: 1234567 → "1 234 567".
pub fn format_number_triads(value: f64, decimals: usize) -> String {
    let raw = format!("{:.*}", decimals, value.max(0.0));
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
        whole_grouped
    } else {
        format!("{whole_grouped}.{fraction}")
    }
}

fn bar_geometry(
    point: &ChartPoint,
    max: f64,
    width: f64,
    height: f64,
    bucket_count: u32,
) -> (f64, f64, f64, f64) {
    let value_range = max.max(1.0);
    let bucket_total = bucket_count.max(1);
    let offset = point.offset.min(bucket_total.saturating_sub(1));
    let bucket_total_f = bucket_total as f64;
    let start = (offset as f64 / bucket_total_f) * width;
    let end = ((offset + 1) as f64 / bucket_total_f) * width;
    // A tiny overlap hides sub-pixel anti-aliasing cracks between adjacent bars.
    let overlap = if bucket_total > 1 { 0.08 } else { 0.0 };
    let x = if offset == 0 { start } else { start - overlap };
    let right = if offset + 1 >= bucket_total {
        width
    } else {
        end + overlap
    };
    let bar_width = (right - x).max(0.2);
    let bar_height = ((point.value / value_range) * height).max(1.0);
    let y = height - bar_height;
    (x, y, bar_width, bar_height)
}

fn axis_ticks(scale: ChartScale, bucket_count: u32, date_from: &str) -> Vec<(u32, String)> {
    match scale {
        ChartScale::Day => (0..=24)
            .map(|hour| (hour * 60, format!("{:02}:00", hour % 24)))
            .filter(|(offset, _)| *offset < bucket_count)
            .collect(),
        ChartScale::Week => {
            let start = NaiveDate::parse_from_str(date_from, "%Y-%m-%d").ok();
            (0..7)
                .map(|day| {
                    let label = start
                        .map(|d| (d + Duration::days(day)).format("%d.%m").to_string())
                        .unwrap_or_else(|| format!("Д{}", day + 1));
                    ((day as u32) * 24 * 12, label)
                })
                .filter(|(offset, _)| *offset < bucket_count)
                .collect()
        }
        ChartScale::Month => {
            let start = NaiveDate::parse_from_str(date_from, "%Y-%m-%d").ok();
            let day_count = (bucket_count / 24).max(1);
            (0..day_count)
                .step_by(5)
                .map(|day| {
                    let label = start
                        .map(|d| (d + Duration::days(day as i64)).format("%d.%m").to_string())
                        .unwrap_or_else(|| (day + 1).to_string());
                    (day * 24, label)
                })
                .collect()
        }
    }
}

/// Столбчатый график по бакетам.
///
/// `summary` — уже отформатированный итог за период: вызывающий сам решает, сумма это
/// или среднее (для средней длительности сумма была бы бессмысленна).
#[component]
pub fn TimeBarChart(
    points: Vec<ChartPoint>,
    bucket_count: u32,
    date_from: String,
    scale: ChartScale,
    title: String,
    summary: String,
    /// Форматтер значения точки для подписи максимума и тултипов.
    format_value: Callback<(f64,), String>,
) -> impl IntoView {
    let plot_width = 990.0;
    let left_padding = 72.0;
    let right_padding = 36.0;
    let width = left_padding + plot_width + right_padding;
    let top_padding = 24.0;
    let plot_height = 343.0;
    let axis_height = 33.0;
    let svg_height = top_padding + plot_height + axis_height;
    let max = points.iter().map(|point| point.value).fold(0.0, f64::max);
    let ticks = axis_ticks(scale, bucket_count, &date_from);
    let hovered = RwSignal::new(None::<ChartPoint>);
    let max_label = format_value.run((max,));
    let point_count = points.len();

    view! {
        <div style="width:min(100%, 1370px);margin:0 auto;box-sizing:border-box;">
            <div style="display:flex;justify-content:space-between;gap:16px;align-items:flex-start;margin-bottom:12px;">
                <div>
                    <div style="font-size:14px;font-weight:600;color:var(--color-text-primary);">{title}</div>
                    <div style="font-size:12px;color:var(--color-text-secondary);">
                        {format!("Период с {} 00:00 МСК · точек: {}", date_from, point_count)}
                    </div>
                </div>
                <div style="font-size:18px;font-weight:700;font-family:monospace;">
                    {summary}
                </div>
            </div>

            <div style="position:relative;display:flex;justify-content:center;">
                <svg
                    width=format!("{:.0}", width)
                    height=format!("{:.0}", svg_height)
                    viewBox=format!("0 0 {} {}", width, svg_height)
                    preserveAspectRatio="xMidYMid meet"
                    style="width:100%;max-width:1318px;height:auto;display:block;overflow:visible;flex:0 1 auto;"
                >
                    <text x="0" y=format!("{:.1}", top_padding + 4.0) fill="var(--color-text-secondary)" font-size="11">{max_label}</text>
                    <path d=format!("M{:.1} {:.1} L{:.1} {:.1}", left_padding, top_padding, left_padding + plot_width, top_padding) stroke="var(--color-border)" stroke-width="1" stroke-dasharray="4 4" fill="none" opacity="0.7" />
                    <path d=format!("M{:.1} {:.1} L{:.1} {:.1}", left_padding, top_padding + (plot_height / 2.0), left_padding + plot_width, top_padding + (plot_height / 2.0)) stroke="var(--color-border)" stroke-width="1" stroke-dasharray="4 4" fill="none" opacity="0.6" />
                    <path d=format!("M{:.1} {:.1} L{:.1} {:.1}", left_padding, top_padding + plot_height, left_padding + plot_width, top_padding + plot_height) stroke="var(--color-border)" stroke-width="1" fill="none" />
                    {ticks.into_iter().map(|(offset, label)| {
                        let x = if bucket_count <= 1 {
                            left_padding
                        } else {
                            left_padding + (offset.min(bucket_count.saturating_sub(1)) as f64 / bucket_count.saturating_sub(1).max(1) as f64) * plot_width
                        };
                        view! {
                            <g>
                                <line x1=format!("{:.1}", x) y1=format!("{:.1}", top_padding + plot_height) x2=format!("{:.1}", x) y2=format!("{:.1}", top_padding + plot_height + 5.0) stroke="var(--color-border)" stroke-width="1" />
                                <text x=format!("{:.1}", x) y=format!("{:.1}", top_padding + plot_height + 22.0) text-anchor="middle" fill="var(--color-text-secondary)" font-size="11">{label}</text>
                            </g>
                        }
                    }).collect_view()}
                    {points.iter().cloned().map(|point| {
                        let (x, y, bar_width, bar_height) = bar_geometry(
                            &point,
                            max,
                            plot_width,
                            plot_height,
                            bucket_count,
                        );
                        let title_attr = format!(
                            "{} · {}",
                            format_datetime(&point.bucket),
                            format_value.run((point.value,))
                        );
                        let point_for_hover = point.clone();
                        view! {
                            <rect
                                x=format!("{:.1}", left_padding + x)
                                y=format!("{:.1}", top_padding + y)
                                width=format!("{:.1}", bar_width)
                                height=format!("{:.1}", bar_height)
                                fill="var(--colorBrandForeground1)"
                                opacity="0.88"
                                shape-rendering="crispEdges"
                                on:mouseenter=move |_| hovered.set(Some(point_for_hover.clone()))
                                on:mouseleave=move |_| hovered.set(None)
                            >
                                <title>{title_attr}</title>
                            </rect>
                        }
                    }).collect_view()}
                </svg>
            </div>

            {move || hovered.get().map(|point| view! {
                <div style="margin-top:8px;padding:8px 10px;border-radius:var(--radius-md);background:var(--colorNeutralBackground2);font-size:12px;color:var(--color-text-secondary);">
                    <span style="font-weight:600;color:var(--color-text-primary);">{format_datetime(&point.bucket)}</span>
                    " · "
                    <span style="font-family:monospace;">{format_value.run((point.value,))}</span>
                </div>
            })}
        </div>
    }
}

/// Переключатель масштаба «День | Неделя | Месяц».
#[component]
pub fn ScaleSelector(
    scale: RwSignal<ChartScale>,
    /// Вызывается при смене — обычно сбрасывает начало периода под новый масштаб.
    on_change: Callback<(ChartScale,)>,
) -> impl IntoView {
    view! {
        <div style="display:flex;flex-direction:column;gap:6px;">
            <label style="font-size:12px;color:var(--color-text-secondary);">"Масштаб"</label>
            <div style="display:flex;gap:8px;align-items:center;flex-wrap:wrap;">
                {ChartScale::ALL
                    .into_iter()
                    .map(|item| {
                        view! {
                            <button
                                type="button"
                                style=move || {
                                    if scale.get() == item {
                                        "padding:6px 12px;border:1px solid var(--colorBrandStroke1);border-radius:var(--radius-md);background:var(--colorBrandBackground);color:var(--colorNeutralForegroundOnBrand);font-size:13px;cursor:pointer;".to_string()
                                    } else {
                                        "padding:6px 12px;border:1px solid var(--color-border);border-radius:var(--radius-md);background:var(--colorNeutralBackground1);color:var(--color-text-primary);font-size:13px;cursor:pointer;".to_string()
                                    }
                                }
                                on:click=move |_| on_change.run((item,))
                            >
                                {item.label()}
                            </button>
                        }
                    })
                    .collect_view()}
            </div>
        </div>
    }
}

/// Навигация по периоду: ‹ [дата] ›.
#[component]
pub fn PeriodNav(date_from: RwSignal<String>, scale: RwSignal<ChartScale>) -> impl IntoView {
    let move_period = move |direction: i32| {
        let next = shift_date_for_scale(&date_from.get_untracked(), scale.get_untracked(), direction);
        date_from.set(next);
    };

    view! {
        <div style="display:flex;flex-direction:column;gap:6px;">
            <label style="font-size:12px;color:var(--color-text-secondary);">"Начало периода"</label>
            <div style="display:flex;gap:6px;align-items:center;">
                <button
                    type="button"
                    title="Предыдущий период"
                    aria-label="Предыдущий период"
                    style="width:32px;height:32px;padding:0;border:1px solid var(--color-border);border-radius:var(--radius-md);background:var(--colorNeutralBackground1);color:var(--color-text-primary);font-size:13px;cursor:pointer;display:inline-flex;align-items:center;justify-content:center;"
                    on:click=move |_| move_period(-1)
                >
                    {icon("chevron-left")}
                </button>
                <input
                    type="date"
                    style="padding:6px 10px;border:1px solid var(--color-border);border-radius:var(--radius-md);background:var(--colorNeutralBackground1);color:var(--color-text-primary);font-size:13px;"
                    prop:value=move || date_from.get()
                    on:input=move |ev| date_from.set(event_target_value(&ev))
                />
                <button
                    type="button"
                    title="Следующий период"
                    aria-label="Следующий период"
                    style="width:32px;height:32px;padding:0;border:1px solid var(--color-border);border-radius:var(--radius-md);background:var(--colorNeutralBackground1);color:var(--color-text-primary);font-size:13px;cursor:pointer;display:inline-flex;align-items:center;justify-content:center;"
                    on:click=move |_| move_period(1)
                >
                    {icon("chevron-right")}
                </button>
            </div>
        </div>
    }
}
