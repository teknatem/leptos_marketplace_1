use super::details::WbSalesDetail;
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
pub struct WbSalesDto {
    pub id: String,
    pub document_no: String,
    pub sale_date: String,
    pub supplier_article: String,
    pub name: String,
    pub qty: f64,
    pub amount_line: Option<f64>,
    pub event_type: String,
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
            "event_type" => self
                .event_type
                .to_lowercase()
                .cmp(&other.event_type.to_lowercase()),
            _ => Ordering::Equal,
        }
    }
}

#[component]
pub fn WbSalesList() -> impl IntoView {
    let (sales, set_sales) = signal::<Vec<WbSalesDto>>(Vec::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (selected_id, set_selected_id) = signal::<Option<String>>(None);

    // Сортировка
    let (sort_field, set_sort_field) = signal::<String>("sale_date".to_string());
    let (sort_ascending, set_sort_ascending) = signal(false); // По умолчанию - новые сначала

    let load_sales = move || {
        let set_sales = set_sales.clone();
        let set_loading = set_loading.clone();
        let set_error = set_error.clone();
        wasm_bindgen_futures::spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let url = "http://localhost:3000/api/a012/wb-sales";

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

                                        // Упрощенная десериализация - берем только нужные поля
                                        let items: Vec<WbSalesDto> = data
                                            .into_iter()
                                            .enumerate()
                                            .filter_map(|(idx, v)| {
                                                // Дата продажи из state.sale_dt
                                                let sale_date = v
                                                    .get("state")
                                                    .and_then(|s| s.get("sale_dt"))
                                                    .and_then(|d| d.as_str())
                                                    .unwrap_or("")
                                                    .to_string();

                                                // Данные из line
                                                let line = v.get("line")?;
                                                let supplier_article = line
                                                    .get("supplier_article")?
                                                    .as_str()?
                                                    .to_string();
                                                let name = line.get("name")?.as_str()?.to_string();
                                                let qty = line.get("qty")?.as_f64()?;
                                                let amount_line = line
                                                    .get("amount_line")
                                                    .and_then(|a| a.as_f64());

                                                // Event type из state
                                                let event_type = v
                                                    .get("state")
                                                    .and_then(|s| s.get("event_type"))
                                                    .and_then(|e| e.as_str())
                                                    .unwrap_or("unknown")
                                                    .to_string();

                                                let result = Some(WbSalesDto {
                                                    id: v.get("id")?.as_str()?.to_string(),
                                                    document_no: v
                                                        .get("header")?
                                                        .get("document_no")?
                                                        .as_str()?
                                                        .to_string(),
                                                    sale_date,
                                                    supplier_article,
                                                    name,
                                                    qty,
                                                    amount_line,
                                                    event_type,
                                                });

                                                if result.is_none() {
                                                    log!("Failed to parse item {}", idx);
                                                }

                                                result
                                            })
                                            .collect();

                                        log!(
                                            "Successfully parsed {} sales out of {}",
                                            items.len(),
                                            total_count
                                        );
                                        set_sales.set(items);
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
                    log!("Failed to fetch sales: {:?}", e);
                    set_error.set(Some(format!("Failed to fetch sales: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    };

    // Функция для получения отсортированных данных
    let get_sorted_items = move || -> Vec<WbSalesDto> {
        let mut result = sales.get();
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
    load_sales();

    view! {
        <div class="wb-sales-list">
            {move || {
                if let Some(id) = selected_id.get() {
                    view! {
                        <div class="modal-overlay" style="align-items: flex-start; padding-top: 40px;">
                            <div class="modal-content" style="max-width: 1200px; height: calc(100vh - 80px); overflow: hidden; margin: 0;">
                                <WbSalesDetail
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
                                <h2 style="margin: 0;">"Wildberries Sales (A012)"</h2>
                                <button
                                    on:click=move |_| {
                                        load_sales();
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
                    format!("Total: {} records", sales.get().len())
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
                                                {move || format!("Document №{}", get_sort_indicator(&sort_field.get(), "document_no", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("sale_date")
                                                title="Сортировать"
                                            >
                                                {move || format!("Дата продажи{}", get_sort_indicator(&sort_field.get(), "sale_date", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("supplier_article")
                                                title="Сортировать"
                                            >
                                                {move || format!("Артикул{}", get_sort_indicator(&sort_field.get(), "supplier_article", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("name")
                                                title="Сортировать"
                                            >
                                                {move || format!("Название{}", get_sort_indicator(&sort_field.get(), "name", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; text-align: right; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("qty")
                                                title="Сортировать"
                                            >
                                                {move || format!("Кол-во{}", get_sort_indicator(&sort_field.get(), "qty", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; text-align: right; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("amount_line")
                                                title="Сортировать"
                                            >
                                                {move || format!("Сумма{}", get_sort_indicator(&sort_field.get(), "amount_line", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("event_type")
                                                title="Сортировать"
                                            >
                                                {move || format!("Тип{}", get_sort_indicator(&sort_field.get(), "event_type", sort_ascending.get()))}
                                            </th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {move || get_sorted_items().into_iter().map(|sale| {
                                            let short_id = sale.id.chars().take(8).collect::<String>();
                                            let sale_id = sale.id.clone();
                                            let formatted_date = format_date(&sale.sale_date);
                                            let formatted_amount = sale.amount_line
                                                .map(|a| format!("{:.2}", a))
                                                .unwrap_or_else(|| "-".to_string());
                                            let formatted_qty = format!("{:.0}", sale.qty);
                                            view! {
                                                <tr
                                                    on:click=move |_| {
                                                        set_selected_id.set(Some(sale_id.clone()));
                                                    }
                                                    style="cursor: pointer; transition: background 0.2s;"
                                                    onmouseenter="this.style.background='#f5f5f5'"
                                                    onmouseleave="this.style.background='white'"
                                                >
                                                    <td style="border: 1px solid #ddd; padding: 8px;">
                                                        <code style="font-size: 0.85em;">{format!("{}...", short_id)}</code>
                                                    </td>
                                                    <td style="border: 1px solid #ddd; padding: 8px;">{sale.document_no}</td>
                                                    <td style="border: 1px solid #ddd; padding: 8px;">{formatted_date}</td>
                                                    <td style="border: 1px solid #ddd; padding: 8px;"><code style="font-size: 0.85em;">{sale.supplier_article}</code></td>
                                                    <td style="border: 1px solid #ddd; padding: 8px;">{sale.name}</td>
                                                    <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{formatted_qty}</td>
                                                    <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{formatted_amount}</td>
                                                    <td style="border: 1px solid #ddd; padding: 8px;">{sale.event_type}</td>
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
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Document №"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Дата продажи"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Артикул"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Название"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: right;">"Кол-во"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: right;">"Сумма"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Тип"</th>
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
