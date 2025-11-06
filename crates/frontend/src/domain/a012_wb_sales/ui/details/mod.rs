use leptos::prelude::*;
use leptos::logging::log;
use serde::{Deserialize, Serialize};
use gloo_net::http::Request;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbSalesDetailDto {
    pub id: String,
    pub code: String,
    pub description: String,
    pub header: HeaderDto,
    pub line: LineDto,
    pub state: StateDto,
    pub source_meta: SourceMetaDto,
    pub metadata: MetadataDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderDto {
    pub document_no: String,
    pub connection_id: String,
    pub organization_id: String,
    pub marketplace_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineDto {
    pub line_id: String,
    pub supplier_article: String,
    pub nm_id: i64,
    pub barcode: String,
    pub name: String,
    pub qty: f64,
    pub price_list: Option<f64>,
    pub discount_total: Option<f64>,
    pub price_effective: Option<f64>,
    pub amount_line: Option<f64>,
    pub currency_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDto {
    pub event_type: String,
    pub status_norm: String,
    pub sale_dt: String,
    pub last_change_dt: Option<String>,
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

#[component]
pub fn WbSalesDetail(
    id: String,
    #[prop(into)] on_close: Callback<()>,
) -> impl IntoView {
    let (sale, set_sale) = signal::<Option<WbSalesDetailDto>>(None);
    let (raw_json_from_wb, set_raw_json_from_wb) = signal::<Option<String>>(None);
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal::<Option<String>>(None);
    let (active_tab, set_active_tab) = signal("general");

    // Загрузить детальные данные
    Effect::new(move || {
        let id = id.clone();
        wasm_bindgen_futures::spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let url = format!("http://localhost:3000/api/a012/wb-sales/{}", id);

            match Request::get(&url).send().await {
                Ok(response) => {
                    let status = response.status();
                    if status == 200 {
                        match response.text().await {
                            Ok(text) => {
                                // Парсим структуру
                                match serde_json::from_str::<WbSalesDetailDto>(&text) {
                                    Ok(data) => {
                                        // Загружаем raw JSON от WB
                                        let raw_payload_ref = data.source_meta.raw_payload_ref.clone();
                                        set_sale.set(Some(data));
                                        set_loading.set(false);
                                        
                                        // Асинхронная загрузка raw JSON
                                        wasm_bindgen_futures::spawn_local(async move {
                                            let raw_url = format!("http://localhost:3000/api/a012/raw/{}", raw_payload_ref);
                                            match Request::get(&raw_url).send().await {
                                                Ok(resp) => {
                                                    if resp.status() == 200 {
                                                        if let Ok(text) = resp.text().await {
                                                            // Форматируем JSON
                                                            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&text) {
                                                                if let Ok(formatted) = serde_json::to_string_pretty(&json_value) {
                                                                    set_raw_json_from_wb.set(Some(formatted));
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    log!("Failed to load raw JSON from WB: {:?}", e);
                                                }
                                            }
                                        });
                                    }
                                    Err(e) => {
                                        log!("Failed to parse sale: {:?}", e);
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
                    log!("Failed to fetch sale: {:?}", e);
                    set_error.set(Some(format!("Failed to fetch: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    });

    view! {
        <div class="sale-detail" style="padding: 20px; height: 100%; display: flex; flex-direction: column;">
            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; flex-shrink: 0;">
                <h2 style="margin: 0;">"Wildberries Sales Details"</h2>
                <button
                    on:click=move |_| on_close.run(())
                    style="padding: 8px 16px; background: #f44336; color: white; border: none; border-radius: 4px; cursor: pointer;"
                >
                    "✕ Close"
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
                    } else if let Some(sale_data) = sale.get() {
                        view! {
                            <div style="height: 100%; display: flex; flex-direction: column;">
                                // Вкладки
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
                                        "📋 General"
                                    </button>
                                    <button
                                        on:click=move |_| set_active_tab.set("line")
                                        style=move || format!(
                                            "padding: 10px 20px; border: none; border-radius: 4px 4px 0 0; cursor: pointer; margin-right: 5px; font-weight: 500; {}",
                                            if active_tab.get() == "line" { 
                                                "background: #2196F3; color: white; border-bottom: 2px solid #2196F3;" 
                                            } else { 
                                                "background: #f5f5f5; color: #666;" 
                                            }
                                        )
                                    >
                                        "📦 Line Details"
                                    </button>
                                    <button
                                        on:click=move |_| set_active_tab.set("json")
                                        style=move || format!(
                                            "padding: 10px 20px; border: none; border-radius: 4px 4px 0 0; cursor: pointer; font-weight: 500; {}",
                                            if active_tab.get() == "json" { 
                                                "background: #2196F3; color: white; border-bottom: 2px solid #2196F3;" 
                                            } else { 
                                                "background: #f5f5f5; color: #666;" 
                                            }
                                        )
                                    >
                                        "📄 Raw JSON"
                                    </button>
                                </div>

                                // Контент вкладок
                                <div style="flex: 1; overflow-y: auto; padding: 10px 0;">
                                    {move || {
                                let tab = active_tab.get();
                                match tab.as_ref() {
                                    "general" => {
                                        // Helper для рендеринга UUID с кнопкой копирования
                                        let conn_id = sale_data.header.connection_id.clone();
                                        let org_id = sale_data.header.organization_id.clone();
                                        let mp_id = sale_data.header.marketplace_id.clone();
                                        
                                        view! {
                                            <div class="general-info">
                                                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 15px 20px; align-items: center;">
                                                    <div style="font-weight: 600; color: #555;">"Document №:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{sale_data.header.document_no.clone()}</div>
                                                    
                                                    <div style="font-weight: 600; color: #555;">"Code:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{sale_data.code.clone()}</div>
                                                    
                                                    <div style="font-weight: 600; color: #555;">"Description:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{sale_data.description.clone()}</div>
                                                    
                                                    <div style="font-weight: 600; color: #555;">"Event Type:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">
                                                        <span style="padding: 2px 8px; background: #e3f2fd; color: #1976d2; border-radius: 3px; font-weight: 500;">
                                                            {sale_data.state.event_type.clone()}
                                                        </span>
                                                    </div>
                                                    
                                                    <div style="font-weight: 600; color: #555;">"Status:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">
                                                        <span style="padding: 2px 8px; background: #e8f5e9; color: #2e7d32; border-radius: 3px; font-weight: 500;">
                                                            {sale_data.state.status_norm.clone()}
                                                        </span>
                                                    </div>
                                                    
                                                    <div style="font-weight: 600; color: #555;">"Sale Date:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{sale_data.state.sale_dt.clone()}</div>
                                                    
                                                    <div style="font-weight: 600; color: #555;">"Last Change Date:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{sale_data.state.last_change_dt.clone().unwrap_or("—".to_string())}</div>
                                                    
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
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{sale_data.metadata.created_at.clone()}</div>
                                                    
                                                    <div style="font-weight: 600; color: #555;">"Updated At:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{sale_data.metadata.updated_at.clone()}</div>
                                                    
                                                    <div style="font-weight: 600; color: #555;">"Version:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{sale_data.metadata.version}</div>
                                                </div>
                                            </div>
                                        }.into_any()
                                    },
                                    "line" => {
                                        let line = &sale_data.line;
                                        view! {
                                            <div class="line-info">
                                                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 15px 20px; align-items: center;">
                                                    <div style="font-weight: 600; color: #555;">"Line ID:"</div>
                                                    <div style="font-family: monospace; font-size: 14px;">{line.line_id.clone()}</div>
                                                    
                                                    <div style="font-weight: 600; color: #555;">"Supplier Article:"</div>
                                                    <div style="font-family: monospace; font-size: 14px; font-weight: 500;">{line.supplier_article.clone()}</div>
                                                    
                                                    <div style="font-weight: 600; color: #555;">"NM ID:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{line.nm_id}</div>
                                                    
                                                    <div style="font-weight: 600; color: #555;">"Barcode:"</div>
                                                    <div style="font-family: monospace; font-size: 14px;">{line.barcode.clone()}</div>
                                                    
                                                    <div style="font-weight: 600; color: #555;">"Name:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px; font-weight: 500;">{line.name.clone()}</div>
                                                    
                                                    <div style="font-weight: 600; color: #555;">"Quantity:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">
                                                        <span style="padding: 2px 8px; background: #e3f2fd; color: #1976d2; border-radius: 3px; font-weight: 500;">
                                                            {format!("{:.0}", line.qty)}
                                                        </span>
                                                    </div>
                                                    
                                                    <div style="font-weight: 600; color: #555;">"Price List:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">
                                                        {line.price_list.map(|p| format!("{:.2}", p)).unwrap_or("—".to_string())}
                                                        {line.currency_code.as_ref().map(|c| format!(" {}", c)).unwrap_or_default()}
                                                    </div>
                                                    
                                                    <div style="font-weight: 600; color: #555;">"Discount Total:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">
                                                        {line.discount_total.map(|d| format!("{:.2}", d)).unwrap_or("—".to_string())}
                                                        {line.currency_code.as_ref().map(|c| format!(" {}", c)).unwrap_or_default()}
                                                    </div>
                                                    
                                                    <div style="font-weight: 600; color: #555;">"Price Effective:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">
                                                        {line.price_effective.map(|p| format!("{:.2}", p)).unwrap_or("—".to_string())}
                                                        {line.currency_code.as_ref().map(|c| format!(" {}", c)).unwrap_or_default()}
                                                    </div>
                                                    
                                                    <div style="font-weight: 600; color: #555;">"Amount Line:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 16px; font-weight: bold; color: #2e7d32;">
                                                        {line.amount_line.map(|a| format!("{:.2}", a)).unwrap_or("—".to_string())}
                                                        {line.currency_code.as_ref().map(|c| format!(" {}", c)).unwrap_or_default()}
                                                    </div>
                                                    
                                                    <div style="font-weight: 600; color: #555;">"Currency Code:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">
                                                        {line.currency_code.clone().unwrap_or("—".to_string())}
                                                    </div>
                                                </div>
                                            </div>
                                        }.into_any()
                                    },
                                    "json" => view! {
                                        <div class="json-info">
                                            <div style="margin-bottom: 10px;">
                                                <strong>"Raw JSON from WB API:"</strong>
                                            </div>
                                            {move || {
                                                if let Some(json) = raw_json_from_wb.get() {
                                                    view! {
                                                        <pre style="background: #f5f5f5; padding: 15px; border-radius: 4px; overflow-x: auto; font-size: 0.85em;">
                                                            {json}
                                                        </pre>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <div style="padding: 20px; text-align: center; color: #999;">
                                                            "Loading raw JSON from WB..."
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
        </div>
    }
}

