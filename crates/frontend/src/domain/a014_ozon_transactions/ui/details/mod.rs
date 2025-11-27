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
                                        let set_projections_loading = set_projections_loading.clone();
                                        wasm_bindgen_futures::spawn_local(async move {
                                            set_projections_loading.set(true);
                                            let projections_url = format!("http://localhost:3000/api/a014/ozon-transactions/{}/projections", transaction_id);
                                            match Request::get(&projections_url).send().await {
                                                Ok(resp) => {
                                                    if resp.status() == 200 {
                                                        if let Ok(text) = resp.text().await {
                                                            if let Ok(proj_data) = serde_json::from_str::<serde_json::Value>(&text) {
                                                                set_projections.set(Some(proj_data));
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
        <div class="transaction-detail" style="padding: var(--space-xl); height: 100%; display: flex; flex-direction: column; background: var(--color-hover-table); border-radius: var(--radius-lg); box-shadow: var(--shadow-sm);">
            <div style="background: linear-gradient(135deg, #4a5568 0%, #2d3748 100%); padding: var(--space-md) var(--space-xl); border-radius: var(--radius-md) var(--radius-md) 0 0; margin: calc(-1 * var(--space-xl)) calc(-1 * var(--space-xl)) 0 calc(-1 * var(--space-xl)); display: flex; align-items: center; justify-content: space-between; flex-shrink: 0;">
                    <div style="display: flex; align-items: center; gap: var(--space-xl);">
                        <h2 style="margin: 0; font-size: var(--font-size-xl); font-weight: var(--font-weight-semibold); color: var(--color-text-white); letter-spacing: 0.5px;">"💳 Детали транзакции OZON"</h2>
                        <Show when=move || transaction_data.get().is_some()>
                            {move || {
                                let posted = is_posted.get();
                                view! {
                                    <div style=move || format!(
                                        "display: flex; align-items: center; gap: var(--space-xs); padding: 3px var(--space-md); border-radius: var(--radius-sm); font-size: var(--font-size-xs); font-weight: var(--font-weight-semibold); {}",
                                        if posted {
                                            "background: rgba(255,255,255,0.2); color: var(--color-success); border: 1px solid rgba(76,175,80,0.5);"
                                        } else {
                                            "background: rgba(255,255,255,0.2); color: var(--color-warning); border: 1px solid rgba(255,152,0,0.5);"
                                        }
                                    )>
                                        <span style="font-size: var(--font-size-sm);">{if posted { "✓" } else { "○" }}</span>
                                        <span>{if posted { "Проведен" } else { "Не проведен" }}</span>
                                    </div>
                                }
                            }}
                        </Show>
                    </div>
                    <div style="display: flex; gap: var(--space-md);">
                        <Show when=move || transaction_data.get().is_some()>
                            <Show
                                when=move || !is_posted.get()
                                fallback=move || {
                                    view! {
                                        <button
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
                                            disabled=move || posting.get()
                                            style="height: var(--header-height); padding: 0 var(--space-3xl); background: var(--color-warning); color: var(--color-text-white); border: none; border-radius: var(--radius-sm); cursor: pointer; font-size: var(--font-size-sm); font-weight: var(--font-weight-medium); transition: var(--transition-fast);"
                                        >
                                            {move || if posting.get() { "Отмена проведения..." } else { "✗ Отменить проведение" }}
                                        </button>
                                    }
                                }
                            >
                                {
                                    view! {
                                        <button
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
                                            disabled=move || posting.get()
                                            style="height: var(--header-height); padding: 0 var(--space-3xl); background: var(--color-success); color: var(--color-text-white); border: none; border-radius: var(--radius-sm); cursor: pointer; font-size: var(--font-size-sm); font-weight: var(--font-weight-medium); transition: var(--transition-fast);"
                                        >
                                            {move || if posting.get() { "Проведение..." } else { "✓ Провести" }}
                                        </button>
                                    }
                                }
                            </Show>
                        </Show>
                        <button
                            on:click=move |_| on_close.run(())
                            style="height: var(--header-height); padding: 0 var(--space-3xl); background: var(--color-danger); color: var(--color-text-white); border: none; border-radius: var(--radius-sm); cursor: pointer; font-size: var(--font-size-sm); font-weight: var(--font-weight-medium); transition: var(--transition-fast);"
                        >
                            "✕ Закрыть"
                        </button>
                    </div>
                </div>

                <div style="flex: 1; overflow-y: auto; min-height: 0; max-height: calc(90vh - 120px);">
                    {move || {
                        if loading.get() {
                            view! {
                                <div style="text-align: center; padding: 40px;">
                                    <p>"Загрузка..."</p>
                                </div>
                            }.into_any()
                        } else if let Some(err) = error.get() {
                            view! {
                                <div style="padding: 20px; background: #ffebee; border: 1px solid #ffcdd2; border-radius: 4px; color: #c62828; margin: 20px;">
                                    <strong>"Ошибка: "</strong>{err}
                                </div>
                            }.into_any()
                        } else if let Some(data) = transaction_data.get() {
                            view! {
                                <div style="height: 100%; display: flex; flex-direction: column;">
                                    <div class="tabs" style="border-bottom: 2px solid #ddd; margin-bottom: 20px; flex-shrink: 0; background: white; position: sticky; top: 0; z-index: 10;">
                                        <button
                                            on:click=move |_| set_active_tab.set("general")
                                            style=move || format!(
                                                "padding: 10px 20px; border: none; border-radius: 4px 4px 0 0; cursor: pointer; margin-right: 5px; font-weight: 500; {}",
                                                if active_tab.get() == "general" {
                                                    "background: #2196F3; color: white; border-bottom: 2px solid #2196F3;"
                                                } else {
                                                    "background: #f5f5f5; color: #666;"
                                                }
                                            )
                                        >
                                            "Общие данные"
                                        </button>
                                        <button
                                            on:click=move |_| set_active_tab.set("items")
                                            style=move || format!(
                                                "padding: 10px 20px; border: none; border-radius: 4px 4px 0 0; cursor: pointer; margin-right: 5px; font-weight: 500; {}",
                                                if active_tab.get() == "items" {
                                                    "background: #2196F3; color: white; border-bottom: 2px solid #2196F3;"
                                                } else {
                                                    "background: #f5f5f5; color: #666;"
                                                }
                                            )
                                        >
                                            "Товары (" {data.items.len()} ")"
                                        </button>
                                        <button
                                            on:click=move |_| set_active_tab.set("services")
                                            style=move || format!(
                                                "padding: 10px 20px; border: none; border-radius: 4px 4px 0 0; cursor: pointer; margin-right: 5px; font-weight: 500; {}",
                                                if active_tab.get() == "services" {
                                                    "background: #2196F3; color: white; border-bottom: 2px solid #2196F3;"
                                                } else {
                                                    "background: #f5f5f5; color: #666;"
                                                }
                                            )
                                        >
                                            "Сервисы (" {data.services.len()} ")"
                                        </button>
                                        <button
                                            on:click=move |_| set_active_tab.set("projections")
                                            style=move || format!(
                                                "padding: 10px 20px; border: none; border-radius: 4px 4px 0 0; cursor: pointer; margin-right: 5px; font-weight: 500; {}",
                                                if active_tab.get() == "projections" {
                                                    "background: #2196F3; color: white; border-bottom: 2px solid #2196F3;"
                                                } else {
                                                    "background: #f5f5f5; color: #666;"
                                                }
                                            )
                                        >
                                            {move || {
                                                let count = projections.get().as_ref().map(|p| {
                                                    let p900_len = p["p900_sales_register"].as_array().map(|a| a.len()).unwrap_or(0);
                                                    let p902_len = p["p902_ozon_finance"].as_array().map(|a| a.len()).unwrap_or(0);
                                                    let p904_len = p["p904_sales_data"].as_array().map(|a| a.len()).unwrap_or(0);
                                                    p900_len + p902_len + p904_len
                                                }).unwrap_or(0);
                                                format!("📊 Проекции ({})", count)
                                            }}
                                        </button>
                                    </div>

                                    <div style="flex: 1; overflow-y: auto; padding: 20px; background: #fafafa;">
                                        {move || {
                                            let data = transaction_data.get().unwrap();
                                            let tab = active_tab.get();
                                            match tab.as_ref() {
                                                "general" => view! {
                                                    <div style="display: flex; flex-direction: column; gap: 20px;">
                                                        <div style="background: white; padding: 20px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
                                                            <h3 style="margin: 0 0 15px 0; color: #333; font-size: 16px; font-weight: 600; border-bottom: 2px solid #2196F3; padding-bottom: 8px;">"Заголовок транзакции"</h3>
                                                            <div style="display: grid; grid-template-columns: 200px 1fr; gap: 15px 20px; align-items: center;">
                                                                <div style="font-weight: 600; color: #555;">"Operation ID:"</div>
                                                                <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{data.header.operation_id}</div>

                                                                <div style="font-weight: 600; color: #555;">"Тип операции:"</div>
                                                                <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">
                                                                    <span style="padding: 2px 8px; background: #e3f2fd; color: #1976d2; border-radius: 3px; font-weight: 500;">
                                                                        {data.header.operation_type_name.clone()}
                                                                    </span>
                                                                </div>

                                                                <div style="font-weight: 600; color: #555;">"Дата операции:"</div>
                                                                <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{format_datetime(&data.header.operation_date)}</div>

                                                                <div style="font-weight: 600; color: #555;">"Тип транзакции:"</div>
                                                                <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{data.header.transaction_type.clone()}</div>

                                                                <div style="font-weight: 600; color: #555;">"Сумма:"</div>
                                                                <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 18px;">
                                                                    <span style=move || format!(
                                                                        "font-weight: 600; {}",
                                                                        if data.header.amount >= 0.0 {
                                                                            "color: #4caf50;"
                                                                        } else {
                                                                            "color: #f44336;"
                                                                        }
                                                                    )>
                                                                        {format!("{:.2} ₽", data.header.amount)}
                                                                    </span>
                                                                </div>

                                                                <div style="font-weight: 600; color: #555;">"Начисления за продажу:"</div>
                                                                <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{format!("{:.2} ₽", data.header.accruals_for_sale)}</div>

                                                                <div style="font-weight: 600; color: #555;">"Комиссия за продажу:"</div>
                                                                <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{format!("{:.2} ₽", data.header.sale_commission)}</div>

                                                                <div style="font-weight: 600; color: #555;">"Стоимость доставки:"</div>
                                                                <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{format!("{:.2} ₽", data.header.delivery_charge)}</div>
                                                            </div>
                                                        </div>

                                                        <div style="background: white; padding: 20px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
                                                            <h3 style="margin: 0 0 15px 0; color: #333; font-size: 16px; font-weight: 600; border-bottom: 2px solid #2196F3; padding-bottom: 8px;">"Информация о постинге"</h3>
                                                            <div style="display: grid; grid-template-columns: 200px 1fr; gap: 15px 20px; align-items: center;">
                                                                <div style="font-weight: 600; color: #555;">"Posting Number:"</div>
                                                                <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">
                                                                    <span style="color: #2196F3; font-weight: 500;">{data.posting.posting_number.clone()}</span>
                                                                </div>

                                                                <div style="font-weight: 600; color: #555;">"Схема доставки:"</div>
                                                                <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{data.posting.delivery_schema.clone()}</div>

                                                                <div style="font-weight: 600; color: #555;">"Дата заказа:"</div>
                                                                <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{format_datetime(&data.posting.order_date)}</div>

                                                                <div style="font-weight: 600; color: #555;">"Warehouse ID:"</div>
                                                                <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{data.posting.warehouse_id}</div>

                                                                <div style="font-weight: 600; color: #555;">"Документ отгрузки:"</div>
                                                                <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">
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
                                                                                    style="color: #2196F3; text-decoration: underline; cursor: pointer; font-weight: 500;"
                                                                                >
                                                                                    {format!("{} {}", posting_ref_type, data.posting.posting_number.clone())}
                                                                                </a>
                                                                            }.into_any()
                                                                        } else if data.is_posted {
                                                                            view! {
                                                                                <span style="color: #f44336; font-weight: 600;">
                                                                                    "⚠️ Документ отгрузки не найден"
                                                                                </span>
                                                                            }.into_any()
                                                                        } else {
                                                                            view! {
                                                                                <span style="color: #999;">
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
                                                    <div style="background: white; padding: 20px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
                                                        <h3 style="margin: 0 0 15px 0; color: #333; font-size: 16px; font-weight: 600; border-bottom: 2px solid #2196F3; padding-bottom: 8px;">"Товары"</h3>
                                                        {if data.items.is_empty() {
                                                            view! {
                                                                <p style="text-align: center; padding: 40px; color: #999;">"Нет товаров"</p>
                                                            }.into_any()
                                                        } else {
                                                            view! {
                                                                <table style="width: 100%; border-collapse: collapse; font-size: 0.9em;">
                                                                    <thead>
                                                                        <tr style="background: #f5f5f5;">
                                                                            <th style="padding: 12px; text-align: left; font-weight: 600; border-bottom: 2px solid #ddd; color: #555;">"SKU"</th>
                                                                            <th style="padding: 12px; text-align: left; font-weight: 600; border-bottom: 2px solid #ddd; color: #555;">"Название"</th>
                                                                            <th style="padding: 12px; text-align: right; font-weight: 600; border-bottom: 2px solid #ddd; color: #555;">"Цена"</th>
                                                                            <th style="padding: 12px; text-align: right; font-weight: 600; border-bottom: 2px solid #ddd; color: #555;">"Пропорция"</th>
                                                                            <th style="padding: 12px; text-align: left; font-weight: 600; border-bottom: 2px solid #ddd; color: #555;">"Продукт MP"</th>
                                                                            <th style="padding: 12px; text-align: left; font-weight: 600; border-bottom: 2px solid #ddd; color: #555;">"Номенклатура"</th>
                                                                        </tr>
                                                                    </thead>
                                                                    <tbody>
                                                                        {data.items.iter().map(|item| view! {
                                                                            <tr style="border-bottom: 1px solid #eee;">
                                                                                <td style="padding: 12px; font-family: 'Courier New', monospace; color: #333;">{item.sku}</td>
                                                                                <td style="padding: 12px; color: #333;">{item.name.clone()}</td>
                                                                                <td style="padding: 12px; text-align: right; color: #333;">
                                                                                    {item.price.map(|p| format!("{:.2} ₽", p)).unwrap_or("—".to_string())}
                                                                                </td>
                                                                                <td style="padding: 12px; text-align: right; color: #666;">
                                                                                    {item.ratio.map(|r| format!("{:.1}%", r * 100.0)).unwrap_or("—".to_string())}
                                                                                </td>
                                                                                <td style="padding: 12px; font-family: 'Courier New', monospace; font-size: 0.85em; color: #666;">
                                                                                    {item.marketplace_product_ref.as_ref().map(|r| format!("{}...", r.chars().take(8).collect::<String>())).unwrap_or("—".to_string())}
                                                                                </td>
                                                                                <td style="padding: 12px; font-family: 'Courier New', monospace; font-size: 0.85em; color: #666;">
                                                                                    {item.nomenclature_ref.as_ref().map(|r| format!("{}...", r.chars().take(8).collect::<String>())).unwrap_or("—".to_string())}
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
                                                    <div style="background: white; padding: 20px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
                                                        <h3 style="margin: 0 0 15px 0; color: #333; font-size: 16px; font-weight: 600; border-bottom: 2px solid #4caf50; padding-bottom: 8px;">"Сервисы"</h3>
                                                        {if data.services.is_empty() {
                                                            view! {
                                                                <p style="text-align: center; padding: 40px; color: #999;">"Нет сервисов"</p>
                                                            }.into_any()
                                                        } else {
                                                            view! {
                                                                <table style="width: 100%; border-collapse: collapse;">
                                                                    <thead>
                                                                        <tr style="background: #f5f5f5;">
                                                                            <th style="padding: 12px; text-align: left; font-weight: 600; border-bottom: 2px solid #ddd; color: #555;">"Название"</th>
                                                                            <th style="padding: 12px; text-align: right; font-weight: 600; border-bottom: 2px solid #ddd; color: #555;">"Цена"</th>
                                                                        </tr>
                                                                    </thead>
                                                                    <tbody>
                                                                        {data.services.iter().map(|service| view! {
                                                                            <tr style="border-bottom: 1px solid #eee;">
                                                                                <td style="padding: 12px; color: #333;">{service.name.clone()}</td>
                                                                                <td style="padding: 12px; text-align: right; font-weight: 600; color: #4caf50;">{format!("{:.2} ₽", service.price)}</td>
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
                                                                    <div style="padding: 20px; text-align: center; color: #999;">
                                                                        "Загрузка проекций..."
                                                                    </div>
                                                                }.into_any()
                                                            } else if let Some(proj_data) = projections.get() {
                                                                let p900_items = proj_data["p900_sales_register"].as_array().cloned().unwrap_or_default();
                                                                let p902_items = proj_data["p902_ozon_finance"].as_array().cloned().unwrap_or_default();
                                                                let p904_items = proj_data["p904_sales_data"].as_array().cloned().unwrap_or_default();

                                                                view! {
                                                                    <div style="display: flex; flex-direction: column; gap: 20px;">
                                                                        // P900 Sales Register
                                                                        <div style="background: white; padding: 20px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
                                                                            <h3 style="margin: 0 0 15px 0; color: #333; font-size: 16px; font-weight: 600; border-bottom: 2px solid #ff9800; padding-bottom: 8px;">
                                                                                {format!("📊 Sales Register (p900) - {} записей", p900_items.len())}
                                                                            </h3>
                                                                            {if !p900_items.is_empty() {
                                                                                view! {
                                                                                    <div style="overflow-x: auto;">
                                                                                        <table style="width: 100%; border-collapse: collapse; font-size: 0.85em;">
                                                                                            <thead>
                                                                                                <tr style="background: #f5f5f5;">
                                                                                                    <th style="padding: 8px; text-align: left; border: 1px solid #ddd;">"MP"</th>
                                                                                                    <th style="padding: 8px; text-align: left; border: 1px solid #ddd;">"SKU"</th>
                                                                                                    <th style="padding: 8px; text-align: left; border: 1px solid #ddd;">"Title"</th>
                                                                                                    <th style="padding: 8px; text-align: right; border: 1px solid #ddd;">"Qty"</th>
                                                                                                    <th style="padding: 8px; text-align: right; border: 1px solid #ddd;">"Amount"</th>
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
                                                                                                        <tr style="border-bottom: 1px solid #eee;">
                                                                                                            <td style="padding: 8px; border: 1px solid #ddd;">{mp}</td>
                                                                                                            <td style="padding: 8px; border: 1px solid #ddd; font-family: monospace;">{sku}</td>
                                                                                                            <td style="padding: 8px; border: 1px solid #ddd;">{title}</td>
                                                                                                            <td style="padding: 8px; text-align: right; border: 1px solid #ddd;">{qty}</td>
                                                                                                            <td style="padding: 8px; text-align: right; border: 1px solid #ddd; font-weight: 600;">{format!("{:.2}", amount)}</td>
                                                                                                        </tr>
                                                                                                    }
                                                                                                }).collect::<Vec<_>>()}
                                                                                            </tbody>
                                                                                        </table>
                                                                                    </div>
                                                                                }.into_any()
                                                                            } else {
                                                                                view! {
                                                                                    <p style="text-align: center; padding: 20px; color: #999;">"Нет записей"</p>
                                                                                }.into_any()
                                                                            }}
                                                                        </div>

                                                                        // P902 OZON Finance
                                                                        <div style="background: white; padding: 20px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
                                                                            <h3 style="margin: 0 0 15px 0; color: #333; font-size: 16px; font-weight: 600; border-bottom: 2px solid #4caf50; padding-bottom: 8px;">
                                                                                {format!("💰 OZON Finance (p902) - {} записей", p902_items.len())}
                                                                            </h3>
                                                                            {if !p902_items.is_empty() {
                                                                                view! {
                                                                                    <div style="overflow-x: auto;">
                                                                                        <table style="width: 100%; border-collapse: collapse; font-size: 0.85em;">
                                                                                            <thead>
                                                                                                <tr style="background: #f5f5f5;">
                                                                                                    <th style="padding: 8px; text-align: left; border: 1px solid #ddd;">"Posting"</th>
                                                                                                    <th style="padding: 8px; text-align: left; border: 1px solid #ddd;">"SKU"</th>
                                                                                                    <th style="padding: 8px; text-align: right; border: 1px solid #ddd;">"Qty"</th>
                                                                                                    <th style="padding: 8px; text-align: right; border: 1px solid #ddd;">"Amount"</th>
                                                                                                </tr>
                                                                                            </thead>
                                                                                            <tbody>
                                                                                                {p902_items.iter().map(|item| {
                                                                                                    let posting = item["posting_number"].as_str().unwrap_or("—");
                                                                                                    let sku = item["sku"].as_str().unwrap_or("—");
                                                                                                    let qty = item["quantity"].as_f64().unwrap_or(0.0);
                                                                                                    let amount = item["amount"].as_f64().unwrap_or(0.0);
                                                                                                    
                                                                                                    view! {
                                                                                                        <tr style="border-bottom: 1px solid #eee;">
                                                                                                            <td style="padding: 8px; border: 1px solid #ddd; font-family: monospace;">{posting}</td>
                                                                                                            <td style="padding: 8px; border: 1px solid #ddd; font-family: monospace;">{sku}</td>
                                                                                                            <td style="padding: 8px; text-align: right; border: 1px solid #ddd;">{qty}</td>
                                                                                                            <td style="padding: 8px; text-align: right; border: 1px solid #ddd; font-weight: 600;">{format!("{:.2}", amount)}</td>
                                                                                                        </tr>
                                                                                                    }
                                                                                                }).collect::<Vec<_>>()}
                                                                                            </tbody>
                                                                                        </table>
                                                                                    </div>
                                                                                }.into_any()
                                                                            } else {
                                                                                view! {
                                                                                    <p style="text-align: center; padding: 20px; color: #999;">"Нет записей"</p>
                                                                                }.into_any()
                                                                            }}
                                                                        </div>

                                                                        // P904 Sales Data
                                                                        <div style="background: white; padding: 20px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
                                                                            <h3 style="margin: 0 0 15px 0; color: #333; font-size: 16px; font-weight: 600; border-bottom: 2px solid #2196F3; padding-bottom: 8px;">
                                                                                {format!("📈 Sales Data (p904) - {} записей", p904_items.len())}
                                                                            </h3>
                                                                            {if !p904_items.is_empty() {
                                                                                view! {
                                                                                    <div style="overflow-x: auto;">
                                                                                        <table style="width: 100%; border-collapse: collapse; font-size: 0.85em;">
                                                                                            <thead>
                                                                                                <tr style="background: #f5f5f5;">
                                                                                                    <th style="padding: 8px; text-align: left; border: 1px solid #ddd;">"Date"</th>
                                                                                                    <th style="padding: 8px; text-align: left; border: 1px solid #ddd;">"Article"</th>
                                                                                                    <th style="padding: 8px; text-align: right; border: 1px solid #ddd;">"Customer In"</th>
                                                                                                    <th style="padding: 8px; text-align: right; border: 1px solid #ddd;">"Total"</th>
                                                                                                </tr>
                                                                                            </thead>
                                                                                            <tbody>
                                                                                                {p904_items.iter().map(|item| {
                                                                                                    let date = item["date"].as_str().unwrap_or("—");
                                                                                                    let article = item["article"].as_str().unwrap_or("—");
                                                                                                    let customer_in = item["customer_in"].as_f64().unwrap_or(0.0);
                                                                                                    let total = item["total"].as_f64().unwrap_or(0.0);
                                                                                                    
                                                                                                    view! {
                                                                                                        <tr style="border-bottom: 1px solid #eee;">
                                                                                                            <td style="padding: 8px; border: 1px solid #ddd;">{date}</td>
                                                                                                            <td style="padding: 8px; border: 1px solid #ddd; font-family: monospace;">{article}</td>
                                                                                                            <td style="padding: 8px; text-align: right; border: 1px solid #ddd;">{format!("{:.2}", customer_in)}</td>
                                                                                                            <td style="padding: 8px; text-align: right; border: 1px solid #ddd; font-weight: 600;">{format!("{:.2}", total)}</td>
                                                                                                        </tr>
                                                                                                    }
                                                                                                }).collect::<Vec<_>>()}
                                                                                            </tbody>
                                                                                        </table>
                                                                                    </div>
                                                                                }.into_any()
                                                                            } else {
                                                                                view! {
                                                                                    <p style="text-align: center; padding: 20px; color: #999;">"Нет записей"</p>
                                                                                }.into_any()
                                                                            }}
                                                                        </div>
                                                                    </div>
                                                                }.into_any()
                                                            } else {
                                                                view! {
                                                                    <div style="padding: 20px; text-align: center; color: #999;">
                                                                        "Нет данных проекций"
                                                                    </div>
                                                                }.into_any()
                                                            }
                                                        }}
                                                    </div>
                                                }.into_any(),
                                                _ => view! {
                                                    <div>"Unknown tab"</div>
                                                }.into_any()
                                            }
                                        }}
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div>"Нет данных"</div>
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
                                <div style="background: white; border-radius: 8px; box-shadow: 0 4px 16px rgba(0,0,0,0.2); width: 90%; max-width: 1400px; max-height: 90vh; overflow: hidden;">
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
                                <div style="background: white; border-radius: 8px; box-shadow: 0 4px 16px rgba(0,0,0,0.2); width: 90%; max-width: 1400px; max-height: 90vh; overflow: hidden;">
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
