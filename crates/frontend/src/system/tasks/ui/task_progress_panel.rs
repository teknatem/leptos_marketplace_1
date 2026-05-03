//! Общая панель прогресса по [`TaskProgressResponse`] (регламент + usecase-страницы).

use crate::shared::date_utils::format_bytes_compact;
use contracts::system::tasks::progress::{
    task_progress_detail_caption_ru, TaskProgressDetail, TaskProgressResponse,
};
use leptos::prelude::*;
use thaw::*;

#[component]
fn ProgressStatusBadge(status: String) -> impl IntoView {
    let (bg, fg, label) = match status.as_str() {
        "Completed" => (
            "var(--colorSuccessBackground2)",
            "var(--colorSuccessForeground1)",
            "Успешно",
        ),
        "CompletedWithErrors" => (
            "var(--colorPaletteYellowBackground2)",
            "var(--colorPaletteDarkOrangeForeground2)",
            "С ошибками",
        ),
        "Running" => (
            "var(--colorBrandBackground2)",
            "var(--colorBrandForeground1)",
            "В работе",
        ),
        "Failed" => (
            "var(--colorPaletteRedBackground2)",
            "var(--color-error)",
            "Ошибка",
        ),
        _ => (
            "var(--colorNeutralBackground3)",
            "var(--color-text-secondary)",
            "—",
        ),
    };
    view! {
        <span style=format!("display:inline-flex;align-items:center;padding:2px 8px;border-radius:999px;background:{bg};color:{fg};font-size:12px;font-weight:600;")>
            {label}
        </span>
    }
}

fn badge_style(kind: &str) -> &'static str {
    match kind {
        "ins" => "padding:2px 8px;border-radius:6px;font-size:12px;font-weight:600;background:var(--colorPaletteGreenBackground2);color:var(--colorPaletteGreenForeground1);",
        "upd" => "padding:2px 8px;border-radius:6px;font-size:12px;font-weight:600;background:var(--colorPaletteBlueBackground2);color:var(--colorPaletteBlueForeground2);",
        "err" => "padding:2px 8px;border-radius:6px;font-size:12px;font-weight:600;background:var(--colorPaletteRedBackground2);color:var(--color-error);",
        "del" | "default" => "padding:2px 8px;border-radius:6px;font-size:12px;font-weight:600;background:var(--colorNeutralBackground3);color:var(--color-text-secondary);",
        _ => "padding:2px 8px;border-radius:6px;font-size:12px;font-weight:600;background:var(--colorNeutralBackground3);color:var(--color-text-secondary);",
    }
}

