use super::api;
use chrono::Utc;
use contracts::usecases::u501_import_from_ut::{
    progress::{ImportProgress, ImportStatus},
    request::{ImportMode, ImportRequest},
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json;

#[component]
pub fn ImportWidget() -> impl IntoView {
    let (connections, set_connections) = signal(Vec::new());
    let (selected_connection, set_selected_connection) = signal(String::new());
    let (is_loading, set_is_loading) = signal(false);
    let (error_msg, set_error_msg) = signal(String::new());
    let (session_id, set_session_id) = signal(None::<String>);
    let (progress, set_progress) = signal(None::<ImportProgress>);
    let (import_a002, set_import_a002) = signal(true);
    let (import_a003, set_import_a003) = signal(true);
    let (import_a004, set_import_a004) = signal(true);
    let (import_p901, set_import_p901) = signal(true);
    let (delete_obsolete, set_delete_obsolete) = signal(false);

    // Дополнительные загрузки
    let (import_p906, set_import_p906) = signal(false);

    // Ключи для localStorage
    const SESSION_KEY: &str = "u501_session_id";
    const PROGRESS_KEY: &str = "u501_progress";

    // Вспомогательные функции работы с localStorage
    fn storage() -> Option<web_sys::Storage> {
        web_sys::window().and_then(|w| w.local_storage().ok().flatten())
    }
    fn save_session_id(id: &str) {
        if let Some(s) = storage() {
            let _ = s.set_item(SESSION_KEY, id);
        }
    }
    fn load_session_id() -> Option<String> {
        storage().and_then(|s| s.get_item(SESSION_KEY).ok().flatten())
    }
    fn clear_session_storage() {
        if let Some(s) = storage() {
            let _ = s.remove_item(SESSION_KEY);
            let _ = s.remove_item(PROGRESS_KEY);
        }
    }
    fn save_progress_snapshot(p: &ImportProgress) {
        if let Ok(json) = serde_json::to_string(p) {
            if let Some(s) = storage() {
                let _ = s.set_item(PROGRESS_KEY, &json);
            }
        }
    }
    #[allow(dead_code)]
    fn load_progress_snapshot() -> Option<ImportProgress> {
        storage()
            .and_then(|s| s.get_item(PROGRESS_KEY).ok().flatten())
            .and_then(|j| serde_json::from_str::<ImportProgress>(&j).ok())
    }

    // Загрузить список подключений при монтировании
    Effect::new(move || {
        spawn_local(async move {
            match api::get_connections().await {
                Ok(conns) => {
                    if let Some(first) = conns.first() {
                        set_selected_connection.set(first.to_string_id());
                    }
                    set_connections.set(conns);
                }
                Err(e) => {
                    set_error_msg.set(format!("Ошибка загрузки подключений: {}", e));
                }
            }
        });
    });

    // Polling прогресса
    Effect::new(move || {
        if let Some(sid) = session_id.get() {
            let sid_clone = sid.clone();
            spawn_local(async move {
                loop {
                    match api::get_progress(&sid_clone).await {
                        Ok(prog) => {
                            let is_finished = matches!(
                                prog.status,
                                ImportStatus::Completed
                                    | ImportStatus::CompletedWithErrors
                                    | ImportStatus::Failed
                                    | ImportStatus::Cancelled
                            );
                            save_progress_snapshot(&prog);
                            set_progress.set(Some(prog.clone()));
                            if is_finished {
                                clear_session_storage();
                                set_session_id.set(None);
                                break;
                            }
                        }
                        Err(e) => {
                            // При ошибке (особенно 404) очищаем сессию - она больше не существует
                            if e.contains("404") {
                                clear_session_storage();
                                set_session_id.set(None);
                                set_progress.set(None);
                                // Не показываем ошибку пользователю - просто сбрасываем состояние
                            } else {
                                set_error_msg.set(format!("Ошибка получения прогресса: {}", e));
                            }
                            break;
                        }
                    }
                    // Пауза 2 секунды
                    gloo_timers::future::TimeoutFuture::new(2000).await;
                }
            });
        }
    });

    // Восстановить сессию и последний прогресс при монтировании
    // Сначала проверяем, существует ли сессия на сервере
    Effect::new(move || {
        if session_id.get().is_none() {
            if let Some(saved_id) = load_session_id() {
                // Пробуем проверить сессию на сервере
                let saved_id_clone = saved_id.clone();
                spawn_local(async move {
                    match api::get_progress(&saved_id_clone).await {
                        Ok(prog) => {
                            // Сессия существует, восстанавливаем
                            set_session_id.set(Some(saved_id_clone));
                            set_progress.set(Some(prog));
                        }
                        Err(_) => {
                            // Сессия не существует на сервере, очищаем localStorage
                            clear_session_storage();
                        }
                    }
                });
            }
        }
    });

    let on_start_import = move |_| {
        let conn_id = selected_connection.get();
        if conn_id.is_empty() {
            set_error_msg.set("Выберите подключение".to_string());
            return;
        }

        set_is_loading.set(true);
        set_error_msg.set(String::new());
        set_progress.set(None);

        spawn_local(async move {
            let mut targets: Vec<String> = Vec::new();
            if import_a002.get() {
                targets.push("a002_organization".to_string());
            }
            if import_a003.get() {
                targets.push("a003_counterparty".to_string());
            }
            if import_a004.get() {
                targets.push("a004_nomenclature".to_string());
            }
            if import_p901.get() {
                targets.push("p901_barcodes".to_string());
            }
            // Дополнительные загрузки
            if import_p906.get() {
                targets.push("p906_prices".to_string());
            }

            if targets.is_empty() {
                set_error_msg.set("Выберите агрегаты для импорта".to_string());
                set_is_loading.set(false);
                return;
            }

            let request = ImportRequest {
                connection_id: conn_id,
                target_aggregates: targets,
                mode: ImportMode::Interactive,
                delete_obsolete: delete_obsolete.get(),
                period_from: None,
                period_to: None,
            };

            match api::start_import(request).await {
                Ok(response) => {
                    set_session_id.set(Some(response.session_id));
                    if let Some(id) = session_id.get() {
                        save_session_id(&id);
                    }
                    set_is_loading.set(false);
                }
                Err(e) => {
                    set_error_msg.set(format!("Ошибка запуска импорта: {}", e));
                    set_is_loading.set(false);
                }
            }
        });
    };

    view! {
        <div class="w-full flex justify-center px-6 py-8">
            <div class="w-full max-w-5xl space-y-6">
                <div class="rounded-lg border border-[var(--color-border-light)] bg-[var(--color-bg-secondary)] p-6">
                    <div class="mb-4 text-[var(--color-primary)] text-base font-semibold">
                        "Подключение к 1С:"
                    </div>
                    <select
                        class="w-full h-10 rounded-md border border-[var(--color-border)] bg-[var(--color-bg-body)] px-3 text-sm text-[var(--color-text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--color-primary-200)] focus:border-[var(--color-primary-500)] disabled:opacity-60"
                        prop:value=move || selected_connection.get()
                        on:change=move |ev| { set_selected_connection.set(event_target_value(&ev)); }
                        prop:disabled=move || is_loading.get()
                    >
                        {move || connections.get().into_iter().map(|conn| {
                            let id = conn.to_string_id();
                            let id_clone = id.clone();
                            let desc = conn.base.description.clone();
                            view! {
                                <option value={id}>
                                    {desc} " (" {id_clone} ")"
                                </option>
                            }
                        }).collect_view()}
                    </select>
                </div>

                <div class="rounded-lg border border-[var(--color-border-light)] bg-[var(--color-bg-secondary)] p-6">
                    <div class="mb-4 text-[var(--color-primary)] text-base font-semibold">
                        "Агрегаты для импорта:"
                    </div>
                    <div class="space-y-3">
                        <label class="flex items-center gap-3 text-sm text-[var(--color-text-primary)]">
                            <input
                                class="h-4 w-4 accent-[var(--color-primary-500)]"
                                type="checkbox"
                                prop:checked=move || import_a002.get()
                                on:change=move |ev| { set_import_a002.set(event_target_checked(&ev)); }
                            />
                            <span>"Catalog_Организации"</span>
                        </label>
                        <label class="flex items-center gap-3 text-sm text-[var(--color-text-primary)]">
                            <input
                                class="h-4 w-4 accent-[var(--color-primary-500)]"
                                type="checkbox"
                                prop:checked=move || import_a003.get()
                                on:change=move |ev| { set_import_a003.set(event_target_checked(&ev)); }
                            />
                            <span>"Catalog_Контрагенты"</span>
                        </label>
                        <label class="flex items-center gap-3 text-sm text-[var(--color-text-primary)]">
                            <input
                                class="h-4 w-4 accent-[var(--color-primary-500)]"
                                type="checkbox"
                                prop:checked=move || import_a004.get()
                                on:change=move |ev| { set_import_a004.set(event_target_checked(&ev)); }
                            />
                            <span>"Catalog_Номенклатура"</span>
                        </label>
                        <label class="flex items-center gap-3 text-sm text-[var(--color-text-primary)]">
                            <input
                                class="h-4 w-4 accent-[var(--color-primary-500)]"
                                type="checkbox"
                                prop:checked=move || import_p901.get()
                                on:change=move |ev| { set_import_p901.set(event_target_checked(&ev)); }
                            />
                            <span>"InformationRegister_ШтрихкодыНоменклатуры"</span>
                        </label>
                    </div>
                    <div class="mt-4 text-xs text-[var(--color-text-muted)]">
                        "OData коллекции: Catalog_Организации, Catalog_Контрагенты, Catalog_Номенклатура, InformationRegister_ШтрихкодыНоменклатуры"
                    </div>
                </div>

                <div class="rounded-lg border border-[var(--color-border-light)] bg-[var(--color-bg-secondary)] p-6">
                    <div class="mb-4 text-[var(--color-primary)] text-base font-semibold">
                        "Дополнительные загрузки:"
                    </div>
                    <label class="flex items-center gap-3 text-sm text-[var(--color-text-primary)]">
                        <input
                            class="h-4 w-4 accent-[var(--color-primary-500)]"
                            type="checkbox"
                            prop:checked=move || import_p906.get()
                            on:change=move |ev| { set_import_p906.set(event_target_checked(&ev)); }
                        />
                        <span>"p906_prices - Плановые цены номенклатуры"</span>
                    </label>
                    <div class="mt-2 text-xs text-[var(--color-text-muted)]">
                        "HTTP: /hs/mpi_api/prices_plan"
                    </div>
                </div>

                <div class="rounded-lg border border-[var(--color-border-light)] bg-[var(--color-bg-secondary)] p-6">
                    <div class="mb-4 text-[var(--color-primary)] text-base font-semibold">
                        "Опции импорта:"
                    </div>
                    <label class="flex items-center gap-3 text-sm text-[var(--color-text-primary)]">
                        <input
                            class="h-4 w-4 accent-[var(--color-primary-500)]"
                            type="checkbox"
                            prop:checked=move || delete_obsolete.get()
                            on:change=move |ev| { set_delete_obsolete.set(event_target_checked(&ev)); }
                        />
                        <span>"Удалять устаревшие записи (которых нет в 1С)"</span>
                    </label>
                    <div class="mt-4 rounded-md border border-[var(--color-warning-100)] bg-[var(--color-warning-50)] px-4 py-3 text-sm text-[var(--color-text-primary)]">
                        <span class="font-semibold">"⚠️ Внимание: "</span>
                        "Записи, которых нет в источнике 1С, будут удалены из БД (жесткое удаление)"
                    </div>
                </div>

                <div class="rounded-lg border border-[var(--color-border-light)] bg-[var(--color-bg-secondary)] p-6 text-center">
                    <button
                        class="inline-flex items-center justify-center gap-3 rounded-lg bg-[var(--color-primary-600)] px-10 py-4 text-base font-semibold text-white shadow-sm disabled:opacity-60 disabled:cursor-not-allowed"
                        on:click=on_start_import
                        prop:disabled=move || is_loading.get() || session_id.get().is_some()
                    >
                        <span class="text-lg">"▶"</span>
                        <span>
                            {move || if is_loading.get() {
                                "Запуск..."
                            } else if session_id.get().is_some() {
                                "Импорт запущен"
                            } else {
                                "Запустить импорт"
                            }}
                        </span>
                    </button>
                </div>

                {move || {
                    let err = error_msg.get();
                    if !err.is_empty() {
                        view! {
                            <div class="rounded-lg border border-[var(--color-error-100)] bg-[var(--color-error-50)] px-4 py-3 text-sm text-[var(--color-error-700)]">
                                {err}
                            </div>
                        }.into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }
                }}

                {move || {
                    if let Some(prog) = progress.get() {
                        view! {
                            <div class="rounded-lg border border-[var(--color-border-light)] bg-[var(--color-bg-secondary)] p-6 space-y-4">
                                <div class="text-lg font-semibold text-[var(--color-text-primary)]">"Прогресс импорта"</div>

                                <div class="grid grid-cols-[140px_1fr] gap-x-4 gap-y-2 text-sm">
                                    <div class="text-[var(--color-text-secondary)] font-medium">"Session ID:"</div>
                                    <div class="text-[var(--color-text-primary)]">{prog.session_id.clone()}</div>

                                    <div class="text-[var(--color-text-secondary)] font-medium">"Статус:"</div>
                                    <div class="text-[var(--color-text-primary)]">
                                        {format!("{:?}", prog.status)}
                                    </div>

                                    <div class="text-[var(--color-text-secondary)] font-medium">"Обработано:"</div>
                                    <div class="text-[var(--color-text-primary)]">
                                        {prog.total_processed} " | Создано: " {prog.total_inserted} " | Обновлено: " {prog.total_updated} " | Ошибок: " {prog.total_errors}
                                    </div>

                                    <div class="text-[var(--color-text-secondary)] font-medium">"Обновление:"</div>
                                    <div class="text-xs text-[var(--color-text-muted)]">{prog.updated_at.to_rfc3339()}</div>
                                </div>

                                <div class="pt-2">
                                    <div class="text-sm font-semibold text-[var(--color-text-primary)] mb-3">"Детали по агрегатам:"</div>
                                    <div class="space-y-3">
                                        {prog.aggregates.iter().map(|agg| {
                                            let percent = if let Some(total) = agg.total {
                                                if total > 0 {
                                                    (agg.processed as f64 / total as f64 * 100.0) as i32
                                                } else { 0 }
                                            } else { 0 };

                                            view! {
                                                <div class="rounded-md border border-[var(--color-border-light)] bg-[var(--color-bg-body)] p-4">
                                                    <div class="font-semibold text-[var(--color-text-primary)]">
                                                        {agg.aggregate_index.clone()} " - " {agg.aggregate_name.clone()}
                                                    </div>
                                                    <div class="mt-1 text-sm text-[var(--color-text-secondary)]">
                                                        {agg.processed}
                                                        {if let Some(t) = agg.total { format!(" / {}", t) } else { String::new() }}
                                                        {if percent > 0 { format!(" ({}%)", percent) } else { String::new() }}
                                                    </div>
                                                    <div class="mt-2 h-4 rounded bg-[var(--color-neutral-200)] overflow-hidden">
                                                        <div class="h-full bg-[var(--color-primary-500)] transition-[width] duration-300" style={format!("width: {}%;", percent)}></div>
                                                    </div>
                                                    {agg.current_item.as_ref().map(|ci| view! {
                                                        <div class="mt-2 text-xs text-[var(--color-text-muted)]">
                                                            <span class="font-semibold">{"Текущий элемент: "}</span>{ci.clone()}
                                                        </div>
                                                    })}
                                                    <div class="mt-2 text-xs text-[var(--color-text-muted)]">
                                                        "Создано: " {agg.inserted} " | Обновлено: " {agg.updated} " | Пропущено: " {agg.skipped} " | Ошибок: " {agg.errors}
                                                    </div>
                                                    {agg.info.as_ref().map(|info| view! {
                                                        <div class="mt-1 text-xs italic text-[var(--color-text-muted)]">
                                                            {info.clone()}
                                                        </div>
                                                    })}
                                                </div>
                                            }
                                        }).collect_view()}
                                    </div>
                                </div>

                                {if !prog.errors.is_empty() {
                                    view! {
                                        <div class="pt-2">
                                            <div class="text-sm font-semibold text-[var(--color-error-700)] mb-2">"Ошибки импорта:"</div>
                                            <div class="space-y-2">
                                                {prog.errors.iter().map(|err| {
                                                    view! {
                                                        <div class="rounded-md border border-[var(--color-error-100)] bg-[var(--color-error-50)] px-4 py-3 text-sm text-[var(--color-error-700)]">
                                                            <div class="font-semibold">{err.message.clone()}</div>
                                                            {err.details.as_ref().map(|d| view! {
                                                                <div class="mt-1 text-xs text-[var(--color-text-muted)]">{d.clone()}</div>
                                                            })}
                                                        </div>
                                                    }
                                                }).collect_view()}
                                            </div>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <div></div> }.into_any()
                                }}
                            </div>
                        }.into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }
                }}

                {move || {
                    let conn_id = selected_connection.get();
                    if !conn_id.is_empty() {
                        if let Some(conn) = connections.get().iter().find(|c| c.to_string_id() == conn_id) {
                            let base_url = conn.url.trim_end_matches('/');
                            let odata_path = if base_url.contains("/odata/") {
                                base_url.to_string()
                            } else {
                                format!("{}/odata/standard.odata", base_url)
                            };
                            let mut endpoints: Vec<String> = Vec::new();
                            if import_a002.get() { endpoints.push(format!("{}/Catalog_Организации", odata_path)); }
                            if import_a003.get() { endpoints.push(format!("{}/Catalog_Контрагенты", odata_path)); }

                            view! {
                                <div class="rounded-lg border border-[var(--color-primary-100)] bg-[var(--color-primary-50)] p-6 space-y-2">
                                    <div class="text-[var(--color-primary-700)] text-base font-semibold">
                                        "Путь загрузки:"
                                    </div>
                                    {endpoints.iter().map(|e| {
                                        let e_clone = e.clone();
                                        view! {
                                            <div class="font-mono text-xs text-[var(--color-text-secondary)] break-all">
                                                {e_clone}
                                            </div>
                                        }
                                    }).collect_view()}
                                </div>
                            }.into_any()
                        } else {
                            view! { <div></div> }.into_any()
                        }
                    } else {
                        view! { <div></div> }.into_any()
                    }
                }}

                {move || {
                    if let Some(prog) = progress.get() {
                        let is_success = matches!(prog.status, ImportStatus::Completed);
                        let is_error = matches!(prog.status, ImportStatus::Failed | ImportStatus::CompletedWithErrors);
                        let end = prog.completed_at.unwrap_or_else(Utc::now);
                        let secs = (end - prog.started_at).num_seconds();
                        let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
                        let elapsed = format!("{:02}:{:02}:{:02}", h, m, s);

                        if is_success {
                            view! {
                                <div class="rounded-lg border border-[var(--color-success-100)] bg-[var(--color-success-50)] p-6 text-[var(--color-success-700)]">
                                    <div><span class="font-semibold">{"Успех: "}</span>{prog.completed_at.map(|d| d.to_rfc3339()).unwrap_or_else(|| "—".to_string())}</div>
                                    <div><span class="font-semibold">{"Количество элементов: "}</span>{prog.total_processed}</div>
                                    <div><span class="font-semibold">{"Время работы: "}</span>{elapsed}</div>
                                </div>
                            }.into_any()
                        } else if is_error {
                            view! {
                                <div class="rounded-lg border border-[var(--color-error-100)] bg-[var(--color-error-50)] p-6 text-[var(--color-error-700)]">
                                    <div class="font-semibold">{"Ошибка импорта"}</div>
                                    {if let Some(last) = prog.errors.last() {
                                        let details = last.details.clone().unwrap_or_default();
                                        view! { <div class="mt-2"><div class="font-semibold">{last.message.clone()}</div><div class="mt-1 text-xs text-[var(--color-text-muted)]">{details}</div></div> }.into_any()
                                    } else {
                                        view! { <div class="mt-2">{"Нет подробностей ошибки"}</div> }.into_any()
                                    }}
                                    <div class="mt-2 text-xs text-[var(--color-text-muted)]">{"Статус: "}{format!("{:?}", prog.status)}</div>
                                </div>
                            }.into_any()
                        } else {
                            view! { <div></div> }.into_any()
                        }
                    } else {
                        view! { <div></div> }.into_any()
                    }
                }}
            </div>
        </div>
    }
}
