use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

// Import posting detail components
use crate::domain::a010_ozon_fbs_posting::ui::details::OzonFbsPostingDetail;
use crate::domain::a011_ozon_fbo_posting::ui::details::OzonFboPostingDetail;

// DTO структуры для детального представления
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonTransactionsDetailDto {
    pub id: String,
    pub code: String,
    pub description: String,
    pub header: HeaderDto,
    pub posting: PostingDto,
    pub items: Vec<ItemDto>,
    pub services: Vec<ServiceDto>,
    #[serde(rename = "is_posted")]
    pub is_posted: bool,
    #[serde(rename = "posting_ref")]
    pub posting_ref: Option<String>,
    #[serde(rename = "posting_ref_type")]
    pub posting_ref_type: Option<String>,
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(rename = "updated_at")]
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderDto {
    pub operation_id: i64,
    pub operation_type: String,
    pub operation_date: String,
    pub operation_type_name: String,
    pub delivery_charge: f64,
    pub return_delivery_charge: f64,
    pub accruals_for_sale: f64,
    pub sale_commission: f64,
    pub amount: f64,
    pub transaction_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostingDto {
    pub delivery_schema: String,
    pub order_date: String,
    pub posting_number: String,
    pub warehouse_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDto {
    pub name: String,
    pub sku: i64,
    #[serde(default)]
    pub price: Option<f64>,
    #[serde(default)]
    pub ratio: Option<f64>,
    #[serde(default)]
    pub marketplace_product_ref: Option<String>,
    #[serde(default)]
    pub nomenclature_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDto {
    pub name: String,
    pub price: f64,
}

/// Форматирует дату из "2025-10-11 00:00:00" в dd.mm.yyyy HH:MM
fn format_datetime(date_str: &str) -> String {
    // Парсим формат "2025-10-11 00:00:00"
    let parts: Vec<&str> = date_str.split_whitespace().collect();
    if parts.len() >= 2 {
        let date_part = parts[0];
        let time_part = parts[1];
        if let Some((year, rest)) = date_part.split_once('-') {
            if let Some((month, day)) = rest.split_once('-') {
                let time_short: String = time_part.chars().take(5).collect(); // HH:MM
                return format!("{}.{}.{} {}", day, month, year, time_short);
            }
        }
    }
    date_str.to_string() // fallback
}

#[component]
pub fn OzonTransactionsDetail(
    transaction_id: String,
    #[prop(into)] on_close: Callback<()>,
) -> impl IntoView {
    let (transaction_data, set_transaction_data) =
        signal::<Option<OzonTransactionsDetailDto>>(None);
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal::<Option<String>>(None);
    let (active_tab, set_active_tab) = signal("general");
    let (posting, set_posting) = signal(false);
    let (projections, set_projections) = signal::<Option<serde_json::Value>>(None);
    let (projections_loading, set_projections_loading) = signal(false);

    // Signal for selected posting document (type, id)
    let (selected_posting, set_selected_posting) = signal::<Option<(String, String)>>(None);

    // Store transaction ID for use in handlers
    let stored_id = StoredValue::new(transaction_id.clone());
    let transaction_id_for_effect = transaction_id.clone();

    // Memo for posting status
    let is_posted =
        Memo::new(move |_| transaction_data.get().map(|s| s.is_posted).unwrap_or(false));

    // Загрузить детальные данные
    Effect::new(move || {
        let id = transaction_id_for_effect.clone();
        wasm_bindgen_futures::spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let url = format!("http://localhost:3000/api/ozon_transactions/{}", id);

            match Request::get(&url).send().await {
                Ok(response) => {
                    let status = response.status();
                    if status == 200 {
                        match response.text().await {
                            Ok(text) => {
                                match serde_json::from_str::<OzonTransactionsDetailDto>(&text) {
                                    Ok(data) => {
                                        let transaction_id = data.id.clone();
                                        set_transaction_data.set(Some(data));
                                        set_loading.set(false);

                                        // Асинхронная загрузка проекций
                                        let set_projections = set_projections.clone();
                                        let set_projections_loading =
                                            set_projections_loading.clone();
                                        wasm_bindgen_futures::spawn_local(async move {
                                            set_projections_loading.set(true);
                                            let projections_url = format!("http://localhost:3000/api/a014/ozon-transactions/{}/projections", transaction_id);
                                            match Request::get(&projections_url).send().await {
                                                Ok(resp) => {
                                                    if resp.status() == 200 {
                                                        if let Ok(text) = resp.text().await {
                                                            if let Ok(proj_data) =
                                                                serde_json::from_str::<
                                                                    serde_json::Value,
                                                                >(
                                                                    &text
                                                                )
                                                            {
                                                                set_projections
                                                                    .set(Some(proj_data));
                                                            }
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    log!("Failed to load projections: {:?}", e);
                                                }
                                            }
                                            set_projections_loading.set(false);
                                        });
                                    }
                                    Err(e) => {
                                        log!("Failed to parse transaction detail: {:?}", e);
                                        set_error.set(Some(format!("Ошибка парсинга: {}", e)));
                                        set_loading.set(false);
                                    }
                                }
                            }
                            Err(e) => {
                                log!("Failed to read response: {:?}", e);
                                set_error.set(Some(format!("Ошибка чтения ответа: {}", e)));
                                set_loading.set(false);
                            }
                        }
                    } else {
                        set_error.set(Some(format!("Ошибка сервера: {}", status)));
                        set_loading.set(false);
                    }
                }
                Err(e) => {
                    log!("Failed to fetch transaction: {:?}", e);
                    set_error.set(Some(format!("Ошибка загрузки: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    });

    view! {
        <div class="detail-form">
            <div class="detail-form-header">
                <div class="detail-form-header-left">
                    <h2>
                        {move || {
                            transaction_data.get()
                                .map(|d| format!("Транзакция OZON #{}", d.header.operation_id))
                                .unwrap_or_else(|| "Детали транзакции OZON".to_string())
                        }}
                    </h2>
                    <Show when=move || transaction_data.get().is_some()>
                        {move || {
                            let posted = is_posted.get();
                            view! {
                                <div class=move || if posted { "status-badge status-badge-posted" } else { "status-badge status-badge-not-posted" }>
                                    <span class="status-badge-icon">{if posted { "✓" } else { "○" }}</span>
                                    <span>{if posted { "Проведен" } else { "Не проведен" }}</span>
                                </div>
                            }
                        }}
                    </Show>
                </div>
                <div class="detail-form-header-right">
                        <Show when=move || transaction_data.get().is_some()>
                            <Show
                                when=move || !is_posted.get()
                                fallback=move || {
                                    view! {
                                        <button
                                            class="button button--warning"
                                            on:click=move |_| {
                                                let doc_id = stored_id.get_value();
                                                set_posting.set(true);
                                                wasm_bindgen_futures::spawn_local(async move {
                                                    let url = format!("http://localhost:3000/api/a014/ozon-transactions/{}/unpost", doc_id);
                                                    match Request::post(&url).send().await {
                                                        Ok(response) => {
                                                            if response.status() == 200 {
                                                                log!("Transaction unposted successfully");
                                                                // Reload transaction data
                                                                let reload_url = format!("http://localhost:3000/api/ozon_transactions/{}", doc_id);
                                                                if let Ok(resp) = Request::get(&reload_url).send().await {
                                                                    if let Ok(text) = resp.text().await {
                                                                        if let Ok(data) = serde_json::from_str::<OzonTransactionsDetailDto>(&text) {
                                                                            log!("Reloaded transaction, is_posted: {}", data.is_posted);
                                                                            set_transaction_data.set(Some(data));
                                                                        }
                                                                    }
                                                                }
                                                                // Reload projections
                                                                let projections_url = format!("http://localhost:3000/api/a014/ozon-transactions/{}/projections", doc_id);
                                                                if let Ok(resp) = Request::get(&projections_url).send().await {
                                                                    if resp.status() == 200 {
                                                                        if let Ok(text) = resp.text().await {
                                                                            if let Ok(proj_data) = serde_json::from_str::<serde_json::Value>(&text) {
                                                                                set_projections.set(Some(proj_data));
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            } else {
                                                                log!("Failed to unpost: status {}", response.status());
                                                            }
                                                        }
                                                        Err(e) => {
                                                            log!("Error unposting: {:?}", e);
                                                        }
                                                    }
                                                    set_posting.set(false);
                                                });
                                            }
                                            prop:disabled=move || posting.get()
                                        >
                                            {move || if posting.get() { "Отмена..." } else { "✗ Отменить" }}
                                        </button>
                                    }
                                }
                            >
                                {
                                    view! {
                                        <button
                                            class="button button--primary"
                                            on:click=move |_| {
                                                let doc_id = stored_id.get_value();
                                                set_posting.set(true);
                                                wasm_bindgen_futures::spawn_local(async move {
                                                    let url = format!("http://localhost:3000/api/a014/ozon-transactions/{}/post", doc_id);
                                                    match Request::post(&url).send().await {
                                                        Ok(response) => {
                                                            if response.status() == 200 {
                                                                log!("Transaction posted successfully");
                                                                // Reload transaction data
                                                                let reload_url = format!("http://localhost:3000/api/ozon_transactions/{}", doc_id);
                                                                if let Ok(resp) = Request::get(&reload_url).send().await {
                                                                    if let Ok(text) = resp.text().await {
                                                                        if let Ok(data) = serde_json::from_str::<OzonTransactionsDetailDto>(&text) {
                                                                            log!("Reloaded transaction, is_posted: {}", data.is_posted);
                                                                            set_transaction_data.set(Some(data));
                                                                        }
                                                                    }
                                                                }
                                                                // Reload projections
                                                                let projections_url = format!("http://localhost:3000/api/a014/ozon-transactions/{}/projections", doc_id);
                                                                if let Ok(resp) = Request::get(&projections_url).send().await {
                                                                    if resp.status() == 200 {
                                                                        if let Ok(text) = resp.text().await {
                                                                            if let Ok(proj_data) = serde_json::from_str::<serde_json::Value>(&text) {
                                                                                set_projections.set(Some(proj_data));
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            } else {
                                                                log!("Failed to post: status {}", response.status());
                                                            }
                                                        }
                                                        Err(e) => {
                                                            log!("Error posting: {:?}", e);
                                                        }
                                                    }
                                                    set_posting.set(false);
                                                });
                                            }
                                            prop:disabled=move || posting.get()
                                        >
                                            {move || if posting.get() { "Проведение..." } else { "✓ Провести" }}
                                        </button>
                                    }
                                }
                            </Show>
                        </Show>
                        <button
                            class="button button--secondary"
                            on:click=move |_| on_close.run(())
                        >
                            "✕ Закрыть"
                        </button>
                    </div>
                </div>

                <div class="detail-form-content">
                    {move || {
                        if loading.get() {
                            view! {
                                <div style="text-align: center; padding: var(--space-2xl);">
                                    <p style="font-size: var(--font-size-sm);">"Загрузка..."</p>
                                </div>
                            }.into_any()
                        } else if let Some(err) = error.get() {
                            view! {
                                <div style="padding: var(--space-lg); background: var(--color-error-bg); border: 1px solid var(--color-error-border); border-radius: var(--radius-sm); color: var(--color-error); margin: var(--space-lg); font-size: var(--font-size-sm);">
                                    <strong>"Ошибка: "</strong>{err}
                                </div>
                            }.into_any()
                        } else if let Some(data) = transaction_data.get() {
                                view! {
                                <div>
                                    <div class="detail-tabs">
                                        <button
                                            class="detail-tab"
                                            class:active=move || active_tab.get() == "general"
                                            on:click=move |_| set_active_tab.set("general")
                                        >
                                            "Общие данные"
                                        </button>
                                        <button
                                            class="detail-tab"
                                            class:active=move || active_tab.get() == "items"
                                            on:click=move |_| set_active_tab.set("items")
                                        >
                                            "Товары (" {data.items.len()} ")"
                                        </button>
                                        <button
                                            class="detail-tab"
                                            class:active=move || active_tab.get() == "services"
                                            on:click=move |_| set_active_tab.set("services")
                                        >
                                            "Сервисы (" {data.services.len()} ")"
                                        </button>
                                        <button
                                            class="detail-tab"
                                            class:active=move || active_tab.get() == "projections"
                                            on:click=move |_| set_active_tab.set("projections")
                                        >
                                            {move || {
                                                let count = projections.get().as_ref().map(|p| {
                                                    let p900_len = p["p900_sales_register"].as_array().map(|a| a.len()).unwrap_or(0);
                                                    let p902_len = p["p902_ozon_finance"].as_array().map(|a| a.len()).unwrap_or(0);
                                                    let p904_len = p["p904_sales_data"].as_array().map(|a| a.len()).unwrap_or(0);
                                                    p900_len + p902_len + p904_len
                                                }).unwrap_or(0);
                                                format!("Проекции ({})", count)
                                            }}
                                        </button>
                                    </div>

                                    <div style="padding-top: var(--space-lg);">
                                        {move || {
                                            let data = transaction_data.get().unwrap();
                                            let tab = active_tab.get();
                                            match tab.as_ref() {
                                                "general" => view! {
                                                    <div style="display: flex; flex-direction: column; gap: var(--space-lg);">
                                                        <div style="background: var(--color-bg-white); padding: var(--space-lg); border-radius: var(--radius-md); box-shadow: var(--shadow-sm);">
                                                            <h3 style="margin: 0 0 var(--space-md) 0; color: var(--color-text-primary); font-size: var(--font-size-base); font-weight: var(--font-weight-semibold); border-bottom: 2px solid var(--color-primary); padding-bottom: var(--space-sm);">"Заголовок транзакции"</h3>
                                                            <div style="display: grid; grid-template-columns: 400px 1fr; gap: var(--space-md) var(--space-lg); align-items: center;">
                                                                <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary); font-size: var(--font-size-sm);">"Operation ID:"</div>
                                                                <div class="field-value-nowrap">{data.header.operation_id}</div>

                                                                <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary); font-size: var(--font-size-sm);">
                                                                    "Тип операции "
                                                                    <span class="field-label-tech">"/ operation_type_name"</span>
                                                                    ":"
                                                                </div>
                                                                <div class="field-value">
                                                                    <span style="padding: var(--space-2xs) var(--space-sm); background: var(--color-info-bg); color: var(--color-info); border-radius: var(--radius-xs); font-weight: var(--font-weight-medium);">
                                                                        {data.header.operation_type_name.clone()}
                                                                    </span>
                                                                </div>

                                                                <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary); font-size: var(--font-size-sm);">
                                                                    "Дата операции "
                                                                    <span class="field-label-tech">"/ operation_date"</span>
                                                                    ":"
                                                                </div>
                                                                <div class="field-value-nowrap">{format_datetime(&data.header.operation_date)}</div>

                                                                <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary); font-size: var(--font-size-sm);">
                                                                    "Тип транзакции "
                                                                    <span class="field-label-tech">"/ transaction_type"</span>
                                                                    ":"
                                                                </div>
                                                                <div class="field-value-nowrap">{data.header.transaction_type.clone()}</div>

                                                                <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary); font-size: var(--font-size-sm);">
                                                                    "Сумма "
                                                                    <span class="field-label-tech">"/ amount"</span>
                                                                    ":"
                                                                </div>
                                                                <div class="field-value">
                                                                    <span style=move || format!(
                                                                        "font-weight: var(--font-weight-semibold); {}",
                                                                        if data.header.amount >= 0.0 {
                                                                            "color: var(--color-success);"
                                                                        } else {
                                                                            "color: var(--color-error);"
                                                                        }
                                                                    )>
                                                                        {format!("{:.2} ₽", data.header.amount)}
                                                                    </span>
                                                                </div>

                                                                <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary); font-size: var(--font-size-sm);">
                                                                    "Начисления за продажу "
                                                                    <span class="field-label-tech">"/ accruals_for_sale"</span>
                                                                    ":"
                                                                </div>
                                                                <div class="field-value">{format!("{:.2} ₽", data.header.accruals_for_sale)}</div>

                                                                <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary); font-size: var(--font-size-sm);">
                                                                    "Комиссия за продажу "
                                                                    <span class="field-label-tech">"/ sale_commission"</span>
                                                                    ":"
                                                                </div>
                                                                <div class="field-value">{format!("{:.2} ₽", data.header.sale_commission)}</div>

                                                                <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary); font-size: var(--font-size-sm);">
                                                                    "Стоимость доставки "
                                                                    <span class="field-label-tech">"/ delivery_charge"</span>
                                                                    ":"
                                                                </div>
                                                                <div class="field-value">{format!("{:.2} ₽", data.header.delivery_charge)}</div>
                                                            </div>
                                                        </div>

                                                        <div style="background: var(--color-bg-white); padding: var(--space-lg); border-radius: var(--radius-md); box-shadow: var(--shadow-sm);">
                                                            <h3 style="margin: 0 0 var(--space-md) 0; color: var(--color-text-primary); font-size: var(--font-size-base); font-weight: var(--font-weight-semibold); border-bottom: 2px solid var(--color-primary); padding-bottom: var(--space-sm);">"Информация о постинге"</h3>
                                                            <div style="display: grid; grid-template-columns: 400px 1fr; gap: var(--space-md) var(--space-lg); align-items: center;">
                                                                <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary); font-size: var(--font-size-sm);">"Posting Number:"</div>
                                                                <div class="field-value-nowrap">
                                                                    <span style="color: var(--color-primary); font-weight: var(--font-weight-medium);">{data.posting.posting_number.clone()}</span>
                                                                </div>

                                                                <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary); font-size: var(--font-size-sm);">
                                                                    "Схема доставки "
                                                                    <span class="field-label-tech">"/ delivery_schema"</span>
                                                                    ":"
                                                                </div>
                                                                <div class="field-value-nowrap">{data.posting.delivery_schema.clone()}</div>

                                                                <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary); font-size: var(--font-size-sm);">
                                                                    "Дата заказа "
                                                                    <span class="field-label-tech">"/ order_date"</span>
                                                                    ":"
                                                                </div>
                                                                <div class="field-value-nowrap">{format_datetime(&data.posting.order_date)}</div>

                                                                <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary); font-size: var(--font-size-sm);">"Warehouse ID:"</div>
                                                                <div class="field-value-nowrap">{data.posting.warehouse_id}</div>

                                                                <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary); font-size: var(--font-size-sm);">"Документ отгрузки:"</div>
                                                                <div class="field-value-nowrap">
                                                                    {move || {
                                                                        let data = transaction_data.get().unwrap();
                                                                        if let (Some(posting_ref), Some(posting_ref_type)) = (&data.posting_ref, &data.posting_ref_type) {
                                                                            let ref_clone = posting_ref.clone();
                                                                            let type_clone = posting_ref_type.clone();
                                                                            view! {
                                                                                <a
                                                                                    href="#"
                                                                                    on:click=move |ev| {
                                                                                        ev.prevent_default();
                                                                                        set_selected_posting.set(Some((type_clone.clone(), ref_clone.clone())));
                                                                                    }
                                                                                    style="color: var(--color-primary); text-decoration: underline; cursor: pointer; font-weight: var(--font-weight-medium);"
                                                                                >
                                                                                    {format!("{} {}", posting_ref_type, data.posting.posting_number.clone())}
                                                                                </a>
                                                                            }.into_any()
                                                                        } else if data.is_posted {
                                                                            view! {
                                                                                <span style="color: var(--color-error); font-weight: var(--font-weight-semibold);">
                                                                                    "⚠️ Документ отгрузки не найден"
                                                                                </span>
                                                                            }.into_any()
                                                                        } else {
                                                                            view! {
                                                                                <span style="color: var(--color-text-tertiary);">
                                                                                    "—"
                                                                                </span>
                                                                            }.into_any()
                                                                        }
                                                                    }}
                                                                </div>
                                                            </div>
                                                        </div>
                                                    </div>
                                                }.into_any(),
                                                "items" => view! {
                                                    <div style="background: var(--color-bg-white); padding: var(--space-lg); border-radius: var(--radius-md); box-shadow: var(--shadow-sm);">
                                                        <h3 style="margin: 0 0 var(--space-md) 0; color: var(--color-text-primary); font-size: var(--font-size-base); font-weight: var(--font-weight-semibold); border-bottom: 2px solid var(--color-primary); padding-bottom: var(--space-sm);">"Товары"</h3>
                                                        {if data.items.is_empty() {
                                                            view! {
                                                                <p style="text-align: center; padding: var(--space-2xl); color: var(--color-text-tertiary); font-size: var(--font-size-sm);">"Нет товаров"</p>
                                                            }.into_any()
                                                        } else {
                                                            view! {
                                                                <table style="width: 100%; border-collapse: collapse; font-size: var(--font-size-sm);">
                                                                    <thead>
                                                                        <tr style="background: var(--color-bg-secondary);">
                                                                            <th style="padding: var(--space-sm); text-align: left; font-weight: var(--font-weight-semibold); border-bottom: 2px solid var(--color-border); color: var(--color-text-secondary);">"SKU"</th>
                                                                            <th style="padding: var(--space-sm); text-align: left; font-weight: var(--font-weight-semibold); border-bottom: 2px solid var(--color-border); color: var(--color-text-secondary);">"Название"</th>
                                                                            <th style="padding: var(--space-sm); text-align: right; font-weight: var(--font-weight-semibold); border-bottom: 2px solid var(--color-border); color: var(--color-text-secondary);">"Цена"</th>
                                                                            <th style="padding: var(--space-sm); text-align: right; font-weight: var(--font-weight-semibold); border-bottom: 2px solid var(--color-border); color: var(--color-text-secondary);">"Пропорция"</th>
                                                                            <th style="padding: var(--space-sm); text-align: left; font-weight: var(--font-weight-semibold); border-bottom: 2px solid var(--color-border); color: var(--color-text-secondary);">"Продукт MP"</th>
                                                                            <th style="padding: var(--space-sm); text-align: left; font-weight: var(--font-weight-semibold); border-bottom: 2px solid var(--color-border); color: var(--color-text-secondary);">"Номенклатура"</th>
                                                                        </tr>
                                                                    </thead>
                                                                    <tbody>
                                                                        {data.items.iter().map(|item| view! {
                                                                            <tr style="border-bottom: 1px solid var(--color-border-light);">
                                                                                <td class="field-value-mono" style="padding: var(--space-sm); color: var(--color-text-primary);">{item.sku}</td>
                                                                                <td style="padding: var(--space-sm); color: var(--color-text-primary);">{item.name.clone()}</td>
                                                                                <td style="padding: var(--space-sm); text-align: right; color: var(--color-text-primary);">
                                                                                    {item.price.map(|p| format!("{:.2} ₽", p)).unwrap_or("—".to_string())}
                                                                                </td>
                                                                                <td style="padding: var(--space-sm); text-align: right; color: var(--color-text-secondary);">
                                                                                    {item.ratio.map(|r| format!("{:.1}%", r * 100.0)).unwrap_or("—".to_string())}
                                                                                </td>
                                                                                <td class="field-value-mono-sm" style="padding: var(--space-sm);" title={item.marketplace_product_ref.clone().unwrap_or_default()}>
                                                                                    {item.marketplace_product_ref.as_ref().map(|r| r.clone()).unwrap_or("—".to_string())}
                                                                                </td>
                                                                                <td class="field-value-mono-sm" style="padding: var(--space-sm);" title={item.nomenclature_ref.clone().unwrap_or_default()}>
                                                                                    {item.nomenclature_ref.as_ref().map(|r| r.clone()).unwrap_or("—".to_string())}
                                                                                </td>
                                                                            </tr>
                                                                        }).collect::<Vec<_>>()}
                                                                    </tbody>
                                                                </table>
                                                            }.into_any()
                                                        }}
                                                    </div>
                                                }.into_any(),
                                                "services" => view! {
                                                    <div style="background: var(--color-bg-white); padding: var(--space-lg); border-radius: var(--radius-md); box-shadow: var(--shadow-sm);">
                                                        <h3 style="margin: 0 0 var(--space-md) 0; color: var(--color-text-primary); font-size: var(--font-size-base); font-weight: var(--font-weight-semibold); border-bottom: 2px solid var(--color-success); padding-bottom: var(--space-sm);">"Сервисы"</h3>
                                                        {if data.services.is_empty() {
                                                            view! {
                                                                <p style="text-align: center; padding: var(--space-2xl); color: var(--color-text-tertiary); font-size: var(--font-size-sm);">"Нет сервисов"</p>
                                                            }.into_any()
                                                        } else {
                                                            view! {
                                                                <table style="width: 100%; border-collapse: collapse; font-size: var(--font-size-sm);">
                                                                    <thead>
                                                                        <tr style="background: var(--color-bg-secondary);">
                                                                            <th style="padding: var(--space-sm); text-align: left; font-weight: var(--font-weight-semibold); border-bottom: 2px solid var(--color-border); color: var(--color-text-secondary);">"Название"</th>
                                                                            <th style="padding: var(--space-sm); text-align: right; font-weight: var(--font-weight-semibold); border-bottom: 2px solid var(--color-border); color: var(--color-text-secondary);">"Цена"</th>
                                                                        </tr>
                                                                    </thead>
                                                                    <tbody>
                                                                        {data.services.iter().map(|service| view! {
                                                                            <tr style="border-bottom: 1px solid var(--color-border-light);">
                                                                                <td style="padding: var(--space-sm); color: var(--color-text-primary);">{service.name.clone()}</td>
                                                                                <td style="padding: var(--space-sm); text-align: right; font-weight: var(--font-weight-semibold); color: var(--color-success);">{format!("{:.2} ₽", service.price)}</td>
                                                                            </tr>
                                                                        }).collect::<Vec<_>>()}
                                                                    </tbody>
                                                                </table>
                                                            }.into_any()
                                                        }}
                                                    </div>
                                                }.into_any(),
                                                "projections" => view! {
                                                    <div class="projections-info">
                                                        {move || {
                                                            if projections_loading.get() {
                                                                view! {
                                                                    <div style="padding: var(--space-lg); text-align: center; color: var(--color-text-tertiary); font-size: var(--font-size-sm);">
                                                                        "Загрузка проекций..."
                                                                    </div>
                                                                }.into_any()
                                                            } else if let Some(proj_data) = projections.get() {
                                                                let p900_items = proj_data["p900_sales_register"].as_array().cloned().unwrap_or_default();
                                                                let p902_items = proj_data["p902_ozon_finance"].as_array().cloned().unwrap_or_default();
                                                                let p904_items = proj_data["p904_sales_data"].as_array().cloned().unwrap_or_default();

                                                                view! {
                                                                    <div style="display: flex; flex-direction: column; gap: var(--space-sm);">
                                                                        // P900 Sales Register
                                                                        <div style="background: var(--color-bg-white); padding: var(--space-sm); border-radius: var(--radius-md); box-shadow: var(--shadow-sm);">
                                                                            <h3 style="margin: 0 0 var(--space-sm) 0; color: var(--color-text-primary); font-size: var(--font-size-base); font-weight: var(--font-weight-semibold); border-bottom: 2px solid var(--color-warning); padding-bottom: var(--space-xs);">
                                                                                {format!("📊 Sales Register (p900) - {} записей", p900_items.len())}
                                                                            </h3>
                                                                            {if !p900_items.is_empty() {
                                                                                view! {
                                                                                    <div style="overflow-x: auto;">
                                                                                        <table style="width: 100%; border-collapse: collapse; font-size: var(--font-size-sm);">
                                                                                            <thead>
                                                                                                <tr style="background: var(--color-bg-secondary);">
                                                                                                    <th style="padding: var(--space-2xs) var(--space-xs); text-align: left; border: 1px solid var(--color-border);">"MP"</th>
                                                                                                    <th style="padding: var(--space-2xs) var(--space-xs); text-align: left; border: 1px solid var(--color-border);">"SKU"</th>
                                                                                                    <th style="padding: var(--space-2xs) var(--space-xs); text-align: left; border: 1px solid var(--color-border);">"Title"</th>
                                                                                                    <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">"Qty"</th>
                                                                                                    <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">"Amount"</th>
                                                                                                </tr>
                                                                                            </thead>
                                                                                            <tbody>
                                                                                                {p900_items.iter().map(|item| {
                                                                                                    let mp = item["marketplace"].as_str().unwrap_or("—");
                                                                                                    let sku = item["seller_sku"].as_str().unwrap_or("—");
                                                                                                    let title = item["title"].as_str().unwrap_or("—");
                                                                                                    let qty = item["qty"].as_f64().unwrap_or(0.0);
                                                                                                    let amount = item["amount_line"].as_f64().unwrap_or(0.0);

                                                                                                    view! {
                                                                                                        <tr style="border-bottom: 1px solid var(--color-border-light);">
                                                                                                            <td style="padding: var(--space-2xs) var(--space-xs); border: 1px solid var(--color-border);">{mp}</td>
                                                                                                            <td class="field-value-mono" style="padding: var(--space-2xs) var(--space-xs); border: 1px solid var(--color-border);">{sku}</td>
                                                                                                            <td style="padding: var(--space-2xs) var(--space-xs); border: 1px solid var(--color-border);">{title}</td>
                                                                                                            <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">{qty}</td>
                                                                                                            <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border); font-weight: var(--font-weight-semibold);">{format!("{:.2}", amount)}</td>
                                                                                                        </tr>
                                                                                                    }
                                                                                                }).collect::<Vec<_>>()}
                                                                                            </tbody>
                                                                                        </table>
                                                                                    </div>
                                                                                }.into_any()
                                                                            } else {
                                                                                view! {
                                                                                    <p style="text-align: center; padding: var(--space-sm); color: var(--color-text-tertiary); font-size: var(--font-size-sm);">"Нет записей"</p>
                                                                                }.into_any()
                                                                            }}
                                                                        </div>

                                                                        // P902 OZON Finance
                                                                        <div style="background: var(--color-bg-white); padding: var(--space-sm); border-radius: var(--radius-md); box-shadow: var(--shadow-sm);">
                                                                            <h3 style="margin: 0 0 var(--space-sm) 0; color: var(--color-text-primary); font-size: var(--font-size-base); font-weight: var(--font-weight-semibold); border-bottom: 2px solid var(--color-success); padding-bottom: var(--space-xs);">
                                                                                {format!("💰 OZON Finance (p902) - {} записей", p902_items.len())}
                                                                            </h3>
                                                                            {if !p902_items.is_empty() {
                                                                                view! {
                                                                                    <div style="overflow-x: auto;">
                                                                                        <table style="width: 100%; border-collapse: collapse; font-size: var(--font-size-sm);">
                                                                                            <thead>
                                                                                                <tr style="background: var(--color-bg-secondary);">
                                                                                                    <th style="padding: var(--space-2xs) var(--space-xs); text-align: left; border: 1px solid var(--color-border);">"Posting"</th>
                                                                                                    <th style="padding: var(--space-2xs) var(--space-xs); text-align: left; border: 1px solid var(--color-border);">"SKU"</th>
                                                                                                    <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">"Qty"</th>
                                                                                                    <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">"Amount"</th>
                                                                                                </tr>
                                                                                            </thead>
                                                                                            <tbody>
                                                                                                {p902_items.iter().map(|item| {
                                                                                                    let posting = item["posting_number"].as_str().unwrap_or("—");
                                                                                                    let sku = item["sku"].as_str().unwrap_or("—");
                                                                                                    let qty = item["quantity"].as_f64().unwrap_or(0.0);
                                                                                                    let amount = item["amount"].as_f64().unwrap_or(0.0);

                                                                                                    view! {
                                                                                                        <tr style="border-bottom: 1px solid var(--color-border-light);">
                                                                                                            <td class="field-value-mono" style="padding: var(--space-2xs) var(--space-xs); border: 1px solid var(--color-border);">{posting}</td>
                                                                                                            <td class="field-value-mono" style="padding: var(--space-2xs) var(--space-xs); border: 1px solid var(--color-border);">{sku}</td>
                                                                                                            <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">{qty}</td>
                                                                                                            <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border); font-weight: var(--font-weight-semibold);">{format!("{:.2}", amount)}</td>
                                                                                                        </tr>
                                                                                                    }
                                                                                                }).collect::<Vec<_>>()}
                                                                                            </tbody>
                                                                                        </table>
                                                                                    </div>
                                                                                }.into_any()
                                                                            } else {
                                                                                view! {
                                                                                    <p style="text-align: center; padding: var(--space-sm); color: var(--color-text-tertiary); font-size: var(--font-size-sm);">"Нет записей"</p>
                                                                                }.into_any()
                                                                            }}
                                                                        </div>

                                                                        // P904 Sales Data
                                                                        <div style="background: var(--color-bg-white); padding: var(--space-sm); border-radius: var(--radius-md); box-shadow: var(--shadow-sm);">
                                                                            <h3 style="margin: 0 0 var(--space-sm) 0; color: var(--color-text-primary); font-size: var(--font-size-base); font-weight: var(--font-weight-semibold); border-bottom: 2px solid var(--color-primary); padding-bottom: var(--space-xs);">
                                                                                {format!("📈 Sales Data (p904) - {} записей", p904_items.len())}
                                                                            </h3>
                                                                            {if !p904_items.is_empty() {
                                                                                view! {
                                                                                    <div style="overflow-x: auto;">
                                                                                        <table style="width: 100%; border-collapse: collapse; font-size: var(--font-size-sm);">
                                                                                            <thead>
                                                                                                <tr style="background: var(--color-bg-secondary);">
                                                                                                    <th style="padding: var(--space-2xs) var(--space-xs); text-align: left; border: 1px solid var(--color-border);">"Date"</th>
                                                                                                    <th style="padding: var(--space-2xs) var(--space-xs); text-align: left; border: 1px solid var(--color-border);">"Doc No"</th>
                                                                                                    <th style="padding: var(--space-2xs) var(--space-xs); text-align: left; border: 1px solid var(--color-border);">"Article"</th>
                                                                                                    <th style="padding: var(--space-2xs) var(--space-xs); text-align: left; border: 1px solid var(--color-border);">"Cabinet"</th>
                                                                                                    <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">"Cust In"</th>
                                                                                                    <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">"Cust Out"</th>
                                                                                                    <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">"Coinv In"</th>
                                                                                                    <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">"Comm Out"</th>
                                                                                                    <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">"Acq Out"</th>
                                                                                                    <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">"Pen Out"</th>
                                                                                                    <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">"Log Out"</th>
                                                                                                    <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">"Sell Out"</th>
                                                                                                    <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">"Price Full"</th>
                                                                                                    <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">"Price List"</th>
                                                                                                    <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">"Price Ret"</th>
                                                                                                    <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">"Comm %"</th>
                                                                                                    <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">"Coinv %"</th>
                                                                                                    <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">"Total"</th>
                                                                                                </tr>
                                                                                            </thead>
                                                                                            <tbody>
                                                                                                {p904_items.iter().map(|item| {
                                                                                                    let date = item["date"].as_str().unwrap_or("—");
                                                                                                    let document_no = item["document_no"].as_str().unwrap_or("—");
                                                                                                    let article = item["article"].as_str().unwrap_or("—");
                                                                                                    let connection_mp_ref = item["connection_mp_ref"].as_str().unwrap_or("—");
                                                                                                    let customer_in = item["customer_in"].as_f64().unwrap_or(0.0);
                                                                                                    let customer_out = item["customer_out"].as_f64().unwrap_or(0.0);
                                                                                                    let coinvest_in = item["coinvest_in"].as_f64().unwrap_or(0.0);
                                                                                                    let commission_out = item["commission_out"].as_f64().unwrap_or(0.0);
                                                                                                    let acquiring_out = item["acquiring_out"].as_f64().unwrap_or(0.0);
                                                                                                    let penalty_out = item["penalty_out"].as_f64().unwrap_or(0.0);
                                                                                                    let logistics_out = item["logistics_out"].as_f64().unwrap_or(0.0);
                                                                                                    let seller_out = item["seller_out"].as_f64().unwrap_or(0.0);
                                                                                                    let price_full = item["price_full"].as_f64().unwrap_or(0.0);
                                                                                                    let price_list = item["price_list"].as_f64().unwrap_or(0.0);
                                                                                                    let price_return = item["price_return"].as_f64().unwrap_or(0.0);
                                                                                                    let commission_percent = item["commission_percent"].as_f64().unwrap_or(0.0);
                                                                                                    let coinvest_persent = item["coinvest_persent"].as_f64().unwrap_or(0.0);
                                                                                                    let total = item["total"].as_f64().unwrap_or(0.0);

                                                                                                    view! {
                                                                                                        <tr style="border-bottom: 1px solid var(--color-border-light);">
                                                                                                            <td style="padding: var(--space-2xs) var(--space-xs); border: 1px solid var(--color-border); white-space: nowrap;">{date}</td>
                                                                                                            <td class="field-value-mono" style="padding: var(--space-2xs) var(--space-xs); border: 1px solid var(--color-border);">{document_no}</td>
                                                                                                            <td class="field-value-mono" style="padding: var(--space-2xs) var(--space-xs); border: 1px solid var(--color-border);">{article}</td>
                                                                                                            <td style="padding: var(--space-2xs) var(--space-xs); border: 1px solid var(--color-border);">{connection_mp_ref}</td>
                                                                                                            <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">{format!("{:.2}", customer_in)}</td>
                                                                                                            <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">{format!("{:.2}", customer_out)}</td>
                                                                                                            <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">{format!("{:.2}", coinvest_in)}</td>
                                                                                                            <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">{format!("{:.2}", commission_out)}</td>
                                                                                                            <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">{format!("{:.2}", acquiring_out)}</td>
                                                                                                            <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">{format!("{:.2}", penalty_out)}</td>
                                                                                                            <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">{format!("{:.2}", logistics_out)}</td>
                                                                                                            <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">{format!("{:.2}", seller_out)}</td>
                                                                                                            <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">{format!("{:.2}", price_full)}</td>
                                                                                                            <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">{format!("{:.2}", price_list)}</td>
                                                                                                            <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">{format!("{:.2}", price_return)}</td>
                                                                                                            <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">{format!("{:.2}", commission_percent)}</td>
                                                                                                            <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">{format!("{:.2}", coinvest_persent)}</td>
                                                                                                            <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border); font-weight: var(--font-weight-semibold);">{format!("{:.2}", total)}</td>
                                                                                                        </tr>
                                                                                                    }
                                                                                                }).collect::<Vec<_>>()}
                                                                                            </tbody>
                                                                                        </table>
                                                                                    </div>
                                                                                }.into_any()
                                                                            } else {
                                                                                view! {
                                                                                    <p style="text-align: center; padding: var(--space-sm); color: var(--color-text-tertiary); font-size: var(--font-size-sm);">"Нет записей"</p>
                                                                                }.into_any()
                                                                            }}
                                                                        </div>
                                                                    </div>
                                                                }.into_any()
                                                            } else {
                                                                view! {
                                                                    <div style="padding: var(--space-lg); text-align: center; color: var(--color-text-tertiary); font-size: var(--font-size-sm);">
                                                                        "Нет данных проекций"
                                                                    </div>
                                                                }.into_any()
                                                            }
                                                        }}
                                                    </div>
                                                }.into_any(),
                                                _ => view! {
                                                    <div style="font-size: var(--font-size-sm);">"Unknown tab"</div>
                                                }.into_any()
                                            }
                                        }}
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div style="font-size: var(--font-size-sm);">"Нет данных"</div>
                            }.into_any()
                        }
                    }}
                </div>

            // Modal for posting document details (A010 or A011)
            {move || {
                if let Some((posting_type, posting_id)) = selected_posting.get() {
                    if posting_type == "A010" {
                        view! {
                            <div style="position: fixed; top: 0; left: 0; width: 100%; height: 100%; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 2000;">
                                <div style="background: var(--color-bg-white); border-radius: var(--radius-md); box-shadow: var(--shadow-lg); width: 90%; max-width: 1400px; max-height: 90vh; overflow: hidden;">
                                    <OzonFbsPostingDetail
                                        id=posting_id
                                        on_close=move || set_selected_posting.set(None)
                                    />
                                </div>
                            </div>
                        }.into_any()
                    } else if posting_type == "A011" {
                        view! {
                            <div style="position: fixed; top: 0; left: 0; width: 100%; height: 100%; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 2000;">
                                <div style="background: var(--color-bg-white); border-radius: var(--radius-md); box-shadow: var(--shadow-lg); width: 90%; max-width: 1400px; max-height: 90vh; overflow: hidden;">
                                    <OzonFboPostingDetail
                                        id=posting_id
                                        on_close=move || set_selected_posting.set(None)
                                    />
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }
                } else {
                    view! { <div></div> }.into_any()
                }
            }}
        </div>
    }
}
