use crate::domain::a001_connection_1c::ui::details::Connection1CDetails;
use crate::shared::api_utils::api_base;
use crate::shared::icons::icon;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator, Sortable};
use crate::shared::modal_stack::ModalStackService;
use contracts::domain::a001_connection_1c::aggregate::Connection1CDatabase;
use leptos::prelude::*;
use std::cmp::Ordering;
use std::collections::HashSet;
use thaw::*;

#[derive(Clone, Debug)]
pub struct Connection1CRow {
    pub id: String,
    pub description: String,
    pub url: String,
    pub login: String,
    pub is_primary: bool,
    pub comment: String,
    pub created_at: String,
}

impl Connection1CRow {
    fn from_aggregate(c: Connection1CDatabase) -> Self {
        use contracts::domain::common::AggregateId;

        Self {
            id: c.base.id.as_string(),
            description: c.base.description,
            url: c.url,
            login: c.login,
            is_primary: c.is_primary,
            comment: c.base.comment.unwrap_or_else(|| "-".to_string()),
            created_at: format_timestamp(c.base.metadata.created_at),
        }
    }
}

impl Sortable for Connection1CRow {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "description" => self.description.cmp(&other.description),
            "url" => self.url.cmp(&other.url),
            "login" => self.login.cmp(&other.login),
            "is_primary" => self.is_primary.cmp(&other.is_primary),
            "comment" => self.comment.cmp(&other.comment),
            "created_at" => self.created_at.cmp(&other.created_at),
            _ => Ordering::Equal,
        }
    }
}

