pub mod state;

use self::state::create_state;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::components::date_range_picker::DateRangePicker;
use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::components::table::{
    TableCellCheckbox, TableCellMoney, TableCrosshairHighlight, TableHeaderCheckbox,
};
use crate::shared::components::ui::badge::Badge as UiBadge;
use crate::shared::icons::icon;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator, Sortable};
use crate::shared::page_frame::PageFrame;
use crate::shared::table_utils::init_column_resize;
use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use thaw::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures;
use web_sys::{
    Blob, BlobPropertyBag, HtmlAnchorElement, Request as WebRequest, RequestInit, RequestMode,
    Response, Url,
};

use crate::shared::api_utils::api_base;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: String,
    pub code: String,
    pub description: String,
}

/// Форматирует ISO 8601 дату в dd.mm.yyyy
fn format_date(iso_date: &str) -> String {
    if let Some(date_part) = iso_date.split('T').next() {
        if let Some((year, rest)) = date_part.split_once('-') {
            if let Some((month, day)) = rest.split_once('-') {
                return format!("{}.{}.{}", day, month, year);
            }
        }
    }
    iso_date.to_string()
}

/// Форматирует ISO 8601 время в hh:mm:ss
fn format_time(iso_date: &str) -> String {
    if let Some((_, time_part)) = iso_date.split_once('T') {
        let time_clean = time_part
            .split('Z')
            .next()
            .unwrap_or(time_part)
            .split('+')
            .next()
            .unwrap_or(time_part);
        let time_clean = time_clean
            .split('-')
            .next()
            .unwrap_or(time_clean);
        if let Some(hms) = time_clean.split('.').next() {
            let mut parts = hms.split(':');
            if let (Some(h), Some(m), Some(s)) = (parts.next(), parts.next(), parts.next()) {
                return format!("{}:{}:{}", h, m, s);
            }
        }
    }
    "—".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WbOrdersDto {
    pub id: String,
    pub document_no: String,
    pub order_date: String,
    pub supplier_article: String,
    pub brand: Option<String>,
    pub qty: f64,
    pub margin_pro: Option<f64>,
    pub dealer_price_ut: Option<f64>,
    pub finished_price: Option<f64>,
    pub total_price: Option<f64>,
    pub is_cancel: bool,
    pub has_wb_sales: Option<bool>,
    pub organization_name: Option<String>,
    pub marketplace_article: Option<String>,
    pub nomenclature_code: Option<String>,
    pub nomenclature_article: Option<String>,
    pub base_nomenclature_article: Option<String>,
    pub base_nomenclature_description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse {
    pub items: Vec<serde_json::Value>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

impl Sortable for WbOrdersDto {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "document_no" => self
                .document_no
                .to_lowercase()
                .cmp(&other.document_no.to_lowercase()),
            "order_date" => self.order_date.cmp(&other.order_date),
            "supplier_article" => self
                .supplier_article
                .to_lowercase()
                .cmp(&other.supplier_article.to_lowercase()),
            "brand" => match (&self.brand, &other.brand) {
                (Some(a), Some(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            "base_nomenclature_article" => {
                match (
                    &self.base_nomenclature_article,
                    &other.base_nomenclature_article,
                ) {
                    (Some(a), Some(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
                    (Some(_), None) => Ordering::Less,
                    (None, Some(_)) => Ordering::Greater,
                    (None, None) => Ordering::Equal,
                }
            }
            "base_nomenclature_description" => {
                match (
                    &self.base_nomenclature_description,
                    &other.base_nomenclature_description,
                ) {
                    (Some(a), Some(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
                    (Some(_), None) => Ordering::Less,
                    (None, Some(_)) => Ordering::Greater,
                    (None, None) => Ordering::Equal,
                }
            }
            "qty" => self.qty.partial_cmp(&other.qty).unwrap_or(Ordering::Equal),
            "margin_pro" => match (&self.margin_pro, &other.margin_pro) {
                (Some(a), Some(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            "dealer_price_ut" => match (&self.dealer_price_ut, &other.dealer_price_ut) {
                (Some(a), Some(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            "finished_price" => match (&self.finished_price, &other.finished_price) {
                (Some(a), Some(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            "total_price" => match (&self.total_price, &other.total_price) {
                (Some(a), Some(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            "organization_name" => match (&self.organization_name, &other.organization_name) {
                (Some(a), Some(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            _ => Ordering::Equal,
        }
    }
}

const TABLE_ID: &str = "a015-wb-orders-table";
const COLUMN_WIDTHS_KEY: &str = "a015_wb_orders_column_widths";

#[component]
pub fn WbOrdersList() -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let state = create_state();
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);

    // Filter panel expansion state
    let (is_filter_expanded, set_is_filter_expanded) = signal(false);

    // Show cancelled checkbox
    let show_cancelled = RwSignal::new(state.get().show_cancelled);

    // Organizations
    let (organizations, set_organizations) = signal::<Vec<Organization>>(Vec::new());

    // Batch post/unpost state
    let (posting_in_progress, set_posting_in_progress) = signal(false);
    let (operation_notification, set_operation_notification) = signal(None::<String>);

    const FORM_KEY: &str = "a015_wb_orders";

    let open_detail = move |id: String, document_no: String| {
        tabs_store.open_tab(
            &format!("a015_wb_orders_detail_{}", id),
            &format!("WB Order {}", document_no),
        );
    };

    let load_orders = move || {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let date_from_val = state.with_untracked(|s| s.date_from.clone());
            let date_to_val = state.with_untracked(|s| s.date_to.clone());
            let org_id = state.with_untracked(|s| s.selected_organization_id.clone());
            let search_query_val = state.with_untracked(|s| s.search_query.clone());
            let page = state.with_untracked(|s| s.page);
            let page_size = state.with_untracked(|s| s.page_size);
            let sort_field = state.with_untracked(|s| s.sort_field.clone());
            let sort_ascending = state.with_untracked(|s| s.sort_ascending);
            let show_cancelled_val = show_cancelled.get_untracked();
            let offset = page * page_size;
            let cache_buster = js_sys::Date::now() as i64;

            // Build URL with pagination parameters
            let mut url = format!(
                "{}/api/a015/wb-orders?date_from={}&date_to={}&limit={}&offset={}&sort_by={}&sort_desc={}&show_cancelled={}&_ts={}",
                api_base(),
                date_from_val,
                date_to_val,
                page_size,
                offset,
                sort_field,
                !sort_ascending,
                show_cancelled_val,
                cache_buster
            );

            if let Some(org_id) = org_id {
                url.push_str(&format!("&organization_id={}", org_id));
            }
            if !search_query_val.is_empty() {
                url.push_str(&format!("&search_query={}", search_query_val));
            }

            log!("Loading WB orders with URL: {}", url);

            match Request::get(&url)
                .header("Cache-Control", "no-cache, no-store, must-revalidate")
                .header("Pragma", "no-cache")
                .send()
                .await
            {
                Ok(response) => {
                    if response.ok() {
                        match response.json::<PaginatedResponse>().await {
                            Ok(paginated) => {
                                log!("Parsed paginated response: total={}, page={}, page_size={}, total_pages={}", 
                                    paginated.total, paginated.page, paginated.page_size, paginated.total_pages);

                                let parsed_orders: Vec<WbOrdersDto> = paginated
                                    .items
                                    .into_iter()
                                    .filter_map(|item| {
                                        let id = item.get("id")?.as_str()?.to_string();
                                        let document_no =
                                            item.get("document_no")?.as_str()?.to_string();
                                        let order_date = item
                                            .get("document_date")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        let supplier_article = item
                                            .get("supplier_article")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        let brand = item
                                            .get("brand")
                                            .and_then(|v| v.as_str())
                                            .map(String::from);
                                        let qty = item.get("qty")?.as_f64()?;
                                        let margin_pro =
                                            item.get("margin_pro").and_then(|v| v.as_f64());
                                        let dealer_price_ut =
                                            item.get("dealer_price_ut").and_then(|v| v.as_f64());
                                        let finished_price =
                                            item.get("finished_price").and_then(|v| v.as_f64());
                                        let total_price =
                                            item.get("total_price").and_then(|v| v.as_f64());
                                        let is_cancel =
                                            item.get("is_cancel")?.as_bool().unwrap_or(false);
                                        let has_wb_sales =
                                            item.get("has_wb_sales").and_then(|v| v.as_bool());
                                        let organization_name = item
                                            .get("organization_name")
                                            .and_then(|v| v.as_str())
                                            .map(String::from);
                                        let marketplace_article = item
                                            .get("marketplace_article")
                                            .and_then(|v| v.as_str())
                                            .map(String::from);
                                        let nomenclature_code = item
                                            .get("nomenclature_code")
                                            .and_then(|v| v.as_str())
                                            .map(String::from);
                                        let nomenclature_article = item
                                            .get("nomenclature_article")
                                            .and_then(|v| v.as_str())
                                            .map(String::from);
                                        let base_nomenclature_article = item
                                            .get("base_nomenclature_article")
                                            .and_then(|v| v.as_str())
                                            .map(String::from);
                                        let base_nomenclature_description = item
                                            .get("base_nomenclature_description")
                                            .and_then(|v| v.as_str())
                                            .map(String::from);

                                        Some(WbOrdersDto {
                                            id,
                                            document_no,
                                            order_date,
                                            supplier_article,
                                            brand,
                                            qty,
                                            margin_pro,
                                            dealer_price_ut,
                                            finished_price,
                                            total_price,
                                            is_cancel,
                                            has_wb_sales,
                                            organization_name,
                                            marketplace_article,
                                            nomenclature_code,
                                            nomenclature_article,
                                            base_nomenclature_article,
                                            base_nomenclature_description,
                                        })
                                    })
                                    .collect();

                                log!("Successfully parsed {} orders", parsed_orders.len());
                                state.update(|s| {
                                    s.orders = parsed_orders;
                                    s.total_count = paginated.total;
                                    s.total_pages = paginated.total_pages;
                                    s.page = paginated.page;
                                    s.page_size = paginated.page_size;
                                    s.is_loaded = true;
                                });
                                set_loading.set(false);
                            }
                            Err(e) => {
                                log!("Failed to parse response: {}", e);
                                set_error.set(Some(format!("Failed to parse response: {}", e)));
                                set_loading.set(false);
                            }
                        }
                    } else {
                        set_error.set(Some(format!("Server error: {}", response.status())));
                        set_loading.set(false);
                    }
                }
                Err(e) => {
                    log!("Failed to fetch orders: {:?}", e);
                    set_error.set(Some(format!("Failed to fetch orders: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    };

    // Load saved settings on mount
    Effect::new(move |_| {
        if !state.with_untracked(|s| s.is_loaded) {
            // Load organizations first
            spawn_local(async move {
                match fetch_organizations().await {
                    Ok(orgs) => {
                        set_organizations.set(orgs);
                    }
                    Err(e) => {
                        log!("Failed to load organizations: {}", e);
                    }
                }
            });

            spawn_local(async move {
                match load_saved_settings(FORM_KEY).await {
                    Ok(Some(settings)) => {
                        state.update(|s| {
                            if let Some(date_from_val) =
                                settings.get("date_from").and_then(|v| v.as_str())
                            {
                                s.date_from = date_from_val.to_string();
                            }
                            if let Some(date_to_val) =
                                settings.get("date_to").and_then(|v| v.as_str())
                            {
                                s.date_to = date_to_val.to_string();
                            }
                            if let Some(org_id) = settings
                                .get("selected_organization_id")
                                .and_then(|v| v.as_str())
                            {
                                if !org_id.is_empty() {
                                    s.selected_organization_id = Some(org_id.to_string());
                                }
                            }
                        });
                        log!("Loaded saved settings for A015");
                        load_orders();
                    }
                    Ok(None) => {
                        log!("No saved settings found for A015");
                        load_orders();
                    }
                    Err(e) => {
                        log!("Failed to load saved settings: {}", e);
                        load_orders();
                    }
                }
            });
        } else {
            log!("Used cached data for A015");
        }
    });

    // Thaw inputs
    let search_query = RwSignal::new(state.get_untracked().search_query.clone());
    let selected_org_id = RwSignal::new(
        state
            .get_untracked()
            .selected_organization_id
            .clone()
            .unwrap_or_default(),
    );

    Effect::new(move || {
        let v = search_query.get();
        untrack(move || {
            state.update(|s| {
                s.search_query = v;
            });
        });
    });

    Effect::new(move || {
        let v = selected_org_id.get();
        untrack(move || {
            state.update(|s| {
                s.selected_organization_id = if v.is_empty() { None } else { Some(v.clone()) };
            });
        });
    });

    // Initialize column resize
    let resize_initialized = leptos::prelude::StoredValue::new(false);
    Effect::new(move |_| {
        if !resize_initialized.get_value() {
            resize_initialized.set_value(true);
            spawn_local(async move {
                gloo_timers::future::TimeoutFuture::new(100).await;
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
        if s.selected_organization_id.is_some() {
            count += 1;
        }
        if !s.search_query.is_empty() {
            count += 1;
        }
        count
    });

    // Sort handler
    let toggle_sort = move |field: &'static str| {
        state.update(|s| {
            if s.sort_field == field {
                s.sort_ascending = !s.sort_ascending;
            } else {
                s.sort_field = field.to_string();
                s.sort_ascending = true;
            }
            s.page = 0; // Reset to first page on sort change
        });
        load_orders();
    };

    // Pagination functions
    let go_to_page = move |new_page: usize| {
        state.update(|s| s.page = new_page);
        load_orders();
    };

    let change_page_size = move |new_size: usize| {
        state.update(|s| {
            s.page_size = new_size;
            s.page = 0; // Reset to first page
        });
        load_orders();
    };

    // Selection management
    let selected = RwSignal::new(state.with_untracked(|s| s.selected_ids.clone()));

    let toggle_selection = move |id: String, checked: bool| {
        selected.update(|s| {
            if checked {
                s.insert(id.clone());
            } else {
                s.remove(&id);
            }
        });
        state.update(|s| {
            if checked {
                s.selected_ids.insert(id);
            } else {
                s.selected_ids.remove(&id);
            }
        });
    };

    let toggle_all = move |check_all: bool| {
        if check_all {
            let items = state.get().orders;
            selected.update(|s| {
                s.clear();
                for item in items.iter() {
                    s.insert(item.id.clone());
                }
            });
            state.update(|s| {
                s.selected_ids.clear();
                for item in items.iter() {
                    s.selected_ids.insert(item.id.clone());
                }
            });
        } else {
            selected.update(|s| s.clear());
            state.update(|s| s.selected_ids.clear());
        }
    };

    let selected_count = Signal::derive(move || state.with(|s| s.selected_ids.len()));

    let post_selected = move |_: leptos::ev::MouseEvent| {
        let ids: Vec<String> = state.with(|s| s.selected_ids.iter().cloned().collect());
        if ids.is_empty() {
            return;
        }

        set_posting_in_progress.set(true);
        set_operation_notification.set(None);

        spawn_local(async move {
            let mut ok_count = 0usize;
            let mut fail_count = 0usize;

            for id in &ids {
                let url = format!("{}/api/a015/wb-orders/{}/post", api_base(), id);
                match Request::post(&url).send().await {
                    Ok(resp) if resp.ok() => ok_count += 1,
                    Ok(resp) => {
                        fail_count += 1;
                        log!("Post failed for {}: status {}", id, resp.status());
                    }
                    Err(e) => {
                        fail_count += 1;
                        log!("Post failed for {}: {}", id, e);
                    }
                }
            }

            let msg = if fail_count == 0 {
                format!("Post: успешно {}", ok_count)
            } else {
                format!("Post: успешно {}, ошибок {}", ok_count, fail_count)
            };
            set_operation_notification.set(Some(msg));
            selected.update(|s| s.clear());
            state.update(|s| s.selected_ids.clear());
            load_orders();
            set_posting_in_progress.set(false);
        });
    };

    let unpost_selected = move |_: leptos::ev::MouseEvent| {
        let ids: Vec<String> = state.with(|s| s.selected_ids.iter().cloned().collect());
        if ids.is_empty() {
            return;
        }

        set_posting_in_progress.set(true);
        set_operation_notification.set(None);

        spawn_local(async move {
            let mut ok_count = 0usize;
            let mut fail_count = 0usize;

            for id in &ids {
                let url = format!("{}/api/a015/wb-orders/{}/unpost", api_base(), id);
                match Request::post(&url).send().await {
                    Ok(resp) if resp.ok() => ok_count += 1,
                    Ok(resp) => {
                        fail_count += 1;
                        log!("Unpost failed for {}: status {}", id, resp.status());
                    }
                    Err(e) => {
                        fail_count += 1;
                        log!("Unpost failed for {}: {}", id, e);
                    }
                }
            }

            let msg = if fail_count == 0 {
                format!("Unpost: успешно {}", ok_count)
            } else {
                format!("Unpost: успешно {}, ошибок {}", ok_count, fail_count)
            };
            set_operation_notification.set(Some(msg));
            selected.update(|s| s.clear());
            state.update(|s| s.selected_ids.clear());
            load_orders();
            set_posting_in_progress.set(false);
        });
    };

    let items_signal = Signal::derive(move || state.get().orders);
    let selected_signal = Signal::derive(move || selected.get());

    view! {
        <PageFrame page_id="a015_wb_orders--list" category="list">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Заказы Wildberries"</h1>
                    <UiBadge variant="primary".to_string()>
                        {move || state.get().total_count.to_string()}
                    </UiBadge>
                </div>

                <div class="page__header-right">
                    <Space>
                        <Button
                            appearance=ButtonAppearance::Primary
                            on_click=post_selected
                            disabled=Signal::derive(move || {
                                selected_count.get() == 0 || posting_in_progress.get() || loading.get()
                            })
                        >
                            {move || {
                                if posting_in_progress.get() {
                                    "Post..."
                                } else {
                                    "Post"
                                }
                            }}
                            {move || format!(" ({})", selected_count.get())}
                        </Button>
                        <Button
                            appearance=ButtonAppearance::Secondary
                            on_click=unpost_selected
                            disabled=Signal::derive(move || {
                                selected_count.get() == 0 || posting_in_progress.get() || loading.get()
                            })
                        >
                            {move || {
                                if posting_in_progress.get() {
                                    "Unpost..."
                                } else {
                                    "Unpost"
                                }
                            }}
                            {move || format!(" ({})", selected_count.get())}
                        </Button>
                        <Button
                            appearance=ButtonAppearance::Secondary
                            on_click=move |_| {
                                let data = state.get().orders;
                                if let Err(e) = export_to_csv(&data) {
                                    log!("Failed to export: {}", e);
                                }
                            }
                            disabled=Signal::derive(move || loading.get() || state.get().orders.is_empty())
                        >
                            {icon("download")}
                            "Excel"
                        </Button>
                        {move || operation_notification.get().map(|msg| view! {
                            <span style="font-size: 12px; color: var(--colorNeutralForeground2, #666);">{msg}</span>
                        })}
                    </Space>
                </div>
            </div>

            <div class="page__content">
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
                                        <span class="filter-panel__badge">{count}</span>
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
                                page_size_options=vec![50, 100, 200, 500]
                            />
                        </div>
                        <div class="filter-panel-header__right">
                            <Button
                                appearance=ButtonAppearance::Primary
                                on_click=move |_| load_orders()
                                disabled=Signal::derive(move || loading.get())
                            >
                                {move || if loading.get() { "Загрузка..." } else { "Обновить" }}
                            </Button>
                        </div>
                    </div>

                    <Show when=move || is_filter_expanded.get()>
                        <div class="filter-panel-content">
                            <Flex gap=FlexGap::Small align=FlexAlign::End>
                                <div style="min-width: 420px;">
                                    <DateRangePicker
                                        date_from=Signal::derive(move || state.with(|s| s.date_from.clone()))
                                        date_to=Signal::derive(move || state.with(|s| s.date_to.clone()))
                                        on_change=Callback::new(move |(from, to)| {
                                            state.update(|s| {
                                                s.date_from = from;
                                                s.date_to = to;
                                                s.page = 0;
                                            });
                                            load_orders();
                                        })
                                        label="Период:".to_string()
                                    />
                                </div>

                                <div style="width: 260px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Организация:"</Label>
                                        <Select value=selected_org_id>
                                            <option value="">"Все организации"</option>
                                            {move || organizations.get().into_iter().map(|org| {
                                                let id = org.id.clone();
                                                view! {
                                                    <option value=id>{org.description}</option>
                                                }
                                            }).collect_view()}
                                        </Select>
                                    </Flex>
                                </div>

                                <div style="flex: 1; max-width: 300px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Поиск:"</Label>
                                        <Input
                                            value=search_query
                                            placeholder="Артикул, номер, бренд, наименование базы..."
                                        />
                                    </Flex>
                                </div>

                                <div style="width: 180px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>" "</Label>
                                        <Checkbox
                                            checked=show_cancelled
                                            label="Показать отменённые"
                                        />
                                    </Flex>
                                </div>

                            </Flex>
                        </div>
                    </Show>
                </div>

                {move || {
                    error
                        .get()
                        .map(|err| {
                            view! {
                                <div class="alert alert--error">
                                    {err}
                                </div>
                            }
                        })
                }}

                <div class="table-wrapper">
                    <TableCrosshairHighlight table_id=TABLE_ID.to_string() />

                    <Table attr:id=TABLE_ID attr:style="width: 100%; min-width: 1400px;">
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCheckbox
                                    items=items_signal
                                    selected=selected_signal
                                    get_id=Callback::new(|row: WbOrdersDto| row.id.clone())
                                    on_change=Callback::new(toggle_all)
                                />

                                <TableHeaderCell resizable=false min_width=140.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("document_no")>
                                        "Номер заказа"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "document_no"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "document_no", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=110.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("order_date")>
                                        "Дата"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "order_date"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "order_date", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=90.0 class="resizable">
                                    "Время"
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=150.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("organization_name")>
                                        "Организация"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "organization_name"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "organization_name", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=120.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("supplier_article")>
                                        "Артикул"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "supplier_article"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "supplier_article", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=130.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("base_nomenclature_article")>
                                        "Артикул база"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "base_nomenclature_article"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "base_nomenclature_article", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=160.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("base_nomenclature_description")>
                                        "Наименование база"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "base_nomenclature_description"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "base_nomenclature_description", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=120.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("brand")>
                                        "Бренд"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "brand"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "brand", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=90.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("margin_pro")>
                                        "Маржа, %"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "margin_pro"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "margin_pro", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=110.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("dealer_price_ut")>
                                        "Дил. цена УТ"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "dealer_price_ut"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "dealer_price_ut", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=110.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("finished_price")>
                                        "Итоговая цена"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "finished_price"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "finished_price", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=90.0 class="resizable">
                                    "Отменён"
                                </TableHeaderCell>
                            </TableRow>
                        </TableHeader>

                        <TableBody>
                            <For
                                each=move || state.get().orders
                                key=|item| item.id.clone()
                                children=move |order| {
                                    let order_id = order.id.clone();
                                    let order_id_for_link = order_id.clone();
                                    let document_no_for_link = order.document_no.clone();
                                    let document_no_text = order.document_no.clone();
                                    let formatted_date = format_date(&order.order_date);
                                    let formatted_time = format_time(&order.order_date);

                                    view! {
                                        <TableRow>
                                            <TableCellCheckbox
                                                item_id=order_id.clone()
                                                selected=selected_signal
                                                on_change=Callback::new(move |(id, checked)| toggle_selection(id, checked))
                                            />

                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    <a
                                                        href="#"
                                                        class="table__link"
                                                        style=if order.has_wb_sales.unwrap_or(false) {
                                                            "color: #1b5e20; text-decoration: underline; font-weight: 600;"
                                                        } else {
                                                            "color: #0f6cbd; text-decoration: underline;"
                                                        }
                                                        title=if order.has_wb_sales.unwrap_or(false) {
                                                            "Есть связанные WB Sales"
                                                        } else {
                                                            "Открыть документ"
                                                        }
                                                        on:click=move |e| {
                                                            e.prevent_default();
                                                            open_detail(order_id_for_link.clone(), document_no_for_link.clone());
                                                        }
                                                    >
                                                        {document_no_text}
                                                    </a>
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout>
                                                    {formatted_date}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout>
                                                    {formatted_time}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {order.organization_name.clone().unwrap_or_else(|| "—".to_string())}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {order.supplier_article}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {order.base_nomenclature_article.clone().unwrap_or_else(|| "—".to_string())}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {order.base_nomenclature_description.clone().unwrap_or_else(|| "—".to_string())}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {order.brand.clone().unwrap_or_else(|| "—".to_string())}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCellMoney
                                                value=order.margin_pro
                                                show_currency=false
                                                color_by_sign=false
                                            />

                                            <TableCellMoney
                                                value=order.dealer_price_ut
                                                show_currency=false
                                                color_by_sign=false
                                            />

                                            <TableCellMoney
                                                value=order.finished_price.unwrap_or(0.0)
                                                show_currency=false
                                                color_by_sign=false
                                            />

                                            <TableCell>
                                                <TableCellLayout>
                                                    {if order.is_cancel { "Да" } else { "" }}
                                                </TableCellLayout>
                                            </TableCell>
                                        </TableRow>
                                    }
                                }
                            />
                        </TableBody>
                    </Table>
                </div>
            </div>
        </PageFrame>
    }
}

/// Загрузка списка организаций
async fn fetch_organizations() -> Result<Vec<Organization>, String> {
    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/organization", api_base());
    let request = WebRequest::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
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
    let data: Vec<Organization> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}

/// Load saved settings from database
async fn load_saved_settings(form_key: &str) -> Result<Option<serde_json::Value>, String> {
    let url = format!("{}/api/user-form-settings/{}", api_base(), form_key);
    match Request::get(&url).send().await {
        Ok(response) => {
            if response.status() == 200 {
                match response.json::<serde_json::Value>().await {
                    Ok(settings) => Ok(Some(settings)),
                    Err(e) => {
                        log!("Failed to parse settings: {}", e);
                        Ok(None)
                    }
                }
            } else if response.status() == 404 {
                Ok(None)
            } else {
                Err(format!("HTTP {}", response.status()))
            }
        }
        Err(e) => {
            log!("Failed to fetch settings: {:?}", e);
            Err(format!("{:?}", e))
        }
    }
}

/// Экспорт WB Orders в CSV для Excel
fn export_to_csv(data: &[WbOrdersDto]) -> Result<(), String> {
    let mut csv = String::from("\u{FEFF}");

    csv.push_str("Номер заказа;Дата заказа;Организация;Артикул продавца;Артикул МП;Артикул 1С;Артикул база;Наименование база;Бренд;Маржа, %;Дил. цена УТ;Итоговая цена;Отменён\n");

    for order in data {
        let order_date = format_date(&order.order_date);
        let org_name = order
            .organization_name
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("—");
        let mp_article = order
            .marketplace_article
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("—");
        let base_nom_article = order
            .base_nomenclature_article
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("—");
        let base_nom_description = order
            .base_nomenclature_description
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("—");
        let nom_article = order
            .nomenclature_article
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("—");
        let brand = order.brand.as_ref().map(|s| s.as_str()).unwrap_or("—");

        let margin_str = order
            .margin_pro
            .map(|a| format!("{:.2}", a).replace(".", ","))
            .unwrap_or_else(|| "—".to_string());
        let dealer_price_str = order
            .dealer_price_ut
            .map(|a| format!("{:.2}", a).replace(".", ","))
            .unwrap_or_else(|| "—".to_string());
        let finished_price_str = order
            .finished_price
            .map(|a| format!("{:.2}", a).replace(".", ","))
            .unwrap_or_else(|| "—".to_string());
        let is_cancel_str = if order.is_cancel { "Да" } else { "Нет" };

        csv.push_str(&format!(
            "\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";{};{};{};\"{}\"\n",
            order.document_no.replace('\"', "\"\""),
            order_date,
            org_name.replace('\"', "\"\""),
            order.supplier_article.replace('\"', "\"\""),
            mp_article.replace('\"', "\"\""),
            nom_article.replace('\"', "\"\""),
            base_nom_article.replace('\"', "\"\""),
            base_nom_description.replace('\"', "\"\""),
            brand.replace('\"', "\"\""),
            margin_str,
            dealer_price_str,
            finished_price_str,
            is_cancel_str
        ));
    }

    let blob_parts = js_sys::Array::new();
    blob_parts.push(&wasm_bindgen::JsValue::from_str(&csv));

    let blob_props = BlobPropertyBag::new();
    blob_props.set_type("text/csv;charset=utf-8;");

    let blob = Blob::new_with_str_sequence_and_options(&blob_parts, &blob_props)
        .map_err(|e| format!("Failed to create blob: {:?}", e))?;

    let url = Url::create_object_url_with_blob(&blob)
        .map_err(|e| format!("Failed to create URL: {:?}", e))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let document = window.document().ok_or_else(|| "no document".to_string())?;

    let a = document
        .create_element("a")
        .map_err(|e| format!("Failed to create element: {:?}", e))?
        .dyn_into::<HtmlAnchorElement>()
        .map_err(|e| format!("Failed to cast to anchor: {:?}", e))?;

    a.set_href(&url);
    let filename = format!(
        "wb_orders_{}.csv",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );
    a.set_download(&filename);
    a.click();

    Url::revoke_object_url(&url).map_err(|e| format!("Failed to revoke URL: {:?}", e))?;

    Ok(())
}