/// Универсальное отображение ответа прогресса с ветвлением по [`TaskProgressDetail`].
#[component]
pub fn TaskProgressPanel(
    /// Готовый снимок с сервера (или из маппера contracts).
    progress: TaskProgressResponse,
    /// Заголовок секции (uppercase); `None` или пустая строка — строка не показывается.
    #[prop(optional)]
    section_title: Option<String>,
    /// Подпись слева у бейджа статуса; по умолчанию «Выполняется…».
    #[prop(optional)]
    running_title: Option<String>,
) -> impl IntoView {
    let section_title_line = section_title.filter(|s| !s.trim().is_empty());
    let running_title = running_title.unwrap_or_else(|| "Выполняется…".to_string());
    let title_upper: AnyView = section_title_line
        .map(|t| {
            view! {
                <div style="font-size: 12px; font-weight: 700; letter-spacing: 0.04em; text-transform: uppercase; color: var(--color-text-tertiary);">
                    {t}
                </div>
            }
            .into_any()
        })
        .unwrap_or_else(|| view! { <></> }.into_any());

    let status = progress.status.clone();
    let message = progress.message.clone();
    let current_item = progress.current_item.clone();
    let errors = progress.errors.clone();
    let processed = progress.processed_items.unwrap_or(0);
    let total = progress.total_items.unwrap_or(0);
    let legacy_percent = if total > 0 {
        ((processed as f64 / total as f64) * 100.0).clamp(0.0, 100.0) as i32
    } else {
        0
    };

    let detail = progress.detail.clone();

    let http_line = {
        let n = progress.http_request_count.unwrap_or(0);
        let up = progress.http_bytes_sent.unwrap_or(0).max(0) as u64;
        let down = progress.http_bytes_received.unwrap_or(0).max(0) as u64;
        if n == 0 && up == 0 && down == 0 {
            None
        } else {
            Some(format!(
                "HTTP: {n} · ↑{} ↓{}",
                format_bytes_compact(up),
                format_bytes_compact(down)
            ))
        }
    };

    view! {
        <div class="task-progress-panel" style="display: flex; flex-direction: column; gap: 6px; padding: 2px 0; min-width: 0;">
            {title_upper}
            <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center style="min-width: 0;">
                <span style="font-weight: 600; font-size: var(--font-size-sm); color: var(--color-text);">
                    {running_title}
                </span>
                <ProgressStatusBadge status=status />
            </Flex>
            {(!message.is_empty()).then(|| view! {
                <div style="font-size: 12px; color: var(--color-text-secondary);">
                    {message.clone()}
                </div>
            })}
            {http_line.map(|line| view! {
                <div
                    style="font-size: 11px; font-family: monospace; color: var(--color-text-secondary); line-height: 1.35;"
                    title="Количество прочитанных ответов API и объём тел запросов/ответов"
                >
                    {line}
                </div>
            })}
            {match detail.clone() {
                Some(ref d) => {
                    let cap = task_progress_detail_caption_ru(d);
                    view! {
                        <div
                            class="task-progress-panel__summary"
                            style="margin-top: 2px; padding: 8px 10px; border-radius: var(--radius-sm); background: var(--colorNeutralBackground2); border: 1px solid var(--color-border); font-size: 13px; font-weight: 600; color: var(--color-text-primary); line-height: 1.35;"
                        >
                            {cap}
                        </div>
                    }
                    .into_any()
                }
                None => view! { <></> }.into_any(),
            }}
            {match detail {
                Some(TaskProgressDetail::Count { current, total, label }) => {
                    let pct = if total > 0 {
                        ((current as f64 / total as f64) * 100.0).clamp(0.0, 100.0) as i32
                    } else {
                        0
                    };
                    let cap = label.unwrap_or_default();
                    view! {
                        <div style="display: flex; align-items: center; gap: 12px; min-width: 0;">
                            <div style="height: 16px; width: 180px; border-radius: var(--radius-sm); overflow: hidden; background: var(--color-border); flex: 0 0 auto;">
                                <div style=format!("width: {}%; height: 100%; background: var(--colorBrandForeground1); transition: width 0.2s;", pct)></div>
                            </div>
                            <span style="font-size: var(--font-size-sm); color: var(--color-text-secondary); min-width: 90px;">
                                {format!("{} / {}", current, total)}
                                {if !cap.is_empty() { format!(" · {}", cap) } else { String::new() }}
                            </span>
                        </div>
                    }.into_any()
                }
                Some(TaskProgressDetail::Percent { value }) => {
                    let v = value.clamp(0, 100);
                    view! {
                        <div style="display: flex; align-items: center; gap: 12px; min-width: 0;">
                            <div style="height: 16px; width: 180px; border-radius: var(--radius-sm); overflow: hidden; background: var(--color-border); flex: 0 0 auto;">
                                <div style=format!("width: {}%; height: 100%; background: var(--colorBrandForeground1); transition: width 0.2s;", v)></div>
                            </div>
                            <span style="font-size: var(--font-size-sm); color: var(--color-text-secondary);">{format!("{}%", v)}</span>
                        </div>
                    }.into_any()
                }
                Some(TaskProgressDetail::DataDelta { inserted, updated, deleted, errors }) => {
                    view! {
                        <div style="display: flex; align-items: center; gap: 8px; min-width: 0; flex-wrap: wrap;">
                            <span style=badge_style("ins")>{format!("ins {}", inserted)}</span>
                            <span style=badge_style("upd")>{format!("upd {}", updated)}</span>
                            {if deleted > 0 {
                                view! { <span style=badge_style("del")>{format!("del {}", deleted)}</span> }.into_any()
                            } else {
                                view! { <></> }.into_any()
                            }}
                            <span style=badge_style("err")>{format!("err {}", errors)}</span>
                        </div>
                    }.into_any()
                }
                Some(TaskProgressDetail::Pipeline { current_index, total_stages, current_label, stages }) => {
                    let stages = stages.unwrap_or_default();
                    view! {
                        <div style="display: flex; flex-direction: column; gap: 6px; font-size: var(--font-size-sm); color: var(--color-text-secondary);">
                            <div>
                                {format!(
                                    "Этап {} из {}: {}",
                                    (current_index + 1).min(total_stages),
                                    total_stages,
                                    current_label
                                )}
                            </div>
                            {if !stages.is_empty() {
                                view! {
                                    <ul style="margin:0;padding-left:18px;">
                                        {stages.into_iter().enumerate().map(|(i, name)| {
                                            let mark = if i == current_index { "▶ " } else { "" };
                                            view! { <li style="margin:2px 0;">{format!("{}{}", mark, name)}</li> }
                                        }).collect_view()}
                                    </ul>
                                }.into_any()
                            } else {
                                view! { <></> }.into_any()
                            }}
                        </div>
                    }.into_any()
                }
                Some(TaskProgressDetail::Indeterminate { .. }) => {
                    // Текст уже в сводке сверху (`task_progress_detail_caption_ru`).
                    view! { <></> }.into_any()
                }
                None => {
                    view! {
                        <div style="display: flex; align-items: center; gap: 12px; min-width: 0;">
                            <div style="height: 16px; width: 180px; border-radius: var(--radius-sm); overflow: hidden; background: var(--color-border); flex: 0 0 auto;">
                                <div style=format!("width: {}%; height: 100%; background: var(--colorBrandForeground1); transition: width 0.2s;", legacy_percent)></div>
                            </div>
                            <span style="font-size: var(--font-size-sm); color: var(--color-text-secondary); min-width: 90px;">
                                {if total > 0 {
                                    format!("{} / {} ({}%)", processed, total, legacy_percent)
                                } else {
                                    format!("{}", processed)
                                }}
                            </span>
                        </div>
                    }.into_any()
                }
            }}
            {if let Some(item) = current_item.filter(|s| !s.trim().is_empty()) {
                view! {
                    <div style="font-size: var(--font-size-sm); color: var(--color-text-secondary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                        {format!("Текущий элемент: {}", item)}
                    </div>
                }.into_any()
            } else {
                view! { <></> }.into_any()
            }}
            {if let Some(errs) = errors.filter(|e| !e.is_empty()) {
                view! {
                    <div style="font-size: 11px; color: var(--color-error); max-height: 72px; overflow-y: auto;">
                        {errs.join("; ")}
                    </div>
                }.into_any()
            } else {
                view! { <></> }.into_any()
            }}
        </div>
    }
}
