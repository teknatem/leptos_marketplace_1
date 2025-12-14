use leptos::prelude::*;
use leptos::logging::log;
use serde::{Deserialize, Serialize};
use gloo_net::http::Request;
use wasm_bindgen::JsCast;
use crate::domain::a014_ozon_transactions::ui::details::OzonTransactionsDetail;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonFbsPostingDetailDto {
    pub id: String,
    pub code: String,
    pub description: String,
    pub header: HeaderDto,
    pub lines: Vec<LineDto>,
    pub state: StateDto,
    pub source_meta: SourceMetaDto,
    pub metadata: MetadataDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderDto {
    pub document_no: String,
    pub scheme: String,
    pub connection_id: String,
    pub organization_id: String,
    pub marketplace_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineDto {
    pub line_id: String,
    pub product_id: String,
    pub offer_id: String,
    pub name: String,
    pub barcode: Option<String>,
    pub qty: f64,
    pub price_list: Option<f64>,
    pub discount_total: Option<f64>,
    pub price_effective: Option<f64>,
    pub amount_line: Option<f64>,
    pub currency_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDto {
    pub status_raw: String,
    pub status_norm: String,
    pub substatus_raw: Option<String>,
    pub delivered_at: Option<String>,
    pub updated_at_source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMetaDto {
    pub raw_payload_ref: String,
    pub fetched_at: String,
    pub document_version: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataDto {
    pub created_at: String,
    pub updated_at: String,
    pub is_deleted: bool,
    pub is_posted: bool,
    pub version: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesRegisterDto {
    pub marketplace: String,
    pub document_no: String,
    pub line_id: String,
    pub scheme: Option<String>,
    pub document_type: String,
    pub document_version: i32,
    pub connection_mp_ref: String,
    pub organization_ref: String,
    pub marketplace_product_ref: Option<String>,
    pub nomenclature_ref: Option<String>,
    pub registrator_ref: String,
    pub event_time_source: String,
    pub sale_date: String,
    pub source_updated_at: Option<String>,
    pub status_source: String,
    pub status_norm: String,
    pub seller_sku: Option<String>,
    pub mp_item_id: String,
    pub barcode: Option<String>,
    pub title: Option<String>,
    pub qty: f64,
    pub price_list: Option<f64>,
    pub discount_total: Option<f64>,
    pub price_effective: Option<f64>,
    pub amount_line: Option<f64>,
    pub currency_code: Option<String>,
    pub loaded_at_utc: String,
    pub payload_version: i32,
    pub extra: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionListDto {
    pub id: String,
    pub operation_id: i64,
    pub operation_type: String,
    pub operation_type_name: String,
    pub operation_date: String,
    pub posting_number: String,
    pub transaction_type: String,
    pub amount: f64,
    pub is_posted: bool,
}

/// Форматирует дату из "2025-10-11 00:00:00" в dd.mm.yyyy
fn format_transaction_date(date_str: &str) -> String {
    let date_part = date_str.split_whitespace().next().unwrap_or(date_str);
    if let Some((year, rest)) = date_part.split_once('-') {
        if let Some((month, day)) = rest.split_once('-') {
            return format!("{}.{}.{}", day, month, year);
        }
    }
    date_str.to_string()
}

#[component]
pub fn OzonFbsPostingDetail(
    id: String,
    #[prop(into)] on_close: Callback<()>,
    #[prop(optional)] reload_trigger: Option<ReadSignal<u32>>,
) -> impl IntoView {
    let (posting, set_posting) = signal::<Option<OzonFbsPostingDetailDto>>(None);
    let (raw_json_from_ozon, set_raw_json_from_ozon) = signal::<Option<String>>(None);
    let (projections, set_projections) = signal::<Vec<SalesRegisterDto>>(Vec::new());
    let (projections_loading, set_projections_loading) = signal(false);
    let (transactions, set_transactions) = signal::<Vec<TransactionListDto>>(Vec::new());
    let (transactions_loading, set_transactions_loading) = signal(false);
    let (selected_transaction_id, set_selected_transaction_id) = signal::<Option<String>>(None);
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal::<Option<String>>(None);
    let (active_tab, set_active_tab) = signal("general");

    // Загрузить детальные данные
    Effect::new(move || {
        // Отслеживаем reload_trigger если передан
        if let Some(trigger) = reload_trigger {
            let _ = trigger.get(); // Подписываемся на изменения
        }

        let id = id.clone();
        wasm_bindgen_futures::spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let url = format!("http://localhost:3000/api/a010/ozon-fbs-posting/{}", id);

            match Request::get(&url).send().await {
                Ok(response) => {
                    let status = response.status();
                    if status == 200 {
                        match response.text().await {
                            Ok(text) => {
                                // Парсим структуру
                                match serde_json::from_str::<OzonFbsPostingDetailDto>(&text) {
                                    Ok(data) => {
                                        // Загружаем raw JSON от OZON
                                        let raw_payload_ref = data.source_meta.raw_payload_ref.clone();
                                        let posting_id = data.id.clone();
                                        let document_no = data.header.document_no.clone();
                                        set_posting.set(Some(data));
                                        set_loading.set(false);

                                        // Асинхронная загрузка raw JSON
                                        wasm_bindgen_futures::spawn_local(async move {
                                            let raw_url = format!("http://localhost:3000/api/a010/raw/{}", raw_payload_ref);
                                            match Request::get(&raw_url).send().await {
                                                Ok(resp) => {
                                                    if resp.status() == 200 {
                                                        if let Ok(text) = resp.text().await {
                                                            // Форматируем JSON
                                                            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&text) {
                                                                if let Ok(formatted) = serde_json::to_string_pretty(&json_value) {
                                                                    set_raw_json_from_ozon.set(Some(formatted));
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    log!("Failed to load raw JSON from OZON: {:?}", e);
                                                }
                                            }
                                        });

                                        // Асинхронная загрузка проекций p900
                                        let set_projections = set_projections.clone();
                                        let set_projections_loading = set_projections_loading.clone();
                                        wasm_bindgen_futures::spawn_local(async move {
                                            set_projections_loading.set(true);
                                            let projections_url = format!("http://localhost:3000/api/projections/p900/{}", posting_id);
                                            match Request::get(&projections_url).send().await {
                                                Ok(resp) => {
                                                    if resp.status() == 200 {
                                                        if let Ok(text) = resp.text().await {
                                                            if let Ok(items) = serde_json::from_str::<Vec<SalesRegisterDto>>(&text) {
                                                                set_projections.set(items);
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

                                        // Асинхронная загрузка транзакций a014
                                        let set_transactions = set_transactions.clone();
                                        let set_transactions_loading = set_transactions_loading.clone();
                                        wasm_bindgen_futures::spawn_local(async move {
                                            set_transactions_loading.set(true);
                                            // URL-кодируем posting_number для корректной передачи в URL
                                            let encoded_posting_number = urlencoding::encode(&document_no);
                                            let transactions_url = format!("http://localhost:3000/api/ozon_transactions/by-posting/{}", encoded_posting_number);
                                            log!("Loading transactions for posting_number: {} (encoded: {})", document_no, encoded_posting_number);
                                            log!("Request URL: {}", transactions_url);
                                            
                                            match Request::get(&transactions_url).send().await {
                                                Ok(resp) => {
                                                    let status = resp.status();
                                                    log!("Transactions response status: {}", status);
                                                    if status == 200 {
                                                        if let Ok(text) = resp.text().await {
                                                            log!("Transactions response: {}", text);
                                                            if let Ok(items) = serde_json::from_str::<Vec<TransactionListDto>>(&text) {
                                                                log!("Successfully parsed {} transactions", items.len());
                                                                set_transactions.set(items);
                                                            } else {
                                                                log!("Failed to parse transactions JSON");
                                                            }
                                                        }
                                                    } else {
                                                        log!("Transaction request failed with status: {}", status);
                                                    }
                                                }
                                                Err(e) => {
                                                    log!("Failed to load transactions: {:?}", e);
                                                }
                                            }
                                            set_transactions_loading.set(false);
                                        });
                                    }
                                    Err(e) => {
                                        log!("Failed to parse posting: {:?}", e);
                                        set_error.set(Some(format!("Failed to parse: {}", e)));
                                        set_loading.set(false);
                                    }
                                }
                            }
                            Err(e) => {
                                log!("Failed to read response: {:?}", e);
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
                    log!("Failed to fetch posting: {:?}", e);
                    set_error.set(Some(format!("Failed to fetch: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    });

    view! {
        <div class="posting-detail" style="padding: var(--space-xl); height: 100%; display: flex; flex-direction: column; background: var(--color-bg-white); border-radius: var(--radius-lg); box-shadow: var(--shadow-sm);">
            <div style="background: linear-gradient(135deg, #4a5568 0%, #2d3748 100%); padding: var(--space-md) var(--space-xl); border-radius: var(--radius-md) var(--radius-md) 0 0; margin: calc(-1 * var(--space-xl)) calc(-1 * var(--space-xl)) 0 calc(-1 * var(--space-xl)); display: flex; align-items: center; justify-content: space-between; flex-shrink: 0;">
                <h2 style="margin: 0; font-size: var(--font-size-base); font-weight: var(--font-weight-semibold); color: var(--color-text-white);">"OZON FBS Posting Details"</h2>
                <button
                    class="button--ghost"
                    on:click=move |_| on_close.run(())
                >
                    "✕ Закрыть"
                </button>
            </div>

            <div style="flex: 1; overflow-y: auto; min-height: 0;">
                {move || {
                    if loading.get() {
                        view! {
                            <div style="text-align: center; padding: 40px;">
                                <p>"Loading..."</p>
                            </div>
                        }.into_any()
                    } else if let Some(err) = error.get() {
                        view! {
                            <div style="padding: 20px; background: #ffebee; border: 1px solid #ffcdd2; border-radius: 4px; color: #c62828;">
                                <strong>"Error: "</strong>{err}
                            </div>
                        }.into_any()
                    } else if let Some(post) = posting.get() {
                        view! {
                            <div style="height: 100%; display: flex; flex-direction: column;">
                                // Вкладки
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
                                        class:active=move || active_tab.get() == "lines"
                                        on:click=move |_| set_active_tab.set("lines")
                                    >
                                        "Строки (" {post.lines.len()} ")"
                                    </button>
                                    <button
                                        class="detail-tab"
                                        class:active=move || active_tab.get() == "projections"
                                        on:click=move |_| set_active_tab.set("projections")
                                    >
                                        {move || format!("Проекции ({})", projections.get().len())}
                                    </button>
                                    <button
                                        class="detail-tab"
                                        class:active=move || active_tab.get() == "transactions"
                                        on:click=move |_| set_active_tab.set("transactions")
                                    >
                                        {move || format!("Транзакции ({})", transactions.get().len())}
                                    </button>
                                    <button
                                        class="detail-tab"
                                        class:active=move || active_tab.get() == "json"
                                        on:click=move |_| set_active_tab.set("json")
                                    >
                                        "Raw JSON"
                                    </button>
                                </div>

                                // Контент вкладок
                                <div style="flex: 1; overflow-y: auto; padding: 10px 0;">
                                    {move || {
                                let tab = active_tab.get();
                                match tab.as_ref() {
                                    "general" => {
                                        // Helper для рендеринга UUID с кнопкой копирования
                                        let conn_id = post.header.connection_id.clone();
                                        let org_id = post.header.organization_id.clone();
                                        let mp_id = post.header.marketplace_id.clone();
                                        
                                        view! {
                                            <div class="general-info">
                                                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 15px 20px; align-items: center;">
                                                    <div style="font-weight: 600; color: #555;">"Document №:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{post.header.document_no.clone()}</div>
                                                    
                                                    <div style="font-weight: 600; color: #555;">"Code:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{post.code.clone()}</div>
                                                    
                                                    <div style="font-weight: 600; color: #555;">"Description:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{post.description.clone()}</div>
                                                    
                                                    <div style="font-weight: 600; color: #555;">"Scheme:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">
                                                        <span style="padding: 2px 8px; background: #e3f2fd; color: #1976d2; border-radius: 3px; font-weight: 500;">
                                                            {post.header.scheme.clone()}
                                                        </span>
                                                    </div>
                                                    
                                                    <div style="font-weight: 600; color: #555;">"Status:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">
                                                        <span style="padding: 2px 8px; background: #e8f5e9; color: #2e7d32; border-radius: 3px; font-weight: 500;">
                                                            {post.state.status_norm.clone()}
                                                        </span>
                                                    </div>

                                                    <div style="font-weight: 600; color: #555;">"Substatus:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">
                                                        <span style="padding: 2px 8px; background: #fff3e0; color: #f57c00; border-radius: 3px; font-weight: 500;">
                                                            {post.state.substatus_raw.clone().unwrap_or("—".to_string())}
                                                        </span>
                                                    </div>

                                                    <div style="font-weight: 600; color: #555;">"Проведен:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">
                                                        {if post.metadata.is_posted {
                                                            view! {
                                                                <span style="padding: 2px 8px; background: #c8e6c9; color: #2e7d32; border-radius: 3px; font-weight: 500;">
                                                                    "✓ Да"
                                                                </span>
                                                            }
                                                        } else {
                                                            view! {
                                                                <span style="padding: 2px 8px; background: #f5f5f5; color: #999; border-radius: 3px; font-weight: 500;">
                                                                    "○ Нет"
                                                                </span>
                                                            }
                                                        }}
                                                    </div>
                                                    
                                                    <div style="font-weight: 600; color: #555;">"Дата доставки:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{post.state.delivered_at.clone().unwrap_or("—".to_string())}</div>
                                                    
                                                    <div style="font-weight: 600; color: #555;">"Connection ID:"</div>
                                                    <div style="display: flex; align-items: center; gap: 8px; font-family: monospace; font-size: 14px;">
                                                        <span style="color: #666;" title=conn_id.clone()>{format!("{}...", conn_id.chars().take(8).collect::<String>())}</span>
                                                        <button
                                                            on:click=move |_| {
                                                                let uuid_copy = conn_id.clone();
                                                                wasm_bindgen_futures::spawn_local(async move {
                                                                    if let Some(window) = web_sys::window() {
                                                                        let nav = window.navigator().clipboard();
                                                                        let _ = nav.write_text(&uuid_copy);
                                                                    }
                                                                });
                                                            }
                                                            style="padding: 2px 6px; font-size: 11px; border: 1px solid #ddd; background: white; border-radius: 3px; cursor: pointer;"
                                                            title="Copy to clipboard"
                                                        >
                                                            "📋"
                                                        </button>
                                                    </div>
                                                    
                                                    <div style="font-weight: 600; color: #555;">"Organization ID:"</div>
                                                    <div style="display: flex; align-items: center; gap: 8px; font-family: monospace; font-size: 14px;">
                                                        <span style="color: #666;" title=org_id.clone()>{format!("{}...", org_id.chars().take(8).collect::<String>())}</span>
                                                        <button
                                                            on:click=move |_| {
                                                                let uuid_copy = org_id.clone();
                                                                wasm_bindgen_futures::spawn_local(async move {
                                                                    if let Some(window) = web_sys::window() {
                                                                        let nav = window.navigator().clipboard();
                                                                        let _ = nav.write_text(&uuid_copy);
                                                                    }
                                                                });
                                                            }
                                                            style="padding: 2px 6px; font-size: 11px; border: 1px solid #ddd; background: white; border-radius: 3px; cursor: pointer;"
                                                            title="Copy to clipboard"
                                                        >
                                                            "📋"
                                                        </button>
                                                    </div>
                                                    
                                                    <div style="font-weight: 600; color: #555;">"Marketplace ID:"</div>
                                                    <div style="display: flex; align-items: center; gap: 8px; font-family: monospace; font-size: 14px;">
                                                        <span style="color: #666;" title=mp_id.clone()>{format!("{}...", mp_id.chars().take(8).collect::<String>())}</span>
                                                        <button
                                                            on:click=move |_| {
                                                                let uuid_copy = mp_id.clone();
                                                                wasm_bindgen_futures::spawn_local(async move {
                                                                    if let Some(window) = web_sys::window() {
                                                                        let nav = window.navigator().clipboard();
                                                                        let _ = nav.write_text(&uuid_copy);
                                                                    }
                                                                });
                                                            }
                                                            style="padding: 2px 6px; font-size: 11px; border: 1px solid #ddd; background: white; border-radius: 3px; cursor: pointer;"
                                                            title="Copy to clipboard"
                                                        >
                                                            "📋"
                                                        </button>
                                                    </div>
                                                    
                                                    <div style="font-weight: 600; color: #555;">"Created At:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{post.metadata.created_at.clone()}</div>
                                                    
                                                    <div style="font-weight: 600; color: #555;">"Updated At:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{post.metadata.updated_at.clone()}</div>
                                                    
                                                    <div style="font-weight: 600; color: #555;">"Version:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{post.metadata.version}</div>
                                                </div>
                                            </div>
                                        }.into_any()
                                    },
                                    "lines" => view! {
                                        <div class="lines-info">
                                            <table style="width: 100%; border-collapse: collapse;">
                                                <thead>
                                                    <tr style="background: #f5f5f5;">
                                                        <th style="border: 1px solid #ddd; padding: 8px; text-align: left;">"#"</th>
                                                        <th style="border: 1px solid #ddd; padding: 8px; text-align: left;">"Product"</th>
                                                        <th style="border: 1px solid #ddd; padding: 8px; text-align: left;">"Offer ID"</th>
                                                        <th style="border: 1px solid #ddd; padding: 8px; text-align: right;">"Qty"</th>
                                                        <th style="border: 1px solid #ddd; padding: 8px; text-align: right;">"Price"</th>
                                                        <th style="border: 1px solid #ddd; padding: 8px; text-align: right;">"Amount"</th>
                                                    </tr>
                                                </thead>
                                                <tbody>
                                                    {post.lines.iter().enumerate().map(|(idx, line)| {
                                                        view! {
                                                            <tr>
                                                                <td style="border: 1px solid #ddd; padding: 8px;">{idx + 1}</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px;">{line.name.clone()}</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px;"><code style="font-size: 0.85em;">{line.offer_id.clone()}</code></td>
                                                                <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{line.qty}</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">
                                                                    {line.price_effective.map(|p| format!("{:.2}", p)).unwrap_or("—".to_string())}
                                                                    {line.currency_code.as_ref().map(|c| format!(" {}", c)).unwrap_or_default()}
                                                                </td>
                                                                <td style="border: 1px solid #ddd; padding: 8px; text-align: right; font-weight: bold;">
                                                                    {line.amount_line.map(|a| format!("{:.2}", a)).unwrap_or("—".to_string())}
                                                                    {line.currency_code.as_ref().map(|c| format!(" {}", c)).unwrap_or_default()}
                                                                </td>
                                                            </tr>
                                                        }
                                                    }).collect_view()}
                                                </tbody>
                                            </table>
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
                                                } else {
                                                    let items = projections.get();
                                                    if items.is_empty() {
                                                        view! {
                                                            <div style="padding: 20px; text-align: center; color: #999;">
                                                                "Нет записей в проекции p900"
                                                            </div>
                                                        }.into_any()
                                                    } else {
                                                        view! {
                                                            <div>
                                                                <div style="margin-bottom: 10px; padding: 10px; background: #e3f2fd; border-radius: 4px;">
                                                                    <strong>"Записи Sales Register (p900)"</strong>
                                                                    <span style="margin-left: 10px; color: #666;">{format!("Всего: {}", items.len())}</span>
                                                                </div>
                                                                <table style="width: 100%; border-collapse: collapse; font-size: 0.9em;">
                                                                    <thead>
                                                                        <tr style="background: #f5f5f5;">
                                                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: left;">"#"</th>
                                                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: left;">"Marketplace"</th>
                                                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: left;">"SKU"</th>
                                                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: left;">"Barcode"</th>
                                                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: left;">"Title"</th>
                                                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: right;">"Qty"</th>
                                                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: right;">"Price"</th>
                                                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: right;">"Amount"</th>
                                                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: left;">"Sale Date"</th>
                                                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: left;">"Status"</th>
                                                                        </tr>
                                                                    </thead>
                                                                    <tbody>
                                                                        {items.iter().enumerate().map(|(idx, item)| {
                                                                            view! {
                                                                                <tr>
                                                                                    <td style="border: 1px solid #ddd; padding: 8px;">{idx + 1}</td>
                                                                                    <td style="border: 1px solid #ddd; padding: 8px;">
                                                                                        <span style="padding: 2px 6px; background: #e3f2fd; border-radius: 3px; font-weight: 500;">
                                                                                            {item.marketplace.clone()}
                                                                                        </span>
                                                                                    </td>
                                                                                    <td style="border: 1px solid #ddd; padding: 8px;">
                                                                                        <code style="font-size: 0.85em;">
                                                                                            {item.seller_sku.clone().unwrap_or("—".to_string())}
                                                                                        </code>
                                                                                    </td>
                                                                                    <td style="border: 1px solid #ddd; padding: 8px;">
                                                                                        {item.barcode.clone().unwrap_or("—".to_string())}
                                                                                    </td>
                                                                                    <td style="border: 1px solid #ddd; padding: 8px;">
                                                                                        {item.title.clone().unwrap_or("—".to_string())}
                                                                                    </td>
                                                                                    <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">
                                                                                        {item.qty}
                                                                                    </td>
                                                                                    <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">
                                                                                        {item.price_effective.map(|p| format!("{:.2}", p)).unwrap_or("—".to_string())}
                                                                                        {item.currency_code.as_ref().map(|c| format!(" {}", c)).unwrap_or_default()}
                                                                                    </td>
                                                                                    <td style="border: 1px solid #ddd; padding: 8px; text-align: right; font-weight: bold;">
                                                                                        {item.amount_line.map(|a| format!("{:.2}", a)).unwrap_or("—".to_string())}
                                                                                        {item.currency_code.as_ref().map(|c| format!(" {}", c)).unwrap_or_default()}
                                                                                    </td>
                                                                                    <td style="border: 1px solid #ddd; padding: 8px;">
                                                                                        {item.sale_date.clone()}
                                                                                    </td>
                                                                                    <td style="border: 1px solid #ddd; padding: 8px;">
                                                                                        <span style="padding: 2px 6px; background: #e8f5e9; color: #2e7d32; border-radius: 3px;">
                                                                                            {item.status_norm.clone()}
                                                                                        </span>
                                                                                    </td>
                                                                                </tr>
                                                                            }
                                                                        }).collect_view()}
                                                                    </tbody>
                                                                </table>
                                                            </div>
                                                        }.into_any()
                                                    }
                                                }
                                            }}
                                        </div>
                                    }.into_any(),
                                    "transactions" => view! {
                                        <div class="transactions-info">
                                            {move || {
                                                if transactions_loading.get() {
                                                    view! {
                                                        <div style="padding: 20px; text-align: center; color: #999;">
                                                            "Загрузка транзакций..."
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    let mut items = transactions.get();
                                                    // Сортировка по дате (новые сначала)
                                                    items.sort_by(|a, b| b.operation_date.cmp(&a.operation_date));
                                                    
                                                    if items.is_empty() {
                                                        view! {
                                                            <div style="padding: 20px; text-align: center; color: #999;">
                                                                "Нет транзакций для данного постинга"
                                                            </div>
                                                        }.into_any()
                                                    } else {
                                                        view! {
                                                            <div>
                                                                <div style="margin-bottom: 10px; padding: 10px; background: #e3f2fd; border-radius: 4px;">
                                                                    <strong>"Транзакции OZON (a014)"</strong>
                                                                    <span style="margin-left: 10px; color: #666;">{format!("Всего: {}", items.len())}</span>
                                                                </div>
                                                                <table style="width: 100%; border-collapse: collapse; font-size: 0.9em;">
                                                                    <thead>
                                                                        <tr style="background: #f5f5f5;">
                                                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: left;">"#"</th>
                                                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: left;">"Дата операции"</th>
                                                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: left;">"Operation ID"</th>
                                                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: left;">"Тип операции"</th>
                                                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: left;">"Тип транзакции"</th>
                                                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: right;">"Сумма"</th>
                                                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: left;">"Статус"</th>
                                                                        </tr>
                                                                    </thead>
                                                                    <tbody>
                                                                        {items.iter().enumerate().map(|(idx, item)| {
                                                                            let item_id_for_click = item.id.clone();
                                                                            view! {
                                                                                <tr
                                                                                    style="cursor: pointer; transition: background 0.2s;"
                                                                                    on:mouseenter=move |ev| {
                                                                                        if let Some(el) = ev.target().and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok()) {
                                                                                            let _ = el.style().set_property("background", "#f0f0f0");
                                                                                        }
                                                                                    }
                                                                                    on:mouseleave=move |ev| {
                                                                                        if let Some(el) = ev.target().and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok()) {
                                                                                            let _ = el.style().set_property("background", "white");
                                                                                        }
                                                                                    }
                                                                                    on:click=move |_| {
                                                                                        set_selected_transaction_id.set(Some(item_id_for_click.clone()));
                                                                                    }
                                                                                >
                                                                                    <td style="border: 1px solid #ddd; padding: 8px;">{idx + 1}</td>
                                                                                    <td style="border: 1px solid #ddd; padding: 8px;">
                                                                                        {format_transaction_date(&item.operation_date)}
                                                                                    </td>
                                                                                    <td style="border: 1px solid #ddd; padding: 8px;">
                                                                                        <code style="font-size: 0.85em;">
                                                                                            {item.operation_id}
                                                                                        </code>
                                                                                    </td>
                                                                                    <td style="border: 1px solid #ddd; padding: 8px;">
                                                                                        {item.operation_type_name.clone()}
                                                                                    </td>
                                                                                    <td style="border: 1px solid #ddd; padding: 8px;">
                                                                                        <span style="padding: 2px 6px; background: #fff3e0; border-radius: 3px;">
                                                                                            {item.transaction_type.clone()}
                                                                                        </span>
                                                                                    </td>
                                                                                    <td style="border: 1px solid #ddd; padding: 8px; text-align: right; font-weight: bold;">
                                                                                        {format!("{:.2}", item.amount)}
                                                                                    </td>
                                                                                    <td style="border: 1px solid #ddd; padding: 8px;">
                                                                                        {if item.is_posted {
                                                                                            view! {
                                                                                                <span style="padding: 2px 6px; background: #c8e6c9; color: #2e7d32; border-radius: 3px;">
                                                                                                    "✓ Проведен"
                                                                                                </span>
                                                                                            }
                                                                                        } else {
                                                                                            view! {
                                                                                                <span style="padding: 2px 6px; background: #f5f5f5; color: #999; border-radius: 3px;">
                                                                                                    "○ Не проведен"
                                                                                                </span>
                                                                                            }
                                                                                        }}
                                                                                    </td>
                                                                                </tr>
                                                                            }
                                                                        }).collect_view()}
                                                                    </tbody>
                                                                </table>
                                                            </div>
                                                        }.into_any()
                                                    }
                                                }
                                            }}
                                        </div>
                                    }.into_any(),
                                    "json" => view! {
                                        <div class="json-info">
                                            <div style="margin-bottom: 10px;">
                                                <strong>"Raw JSON from OZON API:"</strong>
                                            </div>
                                            {move || {
                                                if let Some(json) = raw_json_from_ozon.get() {
                                                    view! {
                                                        <pre style="background: #f5f5f5; padding: 15px; border-radius: 4px; overflow-x: auto; font-size: 0.85em;">
                                                            {json}
                                                        </pre>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <div style="padding: 20px; text-align: center; color: #999;">
                                                            "Loading raw JSON from OZON..."
                                                        </div>
                                                    }.into_any()
                                                }
                                            }}
                                        </div>
                                    }.into_any(),
                                        _ => view! { <div>"Unknown tab"</div> }.into_any()
                                    }
                                    }}
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! { <div>"No data"</div> }.into_any()
                    }
                }}
            </div>

            // Transaction detail modal
            {move || selected_transaction_id.get().map(|transaction_id| {
                let close_transaction_detail = move || {
                    set_selected_transaction_id.set(None);
                };
                
                view! {
                    <OzonTransactionsDetail
                        transaction_id=transaction_id
                        on_close=close_transaction_detail
                    />
                }
            })}
        </div>
    }
}
