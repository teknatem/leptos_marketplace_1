pub mod state;

use self::state::create_state;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::components::date_input::DateInput;
use crate::shared::components::month_selector::MonthSelector;
use crate::shared::date_utils::format_datetime;
use crate::shared::list_utils::{format_number, get_sort_class, get_sort_indicator, Sortable};
use crate::shared::table_utils::{init_column_resize, was_just_resizing};
use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::cmp::Ordering;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

const TABLE_ID: &str = "a016-ym-returns-table";
const COLUMN_WIDTHS_KEY: &str = "a016_ym_returns_column_widths";

/// Paginated response from backend API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse {
    pub items: Vec<YmReturnDto>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmReturnDto {
    pub id: String,
    pub return_id: i64,
    pub order_id: i64,
    pub return_type: String,
    pub refund_status: String,
    pub total_items: i32,
    pub total_amount: f64,
    pub created_at_source: String,
    pub fetched_at: String,
    pub is_posted: bool,
}

impl Sortable for YmReturnDto {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "return_id" => self.return_id.cmp(&other.return_id),
            "order_id" => self.order_id.cmp(&other.order_id),
            "return_type" => self
                .return_type
                .to_lowercase()
                .cmp(&other.return_type.to_lowercase()),
            "refund_status" => self
                .refund_status
                .to_lowercase()
                .cmp(&other.refund_status.to_lowercase()),
            "total_items" => self.total_items.cmp(&other.total_items),
            "total_amount" => self
                .total_amount
                .partial_cmp(&other.total_amount)
                .unwrap_or(Ordering::Equal),
            "created_at_source" => self.created_at_source.cmp(&other.created_at_source),
            "fetched_at" => self.fetched_at.cmp(&other.fetched_at),
            _ => Ordering::Equal,
        }
    }
}

