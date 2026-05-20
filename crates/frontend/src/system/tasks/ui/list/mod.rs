pub mod state;

use self::state::create_state;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::change_tokens::ChangeTokenContext;
use crate::shared::date_utils::{
    format_duration_ms, format_http_traffic, format_utc_local, TZ_OFFSET_HOURS,
};
use crate::shared::icons::icon;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator, sort_list, Sortable};
use crate::system::tasks::api::{self, RunTaskNowOutcome};
use crate::system::tasks::ui::history::TaskHistoryView;
use chrono::Utc;
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

/// CSS-класс BEM-бейджа для статуса запуска задачи.
/// Использует классы из components.css: `badge badge--*`
fn task_run_status_badge_class(status: &str) -> &'static str {
    match status {
        "Completed" => "badge badge--success",
        "CompletedWithErrors" => "badge badge--warning",
        "Running" => "badge badge--primary",
        "Failed" => "badge badge--error",
        "Cancelled" => "badge badge--neutral",
        _ => "badge badge--neutral",
    }
}

fn task_run_status_label(status: &str) -> &'static str {
    match status {
        "Completed" => "Успешно",
        "CompletedWithErrors" => "С ошибками",
        "Running" => "Выполняется",
        "Failed" => "Ошибка",
        "Cancelled" => "Отменено",
        _ => "—",
    }
}

fn live_progress_caption(p: &TaskProgressResponse) -> String {
    if let Some(current_item) = p.current_item.as_ref().filter(|s| !s.trim().is_empty()) {
        return current_item.clone();
    }

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
            "last_run_status" => self
                .last_run_status
                .as_deref()
                .unwrap_or("")
                .cmp(other.last_run_status.as_deref().unwrap_or("")),
            "is_enabled" => other.is_enabled.cmp(&self.is_enabled),
            _ => Ordering::Equal,
        }
    }
}

impl Sortable for TaskRun {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "task_description" => self
                .task_description
                .as_deref()
                .unwrap_or("")
                .to_lowercase()
                .cmp(
                    &other
                        .task_description
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase(),
                ),
            "started_at" => self.started_at.cmp(&other.started_at),
            "finished_at" => self.finished_at.cmp(&other.finished_at),
            "status" => self.status.cmp(&other.status),
            "triggered_by" => self.triggered_by.cmp(&other.triggered_by),
            "duration_ms" => self.duration_ms.cmp(&other.duration_ms),
            _ => Ordering::Equal,
        }
    }
}

