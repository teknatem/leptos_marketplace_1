use super::logs_api;
use contracts::shared::logger::LogEntry;
use leptos::prelude::*;

#[component]
pub fn RightPanel() -> impl IntoView {
    let (logs, set_logs) = signal::<Vec<LogEntry>>(vec![]);
    let (is_loading, set_is_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);

    let load_logs_action = move || {
        set_is_loading.set(true);
        set_error.set(None);
        wasm_bindgen_futures::spawn_local(async move {
            match logs_api::fetch_logs().await {
                Ok(fetched_logs) => {
                    set_logs.set(fetched_logs);
                    set_is_loading.set(false);
                }
                Err(e) => {
                    set_error.set(Some(format!("Ошибка загрузки: {}", e)));
                    set_is_loading.set(false);
                }
            }
        });
    };

    let clear_logs_action = move || {
        set_is_loading.set(true);
        set_error.set(None);
        wasm_bindgen_futures::spawn_local(async move {
            match logs_api::clear_logs().await {
                Ok(_) => {
                    set_logs.set(vec![]);
                    set_is_loading.set(false);
                }
                Err(e) => {
                    set_error.set(Some(format!("Ошибка очистки: {}", e)));
                    set_is_loading.set(false);
                }
            }
        });
    };

    // Загрузить логи при монтировании компонента
    Effect::new(move |_| {
        load_logs_action();
    });

    view! {
        <div class="right-panel logs-panel">
            <div class="logs-header">
                <h3>{"Логи системы"}</h3>
                <div class="logs-actions">
                    <button
                        class="btn btn-sm btn-secondary"
                        on:click=move |_| load_logs_action()
                        disabled=move || is_loading.get()
                    >
                        {move || if is_loading.get() { "Загрузка..." } else { "Обновить" }}
                    </button>
                    <button
                        class="btn btn-sm btn-danger"
                        on:click=move |_| clear_logs_action()
                        disabled=move || is_loading.get()
                    >
                        {"Очистить"}
                    </button>
                </div>
            </div>

            {move || error.get().map(|e| view! {
                <div class="logs-error">{e}</div>
            })}

            <div class="logs-table-container">
                <table class="logs-table">
                    <thead>
                        <tr>
                            <th class="col-time">{"Время"}</th>
                            <th class="col-source">{"Источник"}</th>
                            <th class="col-category">{"Категория"}</th>
                            <th class="col-message">{"Сообщение"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || logs.get().iter().map(|log| {
                            view! {
                                <tr>
                                    <td class="col-time">{log.timestamp.clone()}</td>
                                    <td class="col-source">{log.source.clone()}</td>
                                    <td class="col-category">{log.category.clone()}</td>
                                    <td class="col-message">{log.message.clone()}</td>
                                </tr>
                            }
                        }).collect_view()}
                    </tbody>
                </table>

                {move || {
                    if logs.get().is_empty() && !is_loading.get() {
                        view! {
                            <div class="logs-empty">{"Логов пока нет"}</div>
                        }.into_any()
                    } else {
                        view! { <></> }.into_any()
                    }
                }}
            </div>
        </div>
    }
}
