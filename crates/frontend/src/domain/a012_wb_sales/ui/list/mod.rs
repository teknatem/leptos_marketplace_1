use crate::domain::a012_wb_sales::state::create_state;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::components::date_input::DateInput;
use crate::shared::components::month_selector::MonthSelector;
use crate::shared::list_utils::{format_number, get_sort_class, get_sort_indicator, Sortable};
use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::cmp::Ordering;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, HtmlElement, MouseEvent as WebMouseEvent, Url};

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
        sale_date: v.get("sale_date")?.as_str()?.to_string(),
        supplier_article: v.get("supplier_article")?.as_str()?.to_string(),
        name: v.get("name")?.as_str()?.to_string(),
        qty: v.get("qty")?.as_f64()?,
        amount_line: v.get("amount_line").and_then(|a| a.as_f64()),
        total_price: v.get("total_price").and_then(|a| a.as_f64()),
        finished_price: v.get("finished_price").and_then(|a| a.as_f64()),
        event_type: v.get("event_type")?.as_str()?.to_string(),
        organization_name: v.get("organization_name").and_then(|a| a.as_str()).map(|s| s.to_string()),
        marketplace_article: v.get("marketplace_article").and_then(|a| a.as_str()).map(|s| s.to_string()),
        nomenclature_code: v.get("nomenclature_code").and_then(|a| a.as_str()).map(|s| s.to_string()),
        nomenclature_article: v.get("nomenclature_article").and_then(|a| a.as_str()).map(|s| s.to_string()),
        operation_date: v.get("operation_date").and_then(|a| a.as_str()).map(|s| s.to_string()),
    });

    if result.is_none() {
        log!("Failed to parse item {}", idx);
    }

    result
}

/// Check if resize just happened (to block sort click)
fn was_just_resizing() -> bool {
    web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.body())
        .map(|b| b.get_attribute("data-was-resizing").as_deref() == Some("true"))
        .unwrap_or(false)
}

/// Clear the resize flag
fn clear_resize_flag() {
    if let Some(body) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.body()) {
        let _ = body.remove_attribute("data-was-resizing");
    }
}

const COLUMN_WIDTHS_KEY: &str = "a012_wb_sales_column_widths";

/// Save column widths to localStorage
fn save_column_widths(table_id: &str) {
    let Some(window) = web_sys::window() else { return };
    let Some(document) = window.document() else { return };
    let Some(storage) = window.local_storage().ok().flatten() else { return };
    let Some(table) = document.get_element_by_id(table_id) else { return };
    
    let headers = table.query_selector_all("th.resizable").ok();
    let Some(headers) = headers else { return };
    
    let mut widths: Vec<i32> = Vec::new();
    for i in 0..headers.length() {
        if let Some(th) = headers.get(i) {
            if let Ok(th) = th.dyn_into::<HtmlElement>() {
                widths.push(th.offset_width());
            }
        }
    }
    
    if let Ok(json) = serde_json::to_string(&widths) {
        let _ = storage.set_item(COLUMN_WIDTHS_KEY, &json);
    }
}

/// Restore column widths from localStorage
fn restore_column_widths(table_id: &str) {
    let Some(window) = web_sys::window() else { return };
    let Some(document) = window.document() else { return };
    let Some(storage) = window.local_storage().ok().flatten() else { return };
    let Some(table) = document.get_element_by_id(table_id) else { return };
    
    let Some(json) = storage.get_item(COLUMN_WIDTHS_KEY).ok().flatten() else { return };
    let Ok(widths): Result<Vec<i32>, _> = serde_json::from_str(&json) else { return };
    
    let headers = table.query_selector_all("th.resizable").ok();
    let Some(headers) = headers else { return };
    
    for (i, width) in widths.iter().enumerate() {
        if let Some(th) = headers.get(i as u32) {
            if let Ok(th) = th.dyn_into::<HtmlElement>() {
                let _ = th.style().set_property("width", &format!("{}px", width));
                let _ = th.style().set_property("min-width", &format!("{}px", width));
            }
        }
    }
}

