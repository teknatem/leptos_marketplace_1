use contracts::system::raw_storage::{
    DbVacuumStatus, RawStorageCleanupMode, RawStorageCleanupRequest, RawStorageStatus,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::Input;

use crate::shared::date_utils::format_bytes_compact;
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_SYSTEM;
use crate::system::auth::guard::RequireAdmin;
use crate::system::raw_storage::api;

#[component]
pub fn RawStoragePage() -> impl IntoView {
    view! {
        <RequireAdmin>
            <RawStorageContent />
        </RequireAdmin>
    }
}

#[component]
fn RawStorageContent() -> impl IntoView {
    let status = RwSignal::<Option<RawStorageStatus>>::new(None);
    let vacuum_status = RwSignal::<Option<DbVacuumStatus>>::new(None);
    let loading = RwSignal::new(false);
    let action_busy = RwSignal::new(false);
    let busy_label = RwSignal::<Option<String>>::new(None);
    let error = RwSignal::<Option<String>>::new(None);
    let notice = RwSignal::<Option<String>>::new(None);
    let older_than_days = RwSignal::new("30".to_string());

    let reload = Callback::new(move |_| {
        loading.set(true);
        busy_label.set(Some("Загрузка данных...".to_string()));
        error.set(None);
        spawn_local(async move {
            match api::fetch_status().await {
                Ok(next) => status.set(Some(next)),
                Err(err) => error.set(Some(err)),
            }
            match api::fetch_vacuum_status().await {
                Ok(next) => vacuum_status.set(Some(next)),
                Err(err) => error.set(Some(err)),
            }
            loading.set(false);
            busy_label.set(None);
        });
    });

    Effect::new(move |_| {
        reload.run(());
    });

    let capture_enabled =
        Signal::derive(move || status.get().map(|s| s.capture_enabled).unwrap_or(false));

    let toggle_capture = move |enabled: bool| {
        action_busy.set(true);
        busy_label.set(Some("Изменение настройки захвата...".to_string()));
        error.set(None);
        notice.set(None);
        spawn_local(async move {
            match api::update_settings(enabled).await {
                Ok(_) => {
                    notice.set(Some(if enabled {
                        "Сохранение raw JSON включено".to_string()
                    } else {
                        "Сохранение raw JSON выключено".to_string()
                    }));
                    reload.run(());
                }
                Err(err) => error.set(Some(err)),
            }
            action_busy.set(false);
            busy_label.set(None);
        });
    };

    let run_cleanup = move |req: RawStorageCleanupRequest, title: &'static str| {
        action_busy.set(true);
        error.set(None);
        notice.set(None);
        spawn_local(async move {
            match api::cleanup_preview(&req).await {
                Ok(preview) => {
                    let msg = format!(
                        "{}: будет удалено {} строк, примерно {}. Продолжить?",
                        title,
                        format_count(preview.rows_to_delete),
                        format_mb(preview.estimated_mb)
                    );
                    let confirmed = web_sys::window()
                        .and_then(|w| w.confirm_with_message(&msg).ok())
                        .unwrap_or(false);
                    if confirmed {
                        busy_label.set(Some(format!("Выполняется: {title}...")));
                        match api::cleanup(&req).await {
                            Ok(result) => {
                                notice.set(Some(format!(
                                    "Очистка выполнена: {} строк, примерно {}",
                                    format_count(result.rows_to_delete),
                                    format_mb(result.estimated_mb)
                                )));
                                reload.run(());
                            }
                            Err(err) => error.set(Some(err)),
                        }
                    }
                }
                Err(err) => error.set(Some(err)),
            }
            action_busy.set(false);
            busy_label.set(None);
        });
    };

    let cleanup_unreferenced = move |_| {
        run_cleanup(
            RawStorageCleanupRequest {
                mode: RawStorageCleanupMode::Unreferenced,
                older_than_days: None,
            },
            "Очистить unreferenced raw JSON",
        );
    };

    let cleanup_duplicates = move |_| {
        run_cleanup(
            RawStorageCleanupRequest {
                mode: RawStorageCleanupMode::Duplicates,
                older_than_days: None,
            },
            "Очистить точные дубли raw JSON",
        );
    };

    let cleanup_old = move |_| {
        let days = older_than_days
            .get_untracked()
            .trim()
            .parse::<i64>()
            .unwrap_or(30)
            .max(0);
        run_cleanup(
            RawStorageCleanupRequest {
                mode: RawStorageCleanupMode::OlderThanDays,
                older_than_days: Some(days),
            },
            "Очистить старые raw JSON",
        );
    };

    let cleanup_all = move |_| {
        run_cleanup(
            RawStorageCleanupRequest {
                mode: RawStorageCleanupMode::All,
                older_than_days: None,
            },
            "Очистить все raw JSON",
        );
    };

    let run_vacuum = move |_| {
        let reclaimable = vacuum_status.get_untracked().map(|s| s.reclaimable_mb);
        let estimate = reclaimable
            .map(format_mb)
            .unwrap_or_else(|| "неизвестно сколько".to_string());
        let msg = format!(
            "VACUUM пересоберёт весь файл базы данных (освободит примерно {estimate}). \
             Операция держит запись занятой на время выполнения — другие пользователи и фоновые \
             задания могут получать ошибки или подвисать, пока VACUUM не завершится. \
             Выполняйте вне рабочего времени. Продолжить?"
        );
        let confirmed = web_sys::window()
            .and_then(|w| w.confirm_with_message(&msg).ok())
            .unwrap_or(false);
        if !confirmed {
            return;
        }

        action_busy.set(true);
        busy_label.set(Some(
            "Выполняется VACUUM — это может занять продолжительное время...".to_string(),
        ));
        error.set(None);
        notice.set(None);
        spawn_local(async move {
            match api::run_vacuum().await {
                Ok(result) => {
                    let wal_message = if result.wal_truncated {
                        format!(
                            "; WAL очищен: {} → {}",
                            format_mb(result.wal_mb_before),
                            format_mb(result.wal_mb_after)
                        )
                    } else {
                        "; WAL пока не усечён из-за активного читателя — повторите «Очистить WAL» позже".to_string()
                    };
                    notice.set(Some(format!(
                        "VACUUM выполнен за {}: файл {} → {} (освобождено {}){}",
                        format_duration(result.duration_ms),
                        format_mb(result.file_mb_before),
                        format_mb(result.file_mb_after),
                        format_mb(result.freed_mb),
                        wal_message
                    )));
                    reload.run(());
                }
                Err(err) => error.set(Some(err)),
            }
            action_busy.set(false);
            busy_label.set(None);
        });
    };

    let truncate_wal = move |_| {
        action_busy.set(true);
        busy_label.set(Some("Выполняется checkpoint и очистка WAL...".to_string()));
        error.set(None);
        notice.set(None);
        spawn_local(async move {
            match api::truncate_wal().await {
                Ok(result) if result.truncated => notice.set(Some(format!(
                    "WAL очищен: {} → {}",
                    format_mb(result.wal_mb_before),
                    format_mb(result.wal_mb_after)
                ))),
                Ok(_) => error.set(Some(
                    "WAL не удалось усечь: база удерживается активным читателем. Повторите позже."
                        .to_string(),
                )),
                Err(err) => error.set(Some(err)),
            }
            reload.run(());
            action_busy.set(false);
            busy_label.set(None);
        });
    };

    view! {
        <PageFrame page_id="sys_raw_storage--system" category=PAGE_CAT_SYSTEM class="page--wide">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Raw JSON"</h1>
                    <p class="page__subtitle">"Отладочное хранилище API payload. В штатном режиме raw JSON не сохраняется."</p>
                </div>
                <div class="page__header-right">
                    <button
                        class="button button--secondary"
                        disabled=move || loading.get()
                        on:click=move |_| reload.run(())
                    >
                        {icon("refresh-cw")}
                        {move || if loading.get() { "Обновление данных..." } else { "Обновить данные" }}
                    </button>
                </div>
            </div>

            <div class="page__content">
                <Show when=move || busy_label.get().is_some()>
                    <div class="raw-storage__busy-banner">
                        <span class="page-action-button__spinner"></span>
                        <span>{move || busy_label.get().unwrap_or_default()}</span>
                    </div>
                </Show>

                {move || error.get().map(|err| view! {
                    <div class="alert alert--error">{err}</div>
                })}
                {move || notice.get().map(|msg| view! {
                    <div class="alert alert--success">{msg}</div>
                })}

                <section class="raw-storage__section">
                    <h2 class="raw-storage__section-title">"Захват payload"</h2>
                    <div class="raw-storage__capture-row">
                        <label class="form__checkbox-wrapper" class:form__checkbox-wrapper--disabled=move || action_busy.get()>
                            <input
                                type="checkbox"
                                class="form__checkbox"
                                prop:checked=move || capture_enabled.get()
                                prop:disabled=move || action_busy.get()
                                on:change=move |ev| {
                                    toggle_capture(event_target_checked(&ev));
                                }
                            />
                            <span class="form__checkbox-label">"Сохранять raw JSON"</span>
                        </label>
                        <span class=move || if capture_enabled.get() { "badge badge--success" } else { "badge badge--neutral" }>
                            {move || if capture_enabled.get() { "Включено" } else { "Выключено" }}
                        </span>
                        <span class="text-muted">
                            {move || if capture_enabled.get() {
                                "Новые API payload будут сохраняться с дедупликацией"
                            } else {
                                "Новые API payload не сохраняются"
                            }}
                        </span>
                    </div>
                </section>

                <section class="raw-storage__section">
                    <h2 class="raw-storage__section-title">"Состояние"</h2>
                    <div class="raw-storage__metrics">
                        <StatTile
                            icon_name="database"
                            label="Строк"
                            value=Signal::derive(move || status.get().map(|s| format_count(s.total_rows)).unwrap_or_else(|| "-".to_string()))
                            warn=Signal::derive(|| false)
                        />
                        <StatTile
                            icon_name="layers"
                            label="Raw JSON"
                            value=Signal::derive(move || status.get().map(|s| format_mb(s.total_mb)).unwrap_or_else(|| "-".to_string()))
                            warn=Signal::derive(|| false)
                        />
                        <StatTile
                            icon_name="link"
                            label="Referenced"
                            value=Signal::derive(move || status.get().map(|s| format_count(s.referenced_rows)).unwrap_or_else(|| "-".to_string()))
                            warn=Signal::derive(|| false)
                        />
                        <StatTile
                            icon_name="alert-triangle"
                            label="Unreferenced"
                            value=Signal::derive(move || status.get().map(|s| format_count(s.unreferenced_rows)).unwrap_or_else(|| "-".to_string()))
                            warn=Signal::derive(move || status.get().map(|s| s.unreferenced_rows > 0).unwrap_or(false))
                        />
                    </div>
                </section>

                <section class="raw-storage__section">
                    <h2 class="raw-storage__section-title">"Очистка"</h2>
                    <div class="raw-storage__list">
                        <div class="raw-storage__list-row">
                            <span class="raw-storage__list-label">"Записи без ссылок на документы"</span>
                            <button class="button button--secondary" disabled=move || action_busy.get() on:click=cleanup_unreferenced>
                                {icon("trash-2")} "Удалить записи без ссылок"
                            </button>
                        </div>
                        <div class="raw-storage__list-row">
                            <span class="raw-storage__list-label">"Точные дубли содержимого"</span>
                            <button class="button button--secondary" disabled=move || action_busy.get() on:click=cleanup_duplicates>
                                {icon("copy")} "Удалить точные дубли"
                            </button>
                        </div>
                        <div class="raw-storage__list-row">
                            <span class="raw-storage__list-label">"Записи старше указанного числа дней"</span>
                            <div class="raw-storage__list-action-group">
                                <label class="raw-storage__inline-label">"Дней"</label>
                                <Input class="raw-storage__days-input" value=older_than_days />
                                <button class="button button--secondary" disabled=move || action_busy.get() on:click=cleanup_old>
                                    {icon("calendar")} "Удалить записи старше срока"
                                </button>
                            </div>
                        </div>
                        <div class="raw-storage__list-row raw-storage__list-row--danger">
                            <span class="raw-storage__list-label">"Всё raw-хранилище целиком"</span>
                            <button class="button button--danger" disabled=move || action_busy.get() on:click=cleanup_all>
                                {icon("trash")} "Удалить всё raw-хранилище"
                            </button>
                        </div>
                    </div>
                </section>

                <section class="raw-storage__section">
                    <h2 class="raw-storage__section-title">"Обслуживание БД (VACUUM)"</h2>
                    <p class="raw-storage__section-hint">
                        "Удаление строк освобождает место внутри файла БД, но не уменьшает сам файл на диске — \
                         SQLite просто помечает страницы как свободные и переиспользует их для новых записей. \
                         VACUUM пересобирает файл целиком и физически возвращает это место операционной системе. \
                         Затрагивает всю базу (не только Raw JSON) и ненадолго блокирует запись, поэтому это ручное \
                         действие для тихого окна, а не часть обычной очистки."
                    </p>
                    <div class="raw-storage__list">
                        <div class="raw-storage__list-row">
                            <span class="raw-storage__list-label">"Файл БД"</span>
                            <span class="raw-storage__list-value">
                                {move || vacuum_status.get().map(|s| format_mb(s.file_mb)).unwrap_or_else(|| "-".to_string())}
                            </span>
                        </div>
                        <div class="raw-storage__list-row">
                            <span class="raw-storage__list-label">"Освободит VACUUM"</span>
                            <span
                                class="raw-storage__list-value"
                                class:raw-storage__list-value--warn=move || vacuum_status.get().map(|s| s.reclaimable_mb > 0.0).unwrap_or(false)
                            >
                                {move || vacuum_status.get().map(|s| format_mb(s.reclaimable_mb)).unwrap_or_else(|| "-".to_string())}
                            </span>
                        </div>
                        <div class="raw-storage__list-row">
                            <span class="raw-storage__list-label">"Файл WAL"</span>
                            <span
                                class="raw-storage__list-value"
                                class:raw-storage__list-value--warn=move || vacuum_status.get().map(|s| s.wal_mb > 0.0).unwrap_or(false)
                            >
                                {move || vacuum_status.get().map(|s| format_mb(s.wal_mb)).unwrap_or_else(|| "-".to_string())}
                            </span>
                        </div>
                        <div class="raw-storage__list-row">
                            <span class="raw-storage__list-label">"Пересобрать файл базы данных"</span>
                            <button class="button button--warning" disabled=move || action_busy.get() on:click=run_vacuum>
                                {icon("shield-check")} "Выполнить VACUUM базы данных"
                            </button>
                        </div>
                        <div class="raw-storage__list-row">
                            <span class="raw-storage__list-label">"Перенести WAL в основной файл БД и усечь журнал"</span>
                            <button class="button button--secondary" disabled=move || action_busy.get() on:click=truncate_wal>
                                {icon("database")} "Очистить WAL"
                            </button>
                        </div>
                    </div>
                </section>

                <section class="raw-storage__section">
                    <h2 class="raw-storage__section-title">"По типам"</h2>
                    <div class="table-wrapper raw-storage__table-wrap">
                        <table class="table__data table--striped">
                            <thead class="table__head">
                                <tr>
                                    <th class="table__header-cell">"Marketplace"</th>
                                    <th class="table__header-cell">"Тип"</th>
                                    <th class="table__header-cell" style="text-align: right;">"Строк"</th>
                                    <th class="table__header-cell" style="text-align: right;">"Raw JSON"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {move || {
                                    status
                                        .get()
                                        .map(|s| s.by_type)
                                        .unwrap_or_default()
                                        .into_iter()
                                        .map(|item| view! {
                                            <tr class="table__row">
                                                <td class="table__cell">{item.marketplace}</td>
                                                <td class="table__cell">{item.document_type}</td>
                                                <td class="table__cell" style="text-align: right;">{format_count(item.rows)}</td>
                                                <td class="table__cell" style="text-align: right;">{format_mb(item.raw_mb)}</td>
                                            </tr>
                                        })
                                        .collect_view()
                                }}
                            </tbody>
                        </table>
                    </div>
                </section>
            </div>
        </PageFrame>
    }
}

#[component]
fn StatTile(
    icon_name: &'static str,
    label: &'static str,
    value: Signal<String>,
    warn: Signal<bool>,
) -> impl IntoView {
    view! {
        <div class=move || if warn.get() { "stat-card stat-card--warning" } else { "stat-card" }>
            <div class="stat-card__icon">{icon(icon_name)}</div>
            <div class="stat-card__content">
                <span class="stat-card__label">{label}</span>
                <span class="stat-card__value">{move || value.get()}</span>
            </div>
        </div>
    }
}

fn format_count(value: u64) -> String {
    let raw = value.to_string();
    let mut formatted = String::with_capacity(raw.len() + raw.len() / 3);
    for (idx, ch) in raw.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            formatted.push(' ');
        }
        formatted.push(ch);
    }
    formatted.chars().rev().collect()
}

fn format_mb(mb: f64) -> String {
    let bytes = (mb.max(0.0) * 1024.0 * 1024.0).round() as u64;
    format_bytes_compact(bytes)
}

fn format_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{ms} мс")
    } else {
        format!("{:.1} с", ms as f64 / 1000.0)
    }
}
