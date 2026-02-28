use super::api;
use chrono::{Duration, Utc};
use contracts::domain::a001_connection_1c::aggregate::Connection1CDatabase;
use contracts::domain::common::AggregateId;
use contracts::usecases::u507_import_from_erp::{
    progress::{ImportProgress, ImportStatus},
    request::ImportRequest,
};
use crate::shared::page_frame::PageFrame;
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;

fn storage() -> Option<web_sys::Storage> {
    web_sys::window().and_then(|w| w.local_storage().ok().flatten())
}

fn session_key() -> &'static str {
    "u507_session_id"
}

fn progress_key() -> &'static str {
    "u507_progress"
}

fn save_session_id(id: &str) {
    if let Some(s) = storage() {
        let _ = s.set_item(session_key(), id);
    }
}

fn load_session_id() -> Option<String> {
    storage().and_then(|s| s.get_item(session_key()).ok().flatten())
}

fn save_progress(p: &ImportProgress) {
    if let Ok(json) = serde_json::to_string(p) {
        if let Some(s) = storage() {
            let _ = s.set_item(progress_key(), &json);
        }
    }
}

fn load_progress() -> Option<ImportProgress> {
    storage()
        .and_then(|s| s.get_item(progress_key()).ok().flatten())
        .and_then(|json| serde_json::from_str(&json).ok())
}

fn clear_storage() {
    if let Some(s) = storage() {
        let _ = s.remove_item(session_key());
        let _ = s.remove_item(progress_key());
    }
}

fn is_finished(p: &ImportProgress) -> bool {
    matches!(
        p.status,
        ImportStatus::Completed | ImportStatus::CompletedWithErrors | ImportStatus::Failed
    )
}