/// Initialize column resize for all resizable headers in a table
fn init_column_resize(table_id: &str) {
    let Some(window) = web_sys::window() else { return };
    let Some(document) = window.document() else { return };
    let Some(table) = document.get_element_by_id(table_id) else { return };
    
    // First restore saved widths
    restore_column_widths(table_id);
    
    let headers = table.query_selector_all("th.resizable").ok();
    let Some(headers) = headers else { return };
    
    let table_id_owned = table_id.to_string();
    
    for i in 0..headers.length() {
        let Some(th) = headers.get(i) else { continue };
        let Ok(th) = th.dyn_into::<HtmlElement>() else { continue };
        
        // Skip if already has resize handle
        if th.query_selector(".resize-handle").ok().flatten().is_some() {
            continue;
        }
        
        // Create resize handle
        let Ok(handle) = document.create_element("div") else { continue };
        handle.set_class_name("resize-handle");
        
        // State for this column
        let resizing = Rc::new(RefCell::new(false));
        let did_resize = Rc::new(RefCell::new(false));
        let start_x = Rc::new(RefCell::new(0i32));
        let start_width = Rc::new(RefCell::new(0i32));
        let th_ref = Rc::new(RefCell::new(th.clone()));
        let table_id_for_save = table_id_owned.clone();
        
        // Mousedown on handle
        let resizing_md = resizing.clone();
        let did_resize_md = did_resize.clone();
        let start_x_md = start_x.clone();
        let start_width_md = start_width.clone();
        let th_md = th_ref.clone();
        
        let mousedown = Closure::wrap(Box::new(move |e: WebMouseEvent| {
            e.prevent_default();
            e.stop_propagation();
            *resizing_md.borrow_mut() = true;
            *did_resize_md.borrow_mut() = false;
            *start_x_md.borrow_mut() = e.client_x();
            *start_width_md.borrow_mut() = th_md.borrow().offset_width();
            
            if let Some(body) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.body()) {
                let _ = body.class_list().add_1("resizing-column");
            }
        }) as Box<dyn FnMut(WebMouseEvent)>);
        
        let _ = handle.add_event_listener_with_callback("mousedown", mousedown.as_ref().unchecked_ref());
        mousedown.forget();
        
        // Mousemove on document
        let resizing_mm = resizing.clone();
        let did_resize_mm = did_resize.clone();
        let start_x_mm = start_x.clone();
        let start_width_mm = start_width.clone();
        let th_mm = th_ref.clone();
        
        let mousemove = Closure::wrap(Box::new(move |e: WebMouseEvent| {
            if !*resizing_mm.borrow() { return; }
            *did_resize_mm.borrow_mut() = true;
            let diff = e.client_x() - *start_x_mm.borrow();
            let new_width = (*start_width_mm.borrow() + diff).max(40);
            let _ = th_mm.borrow().style().set_property("width", &format!("{}px", new_width));
            let _ = th_mm.borrow().style().set_property("min-width", &format!("{}px", new_width));
        }) as Box<dyn FnMut(WebMouseEvent)>);
        
        let _ = document.add_event_listener_with_callback("mousemove", mousemove.as_ref().unchecked_ref());
        mousemove.forget();
        
        // Mouseup on document
        let resizing_mu = resizing.clone();
        let did_resize_mu = did_resize.clone();
        let table_id_mu = table_id_for_save.clone();
        
        let mouseup = Closure::wrap(Box::new(move |_: WebMouseEvent| {
            if !*resizing_mu.borrow() { return; }
            let was_resizing = *did_resize_mu.borrow();
            *resizing_mu.borrow_mut() = false;
            *did_resize_mu.borrow_mut() = false;
            
            if let Some(body) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.body()) {
                let _ = body.class_list().remove_1("resizing-column");
                if was_resizing {
                    // Save column widths to localStorage
                    save_column_widths(&table_id_mu);
                    let _ = body.set_attribute("data-was-resizing", "true");
                    spawn_local(async {
                        gloo_timers::future::TimeoutFuture::new(50).await;
                        clear_resize_flag();
                    });
                }
            }
        }) as Box<dyn FnMut(WebMouseEvent)>);
        
        let _ = document.add_event_listener_with_callback("mouseup", mouseup.as_ref().unchecked_ref());
        mouseup.forget();
        
        let _ = th.append_child(&handle);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbSalesDto {
    pub id: String,
    pub document_no: String,
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
                                        let items: Vec<WbSalesDto> = paginated.items
                                            .into_iter()
                                            .enumerate()
                                            .filter_map(|(idx, v)| {
                                                parse_wb_sales_item(&v, idx)
                                            })
                                            .collect();

                                        log!("Successfully parsed {} sales", items.len());
                                        state.update(|s| {
                                            s.sales = items;
                                            s.total_count = paginated.total;
                                            s.total_pages = paginated.total_pages;
                                            s.is_loaded = true;
                                        });
                                        set_loading.set(false);
                                    }
                                    Err(e) => {
                                        log!("Failed to parse paginated response: {:?}", e);
                                        set_error.set(Some(format!("Failed to parse response: {}", e)));
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
    let get_items = move || -> Vec<WbSalesDto> {
        state.with(|s| s.sales.clone())
    };

    // Мемоизированные итоги - вычисляются только при изменении sales
    let totals = Memo::new(move |_| {
        state.with(|s| {
            let total_qty: f64 = s.sales.iter().map(|item| item.qty).sum();
            let total_amount: f64 = s.sales.iter().filter_map(|item| item.amount_line).sum();
            let total_price: f64 = s.sales.iter().filter_map(|item| item.total_price).sum();
            let total_finished: f64 = s.sales.iter().filter_map(|item| item.finished_price).sum();
            (
                s.sales.len(),
                total_qty,
                total_amount,
                total_price,
                total_finished,
            )
        })
    });

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
    let post_selected = move |_| {
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
    let unpost_selected = move |_| {
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
                let response = Request::post("http://localhost:3000/api/a012/wb-sales/batch-unpost")
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
    let save_settings_to_db = move |_| {
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
    let restore_settings = move |_| {
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
        <div class="wb-sales-list" style="background: #f8f9fa; padding: 12px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
            // Header - Row 1: Title with Pagination, Post/Unpost and Settings Buttons
            <div style="background: linear-gradient(135deg, #4a5568 0%, #2d3748 100%); padding: 8px 12px; border-radius: 6px 6px 0 0; margin: -12px -12px 0 -12px; display: flex; align-items: center; justify-content: space-between;">
                <div style="display: flex; align-items: center; gap: 12px;">
                    <h2 style="margin: 0; font-size: 1.1rem; font-weight: 600; color: white; letter-spacing: 0.5px;">"📋 WB Sales"</h2>

                    // === PAGINATION CONTROLS ===
                    <div style="display: flex; align-items: center; gap: 6px; background: rgba(255,255,255,0.15); padding: 4px 10px; border-radius: 6px;">
                        // First page button
                        <button
                            style="background: none; border: none; color: white; cursor: pointer; padding: 4px 6px; border-radius: 4px; font-size: 12px; opacity: 0.9; transition: all 0.2s;"
                            prop:disabled=move || state.with(|s| s.page == 0) || loading.get()
                            on:click=move |_| go_to_page(0)
                            title="Первая страница"
                        >
                            "⏮"
                        </button>

                        // Previous page button
                        <button
                            style="background: none; border: none; color: white; cursor: pointer; padding: 4px 6px; border-radius: 4px; font-size: 12px; opacity: 0.9; transition: all 0.2s;"
                            prop:disabled=move || state.with(|s| s.page == 0) || loading.get()
                            on:click=move |_| {
                                let current = state.with(|s| s.page);
                                if current > 0 {
                                    go_to_page(current - 1);
                                }
                            }
                            title="Предыдущая страница"
                        >
                            "◀"
                        </button>

                        // Page info
                        <span style="color: white; font-size: 12px; font-weight: 500; min-width: 100px; text-align: center;">
                            {move || {
                                let page = state.with(|s| s.page);
                                let total_pages = state.with(|s| s.total_pages);
                                let total = state.with(|s| s.total_count);
                                format!("{} / {} ({})", page + 1, total_pages.max(1), total)
                            }}
                        </span>

                        // Next page button
                        <button
                            style="background: none; border: none; color: white; cursor: pointer; padding: 4px 6px; border-radius: 4px; font-size: 12px; opacity: 0.9; transition: all 0.2s;"
                            prop:disabled=move || state.with(|s| s.page >= s.total_pages.saturating_sub(1)) || loading.get()
                            on:click=move |_| {
                                let current = state.with(|s| s.page);
                                let max_page = state.with(|s| s.total_pages.saturating_sub(1));
                                if current < max_page {
                                    go_to_page(current + 1);
                                }
                            }
                            title="Следующая страница"
                        >
                            "▶"
                        </button>

                        // Last page button
                        <button
                            style="background: none; border: none; color: white; cursor: pointer; padding: 4px 6px; border-radius: 4px; font-size: 12px; opacity: 0.9; transition: all 0.2s;"
                            prop:disabled=move || state.with(|s| s.page >= s.total_pages.saturating_sub(1)) || loading.get()
                            on:click=move |_| {
                                let max_page = state.with(|s| s.total_pages.saturating_sub(1));
                                go_to_page(max_page);
                            }
                            title="Последняя страница"
                        >
                            "⏭"
                        </button>

                        // Divider
                        <div style="width: 1px; height: 18px; background: rgba(255,255,255,0.3); margin: 0 4px;"></div>

                        // Page size selector
                        <select
                            style="background: rgba(255,255,255,0.2); color: white; border: 1px solid rgba(255,255,255,0.3); border-radius: 4px; padding: 3px 6px; font-size: 11px; cursor: pointer;"
                            prop:value=move || state.with(|s| s.page_size.to_string())
                            on:change=move |ev| {
                                if let Ok(size) = event_target_value(&ev).parse::<usize>() {
                                    change_page_size(size);
                                }
                            }
                        >
                            <option value="50" style="color: black;">"50"</option>
                            <option value="100" style="color: black;">"100"</option>
                            <option value="200" style="color: black;">"200"</option>
                            <option value="500" style="color: black;">"500"</option>
                        </select>
                        <span style="color: rgba(255,255,255,0.8); font-size: 10px;">"на стр."</span>
                    </div>
                    // === END PAGINATION ===

                    // Post/Unpost buttons
                    <button
                        class="btn btn-success"
                        prop:disabled=move || state.with(|s| s.selected_ids.is_empty()) || posting_in_progress.get()
                        on:click=post_selected
                    >
                        {move || format!("✓ Post ({})", state.with(|s| s.selected_ids.len()))}
                    </button>
                    <button
                        class="btn btn-warning"
                        prop:disabled=move || state.with(|s| s.selected_ids.is_empty()) || posting_in_progress.get()
                        on:click=unpost_selected
                    >
                        {move || format!("✗ Unpost ({})", state.with(|s| s.selected_ids.len()))}
                    </button>
                </div>

                <div style="display: flex; gap: 8px; align-items: center;">
                    // Excel export button
                    <button
                        class="btn btn-excel"
                        on:click=move |_| {
                            let data = get_items();
                            if let Err(e) = export_to_csv(&data) {
                                log!("Failed to export: {}", e);
                            }
                        }
                        prop:disabled=move || loading.get() || state.with(|s| s.sales.is_empty())
                    >
                        "📊 Excel"
                    </button>

                    {move || {
                        if let Some(msg) = save_notification.get() {
                            view! {
                                <span style="font-size: 0.75rem; color: white; font-weight: 500; margin-right: 8px;">{msg}</span>
                            }.into_any()
                        } else {
                            view! { <></> }.into_any()
                        }
                    }}
                    <button
                        class="btn btn-icon btn-icon-transparent"
                        on:click=restore_settings
                        title="Восстановить настройки из базы данных"
                    >
                        "🔄"
                    </button>
                    <button
                        class="btn btn-icon btn-icon-transparent"
                        on:click=save_settings_to_db
                        title="Сохранить настройки в базу данных"
                    >
                        "💾"
                    </button>
                </div>
            </div>

            // Header - Row 2: Filters and Actions - All in one row
            <div style="background: white; padding: 8px 12px; margin: 0 -12px 10px -12px; border-bottom: 1px solid #e9ecef; display: flex; align-items: center; gap: 12px; flex-wrap: wrap;">
                // Period section
                <div style="display: flex; align-items: center; gap: 8px;">
                    <label style="margin: 0; font-size: 0.875rem; font-weight: 500; color: #495057; white-space: nowrap;">"Период:"</label>
                    <DateInput
                        value=Signal::derive(move || state.get().date_from)
                        on_change=move |val| state.update(|s| s.date_from = val)
                    />
                    <span style="color: #6c757d;">"—"</span>
                    <DateInput
                        value=Signal::derive(move || state.get().date_to)
                        on_change=move |val| state.update(|s| s.date_to = val)
                    />
                    <MonthSelector
                        on_select=Callback::new(move |(from, to)| {
                            state.update(|s| {
                                s.date_from = from;
                                s.date_to = to;
                            });
                        })
                    />
                </div>

                // Organization filter
                <div style="display: flex; align-items: center; gap: 8px;">
                    <label style="margin: 0; font-size: 0.875rem; font-weight: 500; color: #495057; white-space: nowrap;">"Организация:"</label>
                    <select
                        prop:value=move || state.get().selected_organization_id.clone().unwrap_or_default()
                        on:change=move |ev| {
                            let value = event_target_value(&ev);
                            state.update(|s| {
                                if value.is_empty() {
                                    s.selected_organization_id = None;
                                } else {
                                    s.selected_organization_id = Some(value);
                                }
                            });
                        }
                        style="padding: 6px 10px; border: 1px solid #ced4da; border-radius: 4px; font-size: 0.875rem; min-width: 200px; background: #fff;"
                    >
                        <option value="">"Все организации"</option>
                        {move || organizations.get().into_iter().map(|org| {
                            let org_id = org.id.clone();
                            let org_id_for_selected = org.id.clone();
                            let org_desc = org.description.clone();
                            view! {
                                <option value=org_id selected=move || {
                                    state.get().selected_organization_id.as_ref() == Some(&org_id_for_selected)
                                }>
                                    {org_desc}
                                </option>
                            }
                        }).collect_view()}
                    </select>
                </div>

                // Update button
                <button
                    class="btn btn-success"
                    on:click=move |_| {
                        load_sales();
                    }
                    prop:disabled=move || loading.get()
                >
                    "↻ Обновить"
                </button>
            </div>

            // Totals display (for current page)
            {move || if !loading.get() {
                let (count, total_qty, total_amount, total_price, total_finished) = totals.get();
                let total_count = state.with(|s| s.total_count);
                view! {
                    <div style="margin-bottom: 10px; padding: 3px 12px; background: var(--color-background-alt, #f5f5f5); border-radius: 4px; display: flex; align-items: center; flex-wrap: wrap;">
                        <span style="font-size: 0.875rem; font-weight: 600; color: var(--color-text);">
                            "На странице: " {format_number(count as f64)} " из " {format_number(total_count as f64)} " | "
                            "Кол-во: " {format_number(total_qty)} " | "
                            "К выплате: " {format_number(total_amount)} " | "
                            "Полная цена: " {format_number(total_price)} " | "
                            "Итоговая: " {format_number(total_finished)}
                        </span>
                    </div>
                }.into_any()
            } else {
                view! { <></> }.into_any()
            }}

            // Selection summary panel (shows when items are selected)
            {move || {
                let selected_count = state.with(|s| s.selected_ids.len());
                let is_processing = current_operation.get().is_some();

                if selected_count > 0 || is_processing {
                    let selected_totals = move || {
                        let sel_ids = state.with(|s| s.selected_ids.clone());
                        let all_items = get_items();
                        let selected_items: Vec<_> = all_items.into_iter()
                            .filter(|item| sel_ids.contains(&item.id))
                            .collect();

                        let count = selected_items.len();
                        let total_qty: f64 = selected_items.iter().map(|s| s.qty).sum();
                        let total_amount: f64 = selected_items.iter().filter_map(|s| s.amount_line).sum();
                        let total_price: f64 = selected_items.iter().filter_map(|s| s.total_price).sum();
                        let total_finished: f64 = selected_items.iter().filter_map(|s| s.finished_price).sum();

                        (count, total_qty, total_amount, total_price, total_finished)
                    };

                    let (sel_count, sel_qty, sel_amount, sel_price, sel_finished) = selected_totals();

                    let progress_percent = if let Some((processed, total)) = current_operation.get() {
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
                            "background: #ffffff; border: 1px solid #4CAF50; border-radius: 4px; padding: 8px 12px; margin-bottom: 8px;".to_string()
                        } else {
                            format!("background: linear-gradient(to right, #c8e6c9 {}%, #ffffff {}%); border: 1px solid #4CAF50; border-radius: 4px; padding: 8px 12px; margin-bottom: 8px;", progress_percent, progress_percent)
                        }
                    } else {
                        "background: #c8e6c9; border: 1px solid #4CAF50; border-radius: 4px; padding: 8px 12px; margin-bottom: 8px;".to_string()
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
                                    <button
                                        class="btn btn-secondary"
                                        on:click=move |_| state.update(|s| s.selected_ids.clear())
                                        prop:disabled=move || state.with(|s| s.selected_ids.is_empty()) || posting_in_progress.get()
                                    >
                                        "✕ Clear"
                                    </button>
                                </div>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }}

            {move || error.get().map(|err| view! {
                <div class="error-message" style="padding: 12px; background: #ffebee; border: 1px solid #ffcdd2; border-radius: 4px; color: #c62828; margin-bottom: 10px;">{err}</div>
            })}

            {move || {
                if loading.get() {
                    view! {
                        <div class="loading-spinner" style="text-align: center; padding: 40px;">"Загрузка продаж..."</div>
                    }.into_any()
                } else {
                    let items = get_items();
                    let current_sort_field = state.with(|s| s.sort_field.clone());
                    let current_sort_asc = state.with(|s| s.sort_ascending);

                    // Initialize column resize after each render
                    spawn_local(async {
                        gloo_timers::future::TimeoutFuture::new(50).await;
                        init_column_resize("wb-sales-table");
                    });

                    view! {
                        <div class="table-container" style="overflow: auto; max-height: calc(100vh - 240px); position: relative;">
                            <table id="wb-sales-table" class="data-table table-striped" style="min-width: 1600px; table-layout: fixed;">
                                <thead>
                                    <tr>
                                        <th class="checkbox-cell" style="width: 40px; min-width: 40px;">
                                            <input
                                                type="checkbox"
                                                on:change=toggle_all
                                                prop:checked=move || all_selected()
                                            />
                                        </th>
                                        <th class="resizable" style="width: 130px; min-width: 80px;" on:click=move |_| toggle_sort("document_no")>
                                            <span class="sortable-header">"Document №" <span class={get_sort_class("document_no", &current_sort_field)}>{get_sort_indicator("document_no", &current_sort_field, current_sort_asc)}</span></span>
                                        </th>
                                        <th class="resizable" style="width: 85px; min-width: 60px;" on:click=move |_| toggle_sort("sale_date")>
                                            <span class="sortable-header">"Дата" <span class={get_sort_class("sale_date", &current_sort_field)}>{get_sort_indicator("sale_date", &current_sort_field, current_sort_asc)}</span></span>
                                        </th>
                                        <th class="resizable" style="width: 85px; min-width: 60px;" on:click=move |_| toggle_sort("operation_date")>
                                            <span class="sortable-header">"Операция" <span class={get_sort_class("operation_date", &current_sort_field)}>{get_sort_indicator("operation_date", &current_sort_field, current_sort_asc)}</span></span>
                                        </th>
                                        <th class="resizable" style="width: 140px; min-width: 80px;" on:click=move |_| toggle_sort("organization_name")>
                                            <span class="sortable-header">"Организация" <span class={get_sort_class("organization_name", &current_sort_field)}>{get_sort_indicator("organization_name", &current_sort_field, current_sort_asc)}</span></span>
                                        </th>
                                        <th class="resizable" style="width: 100px; min-width: 60px;" on:click=move |_| toggle_sort("supplier_article")>
                                            <span class="sortable-header">"Артикул" <span class={get_sort_class("supplier_article", &current_sort_field)}>{get_sort_indicator("supplier_article", &current_sort_field, current_sort_asc)}</span></span>
                                        </th>
                                        <th class="resizable" style="width: 90px; min-width: 60px;" on:click=move |_| toggle_sort("marketplace_article")>
                                            <span class="sortable-header">"Арт. МП" <span class={get_sort_class("marketplace_article", &current_sort_field)}>{get_sort_indicator("marketplace_article", &current_sort_field, current_sort_asc)}</span></span>
                                        </th>
                                        <th class="resizable" style="width: 90px; min-width: 60px;" on:click=move |_| toggle_sort("nomenclature_article")>
                                            <span class="sortable-header">"Арт. 1С" <span class={get_sort_class("nomenclature_article", &current_sort_field)}>{get_sort_indicator("nomenclature_article", &current_sort_field, current_sort_asc)}</span></span>
                                        </th>
                                        <th class="resizable" style="width: 70px; min-width: 50px;" on:click=move |_| toggle_sort("nomenclature_code")>
                                            <span class="sortable-header">"Код" <span class={get_sort_class("nomenclature_code", &current_sort_field)}>{get_sort_indicator("nomenclature_code", &current_sort_field, current_sort_asc)}</span></span>
                                        </th>
                                        <th class="resizable" style="min-width: 150px;" on:click=move |_| toggle_sort("name")>
                                            <span class="sortable-header">"Название" <span class={get_sort_class("name", &current_sort_field)}>{get_sort_indicator("name", &current_sort_field, current_sort_asc)}</span></span>
                                        </th>
                                        <th class="resizable text-right" style="width: 55px; min-width: 45px;" on:click=move |_| toggle_sort("qty")>
                                            <span class="sortable-header" style="justify-content: flex-end;">"Кол" <span class={get_sort_class("qty", &current_sort_field)}>{get_sort_indicator("qty", &current_sort_field, current_sort_asc)}</span></span>
                                        </th>
                                        <th class="resizable text-right" style="width: 90px; min-width: 70px;" on:click=move |_| toggle_sort("amount_line")>
                                            <span class="sortable-header" style="justify-content: flex-end;">"К выплате" <span class={get_sort_class("amount_line", &current_sort_field)}>{get_sort_indicator("amount_line", &current_sort_field, current_sort_asc)}</span></span>
                                        </th>
                                        <th class="resizable text-right" style="width: 80px; min-width: 60px;" on:click=move |_| toggle_sort("total_price")>
                                            <span class="sortable-header" style="justify-content: flex-end;">"Полная" <span class={get_sort_class("total_price", &current_sort_field)}>{get_sort_indicator("total_price", &current_sort_field, current_sort_asc)}</span></span>
                                        </th>
                                        <th class="resizable text-right" style="width: 70px; min-width: 50px;" on:click=move |_| toggle_sort("finished_price")>
                                            <span class="sortable-header" style="justify-content: flex-end;">"Итог" <span class={get_sort_class("finished_price", &current_sort_field)}>{get_sort_indicator("finished_price", &current_sort_field, current_sort_asc)}</span></span>
                                        </th>
                                        <th class="resizable" style="width: 60px; min-width: 45px;" on:click=move |_| toggle_sort("event_type")>
                                            <span class="sortable-header">"Тип" <span class={get_sort_class("event_type", &current_sort_field)}>{get_sort_indicator("event_type", &current_sort_field, current_sort_asc)}</span></span>
                                        </th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {items.into_iter().map(|item| {
                                        // Pre-compute all values once
                                        let id = item.id.clone();
                                        let doc_no = item.document_no.clone();
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
                                        
                                        // Clone once for closures
                                        let id_check = id.clone();
                                        let id_toggle = id.clone();
                                        let id_row = id.clone();
                                        let doc_row = doc_no.clone();
                                        
                                        // Single click handler for entire row
                                        let on_row_click = move |_| {
                                            open_detail(id_row.clone(), doc_row.clone());
                                        };
                                        
                                        view! {
                                            <tr on:click=on_row_click.clone()>
                                                <td class="checkbox-cell" on:click=move |e| e.stop_propagation()>
                                                    <input
                                                        type="checkbox"
                                                        prop:checked=move || is_selected(&id_check)
                                                        on:change=move |_| toggle_selection(id_toggle.clone())
                                                    />
                                                </td>
                                                <td class="cell-truncate">{doc_no}</td>
                                                <td>{date}</td>
                                                <td style="color: #c62828; font-weight: 500;">{op_date}</td>
                                                <td class="cell-truncate">{org_name}</td>
                                                <td class="cell-truncate">{supplier_art}</td>
                                                <td class="cell-truncate" style="color: #1565c0;">{mp_art}</td>
                                                <td class="cell-truncate" style="color: #2e7d32;">{nom_art}</td>
                                                <td class="cell-truncate" style="color: #2e7d32;">{nom_code}</td>
                                                <td class="cell-truncate">{name}</td>
                                                <td class="text-right">{qty}</td>
                                                <td class="text-right">{amount}</td>
                                                <td class="text-right">{total}</td>
                                                <td class="text-right">{finished}</td>
                                                <td>{event}</td>
                                            </tr>
                                        }
                                    }).collect::<Vec<_>>()}
                                </tbody>
                            </table>
                        </div>
                    }.into_any()
                }
            }}
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
