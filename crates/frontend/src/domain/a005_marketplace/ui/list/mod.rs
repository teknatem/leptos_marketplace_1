pub mod state;

use self::state::create_state;
use crate::domain::a005_marketplace::ui::details::MarketplaceDetails;
use crate::shared::api_utils::api_base;
use crate::shared::components::table_checkbox::TableCheckbox;
use crate::shared::icons::icon;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator, sort_list, Sortable};
use crate::shared::modal_stack::ModalStackService;
use contracts::domain::a005_marketplace::aggregate::Marketplace;
use leptos::prelude::*;
use std::cmp::Ordering;
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct MarketplaceRow {
    pub id: String,
    pub code: String,
    pub description: String,
    pub url: String,
    pub logo_path: Option<String>,
    pub comment: String,
    pub created_at: String,
}

impl From<Marketplace> for MarketplaceRow {
    fn from(m: Marketplace) -> Self {
        use contracts::domain::common::AggregateId;

        Self {
            id: m.base.id.as_string(),
            code: m.base.code,
            description: m.base.description,
            url: m.url,
            logo_path: m.logo_path,
            comment: m.base.comment.unwrap_or_else(|| "-".to_string()),
            created_at: format_timestamp(m.base.metadata.created_at),
        }
    }
}

fn format_timestamp(dt: chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

impl Sortable for MarketplaceRow {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "code" => self.code.to_lowercase().cmp(&other.code.to_lowercase()),
            "description" => self
                .description
                .to_lowercase()
                .cmp(&other.description.to_lowercase()),
            "url" => self.url.to_lowercase().cmp(&other.url.to_lowercase()),
            "comment" => self
                .comment
                .to_lowercase()
                .cmp(&other.comment.to_lowercase()),
            "created_at" => self.created_at.cmp(&other.created_at),
            _ => Ordering::Equal,
        }
    }
}

