use leptos::prelude::*;
use leptos::logging::log;
use serde::{Deserialize, Serialize};
use gloo_net::http::Request;
use super::details::OzonTransactionsDetail;
use crate::shared::list_utils::{get_sort_indicator, Sortable};
use std::cmp::Ordering;
use chrono::{Datelike, Utc};
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};
use wasm_bindgen::JsCast;

/// Форматирует дату из "2025-10-11 00:00:00" в dd.mm.yyyy
fn format_date(date_str: &str) -> String {
    // Парсим формат "2025-10-11 00:00:00" или "2025-10-11"
    let date_part = date_str.split_whitespace().next().unwrap_or(date_str);
    if let Some((year, rest)) = date_part.split_once('-') {
        if let Some((month, day)) = rest.split_once('-') {
            return format!("{}.{}.{}", day, month, year);
        }
    }
    date_str.to_string() // fallback
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonTransactionsDto {
    pub id: String,
    #[serde(rename = "operation_id")]
    pub operation_id: i64,
    #[serde(rename = "operation_type")]
    pub operation_type: String,
    #[serde(rename = "operation_type_name")]
    pub operation_type_name: String,
    #[serde(rename = "operation_date")]
    pub operation_date: String,
    #[serde(rename = "posting_number")]
    pub posting_number: String,
    #[serde(rename = "transaction_type")]
    pub transaction_type: String,
    #[serde(rename = "delivery_schema")]
    pub delivery_schema: String,
    pub amount: f64,
    pub accruals_for_sale: f64,
    pub sale_commission: f64,
    pub delivery_charge: f64,
    pub delivering_date: Option<String>,
    #[serde(rename = "is_posted")]
    pub is_posted: bool,
}

impl Sortable for OzonTransactionsDto {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "operation_id" => self.operation_id.cmp(&other.operation_id),
            "operation_type" => self.operation_type.to_lowercase().cmp(&other.operation_type.to_lowercase()),
            "operation_type_name" => self.operation_type_name.to_lowercase().cmp(&other.operation_type_name.to_lowercase()),
            "operation_date" => self.operation_date.cmp(&other.operation_date),
            "posting_number" => self.posting_number.to_lowercase().cmp(&other.posting_number.to_lowercase()),
            "transaction_type" => self.transaction_type.to_lowercase().cmp(&other.transaction_type.to_lowercase()),
            "delivery_schema" => self.delivery_schema.to_lowercase().cmp(&other.delivery_schema.to_lowercase()),
            "amount" => self.amount.partial_cmp(&other.amount).unwrap_or(Ordering::Equal),
            "accruals_for_sale" => self.accruals_for_sale.partial_cmp(&other.accruals_for_sale).unwrap_or(Ordering::Equal),
            "sale_commission" => self.sale_commission.partial_cmp(&other.sale_commission).unwrap_or(Ordering::Equal),
            "delivery_charge" => self.delivery_charge.partial_cmp(&other.delivery_charge).unwrap_or(Ordering::Equal),
            "delivering_date" => match (&self.delivering_date, &other.delivering_date) {
                (Some(a), Some(b)) => a.cmp(b),
                (Some(_), None) => Ordering::Greater,
                (None, Some(_)) => Ordering::Less,
                (None, None) => Ordering::Equal,
            },
            "is_posted" => self.is_posted.cmp(&other.is_posted),
            _ => Ordering::Equal,
        }
    }
}

