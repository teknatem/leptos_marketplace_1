use leptos::prelude::*;
use leptos::logging::log;
use serde::{Deserialize, Serialize};
use gloo_net::http::Request;
use super::details::OzonTransactionsDetail;
use crate::shared::list_utils::{get_sort_indicator, Sortable};
use std::cmp::Ordering;

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
    pub amount: f64,
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
            "amount" => self.amount.partial_cmp(&other.amount).unwrap_or(Ordering::Equal),
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

    let load_transactions = move || {
        wasm_bindgen_futures::spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let url = "http://localhost:3000/api/ozon_transactions";

            match Request::get(url).send().await {
                Ok(response) => {
                    let status = response.status();
                    if status == 200 {
                        match response.text().await {
                            Ok(text) => {
                                log!("Received response text (first 500 chars): {}",
                                    text.chars().take(500).collect::<String>());

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
            <div class="list-header">
                <h2>"OZON Транзакции"</h2>
                <button
                    on:click=move |_| load_transactions()
                    disabled=move || loading.get()
                >
                    {move || if loading.get() { "Загрузка..." } else { "Обновить" }}
                </button>
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
                                        <th on:click=move |_| toggle_sort("posting_number")>
                                            "Posting Number " {move || get_sort_indicator("posting_number", &sort_field.get(), sort_ascending.get())}
                                        </th>
                                        <th on:click=move |_| toggle_sort("transaction_type")>
                                            "Тип " {move || get_sort_indicator("transaction_type", &sort_field.get(), sort_ascending.get())}
                                        </th>
                                        <th on:click=move |_| toggle_sort("amount")>
                                            "Сумма " {move || get_sort_indicator("amount", &sort_field.get(), sort_ascending.get())}
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
                                                <td class="posting-link">{item.posting_number.clone()}</td>
                                                <td>{item.transaction_type.clone()}</td>
                                                <td class="amount">{format!("{:.2}", item.amount)}</td>
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

                        <div class="list-summary">
                            <span>"Всего транзакций: " {move || get_sorted_items().len()}</span>
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
