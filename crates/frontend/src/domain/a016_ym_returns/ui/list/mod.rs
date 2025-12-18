pub mod state;

use self::state::create_state;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::components::date_input::DateInput;
use crate::shared::components::month_selector::MonthSelector;
use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::components::ui::badge::Badge;
use crate::shared::components::ui::button::Button;
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

    // Filter panel expansion state
    let (is_filter_expanded, set_is_filter_expanded) = signal(false);

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

    // Count active filters
    let active_filters_count = Signal::derive(move || {
        let s = state.get();
        let mut count = 0;
        if !s.date_from.is_empty() {
            count += 1;
        }
        if !s.date_to.is_empty() {
            count += 1;
        }
        if !s.search_return_id.is_empty() {
            count += 1;
        }
        if !s.search_order_id.is_empty() {
            count += 1;
        }
        if s.filter_type.is_some() {
            count += 1;
        }
        count
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

    // Clear all filters
    let clear_all_filters = move |_| {
        state.update(|s| {
            s.date_from = String::new();
            s.date_to = String::new();
            s.search_return_id = String::new();
            s.search_order_id = String::new();
            s.filter_type = None;
            s.page = 0;
        });
        load_data();
    };

    // Remove individual filters
    let remove_date_from = Callback::new(move |_| {
        state.update(|s| {
            s.date_from = String::new();
            s.page = 0;
        });
        load_data();
    });

    let remove_date_to = Callback::new(move |_| {
        state.update(|s| {
            s.date_to = String::new();
            s.page = 0;
        });
        load_data();
    });

    let remove_search_return_id = Callback::new(move |_| {
        state.update(|s| {
            s.search_return_id = String::new();
            s.page = 0;
        });
        load_data();
    });

    let remove_search_order_id = Callback::new(move |_| {
        state.update(|s| {
            s.search_order_id = String::new();
            s.page = 0;
        });
        load_data();
    });

    let remove_filter_type = Callback::new(move |_| {
        state.update(|s| {
            s.filter_type = None;
            s.page = 0;
        });
        load_data();
    });

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
        <div class="page">
            <div class="page-header">
                <div class="page-header__content">
                    <div class="page-header__icon">{icon("refresh")}</div>
                    <div class="page-header__text">
                        <h1 class="page-header__title">"Возвраты Яндекс Маркет"</h1>
                        <div class="page-header__badge">
                            <Badge variant="primary".to_string()>
                                {move || state.get().total_count.to_string()}
                            </Badge>
                        </div>
                    </div>
                </div>
                <div class="page-header__actions">
                    <Button
                        variant="primary".to_string()
                        on_click=Callback::new(move |_| load_data())
                        disabled=loading.get()
                    >
                        {icon("refresh")}
                        {move || if loading.get() { "Загрузка..." } else { "Обновить" }}
                    </Button>
                    <Button
                        variant="primary".to_string()
                        on_click=Callback::new(batch_post)
                        disabled=state.get().selected_ids.is_empty() || posting_in_progress.get()
                    >
                        {icon("check")}
                        {move || format!("Post ({})", state.get().selected_ids.len())}
                    </Button>
                    <Button
                        variant="secondary".to_string()
                        on_click=Callback::new(batch_unpost)
                        disabled=state.get().selected_ids.is_empty() || posting_in_progress.get()
                    >
                        {icon("x")}
                        {move || format!("Unpost ({})", state.get().selected_ids.len())}
                    </Button>
                    <Button
                        variant="secondary".to_string()
                        on_click=Callback::new(export_excel)
                        disabled=false
                    >
                        {icon("download")}
                        "Excel"
                    </Button>
                </div>
            </div>

            <div class="filter-panel">
                <div class="filter-panel-header">
                    <div
                        class="filter-panel-header__left"
                        on:click=move |_| set_is_filter_expanded.update(|e| *e = !*e)
                    >
                        <svg
                            width="16"
                            height="16"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            class=move || {
                                if is_filter_expanded.get() {
                                    "filter-panel__chevron filter-panel__chevron--expanded"
                                } else {
                                    "filter-panel__chevron"
                                }
                            }
                        >
                            <polyline points="6 9 12 15 18 9"></polyline>
                        </svg>
                        {icon("filter")}
                        <span class="filter-panel__title">"Фильтры"</span>
                        {move || {
                            let count = active_filters_count.get();
                            if count > 0 {
                                view! {
                                    <Badge variant="primary".to_string()>{count}</Badge>
                                }.into_any()
                            } else {
                                view! { <></> }.into_any()
                            }
                        }}
                    </div>
                    <div class="filter-panel-header__center">
                        <PaginationControls
                            current_page=Signal::derive(move || state.get().page)
                            total_pages=Signal::derive(move || state.get().total_pages)
                            total_count=Signal::derive(move || state.get().total_count)
                            page_size=Signal::derive(move || state.get().page_size)
                            on_page_change=Callback::new(go_to_page)
                            on_page_size_change=Callback::new(change_page_size)
                        />
                    </div>
                </div>

                <div class=move || {
                    if is_filter_expanded.get() {
                        "filter-panel__collapsible filter-panel__collapsible--expanded"
                    } else {
                        "filter-panel__collapsible filter-panel__collapsible--collapsed"
                    }
                }>
                    <div class="filter-panel-content">
                        <div class="filter-grid">
                            <div class="form__group">
                                <label class="form__label">"Период:"</label>
                                <div style="display: flex; gap: var(--spacing-xs); align-items: center; flex-wrap: nowrap; overflow-x: auto;">
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

                        {move || {
                            let has_filters = active_filters_count.get() > 0;

                            if has_filters {
                                view! {
                                    <div class="filter-tags">
                                        {move || {
                                            let s = state.get();
                                            let mut tags = Vec::new();

                                            if !s.date_from.is_empty() {
                                                let date_from = s.date_from.clone();
                                                tags.push(view! {
                                                    <div class="filter-tag">
                                                        <span>{format!("От: {}", date_from)}</span>
                                                        <svg
                                                            width="12"
                                                            height="12"
                                                            viewBox="0 0 24 24"
                                                            fill="none"
                                                            stroke="currentColor"
                                                            stroke-width="2"
                                                            stroke-linecap="round"
                                                            stroke-linejoin="round"
                                                            class="filter-tag__remove"
                                                            on:click=move |e| {
                                                                e.stop_propagation();
                                                                remove_date_from.run(());
                                                            }
                                                        >
                                                            <line x1="18" y1="6" x2="6" y2="18"></line>
                                                            <line x1="6" y1="6" x2="18" y2="18"></line>
                                                        </svg>
                                                    </div>
                                                }.into_any());
                                            }

                                            if !s.date_to.is_empty() {
                                                let date_to = s.date_to.clone();
                                                tags.push(view! {
                                                    <div class="filter-tag">
                                                        <span>{format!("До: {}", date_to)}</span>
                                                        <svg
                                                            width="12"
                                                            height="12"
                                                            viewBox="0 0 24 24"
                                                            fill="none"
                                                            stroke="currentColor"
                                                            stroke-width="2"
                                                            stroke-linecap="round"
                                                            stroke-linejoin="round"
                                                            class="filter-tag__remove"
                                                            on:click=move |e| {
                                                                e.stop_propagation();
                                                                remove_date_to.run(());
                                                            }
                                                        >
                                                            <line x1="18" y1="6" x2="6" y2="18"></line>
                                                            <line x1="6" y1="6" x2="18" y2="18"></line>
                                                        </svg>
                                                    </div>
                                                }.into_any());
                                            }

                                            if !s.search_return_id.is_empty() {
                                                let search_return_id = s.search_return_id.clone();
                                                tags.push(view! {
                                                    <div class="filter-tag">
                                                        <span>{format!("Return ID: {}", search_return_id)}</span>
                                                        <svg
                                                            width="12"
                                                            height="12"
                                                            viewBox="0 0 24 24"
                                                            fill="none"
                                                            stroke="currentColor"
                                                            stroke-width="2"
                                                            stroke-linecap="round"
                                                            stroke-linejoin="round"
                                                            class="filter-tag__remove"
                                                            on:click=move |e| {
                                                                e.stop_propagation();
                                                                remove_search_return_id.run(());
                                                            }
                                                        >
                                                            <line x1="18" y1="6" x2="6" y2="18"></line>
                                                            <line x1="6" y1="6" x2="18" y2="18"></line>
                                                        </svg>
                                                    </div>
                                                }.into_any());
                                            }

                                            if !s.search_order_id.is_empty() {
                                                let search_order_id = s.search_order_id.clone();
                                                tags.push(view! {
                                                    <div class="filter-tag">
                                                        <span>{format!("Order ID: {}", search_order_id)}</span>
                                                        <svg
                                                            width="12"
                                                            height="12"
                                                            viewBox="0 0 24 24"
                                                            fill="none"
                                                            stroke="currentColor"
                                                            stroke-width="2"
                                                            stroke-linecap="round"
                                                            stroke-linejoin="round"
                                                            class="filter-tag__remove"
                                                            on:click=move |e| {
                                                                e.stop_propagation();
                                                                remove_search_order_id.run(());
                                                            }
                                                        >
                                                            <line x1="18" y1="6" x2="6" y2="18"></line>
                                                            <line x1="6" y1="6" x2="18" y2="18"></line>
                                                        </svg>
                                                    </div>
                                                }.into_any());
                                            }

                                            if let Some(ref filter_type) = s.filter_type {
                                                let type_label = match filter_type.as_str() {
                                                    "RETURN" => "Возврат",
                                                    "UNREDEEMED" => "Невыкуп",
                                                    _ => filter_type,
                                                }.to_string();
                                                tags.push(view! {
                                                    <div class="filter-tag">
                                                        <span>{format!("Тип: {}", type_label)}</span>
                                                        <svg
                                                            width="12"
                                                            height="12"
                                                            viewBox="0 0 24 24"
                                                            fill="none"
                                                            stroke="currentColor"
                                                            stroke-width="2"
                                                            stroke-linecap="round"
                                                            stroke-linejoin="round"
                                                            class="filter-tag__remove"
                                                            on:click=move |e| {
                                                                e.stop_propagation();
                                                                remove_filter_type.run(());
                                                            }
                                                        >
                                                            <line x1="18" y1="6" x2="6" y2="18"></line>
                                                            <line x1="6" y1="6" x2="18" y2="18"></line>
                                                        </svg>
                                                    </div>
                                                }.into_any());
                                            }

                                            tags.push(view! {
                                                <Button
                                                    variant="ghost".to_string()
                                                    size="sm".to_string()
                                                    on_click=Callback::new(clear_all_filters)
                                                    disabled=false
                                                >
                                                    "Очистить все"
                                                </Button>
                                            }.into_any());

                                            tags.into_iter().collect_view()
                                        }}
                                    </div>
                                }.into_any()
                            } else {
                                view! { <></> }.into_any()
                            }
                        }}
                    </div>
                </div>
            </div>

            // Error message
            {move || {
                if let Some(err) = error.get() {
                    view! {
                        <div class="warning-box" style="background: var(--color-error-50); border-color: var(--color-error-100); margin: 0 var(--spacing-sm) var(--spacing-xs) var(--spacing-sm);">
                            <span class="warning-box__icon" style="color: var(--color-error);">"⚠"</span>
                            <span class="warning-box__text" style="color: var(--color-error);">{err}</span>
                        </div>
                    }.into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }}

            <div class="page-content">
                <div class="list-container">
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
        </div>
    }
}