#[component]
pub fn ImportWidget() -> impl IntoView {
    let default_date_from = (Utc::now() - Duration::days(7))
        .format("%Y-%m-%d")
        .to_string();
    let default_date_to = Utc::now().format("%Y-%m-%d").to_string();

    let (connections, set_connections) = signal(Vec::<Connection1CDatabase>::new());
    let (selected_connection, set_selected_connection) = signal(String::new());
    let (date_from, set_date_from) = signal(default_date_from.clone());
    let (date_to, set_date_to) = signal(default_date_to.clone());
    let (session_id, set_session_id) = signal(None::<String>);
    let (progress, set_progress) = signal(None::<ImportProgress>);
    let (error_msg, set_error_msg) = signal(String::new());
    let (is_starting, set_is_starting) = signal(false);

    // Загрузить подключения 1С
    Effect::new(move || {
        spawn_local(async move {
            match api::get_connections_1c().await {
                Ok(conns) => {
                    if let Some(first) = conns.first() {
                        set_selected_connection.set(first.base.id.as_string());
                    }
                    set_connections.set(conns);
                }
                Err(e) => set_error_msg.set(format!("Ошибка загрузки подключений: {}", e)),
            }
        });
    });

    // Восстановить сессию из localStorage
    Effect::new(move || {
        if session_id.get().is_none() {
            if let Some(sid) = load_session_id() {
                set_session_id.set(Some(sid));
            }
            if let Some(snap) = load_progress() {
                set_progress.set(Some(snap));
            }
        }
    });

    // Polling прогресса
    Effect::new(move || {
        if let Some(sid) = session_id.get() {
            let sid_clone = sid.clone();
            spawn_local(async move {
                loop {
                    match api::get_progress(&sid_clone).await {
                        Ok(prog) => {
                            save_progress(&prog);
                            let finished = is_finished(&prog);
                            set_progress.set(Some(prog));
                            if finished {
                                clear_storage();
                                set_session_id.set(None);
                                break;
                            }
                        }
                        Err(e) => {
                            if e.contains("404") {
                                clear_storage();
                                set_session_id.set(None);
                                set_progress.set(None);
                            } else {
                                set_error_msg.set(format!("Ошибка получения прогресса: {}", e));
                            }
                            break;
                        }
                    }
                    gloo_timers::future::TimeoutFuture::new(2000).await;
                }
            });
        }
    });

    let on_start = move |_| {
        let conn_id = selected_connection.get();
        if conn_id.is_empty() {
            set_error_msg.set("Сначала выберите подключение к 1С".to_string());
            return;
        }

        let df = date_from.get();
        let dt = date_to.get();

        set_is_starting.set(true);
        set_error_msg.set(String::new());
        set_progress.set(None);

        spawn_local(async move {
            let request = ImportRequest {
                connection_id: conn_id,
                date_from: df,
                date_to: dt,
            };

            match api::start_import(request).await {
                Ok(response) => {
                    save_session_id(&response.session_id);
                    set_session_id.set(Some(response.session_id));
                    set_is_starting.set(false);
                }
                Err(e) => {
                    set_error_msg.set(format!("Ошибка запуска: {}", e));
                    set_is_starting.set(false);
                }
            }
        });
    };

    view! {
        <PageFrame page_id="u507_import_from_erp--usecase" category="usecase">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"u507: Импорт из ERP"</h1>
                </div>
            </div>

            {move || {
                let err = error_msg.get();
                if !err.is_empty() {
                    view! {
                        <div style="padding:12px 16px;border-radius:var(--radius-md);border-left:3px solid var(--color-error);background:var(--color-error-50);margin-bottom:16px;font-size:var(--font-size-base);">
                            {err}
                        </div>
                    }.into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }}

            <div style="display:flex;flex-direction:column;gap:16px;margin-top:16px;">

                // Выбор подключения 1С
                <Flex vertical=false gap=FlexGap::Large justify=FlexJustify::Center align=FlexAlign::Center>
                    <label class="form__label">"Подключение 1С"</label>
                    <select
                        class="doc-filter__select"
                        style="width:100%;max-width:500px;"
                        on:change=move |ev| set_selected_connection.set(event_target_value(&ev))
                    >
                        <option value="">"— выберите подключение —"</option>
                        {move || connections.get().into_iter().map(|conn| {
                            let id = conn.base.id.as_string();
                            let selected = id == selected_connection.get();
                            let caption = conn.base.description.clone();
                            view! { <option selected=selected value={id}>{caption}</option> }
                        }).collect_view()}
                    </select>
                </Flex>

                // Строка импорта Выпуск продукции
                <div style="margin-left:16px;margin-right:16px;">
                    <Card>
                        <div class="doc-filters__row">
                            <Button
                                appearance=ButtonAppearance::Primary
                                on_click=on_start
                                disabled=move || {
                                    selected_connection.get().is_empty()
                                        || is_starting.get()
                                        || session_id.get().is_some()
                                }
                            >
                                {move || if is_starting.get() {
                                    "Запуск..."
                                } else if session_id.get().is_some() {
                                    "В работе"
                                } else {
                                    "Запустить"
                                }}
                            </Button>

                            <div class="doc-filter" style="flex-direction:column;align-items:flex-start;gap:2px;min-width:220px;">
                                <span style="font-size:var(--font-size-base);font-weight:600;">
                                    "Выпуск продукции"
                                </span>
                                <span style="font-size:var(--font-size-sm);color:var(--color-text-secondary);">
                                    "a021_production_output"
                                </span>
                                <div style="font-size:var(--font-size-sm);color:var(--color-text-tertiary);font-family:monospace;">
                                    "/hs/mpi_api/production_output"
                                </div>
                            </div>

                            <Flex vertical=true gap=FlexGap::Small>
                                <div class="doc-filter">
                                    <label class="doc-filter__label">"Период:"</label>
                                    <input
                                        type="date"
                                        class="doc-filter__input"
                                        prop:value=move || date_from.get()
                                        on:change=move |ev| set_date_from.set(event_target_value(&ev))
                                    />
                                    <span>"—"</span>
                                    <input
                                        type="date"
                                        class="doc-filter__input"
                                        prop:value=move || date_to.get()
                                        on:change=move |ev| set_date_to.set(event_target_value(&ev))
                                    />
                                </div>

                                {move || {
                                    if let Some(prog) = progress.get() {
                                        let total = prog.total.unwrap_or(0);
                                        let percent = if total > 0 {
                                            ((prog.processed as f64 / total as f64) * 100.0).clamp(0.0, 100.0) as i32
                                        } else if prog.processed > 0 {
                                            100
                                        } else {
                                            0
                                        };

                                        view! {
                                            <div style="display:flex;align-items:center;gap:10px;flex:1;min-width:0;">
                                                <span style="font-size:var(--font-size-sm);color:var(--color-text-secondary);min-width:120px;">
                                                    {format!("{:?}", prog.status)}
                                                </span>
                                                <div style="height:16px;border-radius:var(--radius-sm);overflow:hidden;background:var(--color-border);flex:1;min-width:200px;">
                                                    <div style={format!("width:{}%;height:100%;background:var(--colorBrandForeground1);transition:width 0.2s;", percent)}></div>
                                                </div>
                                                <span style="font-size:var(--font-size-sm);color:var(--color-text-secondary);min-width:85px;text-align:right;">
                                                    {if total > 0 {
                                                        format!("{} / {}", prog.processed, total)
                                                    } else {
                                                        format!("{}", prog.processed)
                                                    }}
                                                </span>
                                                <span style="font-size:var(--font-size-sm);color:var(--color-text-secondary);min-width:165px;">
                                                    {format!("ins: {}  upd: {}  err: {}", prog.inserted, prog.updated, prog.errors)}
                                                </span>
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <span style="font-size:var(--font-size-sm);color:var(--color-text-secondary);">
                                                "Готово к запуску"
                                            </span>
                                        }.into_any()
                                    }
                                }}

                                {move || {
                                    if let Some(prog) = progress.get() {
                                        if !prog.error_messages.is_empty() {
                                            return view! {
                                                <div style="margin-top:8px;padding:8px 12px;border-radius:var(--radius-md);border-left:3px solid var(--color-error);background:var(--color-error-50);font-size:var(--font-size-sm);max-height:100px;overflow-y:auto;">
                                                    {prog.error_messages.iter().map(|e| view! {
                                                        <div>{e.clone()}</div>
                                                    }).collect_view()}
                                                </div>
                                            }.into_any();
                                        }
                                    }
                                    view! { <></> }.into_any()
                                }}
                            </Flex>
                        </div>
                    </Card>
                </div>
            </div>
        </PageFrame>
    }
}