#[component]
fn SortHeaderCell(
    label: &'static str,
    field: &'static str,
    min_width: f32,
    #[prop(default = true)] resizable: bool,
    sort_field: ReadSignal<String>,
    sort_ascending: ReadSignal<bool>,
    on_toggle: Callback<&'static str>,
) -> impl IntoView {
    view! {
        <TableHeaderCell resizable=resizable min_width=min_width>
            <div
                class="table__sortable-header"
                style="cursor:pointer;user-select:none;"
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
                <Flex vertical=true gap=FlexGap::Small style="min-width:0;">
                    <span style="font-size:13px;font-weight:600;">{item.task_display_name.clone()}</span>
                    <span style="font-size:11px;color:var(--color-text-secondary);font-family:monospace;">{item.task_type.clone()}</span>
                </Flex>
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
    let task_title = run
        .task_description
        .clone()
        .unwrap_or_else(|| run.task_id.clone());
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
    let task_comment = run.task_comment.clone().unwrap_or_else(|| "—".to_string());
    let status = run.status.clone();
    let mon_http_calls = monitoring_http_calls(&run);
    let mon_http_traffic = monitoring_http_traffic(&run);

    let status_badge_class = task_run_status_badge_class(&status);
    let status_label = task_run_status_label(&status);

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
                <TableCellLayout truncate=true>
                    <span style="font-size:13px;">{task_title}</span>
                </TableCellLayout>
            </TableCell>
            <TableCell>
                <span style="font-size:12px;color:var(--color-text-tertiary);">{task_comment}</span>
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
                <span class=status_badge_class>{status_label}</span>
            </TableCell>
            <TableCell>
                <span style="font-size:12px;">{duration}</span>
            </TableCell>
            <TableCell attr:style="text-align:right;">
                <span style="font-size:12px;">{processed}</span>
            </TableCell>
            <TableCell attr:style="text-align:right;">
                <span style="font-size:12px;color:var(--colorPaletteGreenForeground1);">{inserted}</span>
            </TableCell>
            <TableCell attr:style="text-align:right;">
                <span style="font-size:12px;color:var(--colorBrandForeground1);">{updated}</span>
            </TableCell>
            <TableCell attr:style="text-align:right;">
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

    // Scheduler global on/off state
    let scheduler_enabled = RwSignal::new(true);
    // Включён ли планировщик в config.toml ([scheduled_tasks].enabled).
    // Если false — фоновый воркер не запущен, переключатель не имеет эффекта.
    let scheduler_config_enabled = RwSignal::new(true);

    // Дата последнего действия со планировщиком (сохраняется в localStorage)
    const LS_KEY: &str = "sys_tasks_scheduler_last_action";
    let saved_ts = web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item(LS_KEY).ok().flatten());
    let scheduler_last_action = RwSignal::new(saved_ts);

    /// Возвращает текущее время в МСК как строку "dd.mm.yyyy HH:MM:SS".
    fn current_msk_str() -> String {
        let now_ms = js_sys::Date::now() as i64;
        let now_utc = chrono::DateTime::from_timestamp_millis(now_ms).unwrap_or(Utc::now());
        format_utc_local(&now_utc, "%d.%m.%Y %H:%M:%S")
    }

    fn ls_save_scheduler_ts(val: &str) {
        if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = storage.set_item("sys_tasks_scheduler_last_action", val);
        }
    }

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
    let (filter_status, set_filter_status) = signal(String::new());

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

        let fs = filter_status.get();
        if !fs.is_empty() {
            if fs == "__none__" {
                tasks.retain(|t| t.last_run_status.is_none());
            } else {
                tasks.retain(|t| t.last_run_status.as_deref() == Some(fs.as_str()));
            }
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

    // Monitoring sort (по умолчанию — новые сверху)
    let (mon_sort_field, set_mon_sort_field) = signal("started_at".to_string());
    let (mon_sort_ascending, set_mon_sort_ascending) = signal(false);
    let toggle_mon_sort = Callback::new(move |field: &'static str| {
        if mon_sort_field.get_untracked() == field {
            set_mon_sort_ascending.update(|v| *v = !*v);
        } else {
            set_mon_sort_field.set(field.to_string());
            set_mon_sort_ascending.set(true);
        }
    });

    let filtered_runs = Signal::derive(move || {
        let mut runs = recent_runs.get();

        let fs = mon_filter_status.get();
        if !fs.is_empty() {
            runs.retain(|r| r.status == fs);
        }

        let fx = mon_filter_text.get().to_lowercase();
        if !fx.is_empty() {
            runs.retain(|r| {
                r.task_description
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&fx)
            });
        }

        sort_list(&mut runs, &mon_sort_field.get(), mon_sort_ascending.get());
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

    // Загружаем начальное состояние планировщика один раз при монтировании.
    spawn_local(async move {
        if let Ok(status) = api::fetch_scheduler_status().await {
            scheduler_enabled.set(status.enabled);
            scheduler_config_enabled.set(status.config_enabled);
            // Если в localStorage ещё нет сохранённой даты — записываем текущее время.
            if scheduler_last_action.get_untracked().is_none() {
                let ts = current_msk_str();
                ls_save_scheduler_ts(&ts);
                scheduler_last_action.set(Some(ts));
            }
        }
    });

    // Сохраняем в БД при изменении пользователем; пропускаем первый вызов (инициализацию).
    Effect::new(move |prev: Option<bool>| {
        let enabled = scheduler_enabled.get();
        if let Some(prev_val) = prev {
            if prev_val != enabled {
                let ts = current_msk_str();
                ls_save_scheduler_ts(&ts);
                scheduler_last_action.set(Some(ts));
                spawn_local(async move {
                    if let Err(e) = api::set_scheduler_status(enabled).await {
                        log!("Failed to set scheduler status: {}", e);
                    }
                });
            }
        }
        enabled
    });

    // Автообновление списка «Задания» при изменении токена на сервере.
    // prev=None при первом вызове — пропускаем, чтобы не делать двойную загрузку.
    let ct = use_context::<ChangeTokenContext>().expect("ChangeTokenContext not found");
    Effect::new(move |prev: Option<u64>| {
        let token = ct.sys_tasks.get();
        if prev.is_some() {
            load_tasks();
        }
        token
    });

    // Фоновый поллинг живой памяти — всегда, независимо от активной вкладки.
    // Нужен чтобы счётчик в ярлыке «Активные задачи (N)» обновлялся на любой вкладке.
    spawn_local(async move {
        loop {
            TimeoutFuture::new(LIVE_MEMORY_POLL_MS).await;
            match api::get_active_runs_with_progress().await {
                Ok(resp) => set_live_memory_items.set(resp.items),
                Err(e) => log!("Live memory background poll failed: {}", e),
            }
        }
    });

    Effect::new(move |_| match active_tab.get().as_str() {
        // При открытии вкладки — немедленная загрузка со спиннером (не ждём следующего тика).
        "active" => fetch_live_memory(true),
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
                            format_utc_local(&r.started_at, "%d.%m.%Y %H:%M:%S")
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
            <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center gap=FlexGap::Medium style="margin-bottom:16px;">
                <h2 style="margin:0;font-size:24px;font-weight:bold;flex-shrink:0;">"Регламентные задания"</h2>

                <div style="flex-shrink:0;">
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
                    }.into_any(),
                    "monitoring" => view! {
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
                    }.into_any(),
                    _ => view! { <></> }.into_any(),
                }}
                </div>
            </Flex>

            // Полоса-предупреждение: планировщик выключен в config.toml.
            // Показывается только если [scheduled_tasks].enabled = false —
            // в этом случае фоновый воркер не запущен и переключатель
            // «Планировщик работает» не имеет эффекта.
            {move || (!scheduler_config_enabled.get()).then(|| view! {
                <div style="margin-bottom:16px;">
                    <MessageBar intent=MessageBarIntent::Warning>
                        "Планировщик отключён в настройках приложения config.toml. Фоновое выполнение регламентных заданий отключено, переключатель ниже не имеет эффекта."
                    </MessageBar>
                </div>
            })}

            <div style="margin-bottom: 16px;">
                <TabList selected_value=active_tab>
                    <Tab value="tasks".to_string()>"Все"</Tab>
                    <Tab value="active".to_string()>
                        {move || {
                            let count = live_memory_items.get().len();
                            if count > 0 {
                                format!("Активные ({})", count)
                            } else {
                                "Активные".to_string()
                            }
                        }}
                    </Tab>
                    <Tab value="monitoring".to_string()>"Мониторинг"</Tab>
                    <Tab value="history".to_string()>"История"</Tab>
                </TabList>
            </div>

            {move || error.get().map(|err| view! {
                <MessageBar intent=MessageBarIntent::Error>
                    {err}
                </MessageBar>
            })}

            {move || batch_warning.get().map(|w| view! {
                <MessageBar intent=MessageBarIntent::Warning>
                    {w}
                </MessageBar>
            })}

            // Tasks tab
            {move || if active_tab.get() == "tasks" {
                view! {
                    <div>
                        // Фильтры
                        <Flex gap=FlexGap::Small align=FlexAlign::Center style="flex-wrap:wrap;margin-bottom:10px;">
                            <div class="task-filter__search">
                                <span class="task-filter__search-icon">
                                    "🔍"
                                </span>
                                <input
                                    type="text"
                                    placeholder="Поиск по описанию, коду, комментарию…"
                                    class="task-filter__search-input"
                                    prop:value=move || filter_text.get()
                                    on:input=move |ev| set_filter_text.set(event_target_value(&ev))
                                />
                            </div>
                            <select
                                class="task-filter__select"
                                prop:value=move || filter_type.get()
                                on:change=move |ev| set_filter_type.set(event_target_value(&ev))
                            >
                                <option value="">"Все типы"</option>
                                {move || unique_task_types.get().into_iter().map(|t| {
                                    let tv = t.clone();
                                    view! { <option value=tv>{t}</option> }
                                }).collect_view()}
                            </select>
                            <select
                                class="task-filter__select"
                                prop:value=move || filter_status.get()
                                on:change=move |ev| set_filter_status.set(event_target_value(&ev))
                            >
                                <option value="">"Все статусы"</option>
                                <option value="Running">"Выполняется"</option>
                                <option value="Completed">"Успешно"</option>
                                <option value="CompletedWithErrors">"С ошибками"</option>
                                <option value="Failed">"Ошибка"</option>
                                <option value="__none__">"Не запускалась"</option>
                            </select>
                            {move || {
                                let has_filter = !filter_text.get().is_empty()
                                    || !filter_type.get().is_empty()
                                    || !filter_status.get().is_empty();
                                if has_filter {
                                    view! {
                                        <Button
                                            appearance=ButtonAppearance::Secondary
                                            on_click=move |_| {
                                                set_filter_text.set(String::new());
                                                set_filter_type.set(String::new());
                                                set_filter_status.set(String::new());
                                            }
                                        >
                                            "Сбросить"
                                        </Button>
                                    }.into_any()
                                } else {
                                    view! { <></> }.into_any()
                                }
                            }}
                            {move || {
                                let total = state.get().tasks.len();
                                let shown = sorted_tasks.get().len();
                                let has_filter = !filter_text.get().is_empty()
                                    || !filter_type.get().is_empty()
                                    || !filter_status.get().is_empty();
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
                            <Flex align=FlexAlign::Center gap=FlexGap::Small style="margin-left:auto;flex-shrink:0;">
                                <Switch checked=scheduler_enabled />
                                <span style=move || {
                                    let color = if scheduler_enabled.get() {
                                        "var(--colorPaletteGreenForeground1)"
                                    } else {
                                        "var(--color-error)"
                                    };
                                    format!("font-size:14px;font-weight:600;user-select:none;color:{color};")
                                }>
                                    {move || if scheduler_enabled.get() { "Планировщик включен" } else { "Планировщик выключен" }}
                                </span>
                                {move || scheduler_last_action.get().map(|ts| view! {
                                    <span style="font-size:11px;opacity:0.65;user-select:none;">
                                        {ts}
                                    </span>
                                })}
                            </Flex>
                        </Flex>

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
                                    <SortHeaderCell label="Задача" field="description" min_width=180.0
                                        sort_field=sort_field sort_ascending=sort_ascending on_toggle=toggle_sort />
                                    <SortHeaderCell label="Комментарий" field="comment" min_width=150.0
                                        sort_field=sort_field sort_ascending=sort_ascending on_toggle=toggle_sort />
                                    <SortHeaderCell label="Тип" field="task_type" min_width=120.0
                                        sort_field=sort_field sort_ascending=sort_ascending on_toggle=toggle_sort />
                                    <TableHeaderCell resizable=true min_width=130.0>"Расписание"</TableHeaderCell>
                                    <SortHeaderCell label="Последний запуск" field="last_run_at" min_width=120.0
                                        sort_field=sort_field sort_ascending=sort_ascending on_toggle=toggle_sort />
                                    <SortHeaderCell label="Статус" field="last_run_status" min_width=100.0
                                        sort_field sort_ascending on_toggle=toggle_sort />
                                    <SortHeaderCell label="Авто" field="is_enabled" min_width=70.0
                                        resizable=false
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

                                                let status_class = task_run_status_badge_class(
                                                    task.last_run_status.as_deref().unwrap_or(""),
                                                );
                                                let status_label = task_run_status_label(
                                                    task.last_run_status.as_deref().unwrap_or(""),
                                                );

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
                                                                <span
                                                                    title=type_text.clone()
                                                                    style="font-size:var(--font-size-sm);color:var(--color-text-secondary);"
                                                                >
                                                                    {type_text.clone()}
                                                                </span>
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
                                                        <TableCell><span class=status_class>{status_label}</span></TableCell>

                                                        // Вкл
                                                        <TableCell>
                                                            <div
                                                                on:click=move |e: MouseEvent| {
                                                                    e.stop_propagation();
                                                                    toggle_enabled(task_id_toggle.clone(), is_enabled);
                                                                }
                                                                style="text-align:center;cursor:pointer;"
                                                            >
                                                                <Checkbox checked=is_enabled attr:disabled=true />
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
                        <MessageBar intent=MessageBarIntent::Info>
                               "Все серверные задачи. Обновляется каждые 2 с, пока открыта вкладка. Показаны сессии «Running» в трекерах. История во вкладке «Мониторинг»"
                        </MessageBar>
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
                                let active_len = active.len();
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
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Flex gap=FlexGap::Medium align=FlexAlign::Center style="flex-wrap:wrap;padding:8px 10px;border:1px solid var(--color-border);border-radius:var(--radius-md);background:var(--colorNeutralBackground2);font-size:12px;color:var(--color-text-secondary);">
                                            <span>{format!("Активных сессий: {}", active_len)}</span>
                                            <span style="font-family:monospace;">{format!("HTTP: {}", total_http)}</span>
                                            <span style="font-family:monospace;">{format!("Трафик: {}", total_traffic)}</span>
                                        </Flex>
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
                                                            <span title="Число отправленных HTTP-запросов внешнего API">"HTTP"</span>
                                                        </TableHeaderCell>
                                                        <TableHeaderCell min_width=140.0>
                                                            <span title="Суммарный размер запросов и ответов, без сетевых заголовков">"Трафик"</span>
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
                                    </Flex>
                                }.into_any()
                            }
                        }}
                    </div>
                }.into_any()
            } else {
                view! { <></> }.into_any()
            }}

            // Вкладка «История» — график агрегированной истории запусков.
            {move || if active_tab.get() == "history" {
                view! {
                    <TaskHistoryView />
                }.into_any()
            } else {
                view! { <></> }.into_any()
            }}

            // Вкладка «Мониторинг запусков» — только история; по открытию и кнопке «Обновить»
            {move || if active_tab.get() == "monitoring" {
                view! {
                    <div>
                        // Фильтры
                        <Flex gap=FlexGap::Small align=FlexAlign::Center style="flex-wrap:wrap;margin-bottom:10px;">
                            <div class="task-filter__search">
                                <span class="task-filter__search-icon">
                                    "🔍"
                                </span>
                                <input
                                    type="text"
                                    placeholder="Поиск по коду или описанию задачи…"
                                    class="task-filter__search-input"
                                    prop:value=move || mon_filter_text.get()
                                    on:input=move |ev| set_mon_filter_text.set(event_target_value(&ev))
                                />
                            </div>
                            <select
                                class="task-filter__select"
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
                                        <Button
                                            appearance=ButtonAppearance::Secondary
                                            on_click=move |_| {
                                                set_mon_filter_text.set(String::new());
                                                set_mon_filter_status.set(String::new());
                                            }
                                        >
                                            "Сбросить"
                                        </Button>
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
                        </Flex>

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
                                                    <SortHeaderCell label="Задача" field="task_description" min_width=180.0
                                                        sort_field=mon_sort_field sort_ascending=mon_sort_ascending on_toggle=toggle_mon_sort />
                                                    <TableHeaderCell min_width=150.0>"Комментарий"</TableHeaderCell>
                                                    <SortHeaderCell label="Начало" field="started_at" min_width=130.0
                                                        sort_field=mon_sort_field sort_ascending=mon_sort_ascending on_toggle=toggle_mon_sort />
                                                    <SortHeaderCell label="Конец" field="finished_at" min_width=130.0
                                                        sort_field=mon_sort_field sort_ascending=mon_sort_ascending on_toggle=toggle_mon_sort />
                                                    <SortHeaderCell label="Источник" field="triggered_by" min_width=110.0
                                                        sort_field=mon_sort_field sort_ascending=mon_sort_ascending on_toggle=toggle_mon_sort />
                                                    <SortHeaderCell label="Статус" field="status" min_width=100.0
                                                        sort_field=mon_sort_field sort_ascending=mon_sort_ascending on_toggle=toggle_mon_sort />
                                                    <SortHeaderCell label="Длит." field="duration_ms" min_width=80.0
                                                        sort_field=mon_sort_field sort_ascending=mon_sort_ascending on_toggle=toggle_mon_sort />
                                                    <TableHeaderCell attr:style="width:72px;text-align:right;">"Обраб."</TableHeaderCell>
                                                    <TableHeaderCell attr:style="width:72px;text-align:right;">"Новые"</TableHeaderCell>
                                                    <TableHeaderCell attr:style="width:72px;text-align:right;">"Изм."</TableHeaderCell>
                                                    <TableHeaderCell attr:style="width:60px;text-align:right;">"Ошибки"</TableHeaderCell>
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
