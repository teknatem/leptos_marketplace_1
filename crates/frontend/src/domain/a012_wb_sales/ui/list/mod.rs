pub mod state;

use self::state::{create_state, WbSalesTotals};
use crate::layout::global_context::AppGlobalContext;
use crate::shared::components::date_range_picker::DateRangePicker;
use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::components::ui::badge::Badge as UiBadge;
use crate::shared::components::ui::button::Button as UiButton;
use crate::shared::icons::icon;
use crate::shared::list_utils::{
    format_number, get_sort_class, get_sort_indicator, Sortable,
};
use crate::shared::table_utils::{clear_resize_flag, init_column_resize, was_just_resizing};
use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::cmp::Ordering;
use thaw::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: String,
    pub code: String,
    pub description: String,
}

/// Paginated response from backend API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse {
    pub items: Vec<serde_json::Value>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
    /// Серверные итоги по всему датасету
    pub totals: Option<WbSalesTotals>,
}

/// Форматирует ISO 8601 дату в dd.mm.yyyy
fn format_date(iso_date: &str) -> String {
    // Парсим ISO 8601: "2025-11-05T16:52:58.585775200Z"
    if let Some(date_part) = iso_date.split('T').next() {
        if let Some((year, rest)) = date_part.split_once('-') {
            if let Some((month, day)) = rest.split_once('-') {
                return format!("{}.{}.{}", day, month, year);
            }
        }
    }
    iso_date.to_string() // fallback
}

/// Parse a single WbSales item from JSON value (compact DTO format)
fn parse_wb_sales_item(v: &serde_json::Value, idx: usize) -> Option<WbSalesDto> {
    // All fields are now at top level in compact DTO
    let result = Some(WbSalesDto {
        id: v.get("id")?.as_str()?.to_string(),
        document_no: v.get("document_no")?.as_str()?.to_string(),
        sale_id: v
            .get("sale_id")
            .and_then(|a| a.as_str())
            .map(|s| s.to_string()),
        sale_date: v.get("sale_date")?.as_str()?.to_string(),
        supplier_article: v.get("supplier_article")?.as_str()?.to_string(),
        name: v.get("name")?.as_str()?.to_string(),
        qty: v.get("qty")?.as_f64()?,
        amount_line: v.get("amount_line").and_then(|a| a.as_f64()),
        total_price: v.get("total_price").and_then(|a| a.as_f64()),
        finished_price: v.get("finished_price").and_then(|a| a.as_f64()),
        event_type: v.get("event_type")?.as_str()?.to_string(),
        organization_name: v
            .get("organization_name")
            .and_then(|a| a.as_str())
            .map(|s| s.to_string()),
        marketplace_article: v
            .get("marketplace_article")
            .and_then(|a| a.as_str())
            .map(|s| s.to_string()),
        nomenclature_code: v
            .get("nomenclature_code")
            .and_then(|a| a.as_str())
            .map(|s| s.to_string()),
        nomenclature_article: v
            .get("nomenclature_article")
            .and_then(|a| a.as_str())
            .map(|s| s.to_string()),
        operation_date: v
            .get("operation_date")
            .and_then(|a| a.as_str())
            .map(|s| s.to_string()),
    });

    if result.is_none() {
        log!("Failed to parse item {}", idx);
    }

    result
}

const TABLE_ID: &str = "a012-wb-sales-table";
const COLUMN_WIDTHS_KEY: &str = "a012_wb_sales_column_widths";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbSalesDto {
    pub id: String,
    pub document_no: String,
    pub sale_id: Option<String>,
    pub sale_date: String,
    pub supplier_article: String,
    pub name: String,
    pub qty: f64,
    pub amount_line: Option<f64>,
    pub total_price: Option<f64>,
    pub finished_price: Option<f64>,
    pub event_type: String,
    pub organization_name: Option<String>,
    pub marketplace_article: Option<String>,
    pub nomenclature_code: Option<String>,
    pub nomenclature_article: Option<String>,
    pub operation_date: Option<String>,
}