#[component]
pub fn OzonTransactionsList() -> impl IntoView {
    let (transactions, set_transactions) = signal::<Vec<OzonTransactionsDto>>(Vec::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (selected_id, set_selected_id) = signal::<Option<String>>(None);

    // Сортировка
    let (sort_field, set_sort_field) = signal::<String>("operation_date".to_string());
    let (sort_ascending, set_sort_ascending) = signal(false); // По умолчанию - новые сначала

    // Фильтры - период по умолчанию: текущий месяц
    let now = Utc::now().date_naive();
    let year = now.year();
    let month = now.month();
    let month_start =
        chrono::NaiveDate::from_ymd_opt(year, month, 1).expect("Invalid month start date");
    let month_end = if month == 12 {
        chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
            .map(|d| d - chrono::Duration::days(1))
            .expect("Invalid month end date")
    } else {
        chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
            .map(|d| d - chrono::Duration::days(1))
            .expect("Invalid month end date")
    };

    let (date_from, set_date_from) = signal(month_start.format("%Y-%m-%d").to_string());
    let (date_to, set_date_to) = signal(month_end.format("%Y-%m-%d").to_string());
    let (transaction_type_filter, set_transaction_type_filter) = signal("".to_string());
    let (operation_type_name_filter, set_operation_type_name_filter) = signal("".to_string());
    let (posting_number_filter, set_posting_number_filter) = signal("".to_string());

    let load_transactions = move || {
        wasm_bindgen_futures::spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let date_from_val = date_from.get();
            let date_to_val = date_to.get();
            let transaction_type_val = transaction_type_filter.get();
            let operation_type_name_val = operation_type_name_filter.get();
            let posting_number_val = posting_number_filter.get();

            let mut query_params = format!(
                "?date_from={}&date_to={}",
                date_from_val, date_to_val
            );

            if !transaction_type_val.is_empty() {
                query_params.push_str(&format!("&transaction_type={}", transaction_type_val));
            }

            if !operation_type_name_val.is_empty() {
                query_params.push_str(&format!("&operation_type_name={}", operation_type_name_val));
            }

            if !posting_number_val.is_empty() {
                query_params.push_str(&format!("&posting_number={}", posting_number_val));
            }

            let url = format!("http://localhost:3000/api/ozon_transactions{}", query_params);
            log!("Fetching transactions with URL: {}", url);

            match Request::get(&url).send().await {
                Ok(response) => {
                    let status = response.status();
                    if status == 200 {
                        match response.text().await {
                            Ok(text) => {
                                match serde_json::from_str::<Vec<OzonTransactionsDto>>(&text) {
                                    Ok(items) => {
                                        log!("Successfully parsed {} OZON transactions", items.len());
                                        set_transactions.set(items);
                                        set_loading.set(false);
                                    }
                                    Err(e) => {
                                        log!("Failed to parse response: {:?}", e);
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
                    log!("Failed to fetch transactions: {:?}", e);
                    set_error.set(Some(format!("Failed to fetch transactions: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    };

    // Функция для получения отсортированных данных
    let get_sorted_items = move || -> Vec<OzonTransactionsDto> {
        let mut result = transactions.get();
        let field = sort_field.get();
        let ascending = sort_ascending.get();
        result.sort_by(|a, b| {
            if ascending {
                a.compare_by_field(b, &field)
            } else {
                b.compare_by_field(a, &field)
            }
        });
        result
    };

    // Вычисление итогов
    let totals = move || {
        let data = get_sorted_items();
        let total_amount: f64 = data.iter().map(|t| t.amount).sum();
        let total_accruals: f64 = data.iter().map(|t| t.accruals_for_sale).sum();
        let total_commission: f64 = data.iter().map(|t| t.sale_commission).sum();
        let total_delivery: f64 = data.iter().map(|t| t.delivery_charge).sum();
        (data.len(), total_amount, total_accruals, total_commission, total_delivery)
    };

    // Загрузка при монтировании
    Effect::new(move || {
        load_transactions();
    });

    // Функция для изменения сортировки
    let toggle_sort = move |field: &'static str| {
        if sort_field.get() == field {
            set_sort_ascending.update(|asc| *asc = !*asc);
        } else {
            set_sort_field.set(field.to_string());
            set_sort_ascending.set(true);
        }
    };

    // Открыть детальный просмотр
    let open_detail = move |id: String| {
        set_selected_id.set(Some(id));
    };

    // Закрыть детальный просмотр
    let close_detail = move || {
        set_selected_id.set(None);
        load_transactions(); // Перезагрузить список после закрытия
    };

    view! {
        <div class="ozon-transactions-list">
            <div style="display: flex; align-items: center; gap: 12px; margin-bottom: 12px; flex-wrap: wrap;">
                <h2 style="margin: 0; font-size: var(--font-size-h3); line-height: 1.2;">"OZON Транзакции (A014)"</h2>

                <label style="margin: 0; font-size: var(--font-size-sm); white-space: nowrap;">"From:"</label>
                <input
                    type="date"
                    prop:value=date_from
                    on:input=move |ev| {
                        set_date_from.set(event_target_value(&ev));
                    }
                    style="padding: 4px 8px; border: 1px solid var(--color-border-light); border-radius: 4px; font-size: var(--font-size-sm);"
                />

                <label style="margin: 0; font-size: var(--font-size-sm); white-space: nowrap;">"To:"</label>
                <input
                    type="date"
                    prop:value=date_to
                    on:input=move |ev| {
                        set_date_to.set(event_target_value(&ev));
                    }
                    style="padding: 4px 8px; border: 1px solid var(--color-border-light); border-radius: 4px; font-size: var(--font-size-sm);"
                />

                <label style="margin: 0; font-size: var(--font-size-sm); white-space: nowrap;">"Тип транзакции:"</label>
                <select
                    prop:value=transaction_type_filter
                    on:change=move |ev| {
                        set_transaction_type_filter.set(event_target_value(&ev));
                    }
                    style="padding: 4px 8px; border: 1px solid var(--color-border-light); border-radius: 4px; font-size: var(--font-size-sm);"
                >
                    <option value="">"Все"</option>
                    <option value="orders">"orders"</option>
                    <option value="returns">"returns"</option>
                    <option value="client_returns">"client_returns"</option>
                    <option value="services">"services"</option>
                    <option value="other">"other"</option>
                    <option value="transfer_delivery">"transfer_delivery"</option>
                </select>

                <label style="margin: 0; font-size: var(--font-size-sm); white-space: nowrap;">"Тип операции:"</label>
                <input
                    type="text"
                    prop:value=operation_type_name_filter
                    on:input=move |ev| {
                        set_operation_type_name_filter.set(event_target_value(&ev));
                    }
                    placeholder="Введите тип операции"
                    style="padding: 4px 8px; border: 1px solid var(--color-border-light); border-radius: 4px; font-size: var(--font-size-sm); min-width: 150px;"
                />

                <label style="margin: 0; font-size: var(--font-size-sm); white-space: nowrap;">"Posting #:"</label>
                <input
                    type="text"
                    prop:value=posting_number_filter
                    on:input=move |ev| {
                        set_posting_number_filter.set(event_target_value(&ev));
                    }
                    placeholder="Поиск по номеру"
                    style="padding: 4px 8px; border: 1px solid var(--color-border-light); border-radius: 4px; font-size: var(--font-size-sm); min-width: 150px;"
                />

                <button
                    on:click=move |_| {
                        load_transactions();
                    }
                    style="padding: 4px 12px; background: #4CAF50; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: var(--font-size-sm);"
                    disabled=move || loading.get()
                >
                    {move || if loading.get() { "Загрузка..." } else { "Обновить" }}
                </button>

                <button
                    on:click=move |_| {
                        let data = get_sorted_items();
                        if let Err(e) = export_to_csv(&data) {
                            log!("Failed to export: {}", e);
                        }
                    }
                    style="padding: 4px 12px; background: #2196F3; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: var(--font-size-sm);"
                    disabled=move || loading.get() || transactions.get().is_empty()
                >
                    "Экспорт в Excel"
                </button>

                {move || if !loading.get() {
                    let (count, total_amount, total_accruals, total_commission, total_delivery) = totals();
                    view! {
                        <span style="margin-left: 8px; font-size: var(--font-size-base); font-weight: 600; color: var(--color-text); background: var(--color-background-alt, #f5f5f5); padding: 6px 12px; border-radius: 4px;">
                            "Total: " {count} " records | "
                            "Amount: " {format!("{:.2}", total_amount)} " | "
                            "Accruals: " {format!("{:.2}", total_accruals)} " | "
                            "Commission: " {format!("{:.2}", total_commission)} " | "
                            "Delivery: " {format!("{:.2}", total_delivery)}
                        </span>
                    }.into_any()
                } else {
                    view! { <></> }.into_any()
                }}
            </div>

            {move || error.get().map(|err| view! {
                <div class="error-message">{err}</div>
            })}

            {move || {
                if loading.get() {
                    view! {
                        <div class="loading-spinner">"Загрузка транзакций..."</div>
                    }.into_any()
                } else {
                    let items = get_sorted_items();
                    view! {
                        <div class="table-container">
                            <table class="transactions-table">
                                <thead>
                                    <tr>
                                        <th on:click=move |_| toggle_sort("operation_id")>
                                            "Operation ID " {move || get_sort_indicator("operation_id", &sort_field.get(), sort_ascending.get())}
                                        </th>
                                        <th on:click=move |_| toggle_sort("operation_type_name")>
                                            "Тип операции " {move || get_sort_indicator("operation_type_name", &sort_field.get(), sort_ascending.get())}
                                        </th>
                                        <th on:click=move |_| toggle_sort("operation_date")>
                                            "Дата " {move || get_sort_indicator("operation_date", &sort_field.get(), sort_ascending.get())}
                                        </th>
                                        <th on:click=move |_| toggle_sort("delivering_date") style="background-color: #e8f5e9;">
                                            "Дата Доставки " {move || get_sort_indicator("delivering_date", &sort_field.get(), sort_ascending.get())}
                                        </th>
                                        <th on:click=move |_| toggle_sort("posting_number")>
                                            "Posting Number " {move || get_sort_indicator("posting_number", &sort_field.get(), sort_ascending.get())}
                                        </th>
                                        <th on:click=move |_| toggle_sort("transaction_type")>
                                            "Тип " {move || get_sort_indicator("transaction_type", &sort_field.get(), sort_ascending.get())}
                                        </th>
                                        <th on:click=move |_| toggle_sort("delivery_schema")>
                                            "Схема доставки " {move || get_sort_indicator("delivery_schema", &sort_field.get(), sort_ascending.get())}
                                        </th>
                                        <th on:click=move |_| toggle_sort("amount")>
                                            "Сумма " {move || get_sort_indicator("amount", &sort_field.get(), sort_ascending.get())}
                                        </th>
                                        <th on:click=move |_| toggle_sort("accruals_for_sale")>
                                            "Начисления " {move || get_sort_indicator("accruals_for_sale", &sort_field.get(), sort_ascending.get())}
                                        </th>
                                        <th on:click=move |_| toggle_sort("sale_commission")>
                                            "Комиссия " {move || get_sort_indicator("sale_commission", &sort_field.get(), sort_ascending.get())}
                                        </th>
                                        <th on:click=move |_| toggle_sort("delivery_charge")>
                                            "Доставка " {move || get_sort_indicator("delivery_charge", &sort_field.get(), sort_ascending.get())}
                                        </th>
                                        <th>"Статус"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {items.into_iter().map(|item| {
                                        let item_id_for_click = item.id.clone();
                                        view! {
                                            <tr
                                                class="transaction-row"
                                                on:click=move |_| open_detail(item_id_for_click.clone())
                                            >
                                                <td>{item.operation_id}</td>
                                                <td>{item.operation_type_name.clone()}</td>
                                                <td>{format_date(&item.operation_date)}</td>
                                                <td style="background-color: #e8f5e9;">
                                                    {item.delivering_date.as_ref().map(|d| format_date(d)).unwrap_or_default()}
                                                </td>
                                                <td class="posting-link">{item.posting_number.clone()}</td>
                                                <td>{item.transaction_type.clone()}</td>
                                                <td>{item.delivery_schema.clone()}</td>
                                                <td class="amount">{format!("{:.2}", item.amount)}</td>
                                                <td class="amount">{format!("{:.2}", item.accruals_for_sale)}</td>
                                                <td class="amount">{format!("{:.2}", item.sale_commission)}</td>
                                                <td class="amount">{format!("{:.2}", item.delivery_charge)}</td>
                                                <td>
                                                    {if item.is_posted {
                                                        view! { <span class="badge posted">"Проведен"</span> }
                                                    } else {
                                                        view! { <span class="badge not-posted">"Не проведен"</span> }
                                                    }}
                                                </td>
                                            </tr>
                                        }
                                    }).collect::<Vec<_>>()}
                                </tbody>
                            </table>
                        </div>

                    }.into_any()
                }
            }}

            {move || selected_id.get().map(|id| view! {
                <OzonTransactionsDetail
                    transaction_id=id
                    on_close=close_detail
                />
            })}
        </div>
    }
}

/// Экспорт транзакций в CSV для Excel
fn export_to_csv(data: &[OzonTransactionsDto]) -> Result<(), String> {
    // UTF-8 BOM для правильного отображения кириллицы в Excel
    let mut csv = String::from("\u{FEFF}");

    // Заголовок с точкой с запятой как разделитель
    csv.push_str("Operation ID;Operation Type;Operation Date;Delivering Date;Posting Number;Transaction Type;Delivery Schema;Amount;Accruals;Commission;Delivery;Status\n");

    for txn in data {
        let op_date = format_date(&txn.operation_date);
        let delivering_date = txn.delivering_date.as_ref().map(|d| format_date(d)).unwrap_or_default();
        let status = if txn.is_posted { "Проведен" } else { "Не проведен" };
        
        // Форматируем суммы с запятой как десятичный разделитель
        let amount_str = format!("{:.2}", txn.amount).replace(".", ",");
        let accruals_str = format!("{:.2}", txn.accruals_for_sale).replace(".", ",");
        let commission_str = format!("{:.2}", txn.sale_commission).replace(".", ",");
        let delivery_str = format!("{:.2}", txn.delivery_charge).replace(".", ",");

        csv.push_str(&format!(
            "\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";{};{};{};{};\"{}\"\n",
            txn.operation_id,
            txn.operation_type_name.replace('\"', "\"\""),
            op_date,
            delivering_date,
            txn.posting_number.replace('\"', "\"\""),
            txn.transaction_type.replace('\"', "\"\""),
            txn.delivery_schema.replace('\"', "\"\""),
            amount_str,
            accruals_str,
            commission_str,
            delivery_str,
            status
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
    let filename = format!("ozon_transactions_{}.csv", chrono::Utc::now().format("%Y%m%d_%H%M%S"));
    a.set_download(&filename);
    a.click();

    // Освобождаем URL
    Url::revoke_object_url(&url).map_err(|e| format!("Failed to revoke URL: {:?}", e))?;

    Ok(())
}
