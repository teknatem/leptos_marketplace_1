pub mod state;

use self::state::create_state;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::components::table::{
    TableCellCheckbox, TableCrosshairHighlight, TableHeaderCheckbox,
};
use crate::shared::icons::icon;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator, Sortable};
use crate::shared::table_utils::init_column_resize;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cmp::Ordering;
use std::collections::HashSet;
use thaw::*;
use crate::shared::page_frame::PageFrame;

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
        let organization = match fetch_organization_name(&c.organization_ref).await {
            Ok(name) if !name.is_empty() => name,
            _ => c.organization_ref.clone(),
        };

        Self {
            id: c.base.id.as_string(),
            code: c.base.code,
            description: c.base.description,
            marketplace,
            organization,
            is_used: c.is_used,
            test_mode: c.test_mode,
            comment: c.base.comment.unwrap_or_else(|| "-".to_string()),
            created_at: format_timestamp(c.base.metadata.created_at),
        }
    }
}

impl Sortable for ConnectionMPRow {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "description" => self.description.to_lowercase().cmp(&other.description.to_lowercase()),
            "marketplace" => self.marketplace.to_lowercase().cmp(&other.marketplace.to_lowercase()),
            "organization" => self.organization.to_lowercase().cmp(&other.organization.to_lowercase()),
            "is_used" => self.is_used.cmp(&other.is_used),
            "test_mode" => self.test_mode.cmp(&other.test_mode),
            "comment" => self.comment.to_lowercase().cmp(&other.comment.to_lowercase()),
            "created_at" => self.created_at.cmp(&other.created_at),
            _ => Ordering::Equal,
        }
    }
}

