use super::details::YmReturnDetail;
use crate::shared::list_utils::{get_sort_indicator, Sortable};
use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmReturnDto {
    pub id: String,
    pub return_id: i64,
    pub order_id: i64,
    pub return_type: String,
    pub refund_status: String,
    pub total_items: i32,
    pub total_amount: f64,
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
            "fetched_at" => self.fetched_at.cmp(&other.fetched_at),
            _ => Ordering::Equal,
        }
    }
}

#[component]
pub fn YmReturnsList() -> impl IntoView {
    let (returns, set_returns) = signal::<Vec<YmReturnDto>>(Vec::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (selected_id, set_selected_id) = signal::<Option<String>>(None);

    // Сортировка
    let (sort_field, set_sort_field) = signal::<String>("return_id".to_string());
    let (sort_ascending, set_sort_ascending) = signal(false);

    // Поиск
    let (search_return_id, set_search_return_id) = signal(String::new());
    let (search_order_id, set_search_order_id) = signal(String::new());
    
    // Фильтры по типу
    let (filter_type, set_filter_type) = signal::<Option<String>>(None);

    let load_returns = move || {
        let set_returns = set_returns.clone();
        let set_loading = set_loading.clone();
        let set_error = set_error.clone();
        wasm_bindgen_futures::spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let url = "http://localhost:3000/api/a016/ym-returns";

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

                                        let items: Vec<YmReturnDto> = data
                                            .into_iter()
                                            .filter_map(|v| {
                                                let return_id = v
                                                    .get("header")
                                                    .and_then(|h| h.get("return_id"))
                                                    .and_then(|r| r.as_i64())
                                                    .unwrap_or(0);

                                                let order_id = v
                                                    .get("header")
                                                    .and_then(|h| h.get("order_id"))
                                                    .and_then(|o| o.as_i64())
                                                    .unwrap_or(0);

                                                let return_type = v
                                                    .get("header")
                                                    .and_then(|h| h.get("return_type"))
                                                    .and_then(|t| t.as_str())
                                                    .unwrap_or("UNKNOWN")
                                                    .to_string();

                                                let refund_status = v
                                                    .get("state")
                                                    .and_then(|s| s.get("refund_status"))
                                                    .and_then(|s| s.as_str())
                                                    .unwrap_or("UNKNOWN")
                                                    .to_string();

                                                let fetched_at = v
                                                    .get("source_meta")
                                                    .and_then(|s| s.get("fetched_at"))
                                                    .and_then(|f| f.as_str())
                                                    .unwrap_or("")
                                                    .to_string();

                                                let is_posted = v
                                                    .get("is_posted")
                                                    .and_then(|p| p.as_bool())
                                                    .unwrap_or(false);

                                                // Считаем строки и сумму
                                                let lines = v
                                                    .get("lines")
                                                    .and_then(|l| l.as_array())
                                                    .cloned()
                                                    .unwrap_or_default();

                                                let mut total_items = 0i32;
                                                let mut total_amount = 0.0f64;

                                                for line in &lines {
                                                    if let Some(count) =
                                                        line.get("count").and_then(|c| c.as_i64())
                                                    {
                                                        total_items += count as i32;
                                                    }
                                                    if let Some(price) =
                                                        line.get("price").and_then(|p| p.as_f64())
                                                    {
                                                        let count = line
                                                            .get("count")
                                                            .and_then(|c| c.as_i64())
                                                            .unwrap_or(1)
                                                            as f64;
                                                        total_amount += price * count;
                                                    }
                                                }

                                                // Или используем header.amount если доступен
                                                if total_amount == 0.0 {
                                                    if let Some(amount) = v
                                                        .get("header")
                                                        .and_then(|h| h.get("amount"))
                                                        .and_then(|a| a.as_f64())
                                                    {
                                                        total_amount = amount;
                                                    }
                                                }

                                                Some(YmReturnDto {
                                                    id: v.get("id")?.as_str()?.to_string(),
                                                    return_id,
                                                    order_id,
                                                    return_type,
                                                    refund_status,
                                                    total_items,
                                                    total_amount,
                                                    fetched_at,
                                                    is_posted,
                                                })
                                            })
                                            .collect();

                                        log!(
                                            "Successfully parsed {} returns out of {}",
                                            items.len(),
                                            total_count
                                        );
                                        set_returns.set(items);
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
                    log!("Failed to fetch returns: {:?}", e);
                    set_error.set(Some(format!("Failed to fetch returns: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    };

    // Функция для получения отфильтрованных и отсортированных данных
    let get_filtered_sorted_items = move || -> Vec<YmReturnDto> {
        let mut result = returns.get();
        let field = sort_field.get();
        let ascending = sort_ascending.get();
        let search_ret = search_return_id.get();
        let search_ord = search_order_id.get();
        let type_filter = filter_type.get();

        // Фильтрация по return_id
        if !search_ret.is_empty() {
            if let Ok(search_num) = search_ret.parse::<i64>() {
                result.retain(|r| r.return_id == search_num);
            } else {
                result.retain(|r| r.return_id.to_string().contains(&search_ret));
            }
        }

        // Фильтрация по order_id
        if !search_ord.is_empty() {
            if let Ok(search_num) = search_ord.parse::<i64>() {
                result.retain(|r| r.order_id == search_num);
            } else {
                result.retain(|r| r.order_id.to_string().contains(&search_ord));
            }
        }

        // Фильтрация по типу
        if let Some(ref t) = type_filter {
            result.retain(|r| &r.return_type == t);
        }

        // Сортировка
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
    load_returns();

    view! {
        <div class="ym-returns-list" style="padding: 10px;">
            {move || {
                if let Some(id) = selected_id.get() {
                    view! {
                        <div class="modal-overlay" style="align-items: flex-start; padding-top: 40px;">
                            <div class="modal-content" style="max-width: 1200px; height: calc(100vh - 80px); overflow: hidden; margin: 0;">
                                <YmReturnDetail
                                    id=id
                                    on_close=move || set_selected_id.set(None)
                                />
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div>
                            // Header
                            <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 16px;">
                                <h2 style="margin: 0;">"Yandex Market Returns (A016)"</h2>
                                <button
                                    on:click=move |_| {
                                        load_returns();
                                    }
                                    style="padding: 8px 16px; background: #4CAF50; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 14px;"
                                    prop:disabled=move || loading.get()
                                >
                                    {move || if loading.get() { "Загрузка..." } else { "🔄 Обновить" }}
                                </button>
                            </div>

                            // Filters panel
                            <div style="display: flex; flex-wrap: wrap; gap: 16px; margin-bottom: 16px; padding: 12px; background: #f9f9f9; border-radius: 8px; border: 1px solid #eee;">
                                // Search by Return ID
                                <div style="flex: 1; min-width: 150px; max-width: 200px;">
                                    <label style="display: block; font-size: 12px; font-weight: 500; color: #666; margin-bottom: 4px;">"Return ID"</label>
                                    <input
                                        type="text"
                                        placeholder="Поиск..."
                                        style="width: 100%; padding: 6px 10px; border: 1px solid #ddd; border-radius: 4px; font-size: 13px;"
                                        prop:value=move || search_return_id.get()
                                        on:input=move |ev| {
                                            set_search_return_id.set(event_target_value(&ev));
                                        }
                                    />
                                </div>

                                // Search by Order ID
                                <div style="flex: 1; min-width: 150px; max-width: 200px;">
                                    <label style="display: block; font-size: 12px; font-weight: 500; color: #666; margin-bottom: 4px;">"Order ID"</label>
                                    <input
                                        type="text"
                                        placeholder="Поиск..."
                                        style="width: 100%; padding: 6px 10px; border: 1px solid #ddd; border-radius: 4px; font-size: 13px;"
                                        prop:value=move || search_order_id.get()
                                        on:input=move |ev| {
                                            set_search_order_id.set(event_target_value(&ev));
                                        }
                                    />
                                </div>

                                // Filter by Type
                                <div style="flex: 1; min-width: 150px; max-width: 200px;">
                                    <label style="display: block; font-size: 12px; font-weight: 500; color: #666; margin-bottom: 4px;">"Тип"</label>
                                    <select
                                        style="width: 100%; padding: 6px 10px; border: 1px solid #ddd; border-radius: 4px; font-size: 13px; background: white;"
                                        on:change=move |ev| {
                                            let value = event_target_value(&ev);
                                            if value.is_empty() {
                                                set_filter_type.set(None);
                                            } else {
                                                set_filter_type.set(Some(value));
                                            }
                                        }
                                    >
                                        <option value="">"Все"</option>
                                        <option value="RETURN">"Возврат"</option>
                                        <option value="UNREDEEMED">"Невыкуп"</option>
                                    </select>
                                </div>

                                // Clear filters button
                                <div style="display: flex; align-items: flex-end;">
                                    <button
                                        on:click=move |_| {
                                            set_search_return_id.set(String::new());
                                            set_search_order_id.set(String::new());
                                            set_filter_type.set(None);
                                        }
                                        style="padding: 6px 12px; background: #fff; color: #666; border: 1px solid #ddd; border-radius: 4px; cursor: pointer; font-size: 13px;"
                                    >
                                        "✕ Сбросить"
                                    </button>
                                </div>
                            </div>

                            // Summary
                            {move || {
                                let items = get_filtered_sorted_items();
                                let total_count = items.len();
                                let returns_count = items.iter().filter(|r| r.return_type == "RETURN").count();
                                let unredeemed_count = items.iter().filter(|r| r.return_type == "UNREDEEMED").count();
                                let total_amount: f64 = items.iter().map(|r| r.total_amount).sum();
                                let total_items_sum: i32 = items.iter().map(|r| r.total_items).sum();

                                view! {
                                    <div style="display: flex; gap: 16px; margin-bottom: 16px; flex-wrap: wrap;">
                                        <div style="padding: 10px 16px; background: #e3f2fd; border-radius: 6px; border-left: 4px solid #1976d2;">
                                            <div style="font-size: 11px; color: #666; text-transform: uppercase;">"Всего"</div>
                                            <div style="font-size: 20px; font-weight: bold; color: #1976d2;">{total_count}</div>
                                        </div>
                                        <div style="padding: 10px 16px; background: #e8f5e9; border-radius: 6px; border-left: 4px solid #388e3c;">
                                            <div style="font-size: 11px; color: #666; text-transform: uppercase;">"Возвраты"</div>
                                            <div style="font-size: 20px; font-weight: bold; color: #388e3c;">{returns_count}</div>
                                        </div>
                                        <div style="padding: 10px 16px; background: #fff3e0; border-radius: 6px; border-left: 4px solid #f57c00;">
                                            <div style="font-size: 11px; color: #666; text-transform: uppercase;">"Невыкупы"</div>
                                            <div style="font-size: 20px; font-weight: bold; color: #f57c00;">{unredeemed_count}</div>
                                        </div>
                                        <div style="padding: 10px 16px; background: #fce4ec; border-radius: 6px; border-left: 4px solid #c2185b;">
                                            <div style="font-size: 11px; color: #666; text-transform: uppercase;">"Сумма"</div>
                                            <div style="font-size: 20px; font-weight: bold; color: #c2185b;">{format!("{:.2}", total_amount)}</div>
                                        </div>
                                        <div style="padding: 10px 16px; background: #f3e5f5; border-radius: 6px; border-left: 4px solid #7b1fa2;">
                                            <div style="font-size: 11px; color: #666; text-transform: uppercase;">"Товаров"</div>
                                            <div style="font-size: 20px; font-weight: bold; color: #7b1fa2;">{total_items_sum}</div>
                                        </div>
                                    </div>
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
                                    let items = get_filtered_sorted_items();
                                    view! {
                                        <div class="table-container" style="overflow-x: auto;">
                                            <table class="data-table" style="width: 100%; border-collapse: collapse; font-size: 13px;">
                                                <thead>
                                                    <tr style="background: #f5f5f5;">
                                                        <th style="border: 1px solid #ddd; padding: 10px; width: 80px;">"ID"</th>
                                                        <th
                                                            style="border: 1px solid #ddd; padding: 10px; cursor: pointer; user-select: none;"
                                                            on:click=toggle_sort("return_id")
                                                            title="Сортировать"
                                                        >
                                                            {move || format!("Return №{}", get_sort_indicator(&sort_field.get(), "return_id", sort_ascending.get()))}
                                                        </th>
                                                        <th
                                                            style="border: 1px solid #ddd; padding: 10px; cursor: pointer; user-select: none;"
                                                            on:click=toggle_sort("order_id")
                                                            title="Сортировать"
                                                        >
                                                            {move || format!("Order №{}", get_sort_indicator(&sort_field.get(), "order_id", sort_ascending.get()))}
                                                        </th>
                                                        <th
                                                            style="border: 1px solid #ddd; padding: 10px; cursor: pointer; user-select: none;"
                                                            on:click=toggle_sort("return_type")
                                                            title="Сортировать"
                                                        >
                                                            {move || format!("Тип{}", get_sort_indicator(&sort_field.get(), "return_type", sort_ascending.get()))}
                                                        </th>
                                                        <th
                                                            style="border: 1px solid #ddd; padding: 10px; cursor: pointer; user-select: none;"
                                                            on:click=toggle_sort("refund_status")
                                                            title="Сортировать"
                                                        >
                                                            {move || format!("Статус{}", get_sort_indicator(&sort_field.get(), "refund_status", sort_ascending.get()))}
                                                        </th>
                                                        <th
                                                            style="border: 1px solid #ddd; padding: 10px; text-align: right; cursor: pointer; user-select: none;"
                                                            on:click=toggle_sort("total_items")
                                                            title="Сортировать"
                                                        >
                                                            {move || format!("Шт.{}", get_sort_indicator(&sort_field.get(), "total_items", sort_ascending.get()))}
                                                        </th>
                                                        <th
                                                            style="border: 1px solid #ddd; padding: 10px; text-align: right; cursor: pointer; user-select: none;"
                                                            on:click=toggle_sort("total_amount")
                                                            title="Сортировать"
                                                        >
                                                            {move || format!("Сумма{}", get_sort_indicator(&sort_field.get(), "total_amount", sort_ascending.get()))}
                                                        </th>
                                                        <th
                                                            style="border: 1px solid #ddd; padding: 10px; cursor: pointer; user-select: none;"
                                                            on:click=toggle_sort("fetched_at")
                                                            title="Сортировать"
                                                        >
                                                            {move || format!("Загружен{}", get_sort_indicator(&sort_field.get(), "fetched_at", sort_ascending.get()))}
                                                        </th>
                                                        <th style="border: 1px solid #ddd; padding: 10px; text-align: center;">"✓"</th>
                                                    </tr>
                                                </thead>
                                                <tbody>
                                                    {items.into_iter().map(|ret| {
                                                        let short_id = ret.id.chars().take(8).collect::<String>();
                                                        let ret_id = ret.id.clone();
                                                        let formatted_amount = format!("{:.2}", ret.total_amount);
                                                        let formatted_date = format_date(&ret.fetched_at);
                                                        let return_type_label = match ret.return_type.as_str() {
                                                            "UNREDEEMED" => "Невыкуп".to_string(),
                                                            "RETURN" => "Возврат".to_string(),
                                                            _ => ret.return_type.clone(),
                                                        };
                                                        let return_type_style = match ret.return_type.as_str() {
                                                            "UNREDEEMED" => "background: #fff3e0; color: #e65100;",
                                                            "RETURN" => "background: #e3f2fd; color: #1565c0;",
                                                            _ => "background: #f5f5f5; color: #666;",
                                                        };
                                                        let status_style = match ret.refund_status.as_str() {
                                                            "REFUNDED" => "background: #e8f5e9; color: #2e7d32;",
                                                            "NOT_REFUNDED" => "background: #ffebee; color: #c62828;",
                                                            "REFUND_IN_PROGRESS" => "background: #fff3e0; color: #e65100;",
                                                            _ => "background: #f5f5f5; color: #666;",
                                                        };
                                                        view! {
                                                            <tr
                                                                on:click=move |_| {
                                                                    set_selected_id.set(Some(ret_id.clone()));
                                                                }
                                                                style="cursor: pointer; transition: background 0.2s;"
                                                                onmouseenter="this.style.background='#f5f5f5'"
                                                                onmouseleave="this.style.background='white'"
                                                            >
                                                                <td style="border: 1px solid #ddd; padding: 8px;">
                                                                    <code style="font-size: 0.85em; color: #666;">{format!("{}...", short_id)}</code>
                                                                </td>
                                                                <td style="border: 1px solid #ddd; padding: 8px; font-weight: 600; color: #1976d2;">{ret.return_id}</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px;">{ret.order_id}</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px;">
                                                                    <span style={format!("padding: 3px 10px; border-radius: 4px; font-size: 0.85em; font-weight: 500; {}", return_type_style)}>
                                                                        {return_type_label}
                                                                    </span>
                                                                </td>
                                                                <td style="border: 1px solid #ddd; padding: 8px;">
                                                                    <span style={format!("padding: 3px 10px; border-radius: 4px; font-size: 0.85em; font-weight: 500; {}", status_style)}>
                                                                        {ret.refund_status.clone()}
                                                                    </span>
                                                                </td>
                                                                <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{ret.total_items}</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px; text-align: right; font-weight: 500;">{formatted_amount}</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px; font-size: 0.85em; color: #666;">{formatted_date}</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px; text-align: center;">
                                                                    {if ret.is_posted {
                                                                        view! { <span style="color: #2e7d32; font-size: 16px;">"✓"</span> }.into_any()
                                                                    } else {
                                                                        view! { <span style="color: #ccc;">"—"</span> }.into_any()
                                                                    }}
                                                                </td>
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
