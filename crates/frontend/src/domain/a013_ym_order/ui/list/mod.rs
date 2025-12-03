use super::details::YmOrderDetail;
use crate::shared::components::date_input::DateInput;
use crate::shared::components::month_selector::MonthSelector;
use crate::shared::list_utils::{get_sort_indicator, Sortable};
use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

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

/// Export data to CSV (Excel-compatible)
fn export_to_csv(data: &[YmOrderDto]) -> Result<(), String> {
    let mut csv = String::new();

    // BOM for Excel UTF-8
    csv.push('\u{FEFF}');

    // Header
    csv.push_str("Order №;Дата заказа;Дата доставки;Статус;Строк;Шт.;Сумма;Доставка;Субсидии\n");

    // Data rows
    for order in data {
        let creation_date = if !order.creation_date.is_empty() {
            format_date(&order.creation_date)
        } else {
            "".to_string()
        };
        let delivery_date = if !order.delivery_date.is_empty() {
            format_date(&order.delivery_date)
        } else {
            "".to_string()
        };
        let delivery = order
            .delivery_total
            .map(|d| format!("{:.2}", d))
            .unwrap_or_default();
        let subsidies = if order.subsidies_total > 0.0 {
            format!("{:.2}", order.subsidies_total)
        } else {
            "".to_string()
        };

        csv.push_str(&format!(
            "{};{};{};{};{};{:.0};{:.2};{};{}\n",
            order.document_no,
            creation_date,
            delivery_date,
            order.status_norm,
            order.lines_count,
            order.total_qty,
            order.total_amount,
            delivery,
            subsidies
        ));
    }

    // Create blob and download
    let window = web_sys::window().ok_or("No window")?;
    let document = window.document().ok_or("No document")?;

    let blob_parts = js_sys::Array::new();
    blob_parts.push(&JsValue::from_str(&csv));

    let options = BlobPropertyBag::new();
    options.set_type("text/csv;charset=utf-8");

    let blob = Blob::new_with_str_sequence_and_options(&blob_parts, &options)
        .map_err(|_| "Failed to create blob")?;

    let url = Url::create_object_url_with_blob(&blob).map_err(|_| "Failed to create URL")?;

    let a: HtmlAnchorElement = document
        .create_element("a")
        .map_err(|_| "Failed to create element")?
        .dyn_into()
        .map_err(|_| "Failed to cast to anchor")?;

    a.set_href(&url);
    a.set_download("ym_orders.csv");
    a.click();

    let _ = Url::revoke_object_url(&url);

    Ok(())
}

/// DTO для списка заказов (соответствует backend YmOrderListDto)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmOrderDto {
    pub id: String,
    pub document_no: String,
    #[serde(default)]
    pub status_changed_at: String,
    #[serde(default)]
    pub creation_date: String,
    #[serde(default)]
    pub delivery_date: String,
    #[serde(default)]
    pub campaign_id: String,
    #[serde(default)]
    pub status_norm: String,
    #[serde(default)]
    pub total_qty: f64,
    #[serde(default)]
    pub total_amount: f64,
    pub total_amount_api: Option<f64>,
    #[serde(default)]
    pub lines_count: usize,
    pub delivery_total: Option<f64>,
    #[serde(default)]
    pub subsidies_total: f64,
    #[serde(default)]
    pub is_posted: bool,
    #[serde(default)]
    pub is_error: bool,
}

/// Ответ от быстрого API списка
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResponse {
    pub items: Vec<YmOrderDto>,
    pub total: usize,
}

