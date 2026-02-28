use super::api;
use chrono::{Datelike, Utc};
use contracts::usecases::u501_import_from_ut::{
    progress::{ImportProgress, ImportStatus},
    request::{ImportMode, ImportRequest},
};
use crate::shared::page_frame::PageFrame;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json;
use thaw::*;

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
    let (import_a022, set_import_a022) = signal(true);
    let (import_a023, set_import_a023) = signal(false);

    // Период для a023_purchase_of_goods
    let today = chrono::Utc::now().date_naive();
    let month_start = chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
        .unwrap_or(today);
    let (period_from, set_period_from) = signal(Some(month_start.format("%Y-%m-%d").to_string()));
    let (period_to, set_period_to) = signal(Some(today.format("%Y-%m-%d").to_string()));

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
            if import_a022.get() {
                targets.push("a022_kit_variant".to_string());
            }
            // Дополнительные загрузки
            if import_p906.get() {
                targets.push("p906_prices".to_string());
            }
            if import_a023.get() {
                targets.push("a023_purchase_of_goods".to_string());
            }

            if targets.is_empty() {
                set_error_msg.set("Выберите агрегаты для импорта".to_string());
                set_is_loading.set(false);
                return;
            }

            let (period_from_val, period_to_val) = if import_a023.get() {
                (period_from.get(), period_to.get())
            } else {
                (None, None)
            };

            let request = ImportRequest {
                connection_id: conn_id,
                target_aggregates: targets,
                mode: ImportMode::Interactive,
                delete_obsolete: delete_obsolete.get(),
                period_from: period_from_val,
                period_to: period_to_val,
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

    let start_disabled = Signal::derive(move || is_loading.get() || session_id.get().is_some());

    view! {
        <PageFrame page_id="u501_import_from_ut--usecase" category="usecase" class="page--wide">
            <div class="card">
                <div class="card__body">
                    <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center>
                        <h2 class="section-title">"u501: Импорт из УТ 11"</h2>
                        <Button
                            appearance=ButtonAppearance::Primary
                            on_click=on_start_import
                            disabled=start_disabled
                        >
                            "▶ "
                            {move || if is_loading.get() {
                                "Запуск..."
                            } else if session_id.get().is_some() {
                                "Импорт запущен"
                            } else {
                                "Запустить импорт"
                            }}
                        </Button>
                    </Flex>

                    <div class="form-section-group">

                    <div>
                        <h2 class="section-title">"Подключение к 1С"</h2>
                        <div class="form__group">
                            <label class="form__label">"Выберите подключение:"</label>
                            <select
                                class="form__select"
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
                    </div>

                    <div>
                        <h2 class="section-title">"Агрегаты для импорта"</h2>
                        <div class="checkbox-group">
                            <div class="form__checkbox-wrapper">
                                <input
                                    class="form__checkbox"
                                type="checkbox"
                                    prop:checked=move || import_a002.get()
                                    on:change=move |ev| { set_import_a002.set(event_target_checked(&ev)); }
                                />
                                <label class="form__checkbox-label">"Catalog_Организации"</label>
                            </div>
                            <div class="form__checkbox-wrapper">
                                <input
                                    class="form__checkbox"
                                type="checkbox"
                                    prop:checked=move || import_a003.get()
                                    on:change=move |ev| { set_import_a003.set(event_target_checked(&ev)); }
                                />
                                <label class="form__checkbox-label">"Catalog_Контрагенты"</label>
                            </div>
                            <div class="form__checkbox-wrapper">
                                <input
                                    class="form__checkbox"
                                type="checkbox"
                                    prop:checked=move || import_a004.get()
                                    on:change=move |ev| { set_import_a004.set(event_target_checked(&ev)); }
                                />
                                <label class="form__checkbox-label">"Catalog_Номенклатура"</label>
                            </div>
                            <div class="form__checkbox-wrapper">
                                <input
                                    class="form__checkbox"
                                type="checkbox"
                                    prop:checked=move || import_p901.get()
                                    on:change=move |ev| { set_import_p901.set(event_target_checked(&ev)); }
                                />
                                <label class="form__checkbox-label">"InformationRegister_ШтрихкодыНоменклатуры"</label>
                            </div>
                            <div class="form__checkbox-wrapper">
                                <input
                                    class="form__checkbox"
                                type="checkbox"
                                    prop:checked=move || import_a022.get()
                                    on:change=move |ev| { set_import_a022.set(event_target_checked(&ev)); }
                                />
                                <label class="form__checkbox-label">"Catalog_ВариантыКомплектацииНоменклатуры (только ОсновнойВариант)"</label>
                            </div>
                        </div>
                        <div class="info-box">
                            "OData коллекции: Catalog_Организации, Catalog_Контрагенты, Catalog_Номенклатура, InformationRegister_ШтрихкодыНоменклатуры, Catalog_ВариантыКомплектацииНоменклатуры"
                        </div>
                    </div>

                    <div>
                        <h2 class="section-title">"Дополнительные загрузки"</h2>
                        <div class="checkbox-group">
                            <div class="form__checkbox-wrapper">
                                <input
                                    class="form__checkbox"
                                type="checkbox"
                                    prop:checked=move || import_p906.get()
                                    on:change=move |ev| { set_import_p906.set(event_target_checked(&ev)); }
                                />
                                <label class="form__checkbox-label">"p906_prices - Дилерские цены номенклатуры (УТ)"</label>
                            </div>
                            <div class="form__checkbox-wrapper">
                                <input
                                    class="form__checkbox"
                                    type="checkbox"
                                    prop:checked=move || import_a023.get()
                                    on:change=move |ev| { set_import_a023.set(event_target_checked(&ev)); }
                                />
                                <label class="form__checkbox-label">"a023_purchase_of_goods - Приобретение товаров и услуг"</label>
                            </div>
                        </div>
                        <Show when=move || import_a023.get()>
                            <div style="margin-top:var(--spacing-sm);display:flex;gap:var(--spacing-md);align-items:center;flex-wrap:wrap;">
                                <div>
                                    <label class="form__label">"Дата с:"</label>
                                    <input
                                        class="form__input"
                                        type="date"
                                        prop:value=move || period_from.get().unwrap_or_default()
                                        on:change=move |ev| {
                                            let v = event_target_value(&ev);
                                            set_period_from.set(if v.is_empty() { None } else { Some(v) });
                                        }
                                    />
                                </div>
                                <div>
                                    <label class="form__label">"Дата по:"</label>
                                    <input
                                        class="form__input"
                                        type="date"
                                        prop:value=move || period_to.get().unwrap_or_default()
                                        on:change=move |ev| {
                                            let v = event_target_value(&ev);
                                            set_period_to.set(if v.is_empty() { None } else { Some(v) });
                                        }
                                    />
                                </div>
                            </div>
                            <div class="code-box" style="margin-top:var(--spacing-sm);">
                                "OData: Document_ПриобретениеТоваровУслуг | Склад: Московская область | Только проведённые"
                            </div>
                        </Show>
                        <div class="code-box">
                            "HTTP: /hs/mpi_api/prices_dealer"
                        </div>
                    </div>

                    <div>
                        <h2 class="section-title">"Опции импорта"</h2>
                        <div class="form__checkbox-wrapper">
                            <input
                                class="form__checkbox"
                            type="checkbox"
                                prop:checked=move || delete_obsolete.get()
                                on:change=move |ev| { set_delete_obsolete.set(event_target_checked(&ev)); }
                            />
                            <label class="form__checkbox-label">"Удалять устаревшие записи (которых нет в 1С)"</label>
                        </div>
                        <div class="warning-box">
                            <span class="warning-box__icon">"⚠️"</span>
                            <span class="warning-box__text">"Внимание: Записи, которых нет в источнике 1С, будут удалены из БД (жесткое удаление)"</span>
                        </div>
                    </div>

                    {move || {
                        let err = error_msg.get();
                        if !err.is_empty() {
                            view! {
                                <div class="warning-box warning-box--error">
                                    <span class="warning-box__icon">"⚠"</span>
                                    <span class="warning-box__text">{err}</span>
                                </div>
                            }.into_any()
                        } else {
                            view! { <div></div> }.into_any()
                        }
                    }}

                    {move || {
                        if let Some(prog) = progress.get() {
                            view! {
                                <div>
                                    <h3 class="section-title">"Прогресс импорта"</h3>

                                    <div style="display: grid; grid-template-columns: 140px 1fr; gap: var(--spacing-sm); font-size: var(--font-size-sm);">
                                        <div style="color: var(--color-text-secondary); font-weight: 500;">"Session ID:"</div>
                                        <div style="color: var(--color-text-primary); font-family: monospace; font-size: var(--font-size-xs);">{prog.session_id.clone()}</div>

                                        <div style="color: var(--color-text-secondary); font-weight: 500;">"Статус:"</div>
                                        <div style="color: var(--color-text-primary);">
                                            {format!("{:?}", prog.status)}
                                        </div>

                                        <div style="color: var(--color-text-secondary); font-weight: 500;">"Обработано:"</div>
                                        <div style="color: var(--color-text-primary);">
                                            {prog.total_processed} " | Создано: " {prog.total_inserted} " | Обновлено: " {prog.total_updated} " | Ошибок: " {prog.total_errors}
                                        </div>

                                        <div style="color: var(--color-text-secondary); font-weight: 500;">"Обновление:"</div>
                                        <div style="font-size: var(--font-size-xs); color: var(--color-text-muted);">{prog.updated_at.to_rfc3339()}</div>
                                    </div>

                                    <div style="padding-top: var(--spacing-md);">
                                        <h4 class="section-title section-title--spaced">"Детали по агрегатам"</h4>
                                        <div style="display: flex; flex-direction: column; gap: var(--spacing-md);">
                                            {prog.aggregates.iter().map(|agg| {
                                                let percent = if let Some(total) = agg.total {
                                                    if total > 0 {
                                                        (agg.processed as f64 / total as f64 * 100.0) as i32
                                                    } else { 0 }
                                                } else { 0 };

                                                view! {
                                                    <div style="border: 1px solid var(--border-color); background: var(--color-bg-body); padding: var(--spacing-md); border-radius: var(--radius-sm);">
                                                        <div style="font-weight: 600; color: var(--color-text-primary); font-size: var(--font-size-base);">
                                                            {agg.aggregate_index.clone()} " - " {agg.aggregate_name.clone()}
                                                        </div>
                                                        <div style="margin-top: 4px; font-size: var(--font-size-sm); color: var(--color-text-secondary);">
                                                            {agg.processed}
                                                            {if let Some(t) = agg.total { format!(" / {}", t) } else { String::new() }}
                                                            {if percent > 0 { format!(" ({}%)", percent) } else { String::new() }}
                                                        </div>
                                                        <div style="margin-top: 8px; height: 16px; border-radius: var(--radius-sm); background: var(--color-neutral-200); overflow: hidden;">
                                                            <div style={format!("width: {}%; height: 100%; background: var(--color-primary); transition: width 0.3s ease;", percent)}></div>
                                                        </div>
                                                        {agg.current_item.as_ref().map(|ci| view! {
                                                            <div style="margin-top: 8px; font-size: var(--font-size-xs); color: var(--color-text-muted);">
                                                                <span style="font-weight: 600;">{"Текущий элемент: "}</span>{ci.clone()}
                                                            </div>
                                                        })}
                                                        <div style="margin-top: 8px; font-size: var(--font-size-xs); color: var(--color-text-muted);">
                                                            "Создано: " {agg.inserted} " | Обновлено: " {agg.updated} " | Пропущено: " {agg.skipped} " | Ошибок: " {agg.errors}
                                                        </div>
                                                        {agg.info.as_ref().map(|info| view! {
                                                            <div style="margin-top: 4px; font-size: var(--font-size-xs); font-style: italic; color: var(--color-text-muted);">
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
                                            <div style="padding-top: var(--spacing-md);">
                                                <h4 style="font-size: var(--font-size-sm); font-weight: 600; color: var(--color-error); margin-bottom: var(--spacing-sm);">"Ошибки импорта"</h4>
                                                <div style="display: flex; flex-direction: column; gap: var(--spacing-sm);">
                                                    {prog.errors.iter().map(|err| {
                                                        view! {
                                                            <div class="warning-box warning-box--error">
                                                                <span class="warning-box__icon">"⚠"</span>
                                                                <div class="warning-box__text">
                                                                    <div style="font-weight: 600; color: var(--color-error);">{err.message.clone()}</div>
                                                                    {err.details.as_ref().map(|d| view! {
                                                                        <div style="margin-top: 4px; font-size: var(--font-size-xs); color: var(--color-text-muted);">{d.clone()}</div>
                                                                    })}
                                                                </div>
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
                                    <div>
                                        <h3 class="section-title section-title--spaced">"Пути загрузки"</h3>
                                        <div class="code-box">
                                            {endpoints.iter().map(|e| {
                                                let e_clone = e.clone();
                                                view! {
                                                    <div>{e_clone}</div>
                                                }
                                            }).collect_view()}
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

                    {move || {
                        if let Some(prog) = progress.get() {
                            let is_success = matches!(prog.status, ImportStatus::Completed);
                            let is_error = matches!(
                                prog.status,
                                ImportStatus::Failed | ImportStatus::CompletedWithErrors
                            );
                            let end = prog.completed_at.unwrap_or_else(Utc::now);
                            let secs = (end - prog.started_at).num_seconds();
                            let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
                            let elapsed = format!("{:02}:{:02}:{:02}", h, m, s);

                            if is_success {
                                view! {
                                    <div class="info-box">
                                        <div><span style="font-weight: 600;">{"✅ Успех: "}</span>{prog.completed_at.map(|d| d.to_rfc3339()).unwrap_or_else(|| "—".to_string())}</div>
                                        <div><span style="font-weight: 600;">{"Количество элементов: "}</span>{prog.total_processed}</div>
                                        <div><span style="font-weight: 600;">{"Время работы: "}</span>{elapsed}</div>
                                    </div>
                                }
                                .into_any()
                            } else if is_error {
                                view! {
                                    <div class="warning-box warning-box--error">
                                        <span class="warning-box__icon">"❌"</span>
                                        <div class="warning-box__text">
                                            <div style="font-weight: 600; color: var(--color-error); font-size: var(--font-size-base);">{"Ошибка импорта"}</div>
                                            {if let Some(last) = prog.errors.last() {
                                                let details = last.details.clone().unwrap_or_default();
                                                view! {
                                                    <div style="margin-top: var(--spacing-sm);">
                                                        <div style="font-weight: 600; color: var(--color-error);">{last.message.clone()}</div>
                                                        <div style="margin-top: 4px; font-size: var(--font-size-xs); color: var(--color-text-muted);">{details}</div>
                                                    </div>
                                                }.into_any()
                                            } else {
                                                view! { <div style="margin-top: var(--spacing-sm);">{"Нет подробностей ошибки"}</div> }.into_any()
                                            }}
                                            <div style="margin-top: var(--spacing-sm); font-size: var(--font-size-xs); color: var(--color-text-muted);">{"Статус: "}{format!("{:?}", prog.status)}</div>
                                        </div>
                                    </div>
                                }
                                .into_any()
                            } else {
                                view! { <div></div> }.into_any()
                            }
                        } else {
                            view! { <div></div> }.into_any()
                        }
                    }}
                    </div> // form-section-group
                </div> // card__body
            </div> // card
        </PageFrame>
    }
}
