use super::details::YmOrderDetail;
use crate::shared::list_utils::{get_sort_indicator, Sortable};
use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmOrderDto {
    pub id: String,
    pub document_no: String,
    pub status_changed_at: String,
    pub creation_date: String,
    pub delivery_date: String,
    pub campaign_id: String,
    pub status_norm: String,
    pub total_qty: f64,
    pub total_amount: f64,
    pub total_amount_api: Option<f64>,
    pub lines_count: usize,
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
    let (sort_field, set_sort_field) = signal::<String>("status_changed_at".to_string());
    let (sort_ascending, set_sort_ascending) = signal(false); // По умолчанию - новые сначала

    let load_orders = move || {
        let set_orders = set_orders.clone();
        let set_loading = set_loading.clone();
        let set_error = set_error.clone();
        wasm_bindgen_futures::spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let url = "http://localhost:3000/api/a013/ym-order";

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

                                match serde_json::from_str::<Vec<serde_json::Value>>(&text) {
                                    Ok(data) => {
                                        let total_count = data.len();
                                        log!("Parsed {} items from JSON", total_count);

                                        // Десериализация с расчетом сумм по строкам
                                        let items: Vec<YmOrderDto> = data
                                            .into_iter()
                                            .enumerate()
                                            .filter_map(|(idx, v)| {
                                                // Даты из state
                                                let status_changed_at = v
                                                    .get("state")
                                                    .and_then(|s| s.get("status_changed_at"))
                                                    .and_then(|d| d.as_str())
                                                    .unwrap_or("")
                                                    .to_string();

                                                let creation_date = v
                                                    .get("state")
                                                    .and_then(|s| s.get("creation_date"))
                                                    .and_then(|d| d.as_str())
                                                    .unwrap_or("")
                                                    .to_string();

                                                let delivery_date = v
                                                    .get("state")
                                                    .and_then(|s| s.get("delivery_date"))
                                                    .and_then(|d| d.as_str())
                                                    .unwrap_or("")
                                                    .to_string();

                                                // Campaign ID из header
                                                let campaign_id = v
                                                    .get("header")
                                                    .and_then(|h| h.get("campaign_id"))
                                                    .and_then(|c| c.as_str())
                                                    .unwrap_or("")
                                                    .to_string();

                                                // Статус из state
                                                let status_norm = v
                                                    .get("state")
                                                    .and_then(|s| s.get("status_norm"))
                                                    .and_then(|s| s.as_str())
                                                    .unwrap_or("unknown")
                                                    .to_string();

                                                // Total amount из API (header)
                                                let total_amount_api = v
                                                    .get("header")
                                                    .and_then(|h| h.get("total_amount"))
                                                    .and_then(|t| t.as_f64());

                                                // Строки заказа - массив
                                                let lines = v
                                                    .get("lines")
                                                    .and_then(|l| l.as_array())
                                                    .map(|arr| arr.clone())
                                                    .unwrap_or_default();

                                                let lines_count = lines.len();

                                                // Рассчитываем суммы по всем строкам
                                                let mut total_qty = 0.0;
                                                let mut total_amount = 0.0;

                                                for line in lines {
                                                    if let Some(qty) = line.get("qty").and_then(|q| q.as_f64()) {
                                                        total_qty += qty;
                                                    }
                                                    if let Some(amount) = line.get("amount_line").and_then(|a| a.as_f64()) {
                                                        total_amount += amount;
                                                    }
                                                }

                                                let result = Some(YmOrderDto {
                                                    id: v.get("id")?.as_str()?.to_string(),
                                                    document_no: v
                                                        .get("header")?
                                                        .get("document_no")?
                                                        .as_str()?
                                                        .to_string(),
                                                    status_changed_at,
                                                    creation_date,
                                                    delivery_date,
                                                    campaign_id,
                                                    status_norm,
                                                    total_qty,
                                                    total_amount,
                                                    total_amount_api,
                                                    lines_count,
                                                });

                                                if result.is_none() {
                                                    log!("Failed to parse item {}", idx);
                                                }

                                                result
                                            })
                                            .collect();

                                        log!(
                                            "Successfully parsed {} orders out of {}",
                                            items.len(),
                                            total_count
                                        );
                                        set_orders.set(items);
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

    // Функция для получения отсортированных данных
    let get_sorted_items = move || -> Vec<YmOrderDto> {
        let mut result = orders.get();
        let field = sort_field.get();
        let ascending = sort_ascending.get();

        result.sort_by(|a, b| {
            let cmp = a.compare_by_field(b, &field);
            if ascending {
                cmp
            } else {
                cmp.reverse()
            }
        });

        result
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

    // Автоматическая загрузка при открытии
    load_orders();

    view! {
        <div class="ym-order-list">
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
                            <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 8px;">
                                <h2 style="margin: 0;">"Yandex Market Orders (A013)"</h2>
                                <button
                                    on:click=move |_| {
                                        load_orders();
                                    }
                                    style="padding: 6px 12px; background: #4CAF50; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 14px;"
                                >
                                    "Обновить"
                                </button>
                            </div>

            {move || {
                let msg = if loading.get() {
                    "Loading...".to_string()
                } else if let Some(err) = error.get() {
                    err.clone()
                } else {
                    format!("Total: {} records", orders.get().len())
                };

                // Render summary and table; render filled rows only when not loading and no error
                if !loading.get() && error.get().is_none() {
                    view! {
                        <div>
                            <p style="margin: 4px 0 8px 0; font-size: 13px; color: #666;">{msg}</p>
                            <div class="table-container">
                                <table class="data-table" style="width: 100%; border-collapse: collapse;">
                                    <thead>
                                        <tr style="background: #f5f5f5;">
                                            <th style="border: 1px solid #ddd; padding: 8px;">"ID"</th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("document_no")
                                                title="Сортировать"
                                            >
                                                {move || format!("Order №{}", get_sort_indicator(&sort_field.get(), "document_no", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("creation_date")
                                                title="Сортировать"
                                            >
                                                {move || format!("Дата заказа{}", get_sort_indicator(&sort_field.get(), "creation_date", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("delivery_date")
                                                title="Сортировать"
                                            >
                                                {move || format!("Дата доставки{}", get_sort_indicator(&sort_field.get(), "delivery_date", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("status_norm")
                                                title="Сортировать"
                                            >
                                                {move || format!("Статус{}", get_sort_indicator(&sort_field.get(), "status_norm", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; text-align: right; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("lines_count")
                                                title="Сортировать"
                                            >
                                                {move || format!("Строк{}", get_sort_indicator(&sort_field.get(), "lines_count", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; text-align: right; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("total_qty")
                                                title="Сортировать"
                                            >
                                                {move || format!("Всего шт.{}", get_sort_indicator(&sort_field.get(), "total_qty", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; text-align: right; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("total_amount")
                                                title="Сортировать"
                                            >
                                                {move || format!("Сумма{}", get_sort_indicator(&sort_field.get(), "total_amount", sort_ascending.get()))}
                                            </th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {move || get_sorted_items().into_iter().map(|order| {
                                            let short_id = order.id.chars().take(8).collect::<String>();
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
                                            view! {
                                                <tr
                                                    on:click=move |_| {
                                                        set_selected_id.set(Some(order_id.clone()));
                                                    }
                                                    style="cursor: pointer; transition: background 0.2s;"
                                                    onmouseenter="this.style.background='#f5f5f5'"
                                                    onmouseleave="this.style.background='white'"
                                                >
                                                    <td style="border: 1px solid #ddd; padding: 8px;">
                                                        <code style="font-size: 0.85em;">{format!("{}...", short_id)}</code>
                                                    </td>
                                                    <td style="border: 1px solid #ddd; padding: 8px;">{order.document_no}</td>
                                                    <td style="border: 1px solid #ddd; padding: 8px;">{formatted_creation_date}</td>
                                                    <td style="border: 1px solid #ddd; padding: 8px;">{formatted_delivery_date}</td>
                                                    <td style="border: 1px solid #ddd; padding: 8px;">{order.status_norm}</td>
                                                    <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{order.lines_count}</td>
                                                    <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{formatted_qty}</td>
                                                    <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{formatted_amount}</td>
                                                </tr>
                                            }
                                        }).collect_view()}
                                    </tbody>
                                </table>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div>
                            <p style="margin: 4px 0 8px 0; font-size: 13px; color: #666;">{msg}</p>
                            <div class="table-container">
                                <table class="data-table" style="width: 100%; border-collapse: collapse;">
                                    <thead>
                                        <tr style="background: #f5f5f5;">
                                            <th style="border: 1px solid #ddd; padding: 8px;">"ID"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Order №"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Дата заказа"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Дата доставки"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Статус"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: right;">"Строк"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: right;">"Всего шт."</th>
                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: right;">"Сумма"</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        <tr><td colspan="8"></td></tr>
                                    </tbody>
                                </table>
                            </div>
                        </div>
                    }.into_any()
                }
            }}
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}
