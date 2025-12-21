use crate::domain::a006_connection_mp::ui::details::ConnectionMPDetails;
use crate::shared::icons::icon;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use leptos::prelude::*;
use std::collections::HashSet;
use thaw::*;

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
    let show_modal = RwSignal::new(false);
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
        show_modal.set(true);
    };

    let handle_edit = move |id: String| {
        let items_clone = items.get();
        if items_clone.iter().any(|item| item.id == id) {
            set_editing_id.set(Some(id));
            show_modal.set(true);
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
        <div style="padding: 20px;">
            <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center>
                <h1 style="font-size: 24px; font-weight: bold;">{"Подключения маркетплейсов"}</h1>
                <Space>
                    <Button
                        appearance=ButtonAppearance::Primary
                        on_click=move |_| handle_create_new()
                    >
                        {icon("plus")}
                        " Новое подключение"
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| fetch()
                    >
                        {icon("refresh")}
                        " Обновить"
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| delete_selected()
                        disabled=Signal::derive(move || selected.get().is_empty())
                    >
                        {icon("delete")}
                        {move || format!(" Удалить ({})", selected.get().len())}
                    </Button>
                </Space>
            </Flex>
            <div style="margin-top: 16px;">
            {move || error.get().map(|e| view! {
                <div style="padding: 12px; background: var(--color-error-50); border: 1px solid var(--color-error-100); border-radius: 8px; display: flex; align-items: center; gap: 8px;">
                    <span style="color: var(--color-error); font-size: 18px;">"⚠"</span>
                    <span style="color: var(--color-error);">{e}</span>
                </div>
            })}
            </div>

            <Table>
                <TableHeader>
                    <TableRow>
                        <TableHeaderCell resizable=true min_width=40.0 max_width=50.0>
                            <input
                                type="checkbox"
                                style="cursor: pointer;"
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
                        </TableHeaderCell>
                        <TableHeaderCell resizable=true min_width=150.0>"Наименование"</TableHeaderCell>
                        <TableHeaderCell resizable=true min_width=120.0>"Маркетплейс"</TableHeaderCell>
                        <TableHeaderCell resizable=true min_width=150.0>"Организация"</TableHeaderCell>
                        <TableHeaderCell>"Используется"</TableHeaderCell>
                        <TableHeaderCell>"Тестовый режим"</TableHeaderCell>
                        <TableHeaderCell resizable=true min_width=150.0>"Комментарий"</TableHeaderCell>
                        <TableHeaderCell>"Создано"</TableHeaderCell>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {move || items.get().into_iter().map(|row| {
                        let id = row.id.clone();
                        let id_for_link = id.clone();
                        let id_for_checkbox = id.clone();
                        let id_for_toggle = id.clone();
                        view! {
                            <TableRow>
                                <TableCell>
                                    <input
                                        type="checkbox"
                                        style="cursor: pointer;"
                                        prop:checked=move || selected.get().contains(&id_for_checkbox)
                                        on:change=move |ev| {
                                            let checked = event_target_checked(&ev);
                                            toggle_select(id_for_toggle.clone(), checked);
                                        }
                                    />
                                </TableCell>
                                <TableCell>
                                    <TableCellLayout>
                                        <a
                                            href="#"
                                            style="color: var(--colorBrandForeground1); text-decoration: none; cursor: pointer;"
                                            on:click=move |e| {
                                                e.prevent_default();
                                                handle_edit(id_for_link.clone());
                                            }
                                        >
                                            {row.description}
                                        </a>
                                    </TableCellLayout>
                                </TableCell>
                                <TableCell>
                                    <TableCellLayout>
                                        {row.marketplace}
                                    </TableCellLayout>
                                </TableCell>
                                <TableCell>
                                    <TableCellLayout>
                                        {row.organization}
                                    </TableCellLayout>
                                </TableCell>
                                <TableCell>
                                    <TableCellLayout>
                                        {if row.is_used { "Да" } else { "Нет" }}
                                    </TableCellLayout>
                                </TableCell>
                                <TableCell>
                                    <TableCellLayout>
                                        {if row.test_mode { "Да" } else { "Нет" }}
                                    </TableCellLayout>
                                </TableCell>
                                <TableCell>
                                    <TableCellLayout truncate=true>
                                        {row.comment}
                                    </TableCellLayout>
                                </TableCell>
                                <TableCell>
                                    <TableCellLayout>
                                        {row.created_at}
                                    </TableCellLayout>
                                </TableCell>
                            </TableRow>
                        }
                    }).collect_view()}
                </TableBody>
            </Table>

            <Dialog open=show_modal>
                <DialogSurface class="connection-mp-dialog">
                    <DialogBody>
                        <DialogTitle>
                            {move || if editing_id.get().is_some() {
                                "Редактирование подключения"
                            } else {
                                "Новое подключение"
                            }}
                        </DialogTitle>
                        <DialogContent>
                            <style>
                                ".connection-mp-dialog.thaw-dialog-surface {
                                    max-width: min(1400px, 95vw) !important;
                                    width: min(1400px, 95vw) !important;
                                }"
                            </style>
                            <div>
                                <ConnectionMPDetails
                                id=editing_id.get()
                                on_saved=Callback::new(move |_| {
                                    show_modal.set(false);
                                    set_editing_id.set(None);
                                    fetch();
                                })
                                on_cancel=Callback::new(move |_| {
                                    show_modal.set(false);
                                    set_editing_id.set(None);
                                })
                                />
                            </div>
                        </DialogContent>
                    </DialogBody>
                </DialogSurface>
            </Dialog>
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