#[component]
pub fn YmReturnsList() -> impl IntoView {
    let state = create_state();
    let global_ctx = expect_context::<AppGlobalContext>();

    let (items, set_items) = signal::<Vec<YmReturnDto>>(Vec::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (posting_in_progress, set_posting_in_progress) = signal(false);

    // Load data function
    let load_data = move || {
        let current_state = state.get();
        set_loading.set(true);
        set_error.set(None);

        spawn_local(async move {
            let offset = current_state.page * current_state.page_size;
            let sort_desc = !current_state.sort_ascending;

            let mut url = format!(
                "http://localhost:3000/api/a016/ym-returns?limit={}&offset={}&sort_by={}&sort_desc={}&date_from={}&date_to={}",
                current_state.page_size,
                offset,
                current_state.sort_field,
                sort_desc,
                current_state.date_from,
                current_state.date_to
            );

            if let Some(ref t) = current_state.filter_type {
                url.push_str(&format!("&return_type={}", t));
            }
            if !current_state.search_return_id.is_empty() {
                url.push_str(&format!(
                    "&search_return_id={}",
                    current_state.search_return_id
                ));
            }
            if !current_state.search_order_id.is_empty() {
                url.push_str(&format!(
                    "&search_order_id={}",
                    current_state.search_order_id
                ));
            }

            match Request::get(&url).send().await {
                Ok(response) => {
                    if response.status() == 200 {
                        match response.json::<PaginatedResponse>().await {
                            Ok(data) => {
                                set_items.set(data.items);
                                state.update(|s| {
                                    s.total_count = data.total;
                                    s.total_pages = data.total_pages;
                                    s.is_loaded = true;
                                });
                                set_loading.set(false);
                            }
                            Err(e) => {
                                log!("Failed to parse response: {:?}", e);
                                set_error.set(Some(format!("Failed to parse: {}", e)));
                                set_loading.set(false);
                            }
                        }
                    } else {
                        set_error.set(Some(format!("Server error: {}", response.status())));
                        set_loading.set(false);
                    }
                }
                Err(e) => {
                    log!("Failed to fetch: {:?}", e);
                    set_error.set(Some(format!("Failed to fetch: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    };

    // Initial load - only once
    Effect::new(move |_| {
        if !state.with_untracked(|s| s.is_loaded) {
            load_data();
        }
    });

    // Init column resize after data is loaded
    Effect::new(move |_| {
        let is_loaded = state.get().is_loaded;
        if is_loaded {
            // Small delay to ensure DOM is ready
            spawn_local(async move {
                gloo_timers::future::TimeoutFuture::new(50).await;
                init_column_resize(TABLE_ID, COLUMN_WIDTHS_KEY);
            });
        }
    });

    // Handlers
    let toggle_sort = move |field: &'static str| {
        move |_| {
            if was_just_resizing() {
                return;
            }
            state.update(|s| {
                if s.sort_field == field {
                    s.sort_ascending = !s.sort_ascending;
                } else {
                    s.sort_field = field.to_string();
                    s.sort_ascending = true;
                }
                s.page = 0;
            });
            load_data();
        }
    };

    let go_to_page = move |page: usize| {
        state.update(|s| s.page = page);
        load_data();
    };

    let change_page_size = move |size: usize| {
        state.update(|s| {
            s.page_size = size;
            s.page = 0;
        });
        load_data();
    };

    let toggle_select = move |id: String| {
        state.update(|s| {
            if s.selected_ids.contains(&id) {
                s.selected_ids.retain(|x| x != &id);
            } else {
                s.selected_ids.push(id);
            }
        });
    };

    let toggle_select_all = move |_| {
        let current_items = items.get();
        let current_selected = state.get().selected_ids.clone();
        let all_on_page: Vec<String> = current_items.iter().map(|i| i.id.clone()).collect();

        if all_on_page.iter().all(|id| current_selected.contains(id)) {
            // Deselect all on page
            state.update(|s| {
                s.selected_ids.retain(|id| !all_on_page.contains(id));
            });
        } else {
            // Select all on page
            state.update(|s| {
                for id in all_on_page {
                    if !s.selected_ids.contains(&id) {
                        s.selected_ids.push(id);
                    }
                }
            });
        }
    };

    // Batch post
    let batch_post = move |_| {
        let ids = state.get().selected_ids.clone();
        if ids.is_empty() {
            return;
        }
        set_posting_in_progress.set(true);

        spawn_local(async move {
            let body = json!({ "ids": ids });
            match Request::post("http://localhost:3000/api/a016/ym-returns/batch-post")
                .header("Content-Type", "application/json")
                .body(body.to_string())
                .unwrap()
                .send()
                .await
            {
                Ok(resp) => {
                    if resp.status() == 200 {
                        state.update(|s| s.selected_ids.clear());
                        load_data();
                    }
                }
                Err(e) => log!("Batch post error: {:?}", e),
            }
            set_posting_in_progress.set(false);
        });
    };

    // Batch unpost
    let batch_unpost = move |_| {
        let ids = state.get().selected_ids.clone();
        if ids.is_empty() {
            return;
        }
        set_posting_in_progress.set(true);

        spawn_local(async move {
            let body = json!({ "ids": ids });
            match Request::post("http://localhost:3000/api/a016/ym-returns/batch-unpost")
                .header("Content-Type", "application/json")
                .body(body.to_string())
                .unwrap()
                .send()
                .await
            {
                Ok(resp) => {
                    if resp.status() == 200 {
                        state.update(|s| s.selected_ids.clear());
                        load_data();
                    }
                }
                Err(e) => log!("Batch unpost error: {:?}", e),
            }
            set_posting_in_progress.set(false);
        });
    };

    // Export to Excel
    let export_excel = move |_| {
        let data = items.get();
        let mut csv = String::from("\u{FEFF}"); // BOM for Excel
        csv.push_str("Return ID;Order ID;Тип;Статус;Кол-во;Сумма;Дата;Проведен\n");

        for item in data.iter() {
            csv.push_str(&format!(
                "{};{};{};{};{};{};{};{}\n",
                item.return_id,
                item.order_id,
                item.return_type,
                item.refund_status,
                item.total_items,
                format_number(item.total_amount),
                format_datetime(&item.created_at_source),
                if item.is_posted { "Да" } else { "Нет" }
            ));
        }

        // Download file
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                let parts = js_sys::Array::new();
                parts.push(&JsValue::from_str(&csv));
                let opts = BlobPropertyBag::new();
                opts.set_type("text/csv;charset=utf-8");
                if let Ok(blob) = Blob::new_with_str_sequence_and_options(&parts, &opts) {
                    if let Ok(url) = Url::create_object_url_with_blob(&blob) {
                        if let Ok(a) = document.create_element("a") {
                            if let Ok(anchor) = a.dyn_into::<HtmlAnchorElement>() {
                                anchor.set_href(&url);
                                anchor.set_download("ym_returns.csv");
                                anchor.click();
                                let _ = Url::revoke_object_url(&url);
                            }
                        }
                    }
                }
            }
        }
    };

    // Open detail view
    let open_detail = move |id: String| {
        global_ctx.open_tab(
            &format!("a016_ym_returns_detail_{}", id),
            &format!("YM Return {}", &id[..8.min(id.len())]),
        );
    };

    view! {
        <div class="list-container">
            // Header Row 1 - Title, Pagination, Actions
            <div class="list-header-row gradient-header">
                <div class="header-left">
                    <h2 class="list-title">"📦 YM Returns (A016)"</h2>
                </div>

                // Pagination
                <div class="pagination-controls">
                    <button
                        class="btn btn-icon-transparent"
                        on:click=move |_| go_to_page(0)
                        disabled=move || state.get().page == 0
                    >"⏮"</button>
                    <button
                        class="btn btn-icon-transparent"
                        on:click=move |_| {
                            let p = state.get().page;
                            if p > 0 { go_to_page(p - 1); }
                        }
                        disabled=move || state.get().page == 0
                    >"◀"</button>
                    <span class="pagination-info">
                        {move || {
                            let s = state.get();
                            format!("{} / {} ({})", s.page + 1, s.total_pages.max(1), s.total_count)
                        }}
                    </span>
                    <button
                        class="btn btn-icon-transparent"
                        on:click=move |_| {
                            let s = state.get();
                            if s.page + 1 < s.total_pages { go_to_page(s.page + 1); }
                        }
                        disabled=move || {
                            let s = state.get();
                            s.page + 1 >= s.total_pages
                        }
                    >"▶"</button>
                    <button
                        class="btn btn-icon-transparent"
                        on:click=move |_| {
                            let s = state.get();
                            if s.total_pages > 0 { go_to_page(s.total_pages - 1); }
                        }
                        disabled=move || {
                            let s = state.get();
                            s.page + 1 >= s.total_pages
                        }
                    >"⏭"</button>
                    <select
                        class="page-size-select"
                        on:change=move |ev| {
                            let val = event_target_value(&ev).parse().unwrap_or(100);
                            change_page_size(val);
                        }
                    >
                        <option value="50" selected=move || state.get().page_size == 50>"50"</option>
                        <option value="100" selected=move || state.get().page_size == 100>"100"</option>
                        <option value="200" selected=move || state.get().page_size == 200>"200"</option>
                        <option value="500" selected=move || state.get().page_size == 500>"500"</option>
                    </select>
                </div>

                // Action buttons
                <div class="header-right">
                    <button
                        class="btn btn-success"
                        on:click=batch_post
                        disabled=move || state.get().selected_ids.is_empty() || posting_in_progress.get()
                    >
                        {move || format!("✓ Post ({})", state.get().selected_ids.len())}
                    </button>
                    <button
                        class="btn btn-warning"
                        on:click=batch_unpost
                        disabled=move || state.get().selected_ids.is_empty() || posting_in_progress.get()
                    >
                        {move || format!("✗ Unpost ({})", state.get().selected_ids.len())}
                    </button>
                    <button class="btn btn-excel" on:click=export_excel>"📊 Excel"</button>
                </div>
            </div>

            // Header Row 2 - Filters
            <div class="list-header-row filters-row">
                <div class="filter-group">
                    <label>"Период:"</label>
                    <DateInput
                        value=Signal::derive(move || state.get().date_from)
                        on_change=move |val| {
                            state.update(|s| { s.date_from = val; s.page = 0; });
                            load_data();
                        }
                    />
                    <span>" — "</span>
                    <DateInput
                        value=Signal::derive(move || state.get().date_to)
                        on_change=move |val| {
                            state.update(|s| { s.date_to = val; s.page = 0; });
                            load_data();
                        }
                    />
                    <MonthSelector
                        on_select=Callback::new(move |(from, to)| {
                            state.update(|s| {
                                s.date_from = from;
                                s.date_to = to;
                                s.page = 0;
                            });
                            load_data();
                        })
                    />
                </div>

                <div class="filter-group">
                    <label>"Return ID:"</label>
                    <input
                        type="text"
                        class="filter-input"
                        placeholder="Поиск..."
                        prop:value=move || state.get().search_return_id
                        on:input=move |ev| {
                            state.update(|s| {
                                s.search_return_id = event_target_value(&ev);
                                s.page = 0;
                            });
                        }
                        on:keydown=move |ev| {
                            if ev.key() == "Enter" {
                                load_data();
                            }
                        }
                    />
                </div>

                <div class="filter-group">
                    <label>"Order ID:"</label>
                    <input
                        type="text"
                        class="filter-input"
                        placeholder="Поиск..."
                        prop:value=move || state.get().search_order_id
                        on:input=move |ev| {
                            state.update(|s| {
                                s.search_order_id = event_target_value(&ev);
                                s.page = 0;
                            });
                        }
                        on:keydown=move |ev| {
                            if ev.key() == "Enter" {
                                load_data();
                            }
                        }
                    />
                </div>

                <div class="filter-group">
                    <label>"Тип:"</label>
                    <select
                        class="filter-select"
                        on:change=move |ev| {
                            let val = event_target_value(&ev);
                            state.update(|s| {
                                s.filter_type = if val.is_empty() { None } else { Some(val) };
                                s.page = 0;
                            });
                            load_data();
                        }
                    >
                        <option value="">"Все"</option>
                        <option value="RETURN">"Возврат"</option>
                        <option value="UNREDEEMED">"Невыкуп"</option>
                    </select>
                </div>

                <button class="btn btn-primary" on:click=move |_| load_data()>
                    {move || if loading.get() { "⏳ Загрузка..." } else { "🔄 Обновить" }}
                </button>
            </div>

            // Error message
            {move || {
                if let Some(err) = error.get() {
                    view! {
                        <div class="error-message">
                            <strong>"Ошибка: "</strong>{err}
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }}

            // Table
            <div class="table-container">
                <table id=TABLE_ID class="data-table">
                    <thead>
                        <tr>
                            <th class="checkbox-cell">
                                <input
                                    type="checkbox"
                                    on:change=toggle_select_all
                                    prop:checked=move || {
                                        let current_items = items.get();
                                        let selected = state.get().selected_ids;
                                        !current_items.is_empty() && current_items.iter().all(|i| selected.contains(&i.id))
                                    }
                                />
                            </th>
                            <th class="resizable" on:click=toggle_sort("created_at_source")>
                                <span class="sortable-header">
                                    "Дата"
                                    <span class={move || get_sort_class("created_at_source", &state.get().sort_field)}>{move || get_sort_indicator("created_at_source", &state.get().sort_field, state.get().sort_ascending)}</span>
                                </span>
                            </th>
                            <th class="resizable" on:click=toggle_sort("return_id")>
                                <span class="sortable-header">
                                    "Return №"
                                    <span class={move || get_sort_class("return_id", &state.get().sort_field)}>{move || get_sort_indicator("return_id", &state.get().sort_field, state.get().sort_ascending)}</span>
                                </span>
                            </th>
                            <th class="resizable" on:click=toggle_sort("order_id")>
                                <span class="sortable-header">
                                    "Order №"
                                    <span class={move || get_sort_class("order_id", &state.get().sort_field)}>{move || get_sort_indicator("order_id", &state.get().sort_field, state.get().sort_ascending)}</span>
                                </span>
                            </th>
                            <th class="resizable" on:click=toggle_sort("return_type")>
                                <span class="sortable-header">
                                    "Тип"
                                    <span class={move || get_sort_class("return_type", &state.get().sort_field)}>{move || get_sort_indicator("return_type", &state.get().sort_field, state.get().sort_ascending)}</span>
                                </span>
                            </th>
                            <th class="resizable" on:click=toggle_sort("refund_status")>
                                <span class="sortable-header">
                                    "Статус"
                                    <span class={move || get_sort_class("refund_status", &state.get().sort_field)}>{move || get_sort_indicator("refund_status", &state.get().sort_field, state.get().sort_ascending)}</span>
                                </span>
                            </th>
                            <th class="resizable text-right">"Шт."</th>
                            <th class="resizable text-right">"Сумма"</th>
                            <th class="text-center">"✓"</th>
                        </tr>
                        // Totals row
                        <tr class="totals-header-row">
                            <td class="checkbox-cell"></td>
                            <td>
                                {move || format!("Записей: {}", items.get().len())}
                            </td>
                            <td></td>
                            <td></td>
                            <td>
                                {move || {
                                    let data = items.get();
                                    let returns = data.iter().filter(|r| r.return_type == "RETURN").count();
                                    let unredeemed = data.iter().filter(|r| r.return_type == "UNREDEEMED").count();
                                    format!("Возвр: {} / Невык: {}", returns, unredeemed)
                                }}
                            </td>
                            <td></td>
                            <td class="text-right">
                                {move || {
                                    let sum: i32 = items.get().iter().map(|i| i.total_items).sum();
                                    format!("{}", sum)
                                }}
                            </td>
                            <td class="text-right">
                                {move || {
                                    let sum: f64 = items.get().iter().map(|i| i.total_amount).sum();
                                    format_number(sum)
                                }}
                            </td>
                            <td></td>
                        </tr>
                    </thead>
                    <tbody>
                        {move || {
                            items.get().into_iter().map(|item| {
                                let id = item.id.clone();
                                let id_for_click = id.clone();
                                let id_for_checkbox = id.clone();
                                let is_selected = state.get().selected_ids.contains(&id);

                                let return_type_class = match item.return_type.as_str() {
                                    "UNREDEEMED" => "badge badge-warning",
                                    "RETURN" => "badge badge-info",
                                    _ => "badge",
                                };
                                let return_type_label = match item.return_type.as_str() {
                                    "UNREDEEMED" => "Невыкуп".to_string(),
                                    "RETURN" => "Возврат".to_string(),
                                    _ => item.return_type.clone(),
                                };
                                let refund_status = item.refund_status.clone();

                                let status_class = match item.refund_status.as_str() {
                                    "REFUNDED" => "badge badge-success",
                                    "NOT_REFUNDED" => "badge badge-danger",
                                    "REFUND_IN_PROGRESS" => "badge badge-warning",
                                    _ => "badge",
                                };

                                view! {
                                    <tr
                                        class=move || if is_selected { "selected" } else { "" }
                                    >
                                        <td class="checkbox-cell">
                                            <input
                                                type="checkbox"
                                                prop:checked=is_selected
                                                on:change=move |_| toggle_select(id_for_checkbox.clone())
                                            />
                                        </td>
                                        <td on:click=move |_| open_detail(id_for_click.clone())>
                                            {format_datetime(&item.created_at_source)}
                                        </td>
                                        <td on:click=move |_| open_detail(id.clone()) class="font-bold text-primary">
                                            {item.return_id}
                                        </td>
                                        <td>{item.order_id}</td>
                                        <td>
                                            <span class=return_type_class>{return_type_label}</span>
                                        </td>
                                        <td>
                                            <span class=status_class>{refund_status}</span>
                                        </td>
                                        <td class="text-right">{item.total_items}</td>
                                        <td class="text-right font-medium">{format_number(item.total_amount)}</td>
                                        <td class="text-center">
                                            {if item.is_posted {
                                                view! { <span class="text-success">"✓"</span> }.into_any()
                                            } else {
                                                view! { <span class="text-muted">"—"</span> }.into_any()
                                            }}
                                        </td>
                                    </tr>
                                }
                            }).collect_view()
                        }}
                    </tbody>
                </table>
            </div>
        </div>
    }
}
