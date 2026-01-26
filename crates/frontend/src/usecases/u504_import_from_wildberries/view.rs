use super::api;
use chrono::{Duration, NaiveDate, Utc};
use contracts::domain::common::AggregateId;
use contracts::enums::marketplace_type::MarketplaceType;
use contracts::usecases::u504_import_from_wildberries::{
    progress::{ImportProgress, ImportStatus},
    request::{ImportMode, ImportRequest},
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json;
use std::collections::HashMap;
use thaw::*;

#[component]
pub fn ImportWidget() -> impl IntoView {
    let (connections, set_connections) = signal(Vec::new());
    let (selected_connection, set_selected_connection) = signal(String::new());
    let (is_loading, set_is_loading) = signal(false);
    let (error_msg, set_error_msg) = signal(String::new());
    let (session_id, set_session_id) = signal(None::<String>);
    let (progress, set_progress) = signal(None::<ImportProgress>);
    let import_a007 = RwSignal::new(true);
    let import_a015 = RwSignal::new(false);
    let import_a012 = RwSignal::new(false);
    let import_p903 = RwSignal::new(false);
    let import_p905 = RwSignal::new(false);

    // Даты для импорта (по умолчанию: последние 30 дней)
    let default_date_from = Utc::now().naive_utc().date() - Duration::days(30);
    let default_date_to = Utc::now().naive_utc().date();
    let (date_from, set_date_from) = signal(default_date_from.format("%Y-%m-%d").to_string());
    let (date_to, set_date_to) = signal(default_date_to.format("%Y-%m-%d").to_string());

    // Ключи для localStorage
    const SESSION_KEY: &str = "u504_session_id";
    const PROGRESS_KEY: &str = "u504_progress";

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
                            // Фильтруем подключения по marketplace_type == Wildberries
                            let filtered_conns: Vec<_> = conns
                                .into_iter()
                                .filter(|conn| {
                                    marketplace_type_map
                                        .get(&conn.marketplace_id)
                                        .and_then(|mp_type| mp_type.as_ref())
                                        .map(|mp_type| *mp_type == MarketplaceType::Wildberries)
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
            if import_a007.get_untracked() {
                targets.push("a007_marketplace_product".to_string());
            }
            if import_a015.get_untracked() {
                targets.push("a015_wb_orders".to_string());
            }
            if import_a012.get_untracked() {
                targets.push("a012_wb_sales".to_string());
            }
            if import_p903.get_untracked() {
                targets.push("p903_wb_finance_report".to_string());
            }
            if import_p905.get_untracked() {
                targets.push("p905_wb_commission_history".to_string());
            }

            if targets.is_empty() {
                set_error_msg.set("Выберите агрегаты для импорта".to_string());
                set_is_loading.set(false);
                return;
            }

            // Парсим даты
            let parsed_date_from = match NaiveDate::parse_from_str(&date_from.get(), "%Y-%m-%d") {
                Ok(d) => d,
                Err(_) => {
                    set_error_msg.set("Неверный формат даты начала".to_string());
                    set_is_loading.set(false);
                    return;
                }
            };

            let parsed_date_to = match NaiveDate::parse_from_str(&date_to.get(), "%Y-%m-%d") {
                Ok(d) => d,
                Err(_) => {
                    set_error_msg.set("Неверный формат даты окончания".to_string());
                    set_is_loading.set(false);
                    return;
                }
            };

            let request = ImportRequest {
                connection_id: conn_id,
                target_aggregates: targets,
                date_from: parsed_date_from,
                date_to: parsed_date_to,
                mode: ImportMode::Interactive,
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
        <div style="padding: 20px; max-width: 900px; margin: 0 auto;">
            <h1 style="font-size: 24px; font-weight: bold; margin-bottom: 20px;">
                "u504: Импорт из Wildberries"
            </h1>

            <Space vertical=true>
                // Выбор подключения
                <div style="padding: 16px; background: var(--color-background-secondary); border-radius: 8px;">
                    <label style="display: block; margin-bottom: 8px; font-weight: 600; font-size: 14px;">
                        "Подключение к маркетплейсу:"
                    </label>
                    <select
                        style="width: 100%; padding: 10px; border: 1px solid var(--color-border); border-radius: 6px; font-size: 14px; color: var(--colorNeutralForeground1); background: var(--color-background-primary);"
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
                                <option value={id} style = "color: var(--colorNeutralForeground1); background: var(--color-background-primary);">
                                    {desc} " (" {id_clone} ")"
                                </option>
                            }
                        }).collect_view()}
                    </select>
                </div>

                // Период импорта
                <div style="padding: 16px; background: var(--color-background-secondary); border-radius: 8px;">
                    <label style="display: block; margin-bottom: 12px; font-weight: 600; font-size: 14px;">{"Период импорта:"}</label>
                    <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 12px;">
                        <div class="form__group">
                            <label for="date_from" style="display: block; margin-bottom: 6px; font-size: 13px;">{"С даты"}</label>
                            <input
                                type="date"
                                id="date_from"
                                prop:value=move || date_from.get()
                                on:change=move |ev| {
                                    set_date_from.set(event_target_value(&ev));
                                }
                                style="width: 100%; padding: 8px; border: 1px solid var(--color-border); border-radius: 6px; font-size: 14px;"
                            />
                        </div>
                        <div class="form__group">
                            <label for="date_to" style="display: block; margin-bottom: 6px; font-size: 13px;">{"По дату"}</label>
                            <input
                                type="date"
                                id="date_to"
                                prop:value=move || date_to.get()
                                on:change=move |ev| {
                                    set_date_to.set(event_target_value(&ev));
                                }
                                style="width: 100%; padding: 8px; border: 1px solid var(--color-border); border-radius: 6px; font-size: 14px;"
                            />
                        </div>
                    </div>
                    <div style="margin-top: 8px; font-size: 11px; color: var(--color-text-secondary);">
                        "Период используется для импорта заказов (a015_wb_orders) и продаж (a012_wb_sales)"
                    </div>
                </div>

                // Список агрегатов
                <div style="padding: 16px; background: var(--color-background-secondary); border-radius: 8px;">
                    <label style="display: block; margin-bottom: 12px; font-weight: 600; font-size: 14px;">
                        "Агрегаты для импорта:"
                    </label>
                    <Space vertical=true>
                        <Checkbox checked=import_a007 label="a007_marketplace_product - Товары маркетплейса"/>
                        <Checkbox checked=import_a015 label="a015_wb_orders - Заказы Wildberries"/>
                        <Checkbox checked=import_a012 label="a012_wb_sales - Продажи Wildberries"/>
                        <Checkbox checked=import_p903 label="p903_wb_finance_report - Финансовый отчет WB"/>
                        <Checkbox checked=import_p905 label="p905_wb_commission_history - История комиссий WB"/>
                    </Space>
                    <div style="margin-top: 10px; font-size: 11px; color: var(--color-text-secondary);">
                        "API: POST /content/v2/get/cards/list (товары), GET /api/v1/supplier/orders (заказы), GET /api/v1/supplier/sales (продажи), GET /api/v5/supplier/reportDetailByPeriod (финансы)"
                    </div>
                </div>

                // Кнопки управления
                <div>
                    <Button
                        appearance=ButtonAppearance::Primary
                        on_click=on_start_import
                        disabled=Signal::derive(move || is_loading.get() || session_id.get().is_some())
                    >
                        {move || if is_loading.get() {
                            "⏳ Запуск..."
                        } else if session_id.get().is_some() {
                            "✓ Импорт запущен"
                        } else {
                            "▶ Запустить импорт"
                        }}
                    </Button>
                </div>

                // Ошибки
                {move || {
                    let err = error_msg.get();
                    if !err.is_empty() {
                        view! {
                            <div style="padding: 12px; background: var(--color-error-50); border: 1px solid var(--color-error-100); border-radius: 8px; color: var(--color-error); display: flex; align-items: center; gap: 8px;">
                                <span style="font-size: 18px;">"⚠"</span>
                                <span>{err}</span>
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
                            <div style="padding: 16px; background: var(--color-background-secondary); border-radius: 8px; border: 1px solid var(--color-border);">
                                <h3 style="margin: 0 0 12px 0; font-size: 16px; font-weight: 600;">"Прогресс импорта"</h3>
                                <div style="margin: 8px 0; font-size: 13px;">
                                    <strong>"Session ID: "</strong>
                                    <span style="font-family: monospace; font-size: 11px;">{prog.session_id.clone()}</span>
                                </div>
                                <div style="margin: 8px 0;">
                                    <strong>"Статус: "</strong>
                                    <span style={move || format!("color: {}; font-weight: bold; padding: 4px 8px; border-radius: 4px; background: {}; font-size: 13px;",
                                        match prog.status {
                                            ImportStatus::Running => "#0078d4",
                                            ImportStatus::Completed => "#107c10",
                                            ImportStatus::CompletedWithErrors => "#ca5010",
                                            ImportStatus::Failed => "#d13438",
                                            ImportStatus::Cancelled => "#605e5c",
                                        },
                                        match prog.status {
                                            ImportStatus::Running => "rgba(0, 120, 212, 0.1)",
                                            ImportStatus::Completed => "rgba(16, 124, 16, 0.1)",
                                            ImportStatus::CompletedWithErrors => "rgba(202, 80, 16, 0.1)",
                                            ImportStatus::Failed => "rgba(209, 52, 56, 0.1)",
                                            ImportStatus::Cancelled => "rgba(96, 94, 92, 0.1)",
                                        }
                                    )}>
                                        {format!("{:?}", prog.status)}
                                    </span>
                                </div>

                                <div style="margin: 12px 0; padding: 10px; background: var(--color-background-primary); border-radius: 6px; font-size: 13px;">
                                    <strong>"Обработано: "</strong> {prog.total_processed} " | "
                                    <strong>"Создано: "</strong> {prog.total_inserted} " | "
                                    <strong>"Обновлено: "</strong> {prog.total_updated} " | "
                                    <strong>"Ошибок: "</strong> {prog.total_errors}
                                </div>
                                <div style="margin: 8px 0; font-size: 11px; color: var(--color-text-secondary);">
                                    <strong>"Последнее обновление: "</strong>
                                    {prog.updated_at.to_rfc3339()}
                                </div>

                                // Прогресс по агрегатам
                                <div style="margin-top: 12px;">
                                    <h4 style="margin: 0 0 10px 0; font-size: 14px; font-weight: 600;">"Детали по агрегатам:"</h4>
                                    <Space vertical=true>
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
                                                <div style="padding: 12px; background: var(--color-background-primary); border-radius: 6px; border: 1px solid var(--color-border);">
                                                    <div style="font-weight: 600; font-size: 13px; margin-bottom: 6px;">
                                                        {agg.aggregate_index.clone()} " - " {agg.aggregate_name.clone()}
                                                    </div>
                                                    <div style="margin: 6px 0; font-size: 13px;">
                                                        {agg.processed} {if let Some(t) = agg.total { format!(" / {}", t) } else { String::new() }}
                                                        {if percent > 0 { format!(" ({}%)", percent) } else { String::new() }}
                                                    </div>
                                                    <div style="background: var(--color-border); height: 16px; border-radius: 4px; overflow: hidden;">
                                                        <div style={format!("width: {}%; height: 100%; background: var(--colorBrandForeground1); transition: width 0.3s;", percent)}></div>
                                                    </div>
                                                    {agg.current_item.as_ref().map(|ci| view! {
                                                        <div style="margin-top: 6px; font-size: 11px; color: var(--color-text-secondary);">
                                                            <strong>{"Текущий элемент: "}</strong>{ci.clone()}
                                                        </div>
                                                    })}
                                                    <div style="margin-top: 6px; font-size: 11px; color: var(--color-text-secondary);">
                                                        "Создано: " {agg.inserted} " | Обновлено: " {agg.updated} " | Ошибок: " {agg.errors}
                                                    </div>
                                                </div>
                                            }
                                        }).collect_view()}
                                    </Space>
                                </div>

                                // Ошибки
                                {if !prog.errors.is_empty() {
                                    view! {
                                        <div style="margin-top: 12px;">
                                            <h4 style="margin: 0 0 10px 0; font-size: 14px; font-weight: 600; color: var(--color-error);">"Ошибки импорта:"</h4>
                                            <Space vertical=true>
                                                {prog.errors.iter().map(|err| {
                                                    view! {
                                                        <div style="padding: 10px; background: var(--color-error-50); border-left: 3px solid var(--color-error); border-radius: 4px; font-size: 12px;">
                                                            <div style="font-weight: 600; color: var(--color-error);">{err.message.clone()}</div>
                                                            {err.details.as_ref().map(|d| view! {
                                                                <div style="color: var(--color-text-secondary); margin-top: 4px; font-size: 11px;">{d.clone()}</div>
                                                            })}
                                                        </div>
                                                    }
                                                }).collect_view()}
                                            </Space>
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

                // Отображение пути загрузки
                {move || {
                    let conn_id = selected_connection.get();
                    if !conn_id.is_empty() {
                        if let Some(_conn) = connections.get().iter().find(|c| c.to_string_id() == conn_id) {
                            view! {
                                <div style="padding: 12px; background: rgba(0, 120, 212, 0.1); border-radius: 6px; border-left: 3px solid #0078d4;">
                                    <div style="font-weight: 600; margin-bottom: 6px; color: #0078d4; font-size: 13px;">
                                        "API подключения:"
                                    </div>
                                    <div style="font-family: monospace; font-size: 11px; color: var(--color-text-primary);">
                                        "Authorization: ****"
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
                                <div style="padding: 12px; background: rgba(16, 124, 16, 0.1); border-left: 3px solid #107c10; border-radius: 6px;">
                                    <div style="font-weight: 600; font-size: 14px; color: #107c10; margin-bottom: 8px;">{"✓ Импорт успешно завершен"}</div>
                                    <div style="font-size: 13px; margin: 4px 0;"><strong>{"Завершено: "}</strong>{prog.completed_at.map(|d| d.to_rfc3339()).unwrap_or_else(|| "—".to_string())}</div>
                                    <div style="font-size: 13px; margin: 4px 0;"><strong>{"Обработано элементов: "}</strong>{prog.total_processed}</div>
                                    <div style="font-size: 13px; margin: 4px 0;"><strong>{"Время работы: "}</strong>{elapsed}</div>
                                </div>
                            }.into_any()
                        } else if is_error {
                            view! {
                                <div style="padding: 12px; background: var(--color-error-50); border-left: 3px solid var(--color-error); border-radius: 6px;">
                                    <div style="font-weight: 600; font-size: 14px; color: var(--color-error); margin-bottom: 8px;">{"✗ Ошибка импорта"}</div>
                                    {if let Some(last) = prog.errors.last() {
                                        let details = last.details.clone().unwrap_or_default();
                                        view! {
                                            <div>
                                                <div style="font-weight: 600; font-size: 13px; margin: 4px 0; color: var(--color-error);">{last.message.clone()}</div>
                                                {if !details.is_empty() {
                                                    view! { <div style="font-size: 11px; color: var(--color-text-secondary); margin-top: 4px;">{details}</div> }.into_any()
                                                } else {
                                                    view! { <div></div> }.into_any()
                                                }}
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! { <div style="font-size: 13px;">{"Нет подробностей ошибки"}</div> }.into_any()
                                    }}
                                    <div style="margin-top: 6px; font-size: 11px; color: var(--color-text-secondary);">{"Статус: "}{format!("{:?}", prog.status)}</div>
                                </div>
                            }.into_any()
                        } else {
                            view! { <div></div> }.into_any()
                        }
                    } else { view! { <div></div> }.into_any() }
                }}
            </Space>
        </div>
    }
}
