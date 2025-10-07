use crate::domain::a006_connection_mp::ui::details::ConnectionMPDetails;
use crate::shared::icons::icon;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use leptos::prelude::*;
use std::collections::HashSet;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct ConnectionMPRow {
    pub id: String,
    pub code: String,
    pub description: String,
    pub marketplace: String,
    pub organization: String,
    pub is_used: bool,
    pub test_mode: bool,
    pub comment: String,
    pub created_at: String,
}

impl From<ConnectionMP> for ConnectionMPRow {
    fn from(c: ConnectionMP) -> Self {
        use contracts::domain::common::AggregateId;

        Self {
            id: c.base.id.as_string(),
            code: c.base.code,
            description: c.base.description,
            marketplace: c.marketplace_id.clone(), // Показываем ID, позже можно улучшить
            organization: c.organization,
            is_used: c.is_used,
            test_mode: c.test_mode,
            comment: c.base.comment.unwrap_or_else(|| "-".to_string()),
            created_at: format_timestamp(c.base.metadata.created_at),
        }
    }
}

fn format_timestamp(dt: chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

#[component]
#[allow(non_snake_case)]
pub fn ConnectionMPList() -> impl IntoView {
    let (items, set_items) = signal::<Vec<ConnectionMPRow>>(Vec::new());
    let (error, set_error) = signal::<Option<String>>(None);
    let (show_modal, set_show_modal) = signal(false);
    let (editing_id, set_editing_id) = signal::<Option<String>>(None);
    let (selected, set_selected) = signal::<HashSet<String>>(HashSet::new());

    let fetch = move || {
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_connections().await {
                Ok(v) => {
                    let rows: Vec<ConnectionMPRow> = v.into_iter().map(Into::into).collect();
                    set_items.set(rows);
                    set_error.set(None);
                }
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    let handle_create_new = move || {
        set_editing_id.set(None);
        set_show_modal.set(true);
    };

    let handle_edit = move |id: String| {
        let items_clone = items.get();
        if items_clone.iter().any(|item| item.id == id) {
            set_editing_id.set(Some(id));
            set_show_modal.set(true);
        }
    };

    let handle_cancel = move |_| {
        set_show_modal.set(false);
        set_editing_id.set(None);
    };

    let toggle_select = move |id: String, checked: bool| {
        set_selected.update(|s| {
            if checked {
                s.insert(id.clone());
            } else {
                s.remove(&id);
            }
        });
    };

    let clear_selection = move || set_selected.set(HashSet::new());

    let delete_selected = move || {
        let ids: Vec<String> = selected.get().into_iter().collect();
        if ids.is_empty() {
            return;
        }

        let count = ids.len();
        let confirmed = {
            if let Some(win) = web_sys::window() {
                win.confirm_with_message(&format!(
                    "Удалить выбранные элементы? Количество: {}",
                    count
                ))
                .unwrap_or(false)
            } else {
                false
            }
        };
        if !confirmed {
            return;
        }

        wasm_bindgen_futures::spawn_local(async move {
            let mut all_ok = true;
            for id in ids {
                if let Err(_) = delete_connection(&id).await {
                    all_ok = false;
                }
            }
            if all_ok {
                let _ = wasm_bindgen_futures::JsFuture::from(js_sys::Promise::resolve(
                    &wasm_bindgen::JsValue::UNDEFINED,
                ))
                .await;
            }
        });
        fetch();
        clear_selection();
    };

    fetch();

    view! {
        <div class="content">
            <div class="header">
                <h2>{"Подключения маркетплейсов"}</h2>
                <div class="header-actions">
                    <button class="btn btn-primary" on:click=move |_| handle_create_new()>
                        {icon("plus")}
                        {"Новое подключение"}
                    </button>
                    <button class="btn btn-secondary" on:click=move |_| fetch()>
                        {icon("refresh")}
                        {"Обновить"}
                    </button>
                    <button class="btn btn-danger" on:click=move |_| delete_selected() disabled={move || selected.get().is_empty()}>
                        {icon("delete")}
                        {move || format!("Удалить ({})", selected.get().len())}
                    </button>
                </div>
            </div>

            {move || error.get().map(|e| view! { <div class="error">{e}</div> })}

            <div class="table-container">
                <table>
                    <thead>
                        <tr>
                            <th></th>
                            <th>{"Код"}</th>
                            <th>{"Наименование"}</th>
                            <th>{"Маркетплейс"}</th>
                            <th>{"Организация"}</th>
                            <th>{"Используется"}</th>
                            <th>{"Тестовый режим"}</th>
                            <th>{"Комментарий"}</th>
                            <th>{"Создано"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || items.get().into_iter().map(|row| {
                            let id = row.id.clone();
                            view! {
                                <tr on:click=move |_| handle_edit(id.clone())>
                                    <td>
                                        <input type="checkbox"
                                            prop:checked={
                                                let selected = selected.get();
                                                selected.contains(&id)
                                            }
                                            on:click=move |ev| ev.stop_propagation()
                                            on:change={
                                                let id2 = id.clone();
                                                move |ev| {
                                                    let checked = event_target_checked(&ev);
                                                    toggle_select(id2.clone(), checked);
                                                }
                                            }
                                        />
                                    </td>
                                    <td>{row.code}</td>
                                    <td>{row.description}</td>
                                    <td>{row.marketplace}</td>
                                    <td>{row.organization}</td>
                                    <td>{if row.is_used { "Да" } else { "Нет" }}</td>
                                    <td>{if row.test_mode { "Да" } else { "Нет" }}</td>
                                    <td>{row.comment}</td>
                                    <td>{row.created_at}</td>
                                </tr>
                            }
                        }).collect_view()}
                    </tbody>
                </table>
            </div>

            {move || if show_modal.get() {
                view! {
                    <div class="modal-overlay">
                        <div class="modal-content">
                            <ConnectionMPDetails
                                id=editing_id.get()
                                on_saved=Rc::new(move |_| { set_show_modal.set(false); set_editing_id.set(None); fetch(); })
                                on_cancel=Rc::new(move |_| handle_cancel(()))
                            />
                        </div>
                    </div>
                }.into_any()
            } else {
                view! { <></> }.into_any()
            }}
        </div>
    }
}

fn api_base() -> String {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return String::new(),
    };
    let location = window.location();
    let protocol = location.protocol().unwrap_or_else(|_| "http:".to_string());
    let hostname = location
        .hostname()
        .unwrap_or_else(|_| "127.0.0.1".to_string());
    format!("{}//{}:3000", protocol, hostname)
}

async fn fetch_connections() -> Result<Vec<ConnectionMP>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/connection_mp", api_base());
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    let data: Vec<ConnectionMP> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}

async fn delete_connection(id: &str) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("DELETE");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/connection_mp/{}", api_base(), id);
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    Ok(())
}