fn format_timestamp(dt: chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

#[component]
#[allow(non_snake_case)]
pub fn Connection1CList() -> impl IntoView {
    let modal_stack =
        use_context::<ModalStackService>().expect("ModalStackService not found in context");
    let (items, set_items) = signal::<Vec<Connection1CRow>>(Vec::new());
    let (error, set_error) = signal::<Option<String>>(None);
    let show_modal = RwSignal::new(false);
    let (editing_id, set_editing_id) = signal::<Option<String>>(None);
    let (selected, set_selected) = signal::<HashSet<String>>(HashSet::new());

    // Sorting state
    let (raw_items, set_raw_items) = signal::<Vec<Connection1CRow>>(Vec::new());
    let (sort_field, set_sort_field) = signal::<String>("description".to_string());
    let (sort_ascending, set_sort_ascending) = signal(true);

    let fetch = move || {
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_connections().await {
                Ok(v) => {
                    let rows = v.into_iter().map(Connection1CRow::from_aggregate).collect();
                    set_raw_items.set(rows);
                    set_error.set(None);
                }
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    // Auto-sort when data or sort params change
    Effect::new(move |_| {
        let mut sorted = raw_items.get();
        let field = sort_field.get();
        let ascending = sort_ascending.get();

        sorted.sort_by(|a, b| {
            let cmp = a.compare_by_field(b, &field);
            if ascending { cmp } else { cmp.reverse() }
        });

        set_items.set(sorted);
    });

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

    let open_details_modal = move |id: Option<String>| {
        modal_stack.push_with_frame(
            Some("max-width: min(1100px, 95vw); width: min(1100px, 95vw);".to_string()),
            Some("connection-1c-modal".to_string()),
            move |handle| {
                let id_val = id.clone();
                view! {
                    <Connection1CDetails
                        id=Signal::derive(move || id_val.clone())
                        on_saved=Callback::new({
                            let handle = handle.clone();
                            move |_| {
                                handle.close();
                                fetch();
                            }
                        })
                        on_cancel=Callback::new({
                            let handle = handle.clone();
                            move |_| handle.close()
                        })
                    />
                }
                .into_any()
            },
        );
    };

    let toggle_select = move |id: String, checked: bool| {
        set_selected.update(|s| {
            if checked {
                s.insert(id);
            } else {
                s.remove(&id);
            }
        });
    };

    let toggle_sort = move |field: &'static str| {
        if sort_field.get() == field {
            set_sort_ascending.update(|a| *a = !*a);
        } else {
            set_sort_field.set(field.to_string());
            set_sort_ascending.set(true);
        }
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
                <h1 style="font-size: 24px; font-weight: bold;">{"1C Подключения"}</h1>
                <Space>
                    <Button appearance=ButtonAppearance::Primary on_click=move |_| handle_create_new()>
                        {icon("plus")}
                        " Новое подключение"
                    </Button>
                    <Button appearance=ButtonAppearance::Secondary on_click=move |_| fetch()>
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
                        <TableHeaderCell resizable=false class="fixed-checkbox-column">
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
                        <TableHeaderCell resizable=true min_width=180.0>
                            "Наименование"
                            <span
                                class={move || get_sort_class(&sort_field.get(), "description")}
                                style="cursor: pointer; margin-left: 4px;"
                                on:click=move |e| {
                                    e.stop_propagation();
                                    toggle_sort("description");
                                }
                            >
                                {move || get_sort_indicator("description", &sort_field.get(), sort_ascending.get())}
                            </span>
                        </TableHeaderCell>
                        <TableHeaderCell resizable=true min_width=260.0>
                            "URL"
                            <span
                                class={move || get_sort_class(&sort_field.get(), "url")}
                                style="cursor: pointer; margin-left: 4px;"
                                on:click=move |e| {
                                    e.stop_propagation();
                                    toggle_sort("url");
                                }
                            >
                                {move || get_sort_indicator("url", &sort_field.get(), sort_ascending.get())}
                            </span>
                        </TableHeaderCell>
                        <TableHeaderCell resizable=true min_width=140.0>
                            "Логин"
                            <span
                                class={move || get_sort_class(&sort_field.get(), "login")}
                                style="cursor: pointer; margin-left: 4px;"
                                on:click=move |e| {
                                    e.stop_propagation();
                                    toggle_sort("login");
                                }
                            >
                                {move || get_sort_indicator("login", &sort_field.get(), sort_ascending.get())}
                            </span>
                        </TableHeaderCell>
                        <TableHeaderCell resizable=true min_width=100.0>
                            "Основное"
                            <span
                                class={move || get_sort_class(&sort_field.get(), "is_primary")}
                                style="cursor: pointer; margin-left: 4px;"
                                on:click=move |e| {
                                    e.stop_propagation();
                                    toggle_sort("is_primary");
                                }
                            >
                                {move || get_sort_indicator("is_primary", &sort_field.get(), sort_ascending.get())}
                            </span>
                        </TableHeaderCell>
                        <TableHeaderCell resizable=true min_width=180.0>
                            "Комментарий"
                            <span
                                class={move || get_sort_class(&sort_field.get(), "comment")}
                                style="cursor: pointer; margin-left: 4px;"
                                on:click=move |e| {
                                    e.stop_propagation();
                                    toggle_sort("comment");
                                }
                            >
                                {move || get_sort_indicator("comment", &sort_field.get(), sort_ascending.get())}
                            </span>
                        </TableHeaderCell>
                        <TableHeaderCell resizable=true min_width=150.0>
                            "Создано"
                            <span
                                class={move || get_sort_class(&sort_field.get(), "created_at")}
                                style="cursor: pointer; margin-left: 4px;"
                                on:click=move |e| {
                                    e.stop_propagation();
                                    toggle_sort("created_at");
                                }
                            >
                                {move || get_sort_indicator("created_at", &sort_field.get(), sort_ascending.get())}
                            </span>
                        </TableHeaderCell>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {move || items.get().into_iter().map(|row| {
                        let id_for_checkbox = row.id.clone();
                        let id_for_toggle = row.id.clone();
                        let id_for_link = row.id.clone();
                        view! {
                            <TableRow>
                                <TableCell class="fixed-checkbox-column">
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
                                    <TableCellLayout truncate=true>
                                        {row.url}
                                    </TableCellLayout>
                                </TableCell>
                                <TableCell>
                                    <TableCellLayout truncate=true>
                                        {row.login}
                                    </TableCellLayout>
                                </TableCell>
                                <TableCell>
                                    <TableCellLayout>
                                        {if row.is_primary { "Да" } else { "Нет" }}
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

            <Show when=move || show_modal.get()>
                {move || {
                    open_details_modal(editing_id.get());
                    show_modal.set(false);
                    set_editing_id.set(None);
                    view! { <></> }
                }}
            </Show>
        </div>
    }
}

async fn fetch_connections() -> Result<Vec<Connection1CDatabase>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/connection_1c", api_base());
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
    serde_json::from_str(&text).map_err(|e| format!("{e}"))
}

async fn delete_connection(id: &str) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("DELETE");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/connection_1c/{}", api_base(), id);
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