impl Sortable for YmOrderDto {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "document_no" => self
                .document_no
                .to_lowercase()
                .cmp(&other.document_no.to_lowercase()),
            "status_changed_at" => self.status_changed_at.cmp(&other.status_changed_at),
            "creation_date" => self.creation_date.cmp(&other.creation_date),
            "delivery_date" => self.delivery_date.cmp(&other.delivery_date),
            "campaign_id" => self
                .campaign_id
                .to_lowercase()
                .cmp(&other.campaign_id.to_lowercase()),
            "status_norm" => self
                .status_norm
                .to_lowercase()
                .cmp(&other.status_norm.to_lowercase()),
            "total_qty" => self
                .total_qty
                .partial_cmp(&other.total_qty)
                .unwrap_or(Ordering::Equal),
            "total_amount" => self
                .total_amount
                .partial_cmp(&other.total_amount)
                .unwrap_or(Ordering::Equal),
            "delivery_total" => match (&self.delivery_total, &other.delivery_total) {
                (Some(a), Some(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            "subsidies_total" => self
                .subsidies_total
                .partial_cmp(&other.subsidies_total)
                .unwrap_or(Ordering::Equal),
            "lines_count" => self.lines_count.cmp(&other.lines_count),
            _ => Ordering::Equal,
        }
    }
}

#[component]
pub fn YmOrderList() -> impl IntoView {
    let (orders, set_orders) = signal::<Vec<YmOrderDto>>(Vec::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (selected_id, set_selected_id) = signal::<Option<String>>(None);

    // Сортировка
    let (sort_field, set_sort_field) = signal::<String>("delivery_date".to_string());
    let (sort_ascending, set_sort_ascending) = signal(false);

    // Фильтры
    let (search_order_no, set_search_order_no) = signal(String::new());
    let (filter_status, set_filter_status) = signal::<Option<String>>(None);

    // Date range filter - empty by default (no filtering)
    let (date_from, set_date_from) = signal(String::new());
    let (date_to, set_date_to) = signal(String::new());

    // Пагинация
    let (page, set_page) = signal(0usize);
    let (page_size, set_page_size) = signal(50usize);

    let load_orders = move || {
        let set_orders = set_orders.clone();
        let set_loading = set_loading.clone();
        let set_error = set_error.clone();
        wasm_bindgen_futures::spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            // Используем новый быстрый API с денормализованными полями
            let url = "http://localhost:3000/api/a013/ym-order/list";

            match Request::get(url).send().await {
                Ok(response) => {
                    let status = response.status();
                    if status == 200 {
                        match response.text().await {
                            Ok(text) => {
                                log!(
                                    "Received response text (first 500 chars): {}",
                                    text.chars().take(500).collect::<String>()
                                );

                                match serde_json::from_str::<ListResponse>(&text) {
                                    Ok(data) => {
                                        log!(
                                            "Successfully loaded {} orders (total: {})",
                                            data.items.len(),
                                            data.total
                                        );
                                        set_orders.set(data.items);
                                        set_loading.set(false);
                                    }
                                    Err(e) => {
                                        log!("Failed to parse response: {:?}", e);
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
                    log!("Failed to fetch orders: {:?}", e);
                    set_error.set(Some(format!("Failed to fetch orders: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    };

    // Простые функции без Memo (чтобы избежать проблем с реактивностью)
    let get_filtered_sorted = move || {
        let mut result = orders.get();
        let field = sort_field.get();
        let ascending = sort_ascending.get();
        let search = search_order_no.get();
        let status_filter = filter_status.get();
        let from_date = date_from.get();
        let to_date = date_to.get();

        // Фильтрация по дате доставки
        if !from_date.is_empty() && !to_date.is_empty() {
            result.retain(|o| {
                if o.delivery_date.is_empty() {
                    return false;
                }
                let order_date = o.delivery_date.split('T').next().unwrap_or("");
                order_date >= from_date.as_str() && order_date <= to_date.as_str()
            });
        }

        // Фильтрация по номеру заказа
        if !search.is_empty() {
            result.retain(|o| o.document_no.contains(&search));
        }

        // Фильтрация по статусу
        if let Some(ref status) = status_filter {
            result.retain(|o| &o.status_norm == status);
        }

        // Сортировка
        result.sort_by(|a, b| {
            let cmp = a.compare_by_field(b, &field);
            if ascending { cmp } else { cmp.reverse() }
        });

        result
    };

    let get_total_pages = move || {
        let total = orders.get().len(); // Use raw orders count for simplicity
        let ps = page_size.get();
        if total == 0 { 1 } else { (total + ps - 1) / ps }
    };

    let get_paginated = move || {
        let all_items = get_filtered_sorted();
        let p = page.get();
        let ps = page_size.get();
        let start = p * ps;
        let end = (start + ps).min(all_items.len());
        if start >= all_items.len() {
            Vec::new()
        } else {
            all_items[start..end].to_vec()
        }
    };

    // Навигация по страницам
    let go_to_page = move |new_page: usize| {
        set_page.set(new_page);
    };

    // Обработчик переключения сортировки
    let toggle_sort = move |field: &'static str| {
        move |_| {
            if sort_field.get() == field {
                set_sort_ascending.update(|v| *v = !*v);
            } else {
                set_sort_field.set(field.to_string());
                set_sort_ascending.set(true);
            }
        }
    };

    // Флаг для предотвращения повторной загрузки
    let (is_loaded, set_is_loaded) = signal(false);

    // Автоматическая загрузка при открытии (только один раз)
    Effect::new(move |_| {
        if !is_loaded.get_untracked() {
            set_is_loaded.set(true);
            load_orders();
        }
    });

    view! {
        <div class="ym-order-list" style="background: #f8f9fa; padding: 12px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
            {move || {
                if let Some(id) = selected_id.get() {
                    view! {
                        <div class="modal-overlay" style="align-items: flex-start; padding-top: 40px;">
                            <div class="modal-content" style="max-width: 1200px; height: calc(100vh - 80px); overflow: hidden; margin: 0;">
                                <YmOrderDetail
                                    id=id
                                    on_close=move || set_selected_id.set(None)
                                />
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div>
                            // Header Row 1: Title with Pagination and Excel Export
                            <div style="background: linear-gradient(135deg, #1976d2 0%, #0d47a1 100%); padding: 8px 12px; border-radius: 6px 6px 0 0; margin: -12px -12px 0 -12px; display: flex; align-items: center; justify-content: space-between;">
                                <div style="display: flex; align-items: center; gap: 12px;">
                                    <h2 style="margin: 0; font-size: 1.1rem; font-weight: 600; color: white; letter-spacing: 0.5px;">"📦 YM Orders"</h2>

                                    // Simple page info
                                    <span style="color: white; font-size: 12px;">
                                        {move || format!("Стр. {} | {} записей", page.get() + 1, orders.get().len())}
                                    </span>
                                </div>

                                <div style="display: flex; gap: 8px; align-items: center;">
                                    // Excel export button
                                    <button
                                        style="background: #28a745; color: white; border: none; border-radius: 4px; padding: 6px 12px; font-size: 12px; cursor: pointer; font-weight: 500;"
                                        on:click=move |_| {
                                            let data = get_filtered_sorted();
                                            if let Err(e) = export_to_csv(&data) {
                                                log!("Failed to export: {}", e);
                                            }
                                        }
                                        prop:disabled=move || loading.get() || orders.get().is_empty()
                                    >
                                        "📊 Excel"
                                    </button>
                                </div>
                            </div>

                            // Header Row 2: Filters
                            <div style="background: white; padding: 8px 12px; margin: 0 -12px 10px -12px; border-bottom: 1px solid #e9ecef; display: flex; align-items: center; gap: 12px; flex-wrap: wrap;">
                                // Delivery date filter
                                <div style="display: flex; align-items: center; gap: 8px;">
                                    <label style="margin: 0; font-size: 0.875rem; font-weight: 500; color: #495057; white-space: nowrap;">"Дата доставки:"</label>
                                    <DateInput
                                        value=Signal::derive(move || date_from.get())
                                        on_change=move |val| {
                                            set_date_from.set(val);
                                            set_page.set(0);
                                        }
                                    />
                                    <span style="color: #6c757d;">"—"</span>
                                    <DateInput
                                        value=Signal::derive(move || date_to.get())
                                        on_change=move |val| {
                                            set_date_to.set(val);
                                            set_page.set(0);
                                        }
                                    />
                                    <MonthSelector
                                        on_select=Callback::new(move |(from, to)| {
                                            set_date_from.set(from);
                                            set_date_to.set(to);
                                            set_page.set(0);
                                        })
                                    />
                                </div>

                                // Order number search
                                <div style="display: flex; align-items: center; gap: 8px;">
                                    <label style="margin: 0; font-size: 0.875rem; font-weight: 500; color: #495057; white-space: nowrap;">"Order №:"</label>
                                    <input
                                        type="text"
                                        placeholder="Поиск..."
                                        prop:value=move || search_order_no.get()
                                        on:input=move |ev| {
                                            set_search_order_no.set(event_target_value(&ev));
                                            set_page.set(0);
                                        }
                                        style="padding: 6px 10px; border: 1px solid #ced4da; border-radius: 4px; font-size: 0.875rem; width: 120px; background: #fff;"
                                    />
                                </div>

                                // Status filter
                                <div style="display: flex; align-items: center; gap: 8px;">
                                    <label style="margin: 0; font-size: 0.875rem; font-weight: 500; color: #495057; white-space: nowrap;">"Статус:"</label>
                                    <select
                                        style="padding: 6px 10px; border: 1px solid #ced4da; border-radius: 4px; font-size: 0.875rem; min-width: 150px; background: #fff;"
                                        on:change=move |ev| {
                                            let value = event_target_value(&ev);
                                            if value.is_empty() {
                                                set_filter_status.set(None);
                                            } else {
                                                set_filter_status.set(Some(value));
                                            }
                                            set_page.set(0);
                                        }
                                    >
                                        <option value="">"Все"</option>
                                        <option value="DELIVERED">"DELIVERED"</option>
                                        <option value="PROCESSING">"PROCESSING"</option>
                                        <option value="CANCELLED">"CANCELLED"</option>
                                        <option value="PARTIALLY_RETURNED">"PARTIALLY_RETURNED"</option>
                                    </select>
                                </div>

                                // Update button
                                <button
                                    style="background: #28a745; color: white; border: none; border-radius: 4px; padding: 6px 16px; font-size: 0.875rem; cursor: pointer; font-weight: 500;"
                                    on:click=move |_| {
                                        load_orders();
                                    }
                                    prop:disabled=move || loading.get()
                                >
                                    "↻ Обновить"
                                </button>
                            </div>

                            // Summary cards
                            {move || {
                                if !loading.get() {
                                    let items = get_filtered_sorted();
                                    let total_count = items.len();
                                    let delivered_count = items.iter().filter(|o| o.status_norm == "DELIVERED").count();
                                    let cancelled_count = items.iter().filter(|o| o.status_norm == "CANCELLED").count();
                                    let total_amount: f64 = items.iter().map(|o| o.total_amount).sum();
                                    let total_delivery: f64 = items.iter().filter_map(|o| o.delivery_total).sum();
                                    let total_subsidies: f64 = items.iter().map(|o| o.subsidies_total).sum();

                                    view! {
                                        <div style="margin-bottom: 10px; padding: 8px 12px; background: #f5f5f5; border-radius: 4px; display: flex; align-items: center; gap: 20px; flex-wrap: wrap;">
                                            <span style="font-size: 0.875rem; color: #495057;">
                                                <strong>"Всего: "</strong>{total_count}
                                            </span>
                                            <span style="font-size: 0.875rem; color: #28a745;">
                                                <strong>"Доставлено: "</strong>{delivered_count}
                                            </span>
                                            <span style="font-size: 0.875rem; color: #dc3545;">
                                                <strong>"Отменено: "</strong>{cancelled_count}
                                            </span>
                                            <span style="font-size: 0.875rem; color: #f57c00;">
                                                <strong>"Сумма: "</strong>{format!("{:.2}", total_amount)}
                                            </span>
                                            <span style="font-size: 0.875rem; color: #0288d1;">
                                                <strong>"Доставка: "</strong>{format!("{:.2}", total_delivery)}
                                            </span>
                                            <span style="font-size: 0.875rem; color: #7b1fa2;">
                                                <strong>"Субсидии: "</strong>{format!("{:.2}", total_subsidies)}
                                            </span>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <div></div> }.into_any()
                                }
                            }}

                            // Error message
                            {move || {
                                if let Some(err) = error.get() {
                                    view! {
                                        <div style="padding: 12px; background: #ffebee; border: 1px solid #ffcdd2; border-radius: 4px; color: #c62828; margin-bottom: 16px;">
                                            <strong>"Ошибка: "</strong>{err}
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <div></div> }.into_any()
                                }
                            }}

                            // Loading indicator
                            {move || {
                                if loading.get() {
                                    view! {
                                        <div style="text-align: center; padding: 40px; color: #666;">
                                            <div style="font-size: 24px; margin-bottom: 8px;">"⏳"</div>
                                            <div>"Загрузка данных..."</div>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <div></div> }.into_any()
                                }
                            }}

                            // Table
                            {move || {
                                if !loading.get() && error.get().is_none() {
                                    let items = get_paginated();
                                    view! {
                                        <div class="table-container" style="overflow-x: auto;">
                                            <table class="data-table" style="width: 100%; border-collapse: collapse; font-size: 13px;">
                                                <thead>
                                                    <tr style="background: #f5f5f5;">
                                                        <th
                                                            style="border: 1px solid #ddd; padding: 10px; cursor: pointer; user-select: none;"
                                                            on:click=toggle_sort("document_no")
                                                            title="Сортировать"
                                                        >
                                                            {move || format!("Order №{}", get_sort_indicator(&sort_field.get(), "document_no", sort_ascending.get()))}
                                                        </th>
                                                        <th
                                                            style="border: 1px solid #ddd; padding: 10px; cursor: pointer; user-select: none;"
                                                            on:click=toggle_sort("creation_date")
                                                            title="Сортировать"
                                                        >
                                                            {move || format!("Дата заказа{}", get_sort_indicator(&sort_field.get(), "creation_date", sort_ascending.get()))}
                                                        </th>
                                                        <th
                                                            style="border: 1px solid #ddd; padding: 10px; cursor: pointer; user-select: none;"
                                                            on:click=toggle_sort("delivery_date")
                                                            title="Сортировать"
                                                        >
                                                            {move || format!("Дата доставки{}", get_sort_indicator(&sort_field.get(), "delivery_date", sort_ascending.get()))}
                                                        </th>
                                                        <th
                                                            style="border: 1px solid #ddd; padding: 10px; cursor: pointer; user-select: none;"
                                                            on:click=toggle_sort("status_norm")
                                                            title="Сортировать"
                                                        >
                                                            {move || format!("Статус{}", get_sort_indicator(&sort_field.get(), "status_norm", sort_ascending.get()))}
                                                        </th>
                                                        <th
                                                            style="border: 1px solid #ddd; padding: 10px; text-align: right; cursor: pointer; user-select: none;"
                                                            on:click=toggle_sort("lines_count")
                                                            title="Сортировать"
                                                        >
                                                            {move || format!("Строк{}", get_sort_indicator(&sort_field.get(), "lines_count", sort_ascending.get()))}
                                                        </th>
                                                        <th
                                                            style="border: 1px solid #ddd; padding: 10px; text-align: right; cursor: pointer; user-select: none;"
                                                            on:click=toggle_sort("total_qty")
                                                            title="Сортировать"
                                                        >
                                                            {move || format!("Шт.{}", get_sort_indicator(&sort_field.get(), "total_qty", sort_ascending.get()))}
                                                        </th>
                                                        <th
                                                            style="border: 1px solid #ddd; padding: 10px; text-align: right; cursor: pointer; user-select: none;"
                                                            on:click=toggle_sort("total_amount")
                                                            title="Сортировать"
                                                        >
                                                            {move || format!("Сумма{}", get_sort_indicator(&sort_field.get(), "total_amount", sort_ascending.get()))}
                                                        </th>
                                                        <th
                                                            style="border: 1px solid #ddd; padding: 10px; text-align: right; cursor: pointer; user-select: none;"
                                                            on:click=toggle_sort("delivery_total")
                                                            title="Сортировать"
                                                        >
                                                            {move || format!("Доставка{}", get_sort_indicator(&sort_field.get(), "delivery_total", sort_ascending.get()))}
                                                        </th>
                                                        <th
                                                            style="border: 1px solid #ddd; padding: 10px; text-align: right; cursor: pointer; user-select: none;"
                                                            on:click=toggle_sort("subsidies_total")
                                                            title="Сортировать"
                                                        >
                                                            {move || format!("Субсидии{}", get_sort_indicator(&sort_field.get(), "subsidies_total", sort_ascending.get()))}
                                                        </th>
                                                    </tr>
                                                </thead>
                                                <tbody>
                                                    {items.into_iter().map(|order| {
                                                        let order_id = order.id.clone();
                                                        let formatted_creation_date = if !order.creation_date.is_empty() {
                                                            format_date(&order.creation_date)
                                                        } else {
                                                            "—".to_string()
                                                        };
                                                        let formatted_delivery_date = if !order.delivery_date.is_empty() {
                                                            format_date(&order.delivery_date)
                                                        } else {
                                                            "—".to_string()
                                                        };
                                                        let formatted_amount = format!("{:.2}", order.total_amount);
                                                        let formatted_qty = format!("{:.0}", order.total_qty);
                                                        let formatted_delivery = order.delivery_total.map(|d| format!("{:.2}", d)).unwrap_or("—".to_string());
                                                        let formatted_subsidies = if order.subsidies_total > 0.0 {
                                                            format!("{:.2}", order.subsidies_total)
                                                        } else {
                                                            "—".to_string()
                                                        };
                                                        let status_style = match order.status_norm.as_str() {
                                                            "DELIVERED" => "background: #e8f5e9; color: #2e7d32;",
                                                            "CANCELLED" => "background: #ffebee; color: #c62828;",
                                                            "PROCESSING" => "background: #fff3e0; color: #e65100;",
                                                            "PARTIALLY_RETURNED" => "background: #e3f2fd; color: #1565c0;",
                                                            _ => "background: #f5f5f5; color: #666;",
                                                        };
                                                        view! {
                                                            <tr
                                                                on:click=move |_| {
                                                                    set_selected_id.set(Some(order_id.clone()));
                                                                }
                                                                style="cursor: pointer; transition: background 0.2s; background: white;"
                                                                onmouseenter="this.style.background='#f5f5f5'"
                                                                onmouseleave="this.style.background='white'"
                                                            >
                                                                <td style="border: 1px solid #ddd; padding: 8px; font-weight: 600; color: #1976d2;">{order.document_no}</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px;">{formatted_creation_date}</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px;">{formatted_delivery_date}</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px;">
                                                                    <span style={format!("padding: 3px 10px; border-radius: 4px; font-size: 0.85em; font-weight: 500; {}", status_style)}>
                                                                        {order.status_norm}
                                                                    </span>
                                                                </td>
                                                                <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{order.lines_count}</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{formatted_qty}</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px; text-align: right; font-weight: 500;">{formatted_amount}</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px; text-align: right; color: #0288d1;">{formatted_delivery}</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px; text-align: right; color: #7b1fa2;">{formatted_subsidies}</td>
                                                            </tr>
                                                        }
                                                    }).collect_view()}
                                                </tbody>
                                            </table>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <div></div> }.into_any()
                                }
                            }}
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}
