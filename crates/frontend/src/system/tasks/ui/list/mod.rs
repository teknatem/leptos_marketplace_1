pub mod state;

use self::state::create_state;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::date_utils::{
    format_duration_ms, format_http_traffic, format_utc_local, TZ_OFFSET_HOURS,
};
use crate::shared::icons::icon;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator, sort_list, Sortable};
use crate::system::tasks::api::{self, RunTaskNowOutcome};
use contracts::system::tasks::progress::{task_progress_detail_caption_ru, TaskProgressResponse};
use contracts::system::tasks::response::ScheduledTaskResponse;
use contracts::system::tasks::runs::{LiveMemoryProgressItem, TaskRun};
use gloo_timers::future::TimeoutFuture;
use leptos::ev::MouseEvent;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cmp::Ordering;
use std::collections::HashSet;
use thaw::*;

/// Интервал автообновления таблицы «Активные» (только in-memory прогресс).
const LIVE_MEMORY_POLL_MS: u32 = 2000;

/// Вычисляет прошедшее время от `started_at` (UTC) до текущего момента браузера.
fn elapsed_from(started_at: &chrono::DateTime<chrono::Utc>) -> String {
    let now_ms = js_sys::Date::now() as i64;
    let diff_ms = (now_ms - started_at.timestamp_millis()).max(0);
    format_duration_ms(diff_ms)
}

/// Активная статистика HTTP для строки «Активные задачи» (WB и др.).
fn live_http_calls(p: &TaskProgressResponse) -> String {
    let n = p.http_request_count.unwrap_or(0);
    if n == 0 {
        "—".to_string()
    } else {
        n.to_string()
    }
}

fn live_http_traffic(p: &TaskProgressResponse) -> String {
    format_http_traffic(
        p.http_bytes_sent.unwrap_or(0),
        p.http_bytes_received.unwrap_or(0),
    )
    .unwrap_or_else(|| "—".to_string())
}

fn monitoring_http_calls(run: &TaskRun) -> String {
    run.http_request_count
        .filter(|&n| n > 0)
        .map(|n| n.to_string())
        .unwrap_or_else(|| "—".to_string())
}

fn monitoring_http_traffic(run: &TaskRun) -> String {
    format_http_traffic(
        run.http_bytes_sent.unwrap_or(0),
        run.http_bytes_received.unwrap_or(0),
    )
    .unwrap_or_else(|| "—".to_string())
}