impl Sortable for WbSalesDto {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "document_no" => self
                .document_no
                .to_lowercase()
                .cmp(&other.document_no.to_lowercase()),
            "sale_id" => match (&self.sale_id, &other.sale_id) {
                (Some(a), Some(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            "sale_date" => self.sale_date.cmp(&other.sale_date),
            "supplier_article" => self
                .supplier_article
                .to_lowercase()
                .cmp(&other.supplier_article.to_lowercase()),
            "name" => self.name.to_lowercase().cmp(&other.name.to_lowercase()),
            "qty" => self.qty.partial_cmp(&other.qty).unwrap_or(Ordering::Equal),
            "amount_line" => match (&self.amount_line, &other.amount_line) {
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
            "finished_price" => match (&self.finished_price, &other.finished_price) {
                (Some(a), Some(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            "event_type" => self
                .event_type
                .to_lowercase()
                .cmp(&other.event_type.to_lowercase()),
            "organization_name" => match (&self.organization_name, &other.organization_name) {
                (Some(a), Some(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            "marketplace_article" => {
                match (&self.marketplace_article, &other.marketplace_article) {
                    (Some(a), Some(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
                    (Some(_), None) => Ordering::Less,
                    (None, Some(_)) => Ordering::Greater,
                    (None, None) => Ordering::Equal,
                }
            }
            "nomenclature_code" => match (&self.nomenclature_code, &other.nomenclature_code) {
                (Some(a), Some(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            "nomenclature_article" => {
                match (&self.nomenclature_article, &other.nomenclature_article) {
                    (Some(a), Some(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
                    (Some(_), None) => Ordering::Less,
                    (None, Some(_)) => Ordering::Greater,
                    (None, None) => Ordering::Equal,
                }
            }
            "operation_date" => match (&self.operation_date, &other.operation_date) {
                (Some(a), Some(b)) => a.cmp(b),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            _ => Ordering::Equal,
        }
    }
}

#[component]
pub fn WbSalesList() -> impl IntoView {
    let tabs_store = leptos::context::use_context::<AppGlobalContext>()
        .expect("AppGlobalContext context not found");

    let state = create_state();
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);

    // Filter panel expansion state (same pattern as a016_ym_returns)
    let (is_filter_expanded, set_is_filter_expanded) = signal(false);

    // Batch operation state
    let (posting_in_progress, set_posting_in_progress) = signal(false);
    let (_, set_operation_results) = signal::<Vec<(String, bool, Option<String>)>>(Vec::new());
    let (current_operation, set_current_operation) = signal::<Option<(usize, usize)>>(None);

    // Organizations
    let (organizations, set_organizations) = signal::<Vec<Organization>>(Vec::new());

    // State for save settings notification
    let (save_notification, set_save_notification) = signal(None::<String>);

    const FORM_KEY: &str = "a012_wb_sales";

    let load_sales = move || {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let date_from_val = state.with(|s| s.date_from.clone());
            let date_to_val = state.with(|s| s.date_to.clone());
            let org_id = state.with(|s| s.selected_organization_id.clone());
            let page = state.with(|s| s.page);
            let page_size = state.with(|s| s.page_size);
            let sort_field = state.with(|s| s.sort_field.clone());
            let sort_ascending = state.with(|s| s.sort_ascending);
            let search_sale_id = state.with(|s| s.search_sale_id.clone());
            let search_srid = state.with(|s| s.search_srid.clone());
            let offset = page * page_size;

            // Build URL with pagination parameters
            let mut url = format!(
                "http://localhost:3000/api/a012/wb-sales?date_from={}&date_to={}&limit={}&offset={}&sort_by={}&sort_desc={}",
                date_from_val, date_to_val, page_size, offset, sort_field, !sort_ascending
            );

            // Add organization filter if selected
            if let Some(org_id) = org_id {
                url.push_str(&format!("&organization_id={}", org_id));
            }

            // Add search filters
            if !search_sale_id.is_empty() {
                url.push_str(&format!("&search_sale_id={}", search_sale_id));
            }
            if !search_srid.is_empty() {
                url.push_str(&format!("&search_srid={}", search_srid));
            }

            log!("Loading WB sales with URL: {}", url);

            match Request::get(&url).send().await {
                Ok(response) => {
                    let status = response.status();
                    if status == 200 {
                        match response.text().await {
                            Ok(text) => {
                                // Parse paginated response
                                match serde_json::from_str::<PaginatedResponse>(&text) {
                                    Ok(paginated) => {
                                        log!("Parsed paginated response: total={}, page={}, page_size={}, total_pages={}", 
                                            paginated.total, paginated.page, paginated.page_size, paginated.total_pages);

                                        // Parse items from the response
                                        let items: Vec<WbSalesDto> = paginated
                                            .items
                                            .into_iter()
                                            .enumerate()
                                            .filter_map(|(idx, v)| parse_wb_sales_item(&v, idx))
                                            .collect();

                                        log!("Successfully parsed {} sales", items.len());
                                        state.update(|s| {
                                            s.sales = items;
                                            s.total_count = paginated.total;
                                            s.total_pages = paginated.total_pages;
                                            s.server_totals = paginated.totals;
                                            s.is_loaded = true;
                                        });
                                        set_loading.set(false);
                                    }
                                    Err(e) => {
                                        log!("Failed to parse paginated response: {:?}", e);
                                        set_error
                                            .set(Some(format!("Failed to parse response: {}", e)));
                                        set_loading.set(false);
                                    }
                                }
                            }
                            Err(e) => {
                                log!("Failed to read response text: {:?}", e);
                                set_error.set(Some(format!("Failed to read response: {}", e)));
                                set_loading.set(false);
                            }
                        }
                    } else {
                        set_error.set(Some(format!("Server error: {}", status)));
                        set_loading.set(false);
                    }
                }
                Err(e) => {
                    log!("Failed to fetch sales: {:?}", e);
                    set_error.set(Some(format!("Failed to fetch sales: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    };

    // Get items (sorting is now done on server) - no clone, returns reference via signal
    let get_items = move || -> Vec<WbSalesDto> { state.with(|s| s.sales.clone()) };

    // Load saved settings from database on mount IF not already loaded in memory
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
                        log!("Loaded saved settings for A012");
                        load_sales();
                    }
                    Ok(None) => {
                        log!("No saved settings found for A012");
                        load_sales();
                    }
                    Err(e) => {
                        log!("Failed to load saved settings: {}", e);
                        load_sales();
                    }
                }
            });
        } else {
            log!("Used cached data for A012");
        }
    });

    // Thaw inputs: keep local RwSignal, sync -> state (one-way)
    let search_sale_id = RwSignal::new(state.get_untracked().search_sale_id.clone());
    let search_srid = RwSignal::new(state.get_untracked().search_srid.clone());
    let selected_org_id = RwSignal::new(
        state
            .get_untracked()
            .selected_organization_id
            .clone()
            .unwrap_or_default(),
    );

    Effect::new(move || {
        let v = search_sale_id.get();
        untrack(move || {
            state.update(|s| {
                s.search_sale_id = v;
                s.page = 0;
            });
        });
    });

    Effect::new(move || {
        let v = search_srid.get();
        untrack(move || {
            state.update(|s| {
                s.search_srid = v;
                s.page = 0;
            });
        });
    });

    Effect::new(move || {
        let v = selected_org_id.get();
        untrack(move || {
            state.update(|s| {
                s.selected_organization_id = if v.is_empty() { None } else { Some(v.clone()) };
                s.page = 0;
            });
        });
    });

    // Count active filters (same style as a016_ym_returns)
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
        if !s.search_sale_id.is_empty() {
            count += 1;
        }
        if !s.search_srid.is_empty() {
            count += 1;
        }
        count
    });

    // Init column resize after data is rendered
    Effect::new(move |_| {
        let is_loaded = state.get().is_loaded;
        let _page = state.get().page;
        let _len = state.get().sales.len();
        if is_loaded {
            spawn_local(async move {
                gloo_timers::future::TimeoutFuture::new(50).await;
                init_column_resize(TABLE_ID, COLUMN_WIDTHS_KEY);
            });
        }
    });

    // Функция для изменения сортировки (сбрасывает на первую страницу)
    let toggle_sort = move |field: &'static str| {
        // Skip sort if we just finished resizing column
        if was_just_resizing() {
            clear_resize_flag();
            return;
        }

        state.update(|s| {
            if s.sort_field == field {
                s.sort_ascending = !s.sort_ascending;
            } else {
                s.sort_field = field.to_string();
                s.sort_ascending = true;
            }
            s.page = 0; // Reset to first page on sort change
        });
        load_sales();
    };

    // Pagination: go to specific page
    let go_to_page = move |new_page: usize| {
        state.update(|s| s.page = new_page);
        load_sales();
    };

    // Pagination: change page size
    let change_page_size = move |new_size: usize| {
        state.update(|s| {
            s.page_size = new_size;
            s.page = 0; // Reset to first page
        });
        load_sales();
    };

    // Переключение выбора одного документа
    let toggle_selection = move |id: String| {
        state.update(|s| {
            if s.selected_ids.contains(&id) {
                s.selected_ids.retain(|x| x != &id);
            } else {
                s.selected_ids.push(id.clone());
            }
        });
    };

    // Выбрать все / снять все
    let toggle_all = move |_| {
        let items = get_items();
        let all_ids: Vec<String> = items.iter().map(|item| item.id.clone()).collect();
        state.update(|s| {
            if s.selected_ids.len() == all_ids.len() && !all_ids.is_empty() {
                s.selected_ids.clear();
            } else {
                s.selected_ids = all_ids;
            }
        });
    };

    // Проверка, выбраны ли все
    let all_selected = move || {
        let items = get_items();
        let selected_len = state.with(|s| s.selected_ids.len());
        !items.is_empty() && selected_len == items.len()
    };

    // Проверка, выбран ли конкретный документ
    let is_selected = move |id: &str| state.with(|s| s.selected_ids.contains(&id.to_string()));

    // Массовое проведение
    let post_selected = move |_: leptos::ev::MouseEvent| {
        let ids = state.with(|s| s.selected_ids.clone());
        if ids.is_empty() {
            return;
        }

        set_posting_in_progress.set(true);
        set_operation_results.set(Vec::new());
        set_current_operation.set(Some((0, ids.len())));

        spawn_local(async move {
            let mut results = Vec::new();
            let total = ids.len();

            // Разбиваем на чанки по 100
            for (chunk_idx, chunk) in ids.chunks(100).enumerate() {
                set_current_operation.set(Some((chunk_idx * 100 + chunk.len(), total)));

                let payload = json!({ "ids": chunk });
                let response = Request::post("http://localhost:3000/api/a012/wb-sales/batch-post")
                    .header("Content-Type", "application/json")
                    .body(serde_json::to_string(&payload).unwrap_or_default())
                    .map(|req| req.send());

                match response {
                    Ok(future) => match future.await {
                        Ok(resp) => {
                            if resp.status() == 200 {
                                for id in chunk {
                                    results.push((id.clone(), true, None));
                                }
                            } else {
                                for id in chunk {
                                    results.push((
                                        id.clone(),
                                        false,
                                        Some(format!("HTTP {}", resp.status())),
                                    ));
                                }
                            }
                        }
                        Err(e) => {
                            for id in chunk {
                                results.push((id.clone(), false, Some(format!("{:?}", e))));
                            }
                        }
                    },
                    Err(e) => {
                        for id in chunk {
                            results.push((id.clone(), false, Some(format!("{:?}", e))));
                        }
                    }
                }
            }

            set_operation_results.set(results);
            set_posting_in_progress.set(false);
            set_current_operation.set(None);
            state.update(|s| s.selected_ids.clear());

            // Перезагрузить список
            load_sales();
        });
    };

    // Массовая отмена проведения
    let unpost_selected = move |_: leptos::ev::MouseEvent| {
        let ids = state.with(|s| s.selected_ids.clone());
        if ids.is_empty() {
            return;
        }

        set_posting_in_progress.set(true);
        set_operation_results.set(Vec::new());
        set_current_operation.set(Some((0, ids.len())));

        spawn_local(async move {
            let mut results = Vec::new();
            let total = ids.len();

            // Разбиваем на чанки по 100
            for (chunk_idx, chunk) in ids.chunks(100).enumerate() {
                set_current_operation.set(Some((chunk_idx * 100 + chunk.len(), total)));

                let payload = json!({ "ids": chunk });
                let response =
                    Request::post("http://localhost:3000/api/a012/wb-sales/batch-unpost")
                        .header("Content-Type", "application/json")
                        .body(serde_json::to_string(&payload).unwrap_or_default())
                        .map(|req| req.send());

                match response {
                    Ok(future) => match future.await {
                        Ok(resp) => {
                            if resp.status() == 200 {
                                for id in chunk {
                                    results.push((id.clone(), true, None));
                                }
                            } else {
                                for id in chunk {
                                    results.push((
                                        id.clone(),
                                        false,
                                        Some(format!("HTTP {}", resp.status())),
                                    ));
                                }
                            }
                        }
                        Err(e) => {
                            for id in chunk {
                                results.push((id.clone(), false, Some(format!("{:?}", e))));
                            }
                        }
                    },
                    Err(e) => {
                        for id in chunk {
                            results.push((id.clone(), false, Some(format!("{:?}", e))));
                        }
                    }
                }
            }

            set_operation_results.set(results);
            set_posting_in_progress.set(false);
            set_current_operation.set(None);
            state.update(|s| s.selected_ids.clear());

            // Перезагрузить список
            load_sales();
        });
    };

    // Save current settings to database
    let save_settings_to_db = move |_: leptos::ev::MouseEvent| {
        let settings = json!({
            "date_from": state.with(|s| s.date_from.clone()),
            "date_to": state.with(|s| s.date_to.clone()),
            "selected_organization_id": state.with(|s| s.selected_organization_id.clone()).unwrap_or_default(),
        });

        spawn_local(async move {
            match save_settings_to_database(FORM_KEY, settings).await {
                Ok(_) => {
                    set_save_notification.set(Some("✓ Настройки сохранены".to_string()));
                    spawn_local(async move {
                        gloo_timers::future::TimeoutFuture::new(3000).await;
                        set_save_notification.set(None);
                    });
                }
                Err(e) => {
                    set_save_notification.set(Some(format!("✗ Ошибка: {}", e)));
                    log!("Failed to save settings: {}", e);
                }
            }
        });
    };

    // Load and restore settings from database
    let restore_settings = move |_: leptos::ev::MouseEvent| {
        spawn_local(async move {
            match load_saved_settings(FORM_KEY).await {
                Ok(Some(settings)) => {
                    state.update(|s| {
                        if let Some(date_from_val) =
                            settings.get("date_from").and_then(|v| v.as_str())
                        {
                            s.date_from = date_from_val.to_string();
                        }
                        if let Some(date_to_val) = settings.get("date_to").and_then(|v| v.as_str())
                        {
                            s.date_to = date_to_val.to_string();
                        }
                        if let Some(org_id) = settings
                            .get("selected_organization_id")
                            .and_then(|v| v.as_str())
                        {
                            if !org_id.is_empty() {
                                s.selected_organization_id = Some(org_id.to_string());
                            } else {
                                s.selected_organization_id = None;
                            }
                        }
                    });
                    set_save_notification.set(Some("✓ Настройки восстановлены".to_string()));
                    spawn_local(async move {
                        gloo_timers::future::TimeoutFuture::new(3000).await;
                        set_save_notification.set(None);
                    });
                    log!("Restored saved settings for A012");
                    load_sales();
                }
                Ok(None) => {
                    set_save_notification.set(Some("ℹ Нет сохраненных настроек".to_string()));
                    spawn_local(async move {
                        gloo_timers::future::TimeoutFuture::new(3000).await;
                        set_save_notification.set(None);
                    });
                    log!("No saved settings found for A012");
                }
                Err(e) => {
                    set_save_notification.set(Some(format!("✗ Ошибка: {}", e)));
                    log!("Failed to load saved settings: {}", e);
                }
            }
        });
    };

    // Открыть детальный просмотр
    let open_detail = move |id: String, document_no: String| {
        tabs_store.open_tab(
            &format!("a012_wb_sales_detail_{}", id),
            &format!("WB Sales {}", document_no),
        );
    };

    view! {
        <div class="page page--wide">
            <div class="page-header">
                <div class="page-header__content">
                    <div class="page-header__icon">{icon("trending-up")}</div>
                    <div class="page-header__text">
                        <h1 class="page-header__title">"Продажи Wildberries"</h1>
                        <div class="page-header__badge">
                            <UiBadge variant="primary".to_string()>
                                {move || state.get().total_count.to_string()}
                            </UiBadge>
                        </div>
                    </div>
                </div>

                <div class="page-header__actions">
                    <Space>
                        <UiButton
                            variant="primary".to_string()
                            on_click=Callback::new(post_selected)
                            disabled=state.get().selected_ids.is_empty() || posting_in_progress.get()
                        >
                            {icon("check")}
                            {move || format!("Post ({})", state.get().selected_ids.len())}
                        </UiButton>
                        <UiButton
                            variant="secondary".to_string()
                            on_click=Callback::new(unpost_selected)
                            disabled=state.get().selected_ids.is_empty() || posting_in_progress.get()
                        >
                            {icon("x")}
                            {move || format!("Unpost ({})", state.get().selected_ids.len())}
                        </UiButton>
                        <UiButton
                            variant="secondary".to_string()
                            on_click=Callback::new(move |_| {
                                let data = get_items();
                                if let Err(e) = export_to_csv(&data) {
                                    log!("Failed to export: {}", e);
                                }
                            })
                            disabled=loading.get() || state.get().sales.is_empty()
                        >
                            {icon("download")}
                            "Excel"
                        </UiButton>
                        <UiButton
                            variant="ghost".to_string()
                            size="sm".to_string()
                            on_click=Callback::new(restore_settings)
                            disabled=false
                        >
                            {icon("refresh")}
                        </UiButton>
                        <UiButton
                            variant="ghost".to_string()
                            size="sm".to_string()
                            on_click=Callback::new(save_settings_to_db)
                            disabled=false
                        >
                            {icon("save")}
                        </UiButton>
                        {move || save_notification.get().map(|msg| view! {
                            <span style="font-size: 12px; color: var(--colorNeutralForeground2, #666);">{msg}</span>
                        })}
                    </Space>
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
                                    <UiBadge variant="primary".to_string()>{count}</UiBadge>
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
                            page_size_options=vec![50, 100, 200, 500, 10000]
                        />
                    </div>

                    <div class="filter-panel-header__right">
                        <thaw::Button
                            appearance=ButtonAppearance::Subtle
                            on_click=move |_| load_sales()
                            disabled=loading.get()
                        >
                            {icon("refresh")}
                            {move || if loading.get() { "Загрузка..." } else { "Обновить" }}
                        </thaw::Button>
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
                        <Flex gap=FlexGap::Small align=FlexAlign::End>
                            <div style="min-width: 420px;">
                                <DateRangePicker
                                    date_from=Signal::derive(move || state.get().date_from)
                                    date_to=Signal::derive(move || state.get().date_to)
                                    on_change=Callback::new(move |(from, to)| {
                                        state.update(|s| {
                                            s.date_from = from;
                                            s.date_to = to;
                                            s.page = 0;
                                        });
                                        load_sales();
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

                            <div style="width: 150px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Sale ID:"</Label>
                                    <Input value=search_sale_id placeholder="S9100426422573" />
                                </Flex>
                            </div>

                            <div style="width: 150px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"SRID:"</Label>
                                    <Input value=search_srid placeholder="Document №" />
                                </Flex>
                            </div>
                        </Flex>
                    </div>
                </div>
            </div>

            // Selection summary panel (shows when items are selected) - keep existing logic
            {move || {
                let selected_count = state.with(|s| s.selected_ids.len());
                let is_processing = current_operation.get().is_some();

                if selected_count > 0 || is_processing {
                    let selected_totals = move || {
                        let sel_ids = state.with(|s| s.selected_ids.clone());
                        let all_items = get_items();
                        let selected_items: Vec<_> = all_items
                            .into_iter()
                            .filter(|item| sel_ids.contains(&item.id))
                            .collect();

                        let count = selected_items.len();
                        let total_qty: f64 = selected_items.iter().map(|s| s.qty).sum();
                        let total_amount: f64 =
                            selected_items.iter().filter_map(|s| s.amount_line).sum();
                        let total_price: f64 =
                            selected_items.iter().filter_map(|s| s.total_price).sum();
                        let total_finished: f64 =
                            selected_items.iter().filter_map(|s| s.finished_price).sum();

                        (count, total_qty, total_amount, total_price, total_finished)
                    };

                    let (sel_count, sel_qty, sel_amount, sel_price, sel_finished) = selected_totals();

                    let progress_percent =
                        if let Some((processed, total)) = current_operation.get() {
                            if total > 0 {
                                (processed as f64 / total as f64 * 100.0) as i32
                            } else {
                                0
                            }
                        } else {
                            0
                        };

                    let background_style = if is_processing {
                        if progress_percent == 0 {
                            "background: #ffffff; border: 1px solid #4CAF50; border-radius: 4px; padding: 8px 12px; margin: 0 var(--spacing-sm) var(--spacing-xs) var(--spacing-sm);"
                                .to_string()
                        } else {
                            format!(
                                "background: linear-gradient(to right, #c8e6c9 {}%, #ffffff {}%); border: 1px solid #4CAF50; border-radius: 4px; padding: 8px 12px; margin: 0 var(--spacing-sm) var(--spacing-xs) var(--spacing-sm);",
                                progress_percent, progress_percent
                            )
                        }
                    } else {
                        "background: #c8e6c9; border: 1px solid #4CAF50; border-radius: 4px; padding: 8px 12px; margin: 0 var(--spacing-sm) var(--spacing-xs) var(--spacing-sm);"
                            .to_string()
                    };

                    view! {
                        <div style=background_style>
                            <div style="display: flex; align-items: center; gap: 10px; flex-wrap: wrap;">
                                <span style="font-weight: 600; color: #2e7d32; font-size: 0.875rem;">
                                    "Выделено: " {sel_count} " строк"
                                </span>
                                <span style="font-size: 0.875rem; color: #424242;">
                                    "Кол-во: " {format_number(sel_qty)} " | "
                                    "К выплате: " {format_number(sel_amount)} " | "
                                    "Полная цена: " {format_number(sel_price)} " | "
                                    "Итоговая: " {format_number(sel_finished)}
                                </span>
                                <div style="margin-left: auto;">
                                    <thaw::Button
                                        appearance=ButtonAppearance::Subtle
                                        on_click=move |_| state.update(|s| s.selected_ids.clear())
                                        disabled=state.get().selected_ids.is_empty() || posting_in_progress.get()
                                    >
                                        {icon("x")}
                                        "Clear"
                                    </thaw::Button>
                                </div>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }}

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
                    {move || {
                        if loading.get() {
                            return view! {
                                <div class="loading-spinner" style="text-align: center; padding: 40px;">
                                    "Загрузка продаж..."
                                </div>
                            }.into_any();
                        }

                        let items = get_items();

                        view! {
                            // Only horizontal scrolling here; vertical scrolling is handled by `.page`
                            <div class="table-container" style="overflow-x: auto; overflow-y: visible;">
                                <Table attr:id=TABLE_ID attr:style="width: 100%; min-width: 1740px;">
                                    <TableHeader>
                                        <TableRow>
                                            <TableHeaderCell resizable=false class="fixed-checkbox-column">
                                                <input
                                                    type="checkbox"
                                                    style="cursor: pointer;"
                                                    on:change=toggle_all
                                                    prop:checked=move || all_selected()
                                                />
                                            </TableHeaderCell>

                                        <TableHeaderCell resizable=false min_width=130.0 class="resizable">
                                            <div
                                                class="table__sortable-header"
                                                style="cursor: pointer;"
                                                on:click=move |_| toggle_sort("document_no")
                                            >
                                                "SRID"
                                                <span class=move || state.with(|s| get_sort_class(&s.sort_field, "document_no"))>
                                                    {move || state.with(|s| get_sort_indicator(&s.sort_field, "document_no", s.sort_ascending))}
                                                </span>
                                            </div>
                                        </TableHeaderCell>
                                        <TableHeaderCell resizable=false min_width=140.0 class="resizable">
                                            <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("sale_id")>
                                                "Sale ID"
                                                <span class=move || state.with(|s| get_sort_class(&s.sort_field, "sale_id"))>
                                                    {move || state.with(|s| get_sort_indicator(&s.sort_field, "sale_id", s.sort_ascending))}
                                                </span>
                                            </div>
                                        </TableHeaderCell>
                                        <TableHeaderCell resizable=false min_width=85.0 class="resizable">
                                            <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("sale_date")>
                                                "Дата"
                                                <span class=move || state.with(|s| get_sort_class(&s.sort_field, "sale_date"))>
                                                    {move || state.with(|s| get_sort_indicator(&s.sort_field, "sale_date", s.sort_ascending))}
                                                </span>
                                            </div>
                                        </TableHeaderCell>
                                        <TableHeaderCell resizable=false min_width=85.0 class="resizable">
                                            <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("operation_date")>
                                                "Операция"
                                                <span class=move || state.with(|s| get_sort_class(&s.sort_field, "operation_date"))>
                                                    {move || state.with(|s| get_sort_indicator(&s.sort_field, "operation_date", s.sort_ascending))}
                                                </span>
                                            </div>
                                        </TableHeaderCell>
                                        <TableHeaderCell resizable=false min_width=140.0 class="resizable">
                                            <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("organization_name")>
                                                "Организация"
                                                <span class=move || state.with(|s| get_sort_class(&s.sort_field, "organization_name"))>
                                                    {move || state.with(|s| get_sort_indicator(&s.sort_field, "organization_name", s.sort_ascending))}
                                                </span>
                                            </div>
                                        </TableHeaderCell>
                                        <TableHeaderCell resizable=false min_width=100.0 class="resizable">
                                            <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("supplier_article")>
                                                "Артикул"
                                                <span class=move || state.with(|s| get_sort_class(&s.sort_field, "supplier_article"))>
                                                    {move || state.with(|s| get_sort_indicator(&s.sort_field, "supplier_article", s.sort_ascending))}
                                                </span>
                                            </div>
                                        </TableHeaderCell>
                                        <TableHeaderCell resizable=false min_width=90.0 class="resizable">
                                            <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("marketplace_article")>
                                                "Арт. МП"
                                                <span class=move || state.with(|s| get_sort_class(&s.sort_field, "marketplace_article"))>
                                                    {move || state.with(|s| get_sort_indicator(&s.sort_field, "marketplace_article", s.sort_ascending))}
                                                </span>
                                            </div>
                                        </TableHeaderCell>
                                        <TableHeaderCell resizable=false min_width=90.0 class="resizable">
                                            <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("nomenclature_article")>
                                                "Арт. 1С"
                                                <span class=move || state.with(|s| get_sort_class(&s.sort_field, "nomenclature_article"))>
                                                    {move || state.with(|s| get_sort_indicator(&s.sort_field, "nomenclature_article", s.sort_ascending))}
                                                </span>
                                            </div>
                                        </TableHeaderCell>
                                        <TableHeaderCell resizable=false min_width=70.0 class="resizable">
                                            <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("nomenclature_code")>
                                                "Код"
                                                <span class=move || state.with(|s| get_sort_class(&s.sort_field, "nomenclature_code"))>
                                                    {move || state.with(|s| get_sort_indicator(&s.sort_field, "nomenclature_code", s.sort_ascending))}
                                                </span>
                                            </div>
                                        </TableHeaderCell>
                                        <TableHeaderCell resizable=false min_width=200.0 class="resizable">
                                            <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("name")>
                                                "Название"
                                                <span class=move || state.with(|s| get_sort_class(&s.sort_field, "name"))>
                                                    {move || state.with(|s| get_sort_indicator(&s.sort_field, "name", s.sort_ascending))}
                                                </span>
                                            </div>
                                        </TableHeaderCell>
                                        <TableHeaderCell resizable=false min_width=55.0 class="resizable">
                                            <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("qty")>
                                                "Кол"
                                                <span class=move || state.with(|s| get_sort_class(&s.sort_field, "qty"))>
                                                    {move || state.with(|s| get_sort_indicator(&s.sort_field, "qty", s.sort_ascending))}
                                                </span>
                                            </div>
                                        </TableHeaderCell>
                                        <TableHeaderCell resizable=false min_width=90.0 class="resizable">
                                            <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("amount_line")>
                                                "К выплате"
                                                <span class=move || state.with(|s| get_sort_class(&s.sort_field, "amount_line"))>
                                                    {move || state.with(|s| get_sort_indicator(&s.sort_field, "amount_line", s.sort_ascending))}
                                                </span>
                                            </div>
                                        </TableHeaderCell>
                                        <TableHeaderCell resizable=false min_width=80.0 class="resizable">
                                            <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("total_price")>
                                                "Полная"
                                                <span class=move || state.with(|s| get_sort_class(&s.sort_field, "total_price"))>
                                                    {move || state.with(|s| get_sort_indicator(&s.sort_field, "total_price", s.sort_ascending))}
                                                </span>
                                            </div>
                                        </TableHeaderCell>
                                        <TableHeaderCell resizable=false min_width=70.0 class="resizable">
                                            <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("finished_price")>
                                                "Итог"
                                                <span class=move || state.with(|s| get_sort_class(&s.sort_field, "finished_price"))>
                                                    {move || state.with(|s| get_sort_indicator(&s.sort_field, "finished_price", s.sort_ascending))}
                                                </span>
                                            </div>
                                        </TableHeaderCell>
                                        <TableHeaderCell resizable=false min_width=70.0 class="resizable">
                                            <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("event_type")>
                                                "Тип"
                                                <span class=move || state.with(|s| get_sort_class(&s.sort_field, "event_type"))>
                                                    {move || state.with(|s| get_sort_indicator(&s.sort_field, "event_type", s.sort_ascending))}
                                                </span>
                                            </div>
                                        </TableHeaderCell>
                                        </TableRow>
                                    </TableHeader>

                                    <TableBody>
                                        {items.into_iter().map(|item| {
                                            let id = item.id.clone();
                                            let doc_no = item.document_no.clone();
                                            let sale_id = item.sale_id.clone().unwrap_or_else(|| "—".to_string());
                                            let date = format_date(&item.sale_date);
                                            let op_date = item.operation_date.clone().unwrap_or_else(|| "—".to_string());
                                            let org_name = item.organization_name.clone().unwrap_or_else(|| "—".to_string());
                                            let supplier_art = item.supplier_article;
                                            let mp_art = item.marketplace_article.clone().unwrap_or_else(|| "—".to_string());
                                            let nom_art = item.nomenclature_article.clone().unwrap_or_else(|| "—".to_string());
                                            let nom_code = item.nomenclature_code.clone().unwrap_or_else(|| "—".to_string());
                                            let name = item.name;
                                            let qty = format!("{:.0}", item.qty);
                                            let amount = item.amount_line.map(|a| format!("{:.2}", a)).unwrap_or_else(|| "—".to_string());
                                            let total = item.total_price.map(|a| format!("{:.2}", a)).unwrap_or_else(|| "—".to_string());
                                            let finished = item.finished_price.map(|a| format!("{:.2}", a)).unwrap_or_else(|| "—".to_string());
                                            let event = item.event_type;

                                            let id_check = id.clone();
                                            let id_toggle = id.clone();
                                            let id_for_open = id.clone();
                                            let doc_for_open = doc_no.clone();

                                            view! {
                                                <TableRow>
                                                    <TableCell class="fixed-checkbox-column">
                                                        <input
                                                            type="checkbox"
                                                            style="cursor: pointer;"
                                                            on:click=|e| e.stop_propagation()
                                                            prop:checked=move || is_selected(&id_check)
                                                            on:change=move |_ev| toggle_selection(id_toggle.clone())
                                                        />
                                                    </TableCell>
                                                    <TableCell>
                                                        <TableCellLayout truncate=true>
                                                            <a
                                                                href="#"
                                                                style="color: var(--colorBrandForeground1); text-decoration: none; cursor: pointer;"
                                                                on:click=move |e| {
                                                                    e.prevent_default();
                                                                    open_detail(id_for_open.clone(), doc_for_open.clone());
                                                                }
                                                            >
                                                                {doc_no}
                                                            </a>
                                                        </TableCellLayout>
                                                    </TableCell>
                                                    <TableCell><TableCellLayout truncate=true>{sale_id}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{date}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{op_date}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout truncate=true>{org_name}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout truncate=true>{supplier_art}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout truncate=true>{mp_art}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout truncate=true>{nom_art}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout truncate=true>{nom_code}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout truncate=true>{name}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{qty}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{amount}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{total}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{finished}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{event}</TableCellLayout></TableCell>
                                                </TableRow>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </TableBody>
                                </Table>
                            </div>
                        }.into_any()
                    }}
                </div>
            </div>
        </div>
    }
}

async fn load_saved_settings(form_key: &str) -> Result<Option<serde_json::Value>, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("/api/form-settings/{}", form_key);
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;

    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let text = JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;

    // Response is Option<FormSettings>
    let response: Option<serde_json::Value> =
        serde_json::from_str(&text).map_err(|e| format!("{e}"))?;

    if let Some(form_settings) = response {
        if let Some(settings_json) = form_settings.get("settings_json").and_then(|v| v.as_str()) {
            let settings: serde_json::Value =
                serde_json::from_str(settings_json).map_err(|e| format!("{e}"))?;
            return Ok(Some(settings));
        }
    }

    Ok(None)
}

async fn save_settings_to_database(
    form_key: &str,
    settings: serde_json::Value,
) -> Result<(), String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let request_body = json!({
        "form_key": form_key,
        "settings": settings,
    });

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    let body_str = serde_json::to_string(&request_body).map_err(|e| format!("{e}"))?;
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body_str));

    let url = "/api/form-settings";
    let request = Request::new_with_str_and_init(url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;

    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }

    Ok(())
}

/// Загрузка списка организаций
async fn fetch_organizations() -> Result<Vec<Organization>, String> {
    use web_sys::{Request as WebRequest, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = "http://localhost:3000/api/organization";
    let request = WebRequest::new_with_str_and_init(url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;

    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let text = JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    let data: Vec<Organization> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}

/// Экспорт WB Sales в CSV для Excel
fn export_to_csv(data: &[WbSalesDto]) -> Result<(), String> {
    // UTF-8 BOM для правильного отображения кириллицы в Excel
    let mut csv = String::from("\u{FEFF}");

    // Заголовок с точкой с запятой как разделитель
    csv.push_str("Document №;Дата продажи;Дата операции;Организация;Артикул;Артикул МП;Артикул 1С;Код 1С;Название;Количество;К выплате;Полная цена;Итоговая цена;Тип\n");

    for sale in data {
        let sale_date = format_date(&sale.sale_date);
        let operation_date = sale
            .operation_date
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("—");
        let org_name = sale
            .organization_name
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("—");
        let mp_article = sale
            .marketplace_article
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("—");
        let nom_code = sale
            .nomenclature_code
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("—");
        let nom_article = sale
            .nomenclature_article
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("—");

        // Форматируем суммы с запятой как десятичный разделитель
        let qty_str = format!("{:.0}", sale.qty);
        let amount_str = sale
            .amount_line
            .map(|a| format!("{:.2}", a).replace(".", ","))
            .unwrap_or_else(|| "—".to_string());
        let total_price_str = sale
            .total_price
            .map(|a| format!("{:.2}", a).replace(".", ","))
            .unwrap_or_else(|| "—".to_string());
        let finished_price_str = sale
            .finished_price
            .map(|a| format!("{:.2}", a).replace(".", ","))
            .unwrap_or_else(|| "—".to_string());

        csv.push_str(&format!(
            "\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";{};{};{};{};\"{}\"\n",
            sale.document_no.replace('\"', "\"\""),
            sale_date,
            operation_date.replace('\"', "\"\""),
            org_name.replace('\"', "\"\""),
            sale.supplier_article.replace('\"', "\"\""),
            mp_article.replace('\"', "\"\""),
            nom_article.replace('\"', "\"\""),
            nom_code.replace('\"', "\"\""),
            sale.name.replace('\"', "\"\""),
            qty_str,
            amount_str,
            total_price_str,
            finished_price_str,
            sale.event_type.replace('\"', "\"\"")
        ));
    }

    // Создаем Blob с CSV данными
    let blob_parts = js_sys::Array::new();
    blob_parts.push(&wasm_bindgen::JsValue::from_str(&csv));

    let blob_props = BlobPropertyBag::new();
    blob_props.set_type("text/csv;charset=utf-8;");

    let blob = Blob::new_with_str_sequence_and_options(&blob_parts, &blob_props)
        .map_err(|e| format!("Failed to create blob: {:?}", e))?;

    // Создаем URL для blob
    let url = Url::create_object_url_with_blob(&blob)
        .map_err(|e| format!("Failed to create URL: {:?}", e))?;

    // Создаем временную ссылку для скачивания
    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let document = window.document().ok_or_else(|| "no document".to_string())?;

    let a = document
        .create_element("a")
        .map_err(|e| format!("Failed to create element: {:?}", e))?
        .dyn_into::<HtmlAnchorElement>()
        .map_err(|e| format!("Failed to cast to anchor: {:?}", e))?;

    a.set_href(&url);
    let filename = format!(
        "wb_sales_{}.csv",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );
    a.set_download(&filename);
    a.click();

    // Освобождаем URL
    Url::revoke_object_url(&url).map_err(|e| format!("Failed to revoke URL: {:?}", e))?;

    Ok(())
}
