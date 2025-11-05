use super::api;
use chrono::Utc;
use contracts::domain::common::AggregateId;
use contracts::enums::marketplace_type::MarketplaceType;
use contracts::usecases::u502_import_from_ozon::{
    progress::{ImportProgress, ImportStatus},
    request::{ImportMode, ImportRequest},
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json;
use std::collections::HashMap;

#[component]
pub fn ImportWidget() -> impl IntoView {
    let (connections, set_connections) = signal(Vec::new());
    let (selected_connection, set_selected_connection) = signal(String::new());
    let (is_loading, set_is_loading) = signal(false);
    let (error_msg, set_error_msg) = signal(String::new());
    let (session_id, set_session_id) = signal(None::<String>);
    let (progress, set_progress) = signal(None::<ImportProgress>);
    let (import_a007, set_import_a007) = signal(true);
    let (import_a008, set_import_a008) = signal(false);
    let (import_a009, set_import_a009) = signal(false);
    // Даты периода (по умолчанию вчера)
    let now = Utc::now().date_naive();
    let yesterday = now - chrono::Duration::days(1);
    let (date_from, set_date_from) = signal(yesterday);
    let (date_to, set_date_to) = signal(yesterday);

    // Ключи для localStorage
    const SESSION_KEY: &str = "u502_session_id";
    const PROGRESS_KEY: &str = "u502_progress";

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
    fn load_progress_snapshot() -> Option<ImportProgress> {
        storage()
            .and_then(|s| s.get_item(PROGRESS_KEY).ok().flatten())
            .and_then(|j| serde_json::from_str::<ImportProgress>(&j).ok())
    }

    // Загрузить список подключений и маркетплейсов при монтировании
    Effect::new(move || {
        spawn_local(async move {
            // Загружаем маркетплейсы сначала
            match api::get_marketplaces().await {
                Ok(marketplaces) => {
                    // Создаем маппинг marketplace_id -> marketplace_type
                    let marketplace_type_map: HashMap<String, Option<MarketplaceType>> =
                        marketplaces
                            .into_iter()
                            .map(|mp| {
                                let id = mp.base.id.as_string();
                                let mp_type = mp.marketplace_type;
                                (id, mp_type)
                            })
                            .collect();

                    // Затем загружаем подключения
                    match api::get_connections().await {
                        Ok(conns) => {
                            // Фильтруем подключения по marketplace_type == Ozon
                            let filtered_conns: Vec<_> = conns
                                .into_iter()
                                .filter(|conn| {
                                    marketplace_type_map
                                        .get(&conn.marketplace_id)
                                        .and_then(|mp_type| mp_type.as_ref())
                                        .map(|mp_type| *mp_type == MarketplaceType::Ozon)
                                        .unwrap_or(false)
                                })
                                .collect();

                            if let Some(first) = filtered_conns.first() {
                                set_selected_connection.set(first.to_string_id());
                            }
                            set_connections.set(filtered_conns);
                        }
                        Err(e) => {
                            set_error_msg.set(format!("Ошибка загрузки подключений: {}", e));
                        }
                    }
                }
                Err(e) => {
                    set_error_msg.set(format!("Ошибка загрузки маркетплейсов: {}", e));
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
                            // Если сессия не найдена (404), очищаем устаревшие данные
                            if e.contains("404") {
                                clear_session_storage();
                                set_session_id.set(None);
                                set_progress.set(None);
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
    Effect::new(move || {
        if session_id.get().is_none() {
            if let Some(saved_id) = load_session_id() {
                set_session_id.set(Some(saved_id));
                if let Some(snapshot) = load_progress_snapshot() {
                    set_progress.set(Some(snapshot));
                }
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
            if import_a007.get() {
                targets.push("a007_marketplace_product".to_string());
            }
            if import_a008.get() {
                targets.push("a008_marketplace_sales".to_string());
            }
            if import_a009.get() {
                targets.push("a009_ozon_returns".to_string());
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
                date_from: date_from.get(),
                date_to: date_to.get(),
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
        <div class="import-widget" style="padding: 20px; border: 1px solid #ccc; border-radius: 8px; max-width: 800px; margin: 20px auto; max-height: 80vh; overflow-y: auto;">
            <h2>"u502: Импорт из OZON"</h2>

            // Выбор подключения
            <div style="margin: 20px 0;">
                <label style="display: block; margin-bottom: 8px; font-weight: bold;">
                    "Подключение к маркетплейсу:"
                </label>
                <select
                    style="width: 100%; padding: 8px; border: 1px solid #ddd; border-radius: 4px;"
                    on:change=move |ev| {
                        set_selected_connection.set(event_target_value(&ev));
                    }
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

            // Список агрегатов
            <div style="margin: 20px 0;">
                <label style="display: block; margin-bottom: 8px; font-weight: bold;">
                    "Агрегаты для импорта:"
                </label>
                <div style="padding: 8px; background: #f5f5f5; border-radius: 4px;">
                    <label>
                        <input
                            type="checkbox"
                            prop:checked=move || import_a007.get()
                            on:change=move |ev| { set_import_a007.set(event_target_checked(&ev)); }
                        />
                        " a007_marketplace_product - Товары маркетплейса"
                    </label>
                    <br/>
                    <label>
                        <input
                            type="checkbox"
                            prop:checked=move || import_a008.get()
                            on:change=move |ev| { set_import_a008.set(event_target_checked(&ev)); }
                        />
                        " a008_marketplace_sales - Продажи (фин. транзакции)"
                    </label>
                    <br/>
                    <label>
                        <input
                            type="checkbox"
                            prop:checked=move || import_a009.get()
                            on:change=move |ev| { set_import_a009.set(event_target_checked(&ev)); }
                        />
                        " a009_ozon_returns - Возвраты OZON"
                    </label>
                </div>
                <div style="margin-top: 5px; font-size: 12px; color: #666;">
                    "API: POST /v3/product/list, POST /v3/product/info/list, POST /v3/finance/transaction/list, POST /v1/returns/list"
                </div>
            </div>

            // Период
            <div style="margin: 20px 0;">
                <label style="display: block; margin-bottom: 8px; font-weight: bold;">{"Период:"}</label>
                <div class="form-row">
                    <div class="form-group">
                        <label for="date_from">{"С даты"}</label>
                        <input
                            type="date"
                            id="date_from"
                            prop:value=move || date_from.get().format("%Y-%m-%d").to_string()
                            on:change=move |ev| {
                                let value = event_target_value(&ev);
                                if let Ok(d) = chrono::NaiveDate::parse_from_str(&value, "%Y-%m-%d") {
                                    set_date_from.set(d);
                                }
                            }
                        />
                    </div>
                    <div class="form-group">
                        <label for="date_to">{"По дату"}</label>
                        <input
                            type="date"
                            id="date_to"
                            prop:value=move || date_to.get().format("%Y-%m-%d").to_string()
                            on:change=move |ev| {
                                let value = event_target_value(&ev);
                                if let Ok(d) = chrono::NaiveDate::parse_from_str(&value, "%Y-%m-%d") {
                                    set_date_to.set(d);
                                }
                            }
                        />
                    </div>
                </div>
                <div style="margin-top: 5px; font-size: 12px; color: #666;">{"По умолчанию выбран вчерашний день."}</div>
            </div>

            // Кнопка запуска
            <div style="margin: 20px 0;">
                <button
                    style="padding: 10px 20px; background: #007bff; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 16px;"
                    on:click=on_start_import
                    prop:disabled=move || is_loading.get() || session_id.get().is_some()
                >
                    {move || if is_loading.get() {
                        "Запуск..."
                    } else if session_id.get().is_some() {
                        "Импорт запущен"
                    } else {
                        "Запустить импорт"
                    }}
                </button>
            </div>

            // Ошибки
            {move || {
                let err = error_msg.get();
                if !err.is_empty() {
                    view! {
                        <div style="padding: 10px; background: #fee; border: 1px solid #fcc; border-radius: 4px; color: #c00; margin: 10px 0;">
                            {err}
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }}

            // Прогресс
            {move || {
                if let Some(prog) = progress.get() {
                    view! {
                        <div style="margin-top: 20px; padding: 15px; background: #f9f9f9; border-radius: 8px; border: 1px solid #ddd;">
                            <h3>"Прогресс импорта"</h3>
                            <div style="margin: 10px 0;">
                                <strong>"Session ID: "</strong> {prog.session_id.clone()}
                            </div>
                            <div style="margin: 10px 0;">
                                <strong>"Статус: "</strong>
                                <span style={move || format!("color: {}; font-weight: bold;",
                                    match prog.status {
                                        ImportStatus::Running => "#007bff",
                                        ImportStatus::Completed => "#28a745",
                                        ImportStatus::CompletedWithErrors => "#ffc107",
                                        ImportStatus::Failed => "#dc3545",
                                        ImportStatus::Cancelled => "#6c757d",
                                    }
                                )}>
                                    {format!("{:?}", prog.status)}
                                </span>
                            </div>

                            <div style="margin: 10px 0;">
                                <strong>"Обработано: "</strong> {prog.total_processed} " | "
                                <strong>"Создано: "</strong> {prog.total_inserted} " | "
                                <strong>"Обновлено: "</strong> {prog.total_updated} " | "
                                <strong>"Ошибок: "</strong> {prog.total_errors}
                            </div>
                            <div style="margin: 10px 0; font-size: 12px; color: #666;">
                                <strong>"Последнее обновление: "</strong>
                                {prog.updated_at.to_rfc3339()}
                            </div>

                            // Прогресс по агрегатам
                            <div style="margin-top: 15px;">
                                <h4>"Детали по агрегатам:"</h4>
                                {prog.aggregates.iter().map(|agg| {
                                    let percent = if let Some(total) = agg.total {
                                        if total > 0 {
                                            (agg.processed as f64 / total as f64 * 100.0) as i32
                                        } else {
                                            0
                                        }
                                    } else {
                                        0
                                    };

                                    view! {
                                        <div style="margin: 10px 0; padding: 10px; background: white; border-radius: 4px; border: 1px solid #ddd;">
                                            <div style="font-weight: bold;">
                                                {agg.aggregate_index.clone()} " - " {agg.aggregate_name.clone()}
                                            </div>
                                            <div style="margin: 5px 0;">
                                                {agg.processed} {if let Some(t) = agg.total { format!(" / {}", t) } else { String::new() }}
                                                {if percent > 0 { format!(" ({}%)", percent) } else { String::new() }}
                                            </div>
                                            <div style="background: #e0e0e0; height: 20px; border-radius: 4px; overflow: hidden;">
                                                <div style={format!("width: {}%; height: 100%; background: #007bff; transition: width 0.3s;", percent)}></div>
                                            </div>
                                            {agg.current_item.as_ref().map(|ci| view! {
                                                <div style="margin-top: 5px; font-size: 12px; color: #333;">
                                                    <strong>{"Текущий элемент: "}</strong>{ci.clone()}
                                                </div>
                                            })}
                                            <div style="margin-top: 5px; font-size: 12px; color: #666;">
                                                "Создано: " {agg.inserted} " | Обновлено: " {agg.updated} " | Ошибок: " {agg.errors}
                                            </div>
                                        </div>
                                    }
                                }).collect_view()}
                            </div>

                            // Ошибки
                            {if !prog.errors.is_empty() {
                                view! {
                                    <div style="margin-top: 15px;">
                                        <h4 style="color: #dc3545;">"Ошибки импорта:"</h4>
                                        {prog.errors.iter().map(|err| {
                                            view! {
                                                <div style="margin: 5px 0; padding: 8px; background: #fee; border: 1px solid #fcc; border-radius: 4px; font-size: 12px;">
                                                    <div style="font-weight: bold;">{err.message.clone()}</div>
                                                    {err.details.as_ref().map(|d| view! {
                                                        <div style="color: #666; margin-top: 3px;">{d.clone()}</div>
                                                    })}
                                                </div>
                                            }
                                        }).collect_view()}
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

            // Отображение пути загрузки (перемещено вниз)
            {move || {
                let conn_id = selected_connection.get();
                if !conn_id.is_empty() {
                    if let Some(conn) = connections.get().iter().find(|c| c.to_string_id() == conn_id) {
                        view! {
                            <div style="margin: 20px 0; padding: 10px; background: #e3f2fd; border-radius: 4px; border: 1px solid #90caf9;">
                                <div style="font-weight: bold; margin-bottom: 5px; color: #1976d2;">
                                    "API подключения:"
                                </div>
                                <div style="font-family: monospace; font-size: 12px; color: #555;">
                                    "Client-Id: " {conn.application_id.clone().unwrap_or_else(|| "—".to_string())}
                                </div>
                                <div style="font-family: monospace; font-size: 12px; color: #555;">
                                    "Api-Key: ****"
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }
                } else {
                    view! { <div></div> }.into_any()
                }
            }}

            // Результаты загрузки
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
                            <div style="margin: 10px 0; padding: 10px; background: #e8f5e9; border: 1px solid #c8e6c9; border-radius: 4px;">
                                <div><strong>{"Успех: "}</strong>{prog.completed_at.map(|d| d.to_rfc3339()).unwrap_or_else(|| "—".to_string())}</div>
                                <div><strong>{"Количество элементов: "}</strong>{prog.total_processed}</div>
                                <div><strong>{"Время работы: "}</strong>{elapsed}</div>
                            </div>
                        }.into_any()
                    } else if is_error {
                        view! {
                            <div style="margin: 10px 0; padding: 10px; background: #ffebee; border: 1px solid #ffcdd2; border-radius: 4px;">
                                <div style="font-weight: bold; color: #c62828;">{"Ошибка импорта"}</div>
                                {if let Some(last) = prog.errors.last() {
                                    let details = last.details.clone().unwrap_or_default();
                                    view! { <div><div><strong>{last.message.clone()}</strong></div><div style="font-size: 12px; color: #666;">{details}</div></div> }.into_any()
                                } else {
                                    view! { <div>{"Нет подробностей ошибки"}</div> }.into_any()
                                }}
                                <div style="margin-top: 5px; font-size: 12px; color: #666;">{"Статус: "}{format!("{:?}", prog.status)}</div>
                            </div>
                        }.into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }
                } else { view! { <div></div> }.into_any() }
            }}
        </div>
    }
}
