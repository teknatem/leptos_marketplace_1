use super::details::OzonTransactionsDetail;
use crate::shared::components::date_input::DateInput;
use crate::shared::components::month_selector::MonthSelector;
use crate::shared::list_utils::{get_sort_indicator, Sortable};
use chrono::{Datelike, Utc};
use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::cmp::Ordering;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

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
    pub substatus: Option<String>,
    pub delivering_date: Option<String>,
    #[serde(rename = "is_posted")]
    pub is_posted: bool,
}

impl Sortable for OzonTransactionsDto {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "operation_id" => self.operation_id.cmp(&other.operation_id),
            "operation_type" => self
                .operation_type
                .to_lowercase()
                .cmp(&other.operation_type.to_lowercase()),
            "operation_type_name" => self
                .operation_type_name
                .to_lowercase()
                .cmp(&other.operation_type_name.to_lowercase()),
            "operation_date" => self.operation_date.cmp(&other.operation_date),
            "posting_number" => self
                .posting_number
                .to_lowercase()
                .cmp(&other.posting_number.to_lowercase()),
            "transaction_type" => self
                .transaction_type
                .to_lowercase()
                .cmp(&other.transaction_type.to_lowercase()),
            "delivery_schema" => self
                .delivery_schema
                .to_lowercase()
                .cmp(&other.delivery_schema.to_lowercase()),
            "amount" => self
                .amount
                .partial_cmp(&other.amount)
                .unwrap_or(Ordering::Equal),
            "accruals_for_sale" => self
                .accruals_for_sale
                .partial_cmp(&other.accruals_for_sale)
                .unwrap_or(Ordering::Equal),
            "sale_commission" => self
                .sale_commission
                .partial_cmp(&other.sale_commission)
                .unwrap_or(Ordering::Equal),
            "delivery_charge" => self
                .delivery_charge
                .partial_cmp(&other.delivery_charge)
                .unwrap_or(Ordering::Equal),
            "substatus" => match (&self.substatus, &other.substatus) {
                (Some(a), Some(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
                (Some(_), None) => Ordering::Greater,
                (None, Some(_)) => Ordering::Less,
                (None, None) => Ordering::Equal,
            },
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

    // Множественный выбор для массовых операций
    let (selected_ids, set_selected_ids) = signal::<Vec<String>>(Vec::new());

    // Статус массовых операций
    let (posting_in_progress, set_posting_in_progress) = signal(false);
    let (_, set_operation_results) = signal::<Vec<(String, bool, Option<String>)>>(Vec::new());
    let (_, set_current_operation) = signal::<Option<(usize, usize)>>(None); // (current, total)

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

    // State for save settings notification
    let (save_notification, set_save_notification) = signal(None::<String>);

    const FORM_KEY: &str = "a014_ozon_transactions";

    let load_transactions = move || {
        wasm_bindgen_futures::spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let date_from_val = date_from.get();
            let date_to_val = date_to.get();
            let transaction_type_val = transaction_type_filter.get();
            let operation_type_name_val = operation_type_name_filter.get();
            let posting_number_val = posting_number_filter.get();

            let mut query_params = format!("?date_from={}&date_to={}", date_from_val, date_to_val);

            if !transaction_type_val.is_empty() {
                query_params.push_str(&format!("&transaction_type={}", transaction_type_val));
            }

            if !operation_type_name_val.is_empty() {
                query_params.push_str(&format!("&operation_type_name={}", operation_type_name_val));
            }

            if !posting_number_val.is_empty() {
                query_params.push_str(&format!("&posting_number={}", posting_number_val));
            }

            let url = format!(
                "http://localhost:3000/api/ozon_transactions{}",
                query_params
            );
            log!("Fetching transactions with URL: {}", url);

            match Request::get(&url).send().await {
                Ok(response) => {
                    let status = response.status();
                    if status == 200 {
                        match response.text().await {
                            Ok(text) => {
                                match serde_json::from_str::<Vec<OzonTransactionsDto>>(&text) {
                                    Ok(items) => {
                                        log!(
                                            "Successfully parsed {} OZON transactions",
                                            items.len()
                                        );
                                        set_transactions.set(items);
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
        (
            data.len(),
            total_amount,
            total_accruals,
            total_commission,
            total_delivery,
        )
    };

    // Load saved settings from database on mount
    Effect::new(move |_| {
        spawn_local(async move {
            match load_saved_settings(FORM_KEY).await {
                Ok(Some(settings)) => {
                    if let Some(date_from_val) = settings.get("date_from").and_then(|v| v.as_str())
                    {
                        set_date_from.set(date_from_val.to_string());
                    }
                    if let Some(date_to_val) = settings.get("date_to").and_then(|v| v.as_str()) {
                        set_date_to.set(date_to_val.to_string());
                    }
                    if let Some(transaction_type_val) = settings
                        .get("transaction_type_filter")
                        .and_then(|v| v.as_str())
                    {
                        set_transaction_type_filter.set(transaction_type_val.to_string());
                    }
                    if let Some(operation_type_name_val) = settings
                        .get("operation_type_name_filter")
                        .and_then(|v| v.as_str())
                    {
                        set_operation_type_name_filter.set(operation_type_name_val.to_string());
                    }
                    if let Some(posting_number_val) = settings
                        .get("posting_number_filter")
                        .and_then(|v| v.as_str())
                    {
                        set_posting_number_filter.set(posting_number_val.to_string());
                    }
                    log!("Loaded saved settings for A014");
                }
                Ok(None) => {
                    log!("No saved settings found for A014");
                }
                Err(e) => {
                    log!("Failed to load saved settings: {}", e);
                }
            }
        });
    });

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

    // Переключение выбора одного документа
    let toggle_selection = move |id: String| {
        set_selected_ids.update(|ids| {
            if ids.contains(&id) {
                ids.retain(|x| x != &id);
                log!(
                    "Deselected transaction: {}, total selected: {}",
                    id,
                    ids.len()
                );
            } else {
                ids.push(id.clone());
                log!(
                    "Selected transaction: {}, total selected: {}",
                    id,
                    ids.len()
                );
            }
        });
    };

    // Выбрать все / снять все
    let toggle_all = move |_| {
        let items = get_sorted_items();
        let selected = selected_ids.get();
        if selected.len() == items.len() && !items.is_empty() {
            set_selected_ids.set(Vec::new()); // Снять все
        } else {
            set_selected_ids.set(items.iter().map(|item| item.id.clone()).collect());
            // Выбрать все
        }
    };

    // Проверка, выбраны ли все
    let all_selected = move || {
        let items = get_sorted_items();
        let selected = selected_ids.get();
        !items.is_empty() && selected.len() == items.len()
    };

    // Проверка, выбран ли конкретный документ
    let is_selected = move |id: &str| selected_ids.get().contains(&id.to_string());

    // Массовое проведение
    let post_selected = move |_| {
        let ids = selected_ids.get();
        if ids.is_empty() {
            return;
        }

        set_posting_in_progress.set(true);
        set_operation_results.set(Vec::new());
        set_current_operation.set(Some((0, ids.len())));

        wasm_bindgen_futures::spawn_local(async move {
            let mut results = Vec::new();
            let total = ids.len();

            for (index, id) in ids.iter().enumerate() {
                set_current_operation.set(Some((index + 1, total)));
                let url = format!(
                    "http://localhost:3000/api/a014/ozon-transactions/{}/post",
                    id
                );
                match Request::post(&url).send().await {
                    Ok(response) => {
                        if response.status() == 200 {
                            results.push((id.clone(), true, None));
                        } else {
                            results.push((
                                id.clone(),
                                false,
                                Some(format!("HTTP {}", response.status())),
                            ));
                        }
                    }
                    Err(e) => {
                        results.push((id.clone(), false, Some(format!("{:?}", e))));
                    }
                }
            }

            set_operation_results.set(results);
            set_posting_in_progress.set(false);
            set_current_operation.set(None);
            set_selected_ids.set(Vec::new());

            // Перезагрузить список
            load_transactions();
        });
    };

    // Массовая отмена проведения
    let unpost_selected = move |_| {
        let ids = selected_ids.get();
        if ids.is_empty() {
            return;
        }

        set_posting_in_progress.set(true);
        set_operation_results.set(Vec::new());
        set_current_operation.set(Some((0, ids.len())));

        wasm_bindgen_futures::spawn_local(async move {
            let mut results = Vec::new();
            let total = ids.len();

            for (index, id) in ids.iter().enumerate() {
                set_current_operation.set(Some((index + 1, total)));
                let url = format!(
                    "http://localhost:3000/api/a014/ozon-transactions/{}/unpost",
                    id
                );
                match Request::post(&url).send().await {
                    Ok(response) => {
                        if response.status() == 200 {
                            results.push((id.clone(), true, None));
                        } else {
                            results.push((
                                id.clone(),
                                false,
                                Some(format!("HTTP {}", response.status())),
                            ));
                        }
                    }
                    Err(e) => {
                        results.push((id.clone(), false, Some(format!("{:?}", e))));
                    }
                }
            }

            set_operation_results.set(results);
            set_posting_in_progress.set(false);
            set_current_operation.set(None);
            set_selected_ids.set(Vec::new());

            // Перезагрузить список
            load_transactions();
        });
    };

    // Save current settings to database
    let save_settings_to_db = move |_| {
        let settings = json!({
            "date_from": date_from.get(),
            "date_to": date_to.get(),
            "transaction_type_filter": transaction_type_filter.get(),
            "operation_type_name_filter": operation_type_name_filter.get(),
            "posting_number_filter": posting_number_filter.get(),
        });

        spawn_local(async move {
            match save_settings_to_database(FORM_KEY, settings).await {
                Ok(_) => {
                    set_save_notification.set(Some("✓ Настройки сохранены".to_string()));
                    // Clear notification after 3 seconds
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
                    if let Some(date_from_val) = settings.get("date_from").and_then(|v| v.as_str())
                    {
                        set_date_from.set(date_from_val.to_string());
                    }
                    if let Some(date_to_val) = settings.get("date_to").and_then(|v| v.as_str()) {
                        set_date_to.set(date_to_val.to_string());
                    }
                    if let Some(transaction_type_val) = settings
                        .get("transaction_type_filter")
                        .and_then(|v| v.as_str())
                    {
                        set_transaction_type_filter.set(transaction_type_val.to_string());
                    }
                    if let Some(operation_type_name_val) = settings
                        .get("operation_type_name_filter")
                        .and_then(|v| v.as_str())
                    {
                        set_operation_type_name_filter.set(operation_type_name_val.to_string());
                    }
                    if let Some(posting_number_val) = settings
                        .get("posting_number_filter")
                        .and_then(|v| v.as_str())
                    {
                        set_posting_number_filter.set(posting_number_val.to_string());
                    }
                    set_save_notification.set(Some("✓ Настройки восстановлены".to_string()));
                    // Clear notification after 3 seconds
                    spawn_local(async move {
                        gloo_timers::future::TimeoutFuture::new(3000).await;
                        set_save_notification.set(None);
                    });
                    log!("Restored saved settings for A014");
                }
                Ok(None) => {
                    set_save_notification.set(Some("ℹ Нет сохраненных настроек".to_string()));
                    spawn_local(async move {
                        gloo_timers::future::TimeoutFuture::new(3000).await;
                        set_save_notification.set(None);
                    });
                    log!("No saved settings found for A014");
                }
                Err(e) => {
                    set_save_notification.set(Some(format!("✗ Ошибка: {}", e)));
                    log!("Failed to load saved settings: {}", e);
                }
            }
        });
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
        <div class="ozon-transactions-list" style="background: #f8f9fa; padding: 12px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
            // Header - Row 1: Title with Post/Unpost and Settings Buttons
            <div style="background: linear-gradient(135deg, #4a5568 0%, #2d3748 100%); padding: 8px 12px; border-radius: 6px 6px 0 0; margin: -12px -12px 0 -12px; display: flex; align-items: center; justify-content: space-between;">
                <div style="display: flex; align-items: center; gap: 12px;">
                    <h2 style="margin: 0; font-size: 1.1rem; font-weight: 600; color: white; letter-spacing: 0.5px;">"📋 OZON Транзакции (A014)"</h2>

                    // Post/Unpost buttons
                    <button
                        prop:disabled=move || selected_ids.get().is_empty() || posting_in_progress.get()
                        on:click=post_selected
                        style="height: 32px; padding: 0 16px; background: #48bb78; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 0.875rem; font-weight: 500; transition: all 0.2s ease; display: flex; align-items: center; gap: 4px;"
                        style:opacity=move || if selected_ids.get().is_empty() || posting_in_progress.get() { "0.5" } else { "1" }
                        style:cursor=move || if selected_ids.get().is_empty() || posting_in_progress.get() { "not-allowed" } else { "pointer" }
                    >
                        {move || format!("✓ Post ({})", selected_ids.get().len())}
                    </button>
                    <button
                        prop:disabled=move || selected_ids.get().is_empty() || posting_in_progress.get()
                        on:click=unpost_selected
                        style="height: 32px; padding: 0 16px; background: #FF9800; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 0.875rem; font-weight: 500; transition: all 0.2s ease; display: flex; align-items: center; gap: 4px;"
                        style:opacity=move || if selected_ids.get().is_empty() || posting_in_progress.get() { "0.5" } else { "1" }
                        style:cursor=move || if selected_ids.get().is_empty() || posting_in_progress.get() { "not-allowed" } else { "pointer" }
                    >
                        {move || format!("✗ Unpost ({})", selected_ids.get().len())}
                    </button>
                </div>

                <div style="display: flex; gap: 8px; align-items: center;">
                    // Excel export button
                    <button
                        on:click=move |_| {
                            let data = get_sorted_items();
                            if let Err(e) = export_to_csv(&data) {
                                log!("Failed to export: {}", e);
                            }
                        }
                        style="height: 32px; padding: 0 16px; background: #217346; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 0.875rem; font-weight: 500; transition: all 0.2s ease; display: flex; align-items: center; gap: 4px;"
                        disabled=move || loading.get() || transactions.get().is_empty()
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
                        on:click=restore_settings
                        style="width: 32px; height: 32px; background: rgba(255,255,255,0.2); color: white; border: 1px solid rgba(255,255,255,0.3); border-radius: 4px; cursor: pointer; font-size: 1rem; transition: all 0.2s ease; display: flex; align-items: center; justify-content: center; padding: 0;"
                        title="Восстановить настройки из базы данных"
                    >
                        "🔄"
                    </button>
                    <button
                        on:click=save_settings_to_db
                        style="width: 32px; height: 32px; background: rgba(255,255,255,0.2); color: white; border: 1px solid rgba(255,255,255,0.3); border-radius: 4px; cursor: pointer; font-size: 1rem; transition: all 0.2s ease; display: flex; align-items: center; justify-content: center; padding: 0;"
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
                        value=date_from
                        on_change=move |val| set_date_from.set(val)
                    />
                    <span style="color: #6c757d;">"—"</span>
                    <DateInput
                        value=date_to
                        on_change=move |val| set_date_to.set(val)
                    />
                    <MonthSelector
                        on_select=Callback::new(move |(from, to)| {
                            set_date_from.set(from);
                            set_date_to.set(to);
                        })
                    />
                </div>

                // Transaction type filter
                <div style="display: flex; align-items: center; gap: 8px;">
                    <label style="margin: 0; font-size: 0.875rem; font-weight: 500; color: #495057; white-space: nowrap;">"Тип транзакции:"</label>
                    <select
                        prop:value=transaction_type_filter
                        on:change=move |ev| {
                            set_transaction_type_filter.set(event_target_value(&ev));
                        }
                        style="padding: 6px 10px; border: 1px solid #ced4da; border-radius: 4px; font-size: 0.875rem; min-width: 120px; background: #fff;"
                    >
                        <option value="">"Все"</option>
                        <option value="orders">"orders"</option>
                        <option value="returns">"returns"</option>
                        <option value="client_returns">"client_returns"</option>
                        <option value="services">"services"</option>
                        <option value="other">"other"</option>
                        <option value="transfer_delivery">"transfer_delivery"</option>
                    </select>
                </div>

                // Operation type filter
                <div style="display: flex; align-items: center; gap: 8px;">
                    <label style="margin: 0; font-size: 0.875rem; font-weight: 500; color: #495057; white-space: nowrap;">"Тип операции:"</label>
                    <input
                        type="text"
                        prop:value=operation_type_name_filter
                        on:input=move |ev| {
                            set_operation_type_name_filter.set(event_target_value(&ev));
                        }
                        placeholder="Введите тип операции"
                        style="padding: 6px 10px; border: 1px solid #ced4da; border-radius: 4px; font-size: 0.875rem; min-width: 150px;"
                    />
                </div>

                // Posting number filter
                <div style="display: flex; align-items: center; gap: 8px;">
                    <label style="margin: 0; font-size: 0.875rem; font-weight: 500; color: #495057; white-space: nowrap;">"Posting #:"</label>
                    <input
                        type="text"
                        prop:value=posting_number_filter
                        on:input=move |ev| {
                            set_posting_number_filter.set(event_target_value(&ev));
                        }
                        placeholder="Поиск по номеру"
                        style="padding: 6px 10px; border: 1px solid #ced4da; border-radius: 4px; font-size: 0.875rem; min-width: 150px;"
                    />
                </div>

                // Update button
                <button
                    on:click=move |_| {
                        load_transactions();
                    }
                    class="action-button action-button-success"
                    style="height: 32px; padding: 0 16px; background: #48bb78; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 0.875rem; font-weight: 500; transition: all 0.2s ease; display: flex; align-items: center; gap: 4px;"
                    disabled=move || loading.get()
                >
                    "↻ Обновить"
                </button>
            </div>

            // Totals display
            {move || if !loading.get() {
                let (count, total_amount, total_accruals, total_commission, total_delivery) = totals();
                view! {
                    <div style="margin-bottom: 10px; padding: 3px 12px; background: var(--color-background-alt, #f5f5f5); border-radius: 4px;">
                        <span style="font-size: 0.875rem; font-weight: 600; color: var(--color-text);">
                            "Total: " {count} " records | "
                            "Amount: " {format!("{:.2}", total_amount)} " | "
                            "Accruals: " {format!("{:.2}", total_accruals)} " | "
                            "Commission: " {format!("{:.2}", total_commission)} " | "
                            "Delivery: " {format!("{:.2}", total_delivery)}
                        </span>
                    </div>
                }.into_any()
            } else {
                view! { <></> }.into_any()
            }}

            {move || error.get().map(|err| view! {
                <div class="error-message" style="padding: 12px; background: #ffebee; border: 1px solid #ffcdd2; border-radius: 4px; color: #c62828; margin-bottom: 10px;">{err}</div>
            })}

            {move || {
                if loading.get() {
                    view! {
                        <div class="loading-spinner" style="text-align: center; padding: 40px;">"Загрузка транзакций..."</div>
                    }.into_any()
                } else {
                    let items = get_sorted_items();
                    view! {
                        <div class="table-container" style="overflow-y: auto; max-height: calc(100vh - 240px); border: 1px solid #e0e0e0;">
                            <table class="transactions-table" style="width: 100%; border-collapse: collapse; margin: 0; font-size: 0.85em;">
                                <thead style="position: sticky; top: 0; z-index: 10; background: var(--color-table-header-bg, #f5f5f5);">
                                    <tr>
                                        <th style="border: 1px solid #e0e0e0; padding: 4px 6px; text-align: center; font-weight: 600;">
                                            <input
                                                type="checkbox"
                                                on:change=toggle_all
                                                prop:checked=move || all_selected()
                                            />
                                        </th>
                                        <th on:click=move |_| toggle_sort("operation_date") style="border: 1px solid #e0e0e0; padding: 4px 6px; cursor: pointer; user-select: none; font-weight: 600;">
                                            "Дата " {move || get_sort_indicator("operation_date", &sort_field.get(), sort_ascending.get())}
                                        </th>
                                        <th on:click=move |_| toggle_sort("operation_id") style="border: 1px solid #e0e0e0; padding: 4px 6px; cursor: pointer; user-select: none; font-weight: 600;">
                                            "Operation ID " {move || get_sort_indicator("operation_id", &sort_field.get(), sort_ascending.get())}
                                        </th>
                                        <th on:click=move |_| toggle_sort("operation_type_name") style="border: 1px solid #e0e0e0; padding: 4px 6px; cursor: pointer; user-select: none; font-weight: 600;">
                                            "Тип операции " {move || get_sort_indicator("operation_type_name", &sort_field.get(), sort_ascending.get())}
                                        </th>
                                        <th on:click=move |_| toggle_sort("substatus") style="border: 1px solid #e0e0e0; padding: 4px 6px; cursor: pointer; user-select: none; font-weight: 600;">
                                            "Substatus " {move || get_sort_indicator("substatus", &sort_field.get(), sort_ascending.get())}
                                        </th>
                                        <th on:click=move |_| toggle_sort("delivering_date") style="border: 1px solid #e0e0e0; padding: 4px 6px; cursor: pointer; user-select: none; font-weight: 600; background-color: #e8f5e9; color: #2e7d32;">
                                            "Дата Доставки " {move || get_sort_indicator("delivering_date", &sort_field.get(), sort_ascending.get())}
                                        </th>
                                        <th on:click=move |_| toggle_sort("posting_number") style="border: 1px solid #e0e0e0; padding: 4px 6px; cursor: pointer; user-select: none; font-weight: 600;">
                                            "Posting Number " {move || get_sort_indicator("posting_number", &sort_field.get(), sort_ascending.get())}
                                        </th>
                                        <th on:click=move |_| toggle_sort("transaction_type") style="border: 1px solid #e0e0e0; padding: 4px 6px; cursor: pointer; user-select: none; font-weight: 600;">
                                            "Тип " {move || get_sort_indicator("transaction_type", &sort_field.get(), sort_ascending.get())}
                                        </th>
                                        <th on:click=move |_| toggle_sort("delivery_schema") style="border: 1px solid #e0e0e0; padding: 4px 6px; cursor: pointer; user-select: none; font-weight: 600;">
                                            "Схема доставки " {move || get_sort_indicator("delivery_schema", &sort_field.get(), sort_ascending.get())}
                                        </th>
                                        <th on:click=move |_| toggle_sort("amount") style="border: 1px solid #e0e0e0; padding: 4px 6px; cursor: pointer; user-select: none; font-weight: 600;">
                                            "Сумма " {move || get_sort_indicator("amount", &sort_field.get(), sort_ascending.get())}
                                        </th>
                                        <th on:click=move |_| toggle_sort("accruals_for_sale") style="border: 1px solid #e0e0e0; padding: 4px 6px; cursor: pointer; user-select: none; font-weight: 600;">
                                            "Начисления " {move || get_sort_indicator("accruals_for_sale", &sort_field.get(), sort_ascending.get())}
                                        </th>
                                        <th on:click=move |_| toggle_sort("sale_commission") style="border: 1px solid #e0e0e0; padding: 4px 6px; cursor: pointer; user-select: none; font-weight: 600;">
                                            "Комиссия " {move || get_sort_indicator("sale_commission", &sort_field.get(), sort_ascending.get())}
                                        </th>
                                        <th on:click=move |_| toggle_sort("delivery_charge") style="border: 1px solid #e0e0e0; padding: 4px 6px; cursor: pointer; user-select: none; font-weight: 600;">
                                            "Доставка " {move || get_sort_indicator("delivery_charge", &sort_field.get(), sort_ascending.get())}
                                        </th>
                                        <th on:click=move |_| toggle_sort("is_posted") style="border: 1px solid #e0e0e0; padding: 4px 6px; cursor: pointer; user-select: none; font-weight: 600;">"Post " {move || get_sort_indicator("is_posted", &sort_field.get(), sort_ascending.get())}</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {items.into_iter().map(|item| {
                                        let item_id = item.id.clone();
                                        let item_id_for_checkbox = item.id.clone();
                                        let item_id_for_checked = item.id.clone();
                                        let substatus_display = item.substatus.clone().unwrap_or_default();
                                        view! {
                                            <tr class="transaction-row" style="cursor: pointer;">
                                                <td
                                                    style="border: 1px solid #e0e0e0; padding: 4px 6px; text-align: center;"
                                                >
                                                    <input
                                                        type="checkbox"
                                                        prop:checked=move || is_selected(&item_id_for_checked)
                                                        on:change=move |_| {
                                                            toggle_selection(item_id_for_checkbox.clone());
                                                        }
                                                        on:click=move |e| {
                                                            e.stop_propagation();
                                                        }
                                                    />
                                                </td>
                                                <td on:click={let id = item_id.clone(); move |_| open_detail(id.clone())} style="border: 1px solid #e0e0e0; padding: 4px 6px;">{format_date(&item.operation_date)}</td>
                                                <td on:click={let id = item_id.clone(); move |_| open_detail(id.clone())} style="border: 1px solid #e0e0e0; padding: 4px 6px;">{item.operation_id}</td>
                                                <td on:click={let id = item_id.clone(); move |_| open_detail(id.clone())} style="border: 1px solid #e0e0e0; padding: 4px 6px;">{item.operation_type_name.clone()}</td>
                                                <td on:click={let id = item_id.clone(); move |_| open_detail(id.clone())} style="border: 1px solid #e0e0e0; padding: 4px 6px;">
                                                    {substatus_display}
                                                </td>
                                                <td on:click={let id = item_id.clone(); move |_| open_detail(id.clone())} style="border: 1px solid #e0e0e0; padding: 4px 6px; color: #2e7d32;">
                                                    {item.delivering_date.as_ref().map(|d| format_date(d)).unwrap_or_default()}
                                                </td>
                                                <td on:click={let id = item_id.clone(); move |_| open_detail(id.clone())} class="posting-link" style="border: 1px solid #e0e0e0; padding: 4px 6px; color: #2196F3;">{item.posting_number.clone()}</td>
                                                <td on:click={let id = item_id.clone(); move |_| open_detail(id.clone())} style="border: 1px solid #e0e0e0; padding: 4px 6px;">{item.transaction_type.clone()}</td>
                                                <td on:click={let id = item_id.clone(); move |_| open_detail(id.clone())} style="border: 1px solid #e0e0e0; padding: 4px 6px;">{item.delivery_schema.clone()}</td>
                                                <td on:click={let id = item_id.clone(); move |_| open_detail(id.clone())} class="amount" style="border: 1px solid #e0e0e0; padding: 4px 6px; text-align: right;">{format!("{:.2}", item.amount)}</td>
                                                <td on:click={let id = item_id.clone(); move |_| open_detail(id.clone())} class="amount" style="border: 1px solid #e0e0e0; padding: 4px 6px; text-align: right;">{format!("{:.2}", item.accruals_for_sale)}</td>
                                                <td on:click={let id = item_id.clone(); move |_| open_detail(id.clone())} class="amount" style="border: 1px solid #e0e0e0; padding: 4px 6px; text-align: right;">{format!("{:.2}", item.sale_commission)}</td>
                                                <td on:click={let id = item_id.clone(); move |_| open_detail(id.clone())} class="amount" style="border: 1px solid #e0e0e0; padding: 4px 6px; text-align: right;">{format!("{:.2}", item.delivery_charge)}</td>
                                                <td on:click={let id = item_id.clone(); move |_| open_detail(id.clone())} style="border: 1px solid #e0e0e0; padding: 4px 6px; text-align: center;">
                                                    {if item.is_posted { "Да" } else { "Нет" }}
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

/// Экспорт транзакций в CSV для Excel
fn export_to_csv(data: &[OzonTransactionsDto]) -> Result<(), String> {
    // UTF-8 BOM для правильного отображения кириллицы в Excel
    let mut csv = String::from("\u{FEFF}");

    // Заголовок с точкой с запятой как разделитель
    csv.push_str("Date;Operation ID;Operation Type;Substatus;Delivering Date;Posting Number;Transaction Type;Delivery Schema;Amount;Accruals;Commission;Delivery;Post\n");

    for txn in data {
        let op_date = format_date(&txn.operation_date);
        let substatus = txn.substatus.as_ref().map(|s| s.as_str()).unwrap_or("");
        let delivering_date = txn
            .delivering_date
            .as_ref()
            .map(|d| format_date(d))
            .unwrap_or_default();
        let status = if txn.is_posted { "Да" } else { "Нет" };

        // Форматируем суммы с запятой как десятичный разделитель
        let amount_str = format!("{:.2}", txn.amount).replace(".", ",");
        let accruals_str = format!("{:.2}", txn.accruals_for_sale).replace(".", ",");
        let commission_str = format!("{:.2}", txn.sale_commission).replace(".", ",");
        let delivery_str = format!("{:.2}", txn.delivery_charge).replace(".", ",");

        csv.push_str(&format!(
            "\"{}\";{};\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";{};{};{};{};\"{}\"
",
            op_date,
            txn.operation_id,
            txn.operation_type_name.replace('\"', "\"\""),
            substatus,
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
    let filename = format!(
        "ozon_transactions_{}.csv",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );
    a.set_download(&filename);
    a.click();

    // Освобождаем URL
    Url::revoke_object_url(&url).map_err(|e| format!("Failed to revoke URL: {:?}", e))?;

    Ok(())
}
