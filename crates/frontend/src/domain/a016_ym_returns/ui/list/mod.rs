pub mod state;

use self::state::create_state;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::components::date_input::DateInput;
use crate::shared::components::month_selector::MonthSelector;
use crate::shared::date_utils::format_datetime;
use crate::shared::icons::icon;
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
        <div style="display: flex; flex-direction: column; height: 100%; width: 100%;">
            // Заголовок страницы с пагинацией в центре
            <div class="list-header-row gradient-header" style="padding: 8px 12px; flex-shrink: 0;">
                <div class="header-left">
                    <h2 class="list-title">"YM Returns"</h2>
                </div>

                // Пагинация в центре
                <div style="display: flex; align-items: center; gap: 8px;">
                    <button
                        class="button button--primary"
                        style="min-width: 32px; padding: 4px 8px;"
                        on:click=move |_| go_to_page(0)
                        disabled=move || state.get().page == 0
                        title="В начало"
                    >
                        {icon("chevrons-left")}
                    </button>
                    <button
                        class="button button--primary"
                        style="min-width: 32px; padding: 4px 8px;"
                        on:click=move |_| {
                            let p = state.get().page;
                            if p > 0 { go_to_page(p - 1); }
                        }
                        disabled=move || state.get().page == 0
                        title="Назад"
                    >
                        {icon("chevron-left")}
                    </button>
                    <span class="pagination-info">
                        {move || {
                            let s = state.get();
                            format!("{} / {} ({})", s.page + 1, s.total_pages.max(1), s.total_count)
                        }}
                    </span>
                    <button
                        class="button button--primary"
                        style="min-width: 32px; padding: 4px 8px;"
                        on:click=move |_| {
                            let s = state.get();
                            if s.page + 1 < s.total_pages { go_to_page(s.page + 1); }
                        }
                        disabled=move || {
                            let s = state.get();
                            s.page + 1 >= s.total_pages
                        }
                        title="Вперёд"
                    >
                        {icon("chevron-right")}
                    </button>
                    <button
                        class="button button--primary"
                        style="min-width: 32px; padding: 4px 8px;"
                        on:click=move |_| {
                            let s = state.get();
                            if s.total_pages > 0 { go_to_page(s.total_pages - 1); }
                        }
                        disabled=move || {
                            let s = state.get();
                            s.page + 1 >= s.total_pages
                        }
                        title="В конец"
                    >
                        {icon("chevrons-right")}
                    </button>
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

                // Кнопки действий справа
                <div class="header-right">
                    <button
                        class="button button--primary"
                        on:click=move |_| load_data()
                        disabled=move || loading.get()
                    >
                        {icon("refresh-cw")}
                        {move || if loading.get() { "Загрузка..." } else { "Обновить" }}
                    </button>
                    <button
                        class="button button--primary"
                        on:click=batch_post
                        disabled=move || state.get().selected_ids.is_empty() || posting_in_progress.get()
                    >
                        {icon("check")}
                        {move || format!("Post ({})", state.get().selected_ids.len())}
                    </button>
                    <button
                        class="button button--warning"
                        on:click=batch_unpost
                        disabled=move || state.get().selected_ids.is_empty() || posting_in_progress.get()
                    >
                        {icon("x")}
                        {move || format!("Unpost ({})", state.get().selected_ids.len())}
                    </button>
                    <button class="button button--secondary" on:click=export_excel>
                        {icon("download")}
                        "Excel"
                    </button>
                </div>
            </div>

            // Фильтры
            <div class="form-section" style="background: var(--color-background-secondary); padding: 12px; margin: 8px 12px; border-radius: 4px; flex-shrink: 0;">
                <div style="display: grid; grid-template-columns: 2fr 1fr 1fr 1fr; gap: 12px;">
                    <div class="form__group" style="grid-column: span 1;">
                        <label class="form__label">"Период:"</label>
                        <div style="display: flex; gap: var(--spacing-xs); align-items: center; flex-wrap: nowrap;">
                            <DateInput
                                value=Signal::derive(move || state.get().date_from)
                                on_change=move |val| {
                                    state.update(|s| { s.date_from = val; s.page = 0; });
                                    load_data();
                                }
                            />
                            <span style="white-space: nowrap;">" — "</span>
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
                    </div>

                    <div class="form__group">
                        <label class="form__label">"Return ID:"</label>
                        <input
                            type="text"
                            class="form__input"
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

                    <div class="form__group">
                        <label class="form__label">"Order ID:"</label>
                        <input
                            type="text"
                            class="form__input"
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

                    <div class="form__group">
                        <label class="form__label">"Тип:"</label>
                        <select
                            class="form__select"
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
                </div>
            </div>

            // Error message
            {move || {
                if let Some(err) = error.get() {
                    view! {
                        <div class="warning-box" style="background: var(--color-error-50); border-color: var(--color-error-100); margin: 0 12px 8px 12px;">
                            <span class="warning-box__icon" style="color: var(--color-error);">"⚠"</span>
                            <span class="warning-box__text" style="color: var(--color-error);">{err}</span>
                        </div>
                    }.into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }}

            // Table
            <div class="table" style="margin: 0 12px 12px 12px; flex: 1; overflow: hidden;">
                <table id=TABLE_ID class="table__data table--striped">
                    <thead class="table__head">
                        <tr>
                            <th class="table__header-cell table__header-cell--checkbox">
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
                            <th class="table__header-cell table__header-cell--sortable resizable" on:click=toggle_sort("created_at_source")>
                                "Дата"
                                <span class={move || get_sort_class("created_at_source", &state.get().sort_field)}>{move || get_sort_indicator("created_at_source", &state.get().sort_field, state.get().sort_ascending)}</span>
                            </th>
                            <th class="table__header-cell table__header-cell--sortable resizable" on:click=toggle_sort("return_id")>
                                "Return №"
                                <span class={move || get_sort_class("return_id", &state.get().sort_field)}>{move || get_sort_indicator("return_id", &state.get().sort_field, state.get().sort_ascending)}</span>
                            </th>
                            <th class="table__header-cell table__header-cell--sortable resizable" on:click=toggle_sort("order_id")>
                                "Order №"
                                <span class={move || get_sort_class("order_id", &state.get().sort_field)}>{move || get_sort_indicator("order_id", &state.get().sort_field, state.get().sort_ascending)}</span>
                            </th>
                            <th class="table__header-cell table__header-cell--sortable resizable" on:click=toggle_sort("return_type")>
                                "Тип"
                                <span class={move || get_sort_class("return_type", &state.get().sort_field)}>{move || get_sort_indicator("return_type", &state.get().sort_field, state.get().sort_ascending)}</span>
                            </th>
                            <th class="table__header-cell table__header-cell--sortable resizable" on:click=toggle_sort("refund_status")>
                                "Статус"
                                <span class={move || get_sort_class("refund_status", &state.get().sort_field)}>{move || get_sort_indicator("refund_status", &state.get().sort_field, state.get().sort_ascending)}</span>
                            </th>
                            <th class="table__header-cell table__header-cell--right resizable">"Шт."</th>
                            <th class="table__header-cell table__header-cell--right resizable">"Сумма"</th>
                            <th class="table__header-cell table__header-cell--center">"✓"</th>
                        </tr>
                        // Totals row
                        <tr style="background: var(--color-background-tertiary); font-weight: 500;">
                            <td class="table__header-cell table__header-cell--checkbox"></td>
                            <td class="table__header-cell">
                                {move || format!("Записей: {}", items.get().len())}
                            </td>
                            <td class="table__header-cell"></td>
                            <td class="table__header-cell"></td>
                            <td class="table__header-cell">
                                {move || {
                                    let data = items.get();
                                    let returns = data.iter().filter(|r| r.return_type == "RETURN").count();
                                    let unredeemed = data.iter().filter(|r| r.return_type == "UNREDEEMED").count();
                                    format!("Возвр: {} / Невык: {}", returns, unredeemed)
                                }}
                            </td>
                            <td class="table__header-cell"></td>
                            <td class="table__header-cell table__header-cell--right">
                                {move || {
                                    let sum: i32 = items.get().iter().map(|i| i.total_items).sum();
                                    format!("{}", sum)
                                }}
                            </td>
                            <td class="table__header-cell table__header-cell--right">
                                {move || {
                                    let sum: f64 = items.get().iter().map(|i| i.total_amount).sum();
                                    format_number(sum)
                                }}
                            </td>
                            <td class="table__header-cell"></td>
                        </tr>
                    </thead>
                    <tbody>
                        {move || {
                            items.get().into_iter().map(|item| {
                                let id = item.id.clone();
                                let id_for_click = id.clone();
                                let id_for_checkbox = id.clone();
                                let is_selected = state.get().selected_ids.contains(&id);

                                let return_type_style = match item.return_type.as_str() {
                                    "UNREDEEMED" => "background: var(--color-warning); color: white;",
                                    "RETURN" => "background: var(--color-info); color: white;",
                                    _ => "background: var(--color-border); color: var(--color-text-primary);",
                                };
                                let return_type_label = match item.return_type.as_str() {
                                    "UNREDEEMED" => "Невыкуп".to_string(),
                                    "RETURN" => "Возврат".to_string(),
                                    _ => item.return_type.clone(),
                                };
                                let refund_status = item.refund_status.clone();

                                let status_style = match item.refund_status.as_str() {
                                    "REFUNDED" => "background: var(--color-success); color: white;",
                                    "NOT_REFUNDED" => "background: var(--color-error); color: white;",
                                    "REFUND_IN_PROGRESS" => "background: var(--color-warning); color: white;",
                                    _ => "background: var(--color-border); color: var(--color-text-primary);",
                                };

                                view! {
                                    <tr
                                        class="table__row"
                                        class:table__row--selected=is_selected
                                    >
                                        <td class="table__cell table__cell--checkbox">
                                            <input
                                                type="checkbox"
                                                prop:checked=is_selected
                                                on:change=move |_| toggle_select(id_for_checkbox.clone())
                                            />
                                        </td>
                                        <td class="table__cell" style="cursor: pointer;" on:click=move |_| open_detail(id_for_click.clone())>
                                            {format_datetime(&item.created_at_source)}
                                        </td>
                                        <td class="table__cell" style="cursor: pointer; font-weight: 600; color: var(--color-primary);" on:click=move |_| open_detail(id.clone())>
                                            {item.return_id}
                                        </td>
                                        <td class="table__cell">{item.order_id}</td>
                                        <td class="table__cell">
                                            <span style={format!("padding: 2px 8px; border-radius: var(--radius-sm); font-size: var(--font-size-xs); {}", return_type_style)}>{return_type_label}</span>
                                        </td>
                                        <td class="table__cell">
                                            <span style={format!("padding: 2px 8px; border-radius: var(--radius-sm); font-size: var(--font-size-xs); {}", status_style)}>{refund_status}</span>
                                        </td>
                                        <td class="table__cell table__cell--right">{item.total_items}</td>
                                        <td class="table__cell table__cell--right" style="font-weight: 500;">{format_number(item.total_amount)}</td>
                                        <td class="table__cell table__cell--center">
                                            {if item.is_posted {
                                                view! { <span style="color: var(--color-success); font-weight: bold;">"✓"</span> }.into_any()
                                            } else {
                                                view! { <span style="color: var(--color-text-tertiary);">"—"</span> }.into_any()
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