fn format_timestamp(dt: chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

const TABLE_ID: &str = "a006-connection-mp-table";
const COLUMN_WIDTHS_KEY: &str = "a006_connection_mp_column_widths";

#[component]
#[allow(non_snake_case)]
pub fn ConnectionMPList() -> impl IntoView {
    let tabs_store =
        use_context::<AppGlobalContext>().expect("AppGlobalContext not found in context");
    
    let state = create_state();
    let (items, set_items) = signal::<Vec<ConnectionMPRow>>(Vec::new());
    let (raw_items, set_raw_items) = signal::<Vec<ConnectionMPRow>>(Vec::new());
    let (error, set_error) = signal::<Option<String>>(None);

    let fetch = move || {
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_connections().await {
                Ok(v) => {
                    let mut rows = Vec::new();
                    for conn in v {
                        rows.push(ConnectionMPRow::from_async(conn).await);
                    }
                    set_raw_items.set(rows);
                    set_error.set(None);
                }
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    // Автоматическая сортировка при изменении данных или параметров
    Effect::new(move |_| {
        let mut sorted = raw_items.get();
        let field = state.sort_field.get();
        let ascending = state.sort_ascending.get();

        sorted.sort_by(|a, b| {
            let cmp = a.compare_by_field(b, &field);
            if ascending {
                cmp
            } else {
                cmp.reverse()
            }
        });

        set_items.set(sorted);
    });

    let handle_create_new = move || {
        tabs_store.open_tab(
            "a006_connection_mp_detail_new",
            "Новое подключение",
        );
    };

    let open_detail = move |id: String, description: String| {
        tabs_store.open_tab(
            &format!("a006_connection_mp_detail_{}", id),
            &format!("Подключение: {}", description),
        );
    };

    let toggle_select = move |id: String, checked: bool| {
        state.selected.update(|s| {
            if checked {
                s.insert(id.clone());
            } else {
                s.remove(&id);
            }
        });
    };

    let toggle_sort = move |field: &'static str| {
        if state.sort_field.get() == field {
            state.sort_ascending.update(|a| *a = !*a);
        } else {
            state.sort_field.set(field.to_string());
            state.sort_ascending.set(true);
        }
    };

    let clear_selection = move || state.selected.set(HashSet::new());

    let delete_selected = move || {
        let ids: Vec<String> = state.selected.get().into_iter().collect();
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

    // Initialize column resize once
    let resize_initialized = StoredValue::new(false);
    Effect::new(move |_| {
        if !resize_initialized.get_value() {
            resize_initialized.set_value(true);
            spawn_local(async move {
                gloo_timers::future::TimeoutFuture::new(100).await;
                init_column_resize(TABLE_ID, COLUMN_WIDTHS_KEY);
            });
        }
    });

    view! {
        <PageFrame page_id="a006_connection_mp--list" category="list">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">{"Подключения маркетплейсов"}</h1>
                </div>
                <div class="page__header-right">
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
                        disabled=Signal::derive(move || state.selected.get().is_empty())
                    >
                        {icon("delete")}
                        {move || format!(" Удалить ({})", state.selected.get().len())}
                    </Button>
                </div>
            </div>

            <div class="page__content">
                {move || error.get().map(|e| view! {
                    <div class="warning-box warning-box--error">
                        <span class="warning-box__icon">"⚠"</span>
                        <span class="warning-box__text">{e}</span>
                    </div>
                })}

                <div class="table-wrapper">
                    <TableCrosshairHighlight table_id=TABLE_ID.to_string() />
                    
                    <Table attr:id=TABLE_ID attr:style="width: 100%; min-width: 900px;">
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCheckbox
                                    items=items
                                    selected=state.selected
                                    get_id=Callback::new(|row: ConnectionMPRow| row.id.clone())
                                    on_change=Callback::new(move |check_all: bool| {
                                        if check_all {
                                            state.selected.update(|s| {
                                                for item in items.get().iter() {
                                                    s.insert(item.id.clone());
                                                }
                                            });
                                        } else {
                                            state.selected.set(HashSet::new());
                                        }
                                    })
                                />
                                <TableHeaderCell resizable=true min_width=200.0 class="resizable">
                                    "Наименование"
                                    <span
                                        class={move || get_sort_class(&state.sort_field.get(), "description")}
                                        style="cursor: pointer; margin-left: 4px;"
                                        on:click=move |e| {
                                            e.stop_propagation();
                                            toggle_sort("description");
                                        }
                                    >
                                        {move || get_sort_indicator("description", &state.sort_field.get(), state.sort_ascending.get())}
                                    </span>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=150.0 class="resizable">
                                    "Маркетплейс"
                                    <span
                                        class={move || get_sort_class(&state.sort_field.get(), "marketplace")}
                                        style="cursor: pointer; margin-left: 4px;"
                                        on:click=move |e| {
                                            e.stop_propagation();
                                            toggle_sort("marketplace");
                                        }
                                    >
                                        {move || get_sort_indicator("marketplace", &state.sort_field.get(), state.sort_ascending.get())}
                                    </span>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=150.0 class="resizable">
                                    "Организация"
                                    <span
                                        class={move || get_sort_class(&state.sort_field.get(), "organization")}
                                        style="cursor: pointer; margin-left: 4px;"
                                        on:click=move |e| {
                                            e.stop_propagation();
                                            toggle_sort("organization");
                                        }
                                    >
                                        {move || get_sort_indicator("organization", &state.sort_field.get(), state.sort_ascending.get())}
                                    </span>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=100.0>
                                    "Используется"
                                    <span
                                        class={move || get_sort_class(&state.sort_field.get(), "is_used")}
                                        style="cursor: pointer; margin-left: 4px;"
                                        on:click=move |e| {
                                            e.stop_propagation();
                                            toggle_sort("is_used");
                                        }
                                    >
                                        {move || get_sort_indicator("is_used", &state.sort_field.get(), state.sort_ascending.get())}
                                    </span>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=100.0>
                                    "Тестовый режим"
                                    <span
                                        class={move || get_sort_class(&state.sort_field.get(), "test_mode")}
                                        style="cursor: pointer; margin-left: 4px;"
                                        on:click=move |e| {
                                            e.stop_propagation();
                                            toggle_sort("test_mode");
                                        }
                                    >
                                        {move || get_sort_indicator("test_mode", &state.sort_field.get(), state.sort_ascending.get())}
                                    </span>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=150.0 class="resizable">
                                    "Комментарий"
                                    <span
                                        class={move || get_sort_class(&state.sort_field.get(), "comment")}
                                        style="cursor: pointer; margin-left: 4px;"
                                        on:click=move |e| {
                                            e.stop_propagation();
                                            toggle_sort("comment");
                                        }
                                    >
                                        {move || get_sort_indicator("comment", &state.sort_field.get(), state.sort_ascending.get())}
                                    </span>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=150.0>
                                    "Создано"
                                    <span
                                        class={move || get_sort_class(&state.sort_field.get(), "created_at")}
                                        style="cursor: pointer; margin-left: 4px;"
                                        on:click=move |e| {
                                            e.stop_propagation();
                                            toggle_sort("created_at");
                                        }
                                    >
                                        {move || get_sort_indicator("created_at", &state.sort_field.get(), state.sort_ascending.get())}
                                    </span>
                                </TableHeaderCell>
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            {move || items.get().into_iter().map(|row| {
                                let id = row.id.clone();
                                let description = row.description.clone();
                                let id_for_link = id.clone();
                                let description_for_link = description.clone();
                                let is_selected = state.selected.get().contains(&id);
                                
                                view! {
                                    <TableRow class:table__row--selected=is_selected>
                                        <TableCellCheckbox
                                            item_id=id.clone()
                                            selected=state.selected
                                            on_change=Callback::new(move |(id, checked)| {
                                                toggle_select(id, checked);
                                            })
                                        />
                                        <TableCell>
                                            <TableCellLayout>
                                                <a
                                                    href="#"
                                                    class="table__link"
                                                    on:click=move |e| {
                                                        e.prevent_default();
                                                        open_detail(id_for_link.clone(), description_for_link.clone());
                                                    }
                                                >
                                                    {description}
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
                </div>
            </div>
        </PageFrame>
    }
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

async fn fetch_organization_name(id: &str) -> Result<String, String> {
    use contracts::domain::a002_organization::aggregate::Organization;
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let normalized_id = id.trim().trim_matches('"').to_string();
    if normalized_id.is_empty() {
        return Ok(String::new());
    }

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/organization/{}", api_base(), normalized_id);
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
    let org: Organization = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(org.base.description)
}