#[component]
#[allow(non_snake_case)]
pub fn MarketplaceList() -> impl IntoView {
    let modal_stack =
        use_context::<ModalStackService>().expect("ModalStackService not found in context");
    let state = create_state();
    let (items, set_items) = signal::<Vec<MarketplaceRow>>(Vec::new());
    let (error, set_error) = signal::<Option<String>>(None);
    let (selected, set_selected) = signal::<HashSet<String>>(HashSet::new());

    let fetch = move || {
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_marketplaces().await {
                Ok(v) => {
                    let rows: Vec<MarketplaceRow> = v.into_iter().map(Into::into).collect();
                    set_items.set(rows);
                    set_error.set(None);
                }
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    let open_details_modal = move |id: Option<String>| {
        let id_val = id.clone();
        modal_stack.push_with_frame(
            Some("max-width: min(1100px, 95vw); width: min(1100px, 95vw);".to_string()),
            Some("marketplace-modal".to_string()),
            move |handle| {
                view! {
                    <MarketplaceDetails
                        id=id_val.clone()
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

    let handle_create_new = move || {
        open_details_modal(None);
    };

    let handle_edit = move |id: String| {
        let items_clone = items.get();
        if items_clone.iter().any(|item| item.id == id) {
            open_details_modal(Some(id));
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

    let toggle_sort = move |field: &'static str| {
        move |_| {
            state.update(|s| {
                if s.sort_field == field {
                    s.sort_ascending = !s.sort_ascending;
                } else {
                    s.sort_field = field.to_string();
                    s.sort_ascending = true;
                }
            });
        }
    };

    let sorted_items = move || {
        let mut items_vec = items.get();
        let s = state.get();
        sort_list(&mut items_vec, &s.sort_field, s.sort_ascending);
        items_vec
    };

    let delete_selected = move || {
        let ids: Vec<String> = selected.get().into_iter().collect();
        if ids.is_empty() {
            return;
        }

        let count = ids.len();
        // Simple confirm dialog via browser
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
                if let Err(_) = delete_marketplace(&id).await {
                    all_ok = false;
                }
            }
            if all_ok {
                // refresh list and clear selection
                // Use window setTimeout microtask to avoid borrowing issues
                let _ = wasm_bindgen_futures::JsFuture::from(js_sys::Promise::resolve(
                    &wasm_bindgen::JsValue::UNDEFINED,
                ))
                .await;
            }
        });
        // Immediately refetch and clear selection (optimistic)
        fetch();
        clear_selection();
    };

    fetch();

    view! {
        <div class="content">
            <div class="header">
                <h2>{"Маркетплейсы"}</h2>
                <div class="header__actions">
                    <button class="button button--primary" on:click=move |_| handle_create_new()>
                        {icon("plus")}
                        {"Новый маркетплейс"}
                    </button>
                    <button class="button button--primary" on:click=move |_| {
                        wasm_bindgen_futures::spawn_local(async move {
                            match fill_test_data().await {
                                Ok(_) => fetch(),
                                Err(e) => set_error.set(Some(format!("Ошибка заполнения: {}", e))),
                            }
                        });
                    }>
                        {icon("download")}
                        {"Заполнить"}
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

            {move || error.get().map(|e| view! { <div class="error">{e}</div> })}

            <div class="table-container">
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
                            <th class="table__header-cell">{"Логотип"}</th>
                            <th class="table__header-cell table__header-cell--sortable" on:click=toggle_sort("code")>
                                "Код"
                                <span class={move || get_sort_class(&state.get().sort_field, "code")}>
                                    {move || get_sort_indicator(&state.get().sort_field, "code", state.get().sort_ascending)}
                                </span>
                            </th>
                            <th class="table__header-cell table__header-cell--sortable" on:click=toggle_sort("description")>
                                "Наименование"
                                <span class={move || get_sort_class(&state.get().sort_field, "description")}>
                                    {move || get_sort_indicator(&state.get().sort_field, "description", state.get().sort_ascending)}
                                </span>
                            </th>
                            <th class="table__header-cell table__header-cell--sortable" on:click=toggle_sort("url")>
                                "URL"
                                <span class={move || get_sort_class(&state.get().sort_field, "url")}>
                                    {move || get_sort_indicator(&state.get().sort_field, "url", state.get().sort_ascending)}
                                </span>
                            </th>
                            <th class="table__header-cell table__header-cell--sortable" on:click=toggle_sort("comment")>
                                "Комментарий"
                                <span class={move || get_sort_class(&state.get().sort_field, "comment")}>
                                    {move || get_sort_indicator(&state.get().sort_field, "comment", state.get().sort_ascending)}
                                </span>
                            </th>
                            <th class="table__header-cell table__header-cell--sortable" on:click=toggle_sort("created_at")>
                                "Создано"
                                <span class={move || get_sort_class(&state.get().sort_field, "created_at")}>
                                    {move || get_sort_indicator(&state.get().sort_field, "created_at", state.get().sort_ascending)}
                                </span>
                            </th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || sorted_items().into_iter().map(|row| {
                            let id = row.id.clone();
                            let id_for_checkbox = id.clone();
                            let id_for_toggle = id.clone();
                            let id_for_selected = id.clone();
                            let id_for_click = id.clone();
                            let logo_path = row.logo_path.clone();
                            view! {
                                <tr
                                    class="table__row"
                                    class:table__row--selected={move || selected.get().contains(&id_for_selected)}
                                    on:click=move |_| handle_edit(id_for_click.clone())
                                >
                                    <TableCheckbox
                                        checked=Signal::derive(move || selected.get().contains(&id_for_checkbox))
                                        on_change=Callback::new(move |checked| toggle_select(id_for_toggle.clone(), checked))
                                    />
                                    <td class="table__cell">
                                        {move || if let Some(ref path) = logo_path {
                                            view! { <img src={path.clone()} alt="logo" style="max-width: 32px; max-height: 32px;" /> }.into_any()
                                        } else {
                                            view! { <span>{"-"}</span> }.into_any()
                                        }}
                                    </td>
                                    <td class="table__cell">{row.code}</td>
                                    <td class="table__cell">{row.description}</td>
                                    <td class="table__cell">{row.url}</td>
                                    <td class="table__cell">{row.comment}</td>
                                    <td class="table__cell">{row.created_at}</td>
                                </tr>
                            }
                        }).collect_view()}
                    </tbody>
                </table>
            </div>

            // Details is opened via ModalStackService
        </div>
    }
}

async fn fetch_marketplaces() -> Result<Vec<Marketplace>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/marketplace", api_base());
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
    let data: Vec<Marketplace> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}

async fn delete_marketplace(id: &str) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("DELETE");
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
    Ok(())
}

async fn fill_test_data() -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/marketplace/testdata", api_base());
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
