use crate::domain::a006_connection_mp::ui::details::ConnectionMPDetails;
use crate::shared::components::table_checkbox::TableCheckbox;
use crate::shared::icons::icon;
use crate::shared::modal::Modal;
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

impl ConnectionMPRow {
    async fn from_async(c: ConnectionMP) -> Self {
        use contracts::domain::common::AggregateId;

        // Загружаем название маркетплейса
        let marketplace = match fetch_marketplace_name(&c.marketplace_id).await {
            Ok(name) => name,
            Err(_) => c.marketplace_id.clone(),
        };

        Self {
            id: c.base.id.as_string(),
            code: c.base.code,
            description: c.base.description,
            marketplace,
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
                    let mut rows = Vec::new();
                    for conn in v {
                        rows.push(ConnectionMPRow::from_async(conn).await);
                    }
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
        <div class="page">
            // Page header with title and action buttons
            <div class="header">
                <div class="header__content">
                    <h1 class="header__title">{"Подключения маркетплейсов"}</h1>
                </div>
                <div class="header__actions">
                    <button class="button button--primary" on:click=move |_| handle_create_new()>
                        {icon("plus")}
                        {"Новое подключение"}
                    </button>
                    <button class="button button--secondary" on:click=move |_| fetch()>
                        {icon("refresh")}
                        {"Обновить"}
                    </button>
                    <button class="button button--secondary" on:click=move |_| delete_selected() disabled={move || selected.get().is_empty()}>
                        {icon("delete")}
                        {move || format!("Удалить ({})", selected.get().len())}
                    </button>
                </div>
            </div>

            {move || error.get().map(|e| view! {
                <div class="warning-box" style="background: var(--color-error-50); border-color: var(--color-error-100);">
                    <span class="warning-box__icon" style="color: var(--color-error);">"⚠"</span>
                    <span class="warning-box__text" style="color: var(--color-error);">{e}</span>
                </div>
            })}

            <div class="table">
                <table class="table__data table--striped">
                    <thead class="table__head">
                        <tr>
                            <th class="table__header-cell table__header-cell--checkbox">
                                <input
                                    type="checkbox"
                                    class="table__checkbox"
                                    on:change=move |ev| {
                                        let checked = event_target_checked(&ev);
                                        let current_items = items.get();
                                        if checked {
                                            set_selected.update(|s| {
                                                for item in current_items.iter() {
                                                    s.insert(item.id.clone());
                                                }
                                            });
                                        } else {
                                            set_selected.set(HashSet::new());
                                        }
                                    }
                                />
                            </th>
                            <th class="table__header-cell">{"Наименование"}</th>
                            <th class="table__header-cell">{"Маркетплейс"}</th>
                            <th class="table__header-cell">{"Организация"}</th>
                            <th class="table__header-cell">{"Используется"}</th>
                            <th class="table__header-cell">{"Тестовый режим"}</th>
                            <th class="table__header-cell">{"Комментарий"}</th>
                            <th class="table__header-cell">{"Создано"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || items.get().into_iter().map(|row| {
                            let id = row.id.clone();
                            let id_for_click = id.clone();
                            let id_for_checkbox = id.clone();
                            let id_for_toggle = id.clone();
                            let is_selected = selected.get().contains(&id);
                            view! {
                                <tr
                                    class="table__row"
                                    class:table__row--selected=is_selected
                                    on:click=move |_| handle_edit(id_for_click.clone())
                                >
                                    <TableCheckbox
                                        checked=Signal::derive(move || selected.get().contains(&id_for_checkbox))
                                        on_change=Callback::new(move |checked| toggle_select(id_for_toggle.clone(), checked))
                                    />
                                    <td class="table__cell">{row.description}</td>
                                    <td class="table__cell">{row.marketplace}</td>
                                    <td class="table__cell">{row.organization}</td>
                                    <td class="table__cell">{if row.is_used { "Да" } else { "Нет" }}</td>
                                    <td class="table__cell">{if row.test_mode { "Да" } else { "Нет" }}</td>
                                    <td class="table__cell">{row.comment}</td>
                                    <td class="table__cell">{row.created_at}</td>
                                </tr>
                            }
                        }).collect_view()}
                    </tbody>
                </table>
            </div>

            <Show when=move || show_modal.get()>
                {move || {
                    let modal_title = if editing_id.get().is_some() {
                        "Редактирование подключения".to_string()
                    } else {
                        "Новое подключение".to_string()
                    };
                    view! {
                        <Modal
                            title=modal_title
                            on_close=Callback::new(move |_| {
                                set_show_modal.set(false);
                                set_editing_id.set(None);
                            })
                        >
                            <ConnectionMPDetails
                                id=editing_id.get()
                                on_saved=Rc::new(move |_| {
                                    set_show_modal.set(false);
                                    set_editing_id.set(None);
                                    fetch();
                                })
                                on_cancel=Rc::new(move |_| {
                                    set_show_modal.set(false);
                                    set_editing_id.set(None);
                                })
                            />
                        </Modal>
                    }
                }}
            </Show>
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

async fn fetch_marketplace_name(id: &str) -> Result<String, String> {
    use contracts::domain::a005_marketplace::aggregate::Marketplace;
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/marketplace/{}", api_base(), id);
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
    let marketplace: Marketplace = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(marketplace.base.description)
}