fn live_progress_caption(p: &TaskProgressResponse) -> String {
    p.detail
        .as_ref()
        .map(task_progress_detail_caption_ru)
        .or_else(|| {
            if p.processed_items.is_some() || p.total_items.is_some() {
                Some(format!(
                    "{} / {}",
                    p.processed_items.unwrap_or(0),
                    p.total_items.unwrap_or(0)
                ))
            } else if !p.message.is_empty() {
                Some(p.message.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "—".to_string())
}

/// Converts a 5-, 6- (sec + standard), or 7-field cron to a short Russian description.
fn describe_cron(cron: &str) -> String {
    let cron = cron.trim();
    if cron.is_empty() {
        return "—".to_string();
    }

    let parts: Vec<&str> = cron.split_whitespace().collect();
    if parts.len() < 5 {
        return cron.to_string();
    }

    // 5: min hour dom mon dow — 6: sec min hour dom mon dow — 7: + year
    let offset = match parts.len() {
        6 | 7 => 1,
        _ => 0,
    };
    if parts.len() < offset + 5 {
        return cron.to_string();
    }

    let (minute, hour, day, _month, weekday) = (
        parts[offset],
        parts[offset + 1],
        parts[offset + 2],
        parts[offset + 3],
        parts[offset + 4],
    );

    // Every N minutes
    if let (m, "*", "*", "*") = (minute, hour, day, weekday) {
        if let Some(n) = m.strip_prefix("*/") {
            return format!("Каждые {} мин", n);
        }
    }

    // Every N hours
    if let ("0", h, "*", "*") = (minute, hour, day, weekday) {
        if let Some(n) = h.strip_prefix("*/") {
            return format!("Каждые {} ч", n);
        }
    }

    // Every hour
    if (minute, hour, day, weekday) == ("0", "*", "*", "*") {
        return "Каждый час".to_string();
    }

    // Daily at HH:MM
    if day == "*" && weekday == "*" && !minute.contains('*') && !hour.contains('*') {
        let h: u8 = hour.parse().unwrap_or(0);
        let m: u8 = minute.parse().unwrap_or(0);
        return format!("Ежедневно {:02}:{:02}", h, m);
    }

    // Weekly on weekday
    if day == "*" && !weekday.contains('*') && !minute.contains('*') && !hour.contains('*') {
        let h: u8 = hour.parse().unwrap_or(0);
        let m: u8 = minute.parse().unwrap_or(0);
        let day_name = match weekday {
            "0" | "7" => "вс",
            "1" => "пн",
            "2" => "вт",
            "3" => "ср",
            "4" => "чт",
            "5" => "пт",
            "6" => "сб",
            other => other,
        };
        return format!("Еженедельно {} {:02}:{:02}", day_name, h, m);
    }

    cron.to_string()
}

impl Sortable for ScheduledTaskResponse {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "code" => self.code.to_lowercase().cmp(&other.code.to_lowercase()),
            "description" => self
                .description
                .to_lowercase()
                .cmp(&other.description.to_lowercase()),
            "comment" => self
                .comment
                .as_deref()
                .unwrap_or("")
                .to_lowercase()
                .cmp(&other.comment.as_deref().unwrap_or("").to_lowercase()),
            "task_type" => self
                .task_type
                .to_lowercase()
                .cmp(&other.task_type.to_lowercase()),
            "last_run_at" => self.last_run_at.cmp(&other.last_run_at),
            "is_enabled" => other.is_enabled.cmp(&self.is_enabled),
            _ => Ordering::Equal,
        }
    }
}

#[component]
fn SortHeaderCell(
    label: &'static str,
    field: &'static str,
    min_width: f32,
    sort_field: ReadSignal<String>,
    sort_ascending: ReadSignal<bool>,
    on_toggle: Callback<&'static str>,
) -> impl IntoView {
    view! {
        <TableHeaderCell resizable=true min_width=min_width>
            <div
                class="table__sortable-header"
                style="cursor:pointer;display:flex;align-items:center;gap:4px;user-select:none;"
                on:click=move |_| on_toggle.run(field)
            >
                {label}
                <span class=move || get_sort_class(&sort_field.get(), field)>
                    {move || get_sort_indicator(&sort_field.get(), field, sort_ascending.get())}
                </span>
            </div>
        </TableHeaderCell>
    }
}

#[component]
fn LiveMemoryProgressRow(item: LiveMemoryProgressItem, on_aborted: Callback<()>) -> impl IntoView {
    let caption = live_progress_caption(&item.progress);
    let sid = item.progress.session_id.clone();
    let sid_for_abort = sid.clone();
    let http_calls_str = live_http_calls(&item.progress);
    let http_traffic_str = live_http_traffic(&item.progress);

    let started_str = item
        .progress
        .started_at
        .as_ref()
        .map(|dt| format_utc_local(dt, "%H:%M:%S"))
        .unwrap_or_else(|| "—".to_string());

    let elapsed_str = item
        .progress
        .started_at
        .as_ref()
        .map(|dt| elapsed_from(dt))
        .unwrap_or_else(|| "—".to_string());

    let aborting = RwSignal::new(false);

    let on_abort = move |_| {
        let session_id = sid_for_abort.clone();
        aborting.set(true);
        spawn_local(async move {
            match api::abort_task_run(&session_id).await {
                Ok(_) => on_aborted.run(()),
                Err(e) => {
                    leptos::logging::log!("Abort error: {}", e);
                    aborting.set(false);
                }
            }
        });
    };

    view! {
        <TableRow>
            <TableCell>
                <div style="display:flex;flex-direction:column;gap:2px;min-width:0;">
                    <span style="font-size:13px;font-weight:600;">{item.task_display_name.clone()}</span>
                    <span style="font-size:11px;color:var(--color-text-secondary);font-family:monospace;">{item.task_type.clone()}</span>
                </div>
            </TableCell>
            <TableCell>
                <span style="font-size:11px;font-family:monospace;color:var(--color-text-secondary);word-break:break-all;">{sid}</span>
            </TableCell>
            <TableCell>
                <span
                    style="font-size:12px;font-weight:500;"
                    title=format!("UTC+{TZ_OFFSET_HOURS}")
                >
                    {started_str}
                </span>
            </TableCell>
            <TableCell>
                <span style="font-size:12px;font-family:monospace;color:var(--color-text-secondary);">
                    {elapsed_str}
                </span>
            </TableCell>
            <TableCell>
                <span style="font-size:11px;line-height:1.35;color:var(--color-text-secondary);display:block;max-width:360px;">
                    {caption}
                </span>
            </TableCell>
            <TableCell>
                <span style="font-size:12px;font-family:monospace;color:var(--color-text-secondary);">
                    {http_calls_str}
                </span>
            </TableCell>
            <TableCell>
                <span
                    style="font-size:11px;font-family:monospace;line-height:1.35;color:var(--color-text-secondary);white-space:nowrap;"
                    title="Отправлено / получено по HTTP (WB API и др.)"
                >
                    {http_traffic_str}
                </span>
            </TableCell>
            <TableCell attr:style="width:100px;text-align:right;">
                <Button
                    appearance=ButtonAppearance::Subtle
                    on_click=on_abort
                    disabled=aborting
                    attr:title="Принудительно прервать задачу"
                >
                    {move || if aborting.get() { "…" } else { "Прервать" }}
                </Button>
            </TableCell>
        </TableRow>
    }
}

#[component]
fn MonitoringRow(run: TaskRun) -> impl IntoView {
    let started = format_utc_local(&run.started_at, "%d.%m.%y %H:%M:%S");
    let finished = run
        .finished_at
        .map(|d| format_utc_local(&d, "%d.%m.%y %H:%M:%S"))
        .unwrap_or_else(|| "—".to_string());
    let task_title = run.task_code.clone().unwrap_or_else(|| run.task_id.clone());
    let task_sub = run.task_description.clone().unwrap_or_default();
    let duration = run
        .duration_ms
        .map(format_duration_ms)
        .unwrap_or_else(|| "—".to_string());
    let processed = run
        .total_processed
        .map(|v| v.to_string())
        .unwrap_or_else(|| "—".to_string());
    let inserted = run
        .total_inserted
        .map(|v| v.to_string())
        .unwrap_or_else(|| "—".to_string());
    let updated = run
        .total_updated
        .map(|v| v.to_string())
        .unwrap_or_else(|| "—".to_string());
    let errors = run
        .total_errors
        .map(|v| v.to_string())
        .unwrap_or_else(|| "0".to_string());
    let status = run.status.clone();
    let mon_http_calls = monitoring_http_calls(&run);
    let mon_http_traffic = monitoring_http_traffic(&run);

    let (status_style, status_label) = match status.as_str() {
        "Completed" => ("padding:2px 8px;border-radius:999px;background:var(--colorSuccessBackground2);color:var(--colorSuccessForeground1);font-size:12px;font-weight:600;", "Успешно"),
        "CompletedWithErrors" => ("padding:2px 8px;border-radius:999px;background:var(--colorPaletteYellowBackground2);color:var(--colorPaletteDarkOrangeForeground2);font-size:12px;font-weight:600;", "С ошибками"),
        "Running"   => ("padding:2px 8px;border-radius:999px;background:var(--colorBrandBackground2);color:var(--colorBrandForeground1);font-size:12px;font-weight:600;",   "В работе"),
        "Failed"    => ("padding:2px 8px;border-radius:999px;background:var(--colorPaletteRedBackground2);color:var(--color-error);font-size:12px;font-weight:600;",         "Ошибка"),
        _           => ("padding:2px 8px;border-radius:999px;background:var(--colorNeutralBackground3);color:var(--color-text-secondary);font-size:12px;font-weight:600;",  "—"),
    };

    let triggered_style = match run.triggered_by.as_str() {
        "Manual" => "padding:2px 8px;border-radius:999px;background:var(--colorPaletteYellowBackground2);color:var(--colorPaletteDarkOrangeForeground2);font-size:12px;font-weight:600;",
        _        => "padding:2px 8px;border-radius:999px;background:var(--colorNeutralBackground3);color:var(--color-text-secondary);font-size:12px;font-weight:600;",
    };
    let triggered_label = match run.triggered_by.as_str() {
        "Manual" => "Вручную",
        _ => "Расписание",
    };

    view! {
        <TableRow>
            <TableCell>
                <div style="display:flex;flex-direction:column;gap:2px;min-width:0;">
                    <span style="font-size:13px;font-weight:600;font-family:monospace;">{task_title}</span>
                    {if !task_sub.is_empty() {
                        view! { <span style="font-size:11px;color:var(--color-text-secondary);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;" title=task_sub.clone()>{task_sub.clone()}</span> }.into_any()
                    } else {
                        view! { <span style="font-size:11px;color:var(--color-text-tertiary);">"—"</span> }.into_any()
                    }}
                </div>
            </TableCell>
            <TableCell>
                <span style="font-size:12px;color:var(--color-text-secondary);">{started}</span>
            </TableCell>
            <TableCell>
                <span style="font-size:12px;color:var(--color-text-secondary);">{finished}</span>
            </TableCell>
            <TableCell>
                <span style={triggered_style}>{triggered_label}</span>
            </TableCell>
            <TableCell>
                <span style={status_style}>{status_label}</span>
            </TableCell>
            <TableCell>
                <span style="font-size:12px;">{duration}</span>
            </TableCell>
            <TableCell>
                <span style="font-size:12px;">{processed}</span>
            </TableCell>
            <TableCell>
                <span style="font-size:12px;color:var(--colorPaletteGreenForeground1);">{inserted}</span>
            </TableCell>
            <TableCell>
                <span style="font-size:12px;color:var(--colorBrandForeground1);">{updated}</span>
            </TableCell>
            <TableCell>
                <span style={format!("font-size:12px;{}", if errors != "0" { "color:var(--color-error);" } else { "" })}>
                    {errors.clone()}
                </span>
            </TableCell>
            <TableCell>
                <span style="font-size:12px;font-family:monospace;color:var(--color-text-secondary);">
                    {mon_http_calls}
                </span>
            </TableCell>
            <TableCell>
                <span
                    style="font-size:11px;font-family:monospace;line-height:1.35;color:var(--color-text-secondary);white-space:nowrap;"
                    title="Итоговый трафик HTTP за запуск (если учитывался)"
                >
                    {mon_http_traffic}
                </span>
            </TableCell>
        </TableRow>
    }
}

#[component]
pub fn ScheduledTaskList() -> impl IntoView {
    let tabs_store = leptos::context::use_context::<AppGlobalContext>()
        .expect("AppGlobalContext context not found");

    let state = create_state();
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let active_tab = RwSignal::new("tasks".to_string());

    let (selected_ids, set_selected_ids) = signal(HashSet::<String>::new());

    // Sorting state
    let (sort_field, set_sort_field) = signal("description".to_string());
    let (sort_ascending, set_sort_ascending) = signal(true);

    let toggle_sort = Callback::new(move |field: &'static str| {
        if sort_field.get_untracked() == field {
            set_sort_ascending.update(|v| *v = !*v);
        } else {
            set_sort_field.set(field.to_string());
            set_sort_ascending.set(true);
        }
    });

    // Filter state
    let (filter_type, set_filter_type) = signal(String::new());
    let (filter_text, set_filter_text) = signal(String::new());

    // Unique task types for filter dropdown (computed from loaded tasks)
    let unique_task_types = Signal::derive(move || {
        let mut types: Vec<String> = state
            .get()
            .tasks
            .iter()
            .map(|t| t.task_type.clone())
            .collect();
        types.sort();
        types.dedup();
        types
    });

    let sorted_tasks = Signal::derive(move || {
        let mut tasks = state.get().tasks;

        let ft = filter_type.get();
        if !ft.is_empty() {
            tasks.retain(|t| t.task_type == ft);
        }

        let fx = filter_text.get().to_lowercase();
        if !fx.is_empty() {
            tasks.retain(|t| {
                t.description.to_lowercase().contains(&fx)
                    || t.code.to_lowercase().contains(&fx)
                    || t.comment
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&fx)
                    || t.task_type.to_lowercase().contains(&fx)
            });
        }

        sort_list(&mut tasks, &sort_field.get(), sort_ascending.get());
        tasks
    });

    // Monitoring state
    let (recent_runs, set_recent_runs) = signal(Vec::<TaskRun>::new());
    let (live_memory_items, set_live_memory_items) = signal(Vec::<LiveMemoryProgressItem>::new());
    let (runs_loading, set_runs_loading) = signal(false);
    let (batch_warning, set_batch_warning) = signal::<Option<String>>(None);

    // Monitoring filters
    let (mon_filter_status, set_mon_filter_status) = signal(String::new());
    let (mon_filter_text, set_mon_filter_text) = signal(String::new());

    let filtered_runs = Signal::derive(move || {
        let mut runs = recent_runs.get();

        let fs = mon_filter_status.get();
        if !fs.is_empty() {
            runs.retain(|r| r.status == fs);
        }

        let fx = mon_filter_text.get().to_lowercase();
        if !fx.is_empty() {
            runs.retain(|r| {
                r.task_code
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&fx)
                    || r.task_description
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&fx)
            });
        }

        runs
    });

    let load_tasks = move || {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            match api::fetch_scheduled_tasks().await {
                Ok(tasks) => {
                    state.update(|s| {
                        s.tasks = tasks;
                        s.is_loaded = true;
                    });
                    set_loading.set(false);
                }
                Err(e) => {
                    log!("Failed to fetch scheduled tasks: {}", e);
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    };

    // Вкладка «Активные»: первая загрузка с индикатором; опрос — без мигания спиннера.
    let fetch_live_memory = move |show_spinner: bool| {
        spawn_local(async move {
            if show_spinner {
                set_runs_loading.set(true);
            }
            match api::get_active_runs_with_progress().await {
                Ok(resp) => set_live_memory_items.set(resp.items),
                Err(e) => log!("Failed to load live memory progress: {}", e),
            }
            if show_spinner {
                set_runs_loading.set(false);
            }
        });
    };

    let load_active_tab = move || fetch_live_memory(true);

    // Только история — вкладка «Мониторинг запусков» и кнопка «Обновить».
    let load_monitoring_recent = move || {
        spawn_local(async move {
            set_runs_loading.set(true);
            match api::get_recent_runs(Some(100)).await {
                Ok(resp) => set_recent_runs.set(resp.runs),
                Err(e) => log!("Failed to load recent runs: {}", e),
            }
            set_runs_loading.set(false);
        });
    };

    Effect::new(move |_| {
        if !state.with_untracked(|s| s.is_loaded) {
            load_tasks();
        }
    });

    Effect::new(move |_| match active_tab.get().as_str() {
        "active" => {
            fetch_live_memory(true);
            spawn_local(async move {
                loop {
                    TimeoutFuture::new(LIVE_MEMORY_POLL_MS).await;
                    if active_tab.get_untracked().as_str() != "active" {
                        break;
                    }
                    match api::get_active_runs_with_progress().await {
                        Ok(resp) => set_live_memory_items.set(resp.items),
                        Err(e) => log!("Live memory poll failed: {}", e),
                    }
                }
            });
        }
        "monitoring" => load_monitoring_recent(),
        _ => {}
    });

    let toggle_enabled = move |id: String, current_status: bool| {
        spawn_local(async move {
            match api::toggle_scheduled_task_enabled(&id, !current_status).await {
                Ok(_) => load_tasks(),
                Err(e) => log!("Failed to toggle task: {}", e),
            }
        });
    };

    let open_details = move |id: String, code: String| {
        tabs_store.open_tab(
            &format!("sys_task_details_{}", id),
            &format!("Задача: {}", code),
        );
    };

    let create_new = move |_| {
        tabs_store.open_tab("sys_task_details", "Новая задача");
    };

    let close_list_tab = move |_| {
        tabs_store.close_tab("sys_tasks");
    };

    let run_selected = move |_| {
        let ids: Vec<String> = selected_ids.get_untracked().iter().cloned().collect();
        if ids.is_empty() {
            return;
        }
        spawn_local(async move {
            set_batch_warning.set(None);
            let mut warns = Vec::new();
            for id in &ids {
                match api::run_task_now(id).await {
                    Ok(RunTaskNowOutcome::Started(_)) => {}
                    Ok(RunTaskNowOutcome::AlreadyRunning(r)) => {
                        warns.push(format!(
                            "Уже выполняется (запущено {})",
                            r.started_at.format("%d.%m.%Y %H:%M:%S")
                        ));
                    }
                    Err(e) => warns.push(e),
                }
            }
            if !warns.is_empty() {
                set_batch_warning.set(Some(warns.join(" · ")));
            }
            set_selected_ids.set(HashSet::new());
            load_tasks();
        });
    };

    let header_all_selected = Signal::derive(move || {
        let tasks = sorted_tasks.get();
        selected_ids.with(|s| !tasks.is_empty() && tasks.iter().all(|t| s.contains(&t.id)))
    });

    let header_select_on_change = move |ev: leptos::ev::Event| {
        let want_all = event_target_checked(&ev);
        let tasks = sorted_tasks.get_untracked();
        set_selected_ids.update(|s| {
            if want_all {
                for t in &tasks {
                    s.insert(t.id.clone());
                }
            } else {
                for t in &tasks {
                    s.remove(&t.id);
                }
            }
        });
    };

    view! {
        <div id="sys_tasks--list" data-page-category="legacy" class="scheduled-task-list" style="padding: 20px;">
            <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center style="margin-bottom: 16px;">
                <h2 style="margin: 0; font-size: 24px; font-weight: bold;">"Регламентные задания"</h2>
                {move || match active_tab.get().as_str() {
                    "tasks" => view! {
                        <Space gap=SpaceGap::Small>
                            <Button appearance=ButtonAppearance::Secondary on_click=create_new>
                                {icon("plus")}
                                " Создать"
                            </Button>
                            <Button
                                appearance=ButtonAppearance::Primary
                                on_click=run_selected
                                disabled=Signal::derive(move || {
                                    selected_ids.with(|s| s.is_empty()) || loading.get()
                                })
                            >
                                <Flex gap=FlexGap::Small align=FlexAlign::Center>
                                    {icon("play")}
                                    <span>"Выполнить"</span>
                                </Flex>
                            </Button>
                            <Button
                                appearance=ButtonAppearance::Secondary
                                on_click=move |_| load_tasks()
                                disabled=loading
                            >
                                <Flex gap=FlexGap::Small align=FlexAlign::Center>
                                    {icon("refresh-cw")}
                                    <span>"Обновить"</span>
                                </Flex>
                            </Button>
                            <Button appearance=ButtonAppearance::Secondary on_click=close_list_tab>
                                {icon("x")}
                                " Закрыть"
                            </Button>
                        </Space>
                    }.into_any(),
                    "active" => view! {
                        <Flex gap=FlexGap::Small align=FlexAlign::Center>
                            <Button
                                appearance=ButtonAppearance::Secondary
                                on_click=move |_| load_active_tab()
                                disabled=runs_loading
                            >
                                <Flex gap=FlexGap::Small align=FlexAlign::Center>
                                    {icon("refresh-cw")}
                                    <span>"Обновить"</span>
                                </Flex>
                            </Button>
                        </Flex>
                    }.into_any(),
                    "monitoring" => view! {
                        <Flex gap=FlexGap::Small align=FlexAlign::Center>
                            <Button
                                appearance=ButtonAppearance::Secondary
                                on_click=move |_| load_monitoring_recent()
                                disabled=runs_loading
                            >
                                <Flex gap=FlexGap::Small align=FlexAlign::Center>
                                    {icon("refresh-cw")}
                                    <span>"Обновить"</span>
                                </Flex>
                            </Button>
                        </Flex>
                    }.into_any(),
                    _ => view! { <></> }.into_any(),
                }}
            </Flex>

            <div style="margin-bottom: 16px;">
                <TabList selected_value=active_tab>
                    <Tab value="tasks".to_string()>"Задания"</Tab>
                    <Tab value="active".to_string()>"Активные задачи"</Tab>
                    <Tab value="monitoring".to_string()>"Мониторинг запусков"</Tab>
                </TabList>
            </div>

            {move || error.get().map(|err| view! {
                <div style="padding: 12px; background: var(--color-error-50); border: 1px solid var(--color-error-100); border-radius: 8px; display: flex; align-items: center; gap: 8px; margin-bottom: 16px;">
                    <span style="color: var(--color-error); font-size: 18px;">"⚠"</span>
                    <span style="color: var(--color-error);">{err}</span>
                </div>
            })}

            {move || batch_warning.get().map(|w| view! {
                <div style="padding: 12px; background: var(--colorPaletteYellowBackground2); border: 1px solid var(--colorPaletteYellowForeground2); border-radius: 8px; margin-bottom: 16px; font-size: 13px; color: var(--color-text-primary);">
                    {w}
                </div>
            })}

            // Tasks tab
            {move || if active_tab.get() == "tasks" {
                view! {
                    <div>
                        // Фильтры
                        <div style="display:flex;gap:10px;align-items:center;margin-bottom:10px;flex-wrap:wrap;">
                            <div style="position:relative;flex:1;min-width:180px;max-width:360px;">
                                <span style="position:absolute;left:8px;top:50%;transform:translateY(-50%);color:var(--color-text-secondary);pointer-events:none;font-size:14px;">
                                    "🔍"
                                </span>
                                <input
                                    type="text"
                                    placeholder="Поиск по описанию, коду, комментарию…"
                                    style="width:100%;box-sizing:border-box;padding:6px 8px 6px 28px;border:1px solid var(--color-border);border-radius:var(--radius-md);background:var(--colorNeutralBackground1);color:var(--color-text-primary);font-size:13px;"
                                    prop:value=move || filter_text.get()
                                    on:input=move |ev| set_filter_text.set(event_target_value(&ev))
                                />
                            </div>
                            <select
                                style="padding:6px 10px;border:1px solid var(--color-border);border-radius:var(--radius-md);background:var(--colorNeutralBackground1);color:var(--color-text-primary);font-size:13px;cursor:pointer;"
                                prop:value=move || filter_type.get()
                                on:change=move |ev| set_filter_type.set(event_target_value(&ev))
                            >
                                <option value="">"Все типы"</option>
                                {move || unique_task_types.get().into_iter().map(|t| {
                                    let tv = t.clone();
                                    view! { <option value=tv>{t}</option> }
                                }).collect_view()}
                            </select>
                            {move || {
                                let has_filter = !filter_text.get().is_empty() || !filter_type.get().is_empty();
                                if has_filter {
                                    view! {
                                        <button
                                            style="padding:5px 12px;border:1px solid var(--color-border);border-radius:var(--radius-md);background:var(--colorNeutralBackground2);color:var(--color-text-secondary);font-size:12px;cursor:pointer;"
                                            on:click=move |_| {
                                                set_filter_text.set(String::new());
                                                set_filter_type.set(String::new());
                                            }
                                        >"✕ Сбросить"</button>
                                    }.into_any()
                                } else {
                                    view! { <></> }.into_any()
                                }
                            }}
                            {move || {
                                let total = state.get().tasks.len();
                                let shown = sorted_tasks.get().len();
                                let has_filter = !filter_text.get().is_empty() || !filter_type.get().is_empty();
                                if has_filter {
                                    view! {
                                        <span style="font-size:12px;color:var(--color-text-secondary);">
                                            {format!("{} из {}", shown, total)}
                                        </span>
                                    }.into_any()
                                } else {
                                    view! { <span style="font-size:12px;color:var(--color-text-secondary);">{format!("{} заданий", total)}</span> }.into_any()
                                }
                            }}
                        </div>

                        <div class="table-wrapper">
                        <Table>
                            <TableHeader>
                                <TableRow>
                                    <TableHeaderCell min_width=40.0 attr:style="width:40px;padding:0;text-align:center;">
                                        <input
                                            type="checkbox"
                                            class="table__checkbox"
                                            prop:checked=move || header_all_selected.get()
                                            on:change=header_select_on_change
                                            on:click=move |e: MouseEvent| e.stop_propagation()
                                            title="Выбрать все / снять выбор"
                                        />
                                    </TableHeaderCell>
                                    <SortHeaderCell label="Описание" field="description" min_width=220.0
                                        sort_field=sort_field sort_ascending=sort_ascending on_toggle=toggle_sort />
                                    <SortHeaderCell label="Комментарий" field="comment" min_width=180.0
                                        sort_field=sort_field sort_ascending=sort_ascending on_toggle=toggle_sort />
                                    <SortHeaderCell label="Тип" field="task_type" min_width=140.0
                                        sort_field=sort_field sort_ascending=sort_ascending on_toggle=toggle_sort />
                                    <TableHeaderCell resizable=true min_width=150.0>"Расписание"</TableHeaderCell>
                                    <SortHeaderCell label="Последний запуск" field="last_run_at" min_width=140.0
                                        sort_field=sort_field sort_ascending=sort_ascending on_toggle=toggle_sort />
                                    <TableHeaderCell resizable=true min_width=100.0>"Статус"</TableHeaderCell>
                                    <SortHeaderCell label="Вкл" field="is_enabled" min_width=60.0
                                        sort_field=sort_field sort_ascending=sort_ascending on_toggle=toggle_sort />
                                </TableRow>
                            </TableHeader>
                            <TableBody>
                                {move || {
                                    if loading.get() {
                                        view! {
                                            <TableRow>
                                                <TableCell attr:colspan="8" attr:style="padding: 40px; text-align: center;">
                                                    <Flex justify=FlexJustify::Center align=FlexAlign::Center gap=FlexGap::Small>
                                                        <Spinner />
                                                        "Загрузка..."
                                                    </Flex>
                                                </TableCell>
                                            </TableRow>
                                        }.into_any()
                                    } else {
                                        let tasks = sorted_tasks.get();
                                        if tasks.is_empty() {
                                            view! {
                                                <TableRow>
                                                    <TableCell attr:colspan="8" attr:style="padding: 40px; text-align: center; color: var(--colorNeutralForeground3);">
                                                        "Заданий не найдено"
                                                    </TableCell>
                                                </TableRow>
                                            }.into_any()
                                        } else {
                                            tasks.into_iter().map(|task: ScheduledTaskResponse| {
                                                let task_id          = task.id.clone();
                                                let task_id_toggle   = task.id.clone();
                                                let task_id_select   = task.id.clone();
                                                let task_id_details  = task.id.clone();
                                                let task_id_for_change = task.id.clone();
                                                let code             = task.code.clone();
                                                let code_for_details = task.code.clone();
                                                let is_enabled       = task.is_enabled;
                                                let comment_text     = task.comment.clone().unwrap_or_default();

                                                let schedule_text = task.schedule_cron
                                                    .as_deref()
                                                    .map(describe_cron)
                                                    .unwrap_or_else(|| "—".to_string());

                                                let status_view = match task.last_run_status.as_deref() {
                                                    Some("Completed") => view! { <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Success>"Успешно"</Badge> }.into_any(),
                                                    Some("CompletedWithErrors") => view! { <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Warning>"С ошибками"</Badge> }.into_any(),
                                                    Some("Running")   => view! { <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>"Запуск"</Badge> }.into_any(),
                                                    Some("Failed")    => view! { <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Danger>"Ошибка"</Badge> }.into_any(),
                                                    _                 => view! { <Badge appearance=BadgeAppearance::Tint>"—"</Badge> }.into_any(),
                                                };

                                                let type_text = task.task_type.clone();

                                                view! {
                                                    <TableRow
                                                        on:dblclick=move |_| open_details(task_id_details.clone(), code_for_details.clone())
                                                        attr:style="cursor:pointer;"
                                                    >
                                                        // Флаг выбора для пакетного запуска
                                                        <TableCell attr:style="width:40px;padding:0 4px;text-align:center;">
                                                            <input
                                                                type="checkbox"
                                                                class="table__checkbox"
                                                                prop:checked=move || {
                                                                    selected_ids.with(|s| s.contains(&task_id_select))
                                                                }
                                                                on:change=move |ev| {
                                                                    let checked = event_target_checked(&ev);
                                                                    let id = task_id_for_change.clone();
                                                                    set_selected_ids.update(|s| {
                                                                        if checked {
                                                                            s.insert(id);
                                                                        } else {
                                                                            s.remove(&id);
                                                                        }
                                                                    });
                                                                }
                                                                on:click=move |e: MouseEvent| e.stop_propagation()
                                                                title="Отметить для выполнения"
                                                            />
                                                        </TableCell>

                                                        // Описание — hyperlink to details
                                                        <TableCell>
                                                            <TableCellLayout truncate=true>
                                                                <a
                                                                    href="#"
                                                                    class="table__link"
                                                                    title=format!("[{}] {}", code, task.description)
                                                                    on:click=move |e: MouseEvent| {
                                                                        e.prevent_default();
                                                                        e.stop_propagation();
                                                                        open_details(task_id.clone(), code.clone());
                                                                    }
                                                                >
                                                                    {task.description.clone()}
                                                                </a>
                                                            </TableCellLayout>
                                                        </TableCell>

                                                        // Комментарий
                                                        <TableCell>
                                                            <TableCellLayout truncate=true>
                                                                {if comment_text.is_empty() {
                                                                    view! { <span style="color:var(--color-text-tertiary);font-size:12px;">"—"</span> }.into_any()
                                                                } else {
                                                                    view! {
                                                                        <span
                                                                            title=comment_text.clone()
                                                                            style="font-size:12px;color:var(--color-text-secondary);"
                                                                        >
                                                                            {comment_text.clone()}
                                                                        </span>
                                                                    }.into_any()
                                                                }}
                                                            </TableCellLayout>
                                                        </TableCell>

                                                        // Тип — truncated
                                                        <TableCell>
                                                            <TableCellLayout truncate=true>
                                                                <code
                                                                    title=type_text.clone()
                                                                    style="background:var(--colorNeutralBackground3);padding:2px 4px;border-radius:4px;font-size:0.82em;"
                                                                >
                                                                    {type_text.clone()}
                                                                </code>
                                                            </TableCellLayout>
                                                        </TableCell>

                                                        // Расписание — текст
                                                        <TableCell>
                                                            <span
                                                                title=task.schedule_cron.clone().unwrap_or_default()
                                                                style="font-size:var(--font-size-base);color:var(--color-text-secondary);"
                                                            >
                                                                {schedule_text}
                                                            </span>
                                                        </TableCell>

                                                        // Последний запуск
                                                        <TableCell>
                                                            <span style="font-size:0.9em;color:var(--colorNeutralForeground2);">
                                                                {task.last_run_at
                                                                    .map(|d| format_utc_local(&d, "%d.%m %H:%M"))
                                                                    .unwrap_or_else(|| "—".to_string())}
                                                            </span>
                                                        </TableCell>

                                                        // Статус
                                                        <TableCell>{status_view}</TableCell>

                                                        // Вкл
                                                        <TableCell>
                                                            <div style="text-align:center;">
                                                                <div
                                                                    on:click=move |e: MouseEvent| {
                                                                        e.stop_propagation();
                                                                        toggle_enabled(task_id_toggle.clone(), is_enabled);
                                                                    }
                                                                    style="cursor:pointer;display:inline-block;"
                                                                >
                                                                    <Checkbox checked=is_enabled attr:disabled=true />
                                                                </div>
                                                            </div>
                                                        </TableCell>
                                                    </TableRow>
                                                }
                                            }).collect_view().into_any()
                                        }
                                    }
                                }}
                            </TableBody>
                        </Table>
                        </div>
                    </div>
                }.into_any()
            } else {
                view! { <></> }.into_any()
            }}

            // Вкладка «Активные задачи» — только по открытию вкладки и кнопке «Обновить»
            {move || if active_tab.get() == "active" {
                view! {
                    <div>
                        <div style="margin-bottom:12px;padding:10px 12px;font-size:13px;line-height:1.45;color:var(--color-text-secondary);background:var(--colorNeutralBackground2);border:1px solid var(--color-border);border-radius:var(--radius-md);">
                            "Только in-memory: без запросов к БД и без чтения логов с диска. Таблица обновляется каждые 2 с, пока открыта вкладка. Показаны сессии «Running» в трекерах. История завершённых запусков — во вкладке «Мониторинг запусков» (БД) и в карточке "
                            <strong>"задачи"</strong>
                            "."
                        </div>
                        {move || if runs_loading.get() {
                            view! {
                                <Flex justify=FlexJustify::Center align=FlexAlign::Center style="padding:32px;">
                                    <Spinner />" Загрузка..."
                                </Flex>
                            }.into_any()
                        } else {
                            view! { <></> }.into_any()
                        }}
                        {move || {
                            let active = live_memory_items.get();
                            if active.is_empty() {
                                view! {
                                    <div style="padding:24px;color:var(--color-text-secondary);font-size:13px;">
                                        "Нет сессий «Running» в памяти трекеров (задачи не выполняются или трекер ещё не создан)."
                                    </div>
                                }.into_any()
                            } else {
                                let total_http: i32 = active
                                    .iter()
                                    .map(|item| item.progress.http_request_count.unwrap_or(0))
                                    .sum();
                                let total_sent: i64 = active
                                    .iter()
                                    .map(|item| item.progress.http_bytes_sent.unwrap_or(0))
                                    .sum();
                                let total_received: i64 = active
                                    .iter()
                                    .map(|item| item.progress.http_bytes_received.unwrap_or(0))
                                    .sum();
                                let total_traffic = format_http_traffic(total_sent, total_received)
                                    .unwrap_or_else(|| "—".to_string());
                                view! {
                                    <div style="display:flex;flex-direction:column;gap:8px;">
                                        <div style="display:flex;gap:16px;align-items:center;flex-wrap:wrap;padding:8px 10px;border:1px solid var(--color-border);border-radius:var(--radius-md);background:var(--colorNeutralBackground2);font-size:12px;color:var(--color-text-secondary);">
                                            <span>{format!("Активных сессий: {}", active.len())}</span>
                                            <span style="font-family:monospace;">{format!("HTTP: {}", total_http)}</span>
                                            <span style="font-family:monospace;">{format!("Трафик: {}", total_traffic)}</span>
                                        </div>
                                        <div class="table-wrapper">
                                            <Table>
                                                <TableHeader>
                                                    <TableRow>
                                                        <TableHeaderCell min_width=200.0>"Задача"</TableHeaderCell>
                                                        <TableHeaderCell min_width=160.0>"Сессия"</TableHeaderCell>
                                                        <TableHeaderCell attr:style="width:80px;">"Начало"</TableHeaderCell>
                                                        <TableHeaderCell attr:style="width:70px;">"Длит."</TableHeaderCell>
                                                        <TableHeaderCell min_width=240.0>"Прогресс"</TableHeaderCell>
                                                        <TableHeaderCell attr:style="width:72px;">
                                                            <span title="Число прочитанных HTTP-ответов внешнего API">"HTTP"</span>
                                                        </TableHeaderCell>
                                                        <TableHeaderCell min_width=140.0>
                                                            <span title="Суммарный размер тел запросов и ответов, без сетевых заголовков">"Трафик"</span>
                                                        </TableHeaderCell>
                                                        <TableHeaderCell attr:style="width:100px;"></TableHeaderCell>
                                                    </TableRow>
                                                </TableHeader>
                                                <TableBody>
                                                    {active.into_iter().map(|item| {
                                                        let on_aborted = Callback::new(move |_| {
                                                            fetch_live_memory(false);
                                                        });
                                                        view! {
                                                            <LiveMemoryProgressRow item=item on_aborted=on_aborted />
                                                        }
                                                    }).collect_view()}
                                                </TableBody>
                                            </Table>
                                        </div>
                                    </div>
                                }.into_any()
                            }
                        }}
                    </div>
                }.into_any()
            } else {
                view! { <></> }.into_any()
            }}

            // Вкладка «Мониторинг запусков» — только история; по открытию и кнопке «Обновить»
            {move || if active_tab.get() == "monitoring" {
                view! {
                    <div>
                        // Фильтры
                        <div style="display:flex;gap:10px;align-items:center;margin-bottom:10px;flex-wrap:wrap;">
                            <div style="position:relative;flex:1;min-width:180px;max-width:360px;">
                                <span style="position:absolute;left:8px;top:50%;transform:translateY(-50%);color:var(--color-text-secondary);pointer-events:none;font-size:14px;">
                                    "🔍"
                                </span>
                                <input
                                    type="text"
                                    placeholder="Поиск по коду или описанию задачи…"
                                    style="width:100%;box-sizing:border-box;padding:6px 8px 6px 28px;border:1px solid var(--color-border);border-radius:var(--radius-md);background:var(--colorNeutralBackground1);color:var(--color-text-primary);font-size:13px;"
                                    prop:value=move || mon_filter_text.get()
                                    on:input=move |ev| set_mon_filter_text.set(event_target_value(&ev))
                                />
                            </div>
                            <select
                                style="padding:6px 10px;border:1px solid var(--color-border);border-radius:var(--radius-md);background:var(--colorNeutralBackground1);color:var(--color-text-primary);font-size:13px;cursor:pointer;"
                                prop:value=move || mon_filter_status.get()
                                on:change=move |ev| set_mon_filter_status.set(event_target_value(&ev))
                            >
                                <option value="">"Все статусы"</option>
                                <option value="Completed">"Успешно"</option>
                                <option value="CompletedWithErrors">"С ошибками"</option>
                                <option value="Failed">"Ошибка"</option>
                                <option value="Running">"В работе"</option>
                            </select>
                            {move || {
                                let has_filter = !mon_filter_text.get().is_empty() || !mon_filter_status.get().is_empty();
                                if has_filter {
                                    view! {
                                        <button
                                            style="padding:5px 12px;border:1px solid var(--color-border);border-radius:var(--radius-md);background:var(--colorNeutralBackground2);color:var(--color-text-secondary);font-size:12px;cursor:pointer;"
                                            on:click=move |_| {
                                                set_mon_filter_text.set(String::new());
                                                set_mon_filter_status.set(String::new());
                                            }
                                        >"✕ Сбросить"</button>
                                    }.into_any()
                                } else {
                                    view! { <></> }.into_any()
                                }
                            }}
                            {move || {
                                let total = recent_runs.get().len();
                                let shown = filtered_runs.get().len();
                                let has_filter = !mon_filter_text.get().is_empty() || !mon_filter_status.get().is_empty();
                                if has_filter {
                                    view! {
                                        <span style="font-size:12px;color:var(--color-text-secondary);">
                                            {format!("{} из {}", shown, total)}
                                        </span>
                                    }.into_any()
                                } else {
                                    view! { <span style="font-size:12px;color:var(--color-text-secondary);">{format!("{} запусков", total)}</span> }.into_any()
                                }
                            }}
                        </div>

                        {move || if runs_loading.get() {
                            view! {
                                <Flex justify=FlexJustify::Center align=FlexAlign::Center style="padding:32px;">
                                    <Spinner />" Загрузка..."
                                </Flex>
                            }.into_any()
                        } else {
                            view! { <></> }.into_any()
                        }}
                        {move || {
                            let runs = filtered_runs.get();
                            if runs.is_empty() {
                                let is_filtered = !mon_filter_text.get_untracked().is_empty() || !mon_filter_status.get_untracked().is_empty();
                                let msg = if is_filtered {
                                    "Нет запусков, соответствующих фильтру."
                                } else {
                                    "История запусков пуста. Запустите задачу вручную или дождитесь планового выполнения."
                                };
                                view! {
                                    <div style="padding:24px;text-align:left;color:var(--color-text-secondary);font-size:13px;">
                                        {msg}
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div class="table-wrapper">
                                        <Table>
                                            <TableHeader>
                                                <TableRow>
                                                    <TableHeaderCell min_width=200.0>"Задача"</TableHeaderCell>
                                                    <TableHeaderCell attr:style="width:140px;">"Начало"</TableHeaderCell>
                                                    <TableHeaderCell attr:style="width:140px;">"Конец"</TableHeaderCell>
                                                    <TableHeaderCell attr:style="width:110px;">"Источник"</TableHeaderCell>
                                                    <TableHeaderCell attr:style="width:90px;">"Статус"</TableHeaderCell>
                                                    <TableHeaderCell attr:style="width:80px;">"Длит."</TableHeaderCell>
                                                    <TableHeaderCell attr:style="width:72px;">"Обраб."</TableHeaderCell>
                                                    <TableHeaderCell attr:style="width:72px;">"Новые"</TableHeaderCell>
                                                    <TableHeaderCell attr:style="width:72px;">"Изм."</TableHeaderCell>
                                                    <TableHeaderCell attr:style="width:60px;">"Ошибки"</TableHeaderCell>
                                                    <TableHeaderCell attr:style="width:72px;">"HTTP"</TableHeaderCell>
                                                    <TableHeaderCell min_width=132.0>"Трафик"</TableHeaderCell>
                                                </TableRow>
                                            </TableHeader>
                                            <TableBody>
                                                {runs.into_iter().map(|run| view! {
                                                    <MonitoringRow run=run />
                                                }).collect_view()}
                                            </TableBody>
                                        </Table>
                                    </div>
                                }.into_any()
                            }
                        }}
                    </div>
                }.into_any()
            } else {
                view! { <></> }.into_any()
            }}
        </div>
    }
}
