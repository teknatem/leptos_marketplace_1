use leptos::prelude::*;
use leptos::logging::log;
use serde::{Deserialize, Serialize};
use gloo_net::http::Request;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmOrderDetailDto {
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
    pub connection_id: String,
    pub organization_id: String,
    pub marketplace_id: String,
    pub campaign_id: String,
    pub total_amount: Option<f64>,
    pub currency: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineDto {
    pub line_id: String,
    pub shop_sku: String,
    pub offer_id: String,
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
    pub status_raw: String,
    pub substatus_raw: Option<String>,
    pub status_norm: String,
    pub status_changed_at: Option<String>,
    pub updated_at_source: Option<String>,
    pub creation_date: Option<String>,
    pub delivery_date: Option<String>,
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
pub fn YmOrderDetail(
    id: String,
    #[prop(into)] on_close: Callback<()>,
) -> impl IntoView {
    let (order, set_order) = signal::<Option<YmOrderDetailDto>>(None);
    let (raw_json_from_ym, set_raw_json_from_ym) = signal::<Option<String>>(None);
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal::<Option<String>>(None);
    let (active_tab, set_active_tab) = signal("general");

    // Загрузить детальные данные
    Effect::new(move || {
        let id = id.clone();
        wasm_bindgen_futures::spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let url = format!("http://localhost:3000/api/a013/ym-order/{}", id);

            match Request::get(&url).send().await {
                Ok(response) => {
                    let status = response.status();
                    if status == 200 {
                        match response.text().await {
                            Ok(text) => {
                                // Парсим структуру
                                match serde_json::from_str::<YmOrderDetailDto>(&text) {
                                    Ok(data) => {
                                        // Загружаем raw JSON от YM
                                        let raw_payload_ref = data.source_meta.raw_payload_ref.clone();
                                        set_order.set(Some(data));
                                        set_loading.set(false);

                                        // Асинхронная загрузка raw JSON
                                        wasm_bindgen_futures::spawn_local(async move {
                                            let raw_url = format!("http://localhost:3000/api/a013/raw/{}", raw_payload_ref);
                                            match Request::get(&raw_url).send().await {
                                                Ok(resp) => {
                                                    if resp.status() == 200 {
                                                        if let Ok(text) = resp.text().await {
                                                            // Форматируем JSON
                                                            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&text) {
                                                                if let Ok(formatted) = serde_json::to_string_pretty(&json_value) {
                                                                    set_raw_json_from_ym.set(Some(formatted));
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    log!("Failed to load raw JSON from YM: {:?}", e);
                                                }
                                            }
                                        });
                                    }
                                    Err(e) => {
                                        log!("Failed to parse order: {:?}", e);
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
                    log!("Failed to fetch order: {:?}", e);
                    set_error.set(Some(format!("Failed to fetch: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    });

    view! {
        <div class="order-detail" style="padding: 20px; height: 100%; display: flex; flex-direction: column;">
            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; flex-shrink: 0;">
                <h2 style="margin: 0;">"Yandex Market Order Details"</h2>
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
                    } else if let Some(order_data) = order.get() {
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
                                        on:click=move |_| set_active_tab.set("lines")
                                        style=move || format!(
                                            "padding: 10px 20px; border: none; border-radius: 4px 4px 0 0; cursor: pointer; margin-right: 5px; font-weight: 500; {}",
                                            if active_tab.get() == "lines" {
                                                "background: #2196F3; color: white; border-bottom: 2px solid #2196F3;"
                                            } else {
                                                "background: #f5f5f5; color: #666;"
                                            }
                                        )
                                    >
                                        "📦 Lines"
                                    </button>
                                    <button
                                        on:click=move |_| set_active_tab.set("campaign")
                                        style=move || format!(
                                            "padding: 10px 20px; border: none; border-radius: 4px 4px 0 0; cursor: pointer; margin-right: 5px; font-weight: 500; {}",
                                            if active_tab.get() == "campaign" {
                                                "background: #2196F3; color: white; border-bottom: 2px solid #2196F3;"
                                            } else {
                                                "background: #f5f5f5; color: #666;"
                                            }
                                        )
                                    >
                                        "🏢 Campaign"
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
                                        let conn_id = order_data.header.connection_id.clone();
                                        let org_id = order_data.header.organization_id.clone();
                                        let mp_id = order_data.header.marketplace_id.clone();

                                        view! {
                                            <div class="general-info">
                                                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 15px 20px; align-items: center;">
                                                    <div style="font-weight: 600; color: #555;">"Order №:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{order_data.header.document_no.clone()}</div>

                                                    <div style="font-weight: 600; color: #555;">"Code:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{order_data.code.clone()}</div>

                                                    <div style="font-weight: 600; color: #555;">"Description:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{order_data.description.clone()}</div>

                                                    <div style="font-weight: 600; color: #555;">"Status:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">
                                                        <span style="padding: 2px 8px; background: #e8f5e9; color: #2e7d32; border-radius: 3px; font-weight: 500;">
                                                            {order_data.state.status_norm.clone()}
                                                        </span>
                                                    </div>

                                                    <div style="font-weight: 600; color: #555;">"Raw Status:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">
                                                        <span style="padding: 2px 8px; background: #e3f2fd; color: #1976d2; border-radius: 3px; font-weight: 500;">
                                                            {order_data.state.status_raw.clone()}
                                                        </span>
                                                    </div>

                                                    <div style="font-weight: 600; color: #555;">"Substatus:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">
                                                        {order_data.state.substatus_raw.clone().unwrap_or("—".to_string())}
                                                    </div>

                                                    <div style="font-weight: 600; color: #555;">"Status Changed At:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{order_data.state.status_changed_at.clone().unwrap_or("—".to_string())}</div>

                                                    <div style="font-weight: 600; color: #555;">"Updated At Source:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{order_data.state.updated_at_source.clone().unwrap_or("—".to_string())}</div>

                                                    <div style="font-weight: 600; color: #555;">"Creation Date:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px; font-weight: 500; color: #1976d2;">{order_data.state.creation_date.clone().unwrap_or("—".to_string())}</div>

                                                    <div style="font-weight: 600; color: #555;">"Delivery Date:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px; font-weight: 500; color: #2e7d32;">{order_data.state.delivery_date.clone().unwrap_or("—".to_string())}</div>

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
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{order_data.metadata.created_at.clone()}</div>

                                                    <div style="font-weight: 600; color: #555;">"Updated At:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{order_data.metadata.updated_at.clone()}</div>

                                                    <div style="font-weight: 600; color: #555;">"Version:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{order_data.metadata.version}</div>
                                                </div>
                                            </div>
                                        }.into_any()
                                    },
                                    "lines" => {
                                        let lines = &order_data.lines;
                                        let total_qty: f64 = lines.iter().map(|l| l.qty).sum();
                                        let total_amount: f64 = lines.iter().filter_map(|l| l.amount_line).sum();

                                        view! {
                                            <div class="lines-info">
                                                <div style="margin-bottom: 15px; padding: 10px; background: #e8f5e9; border-radius: 4px;">
                                                    <strong>"Order Summary: "</strong>
                                                    {format!("{} lines, {} items total, {:.2} total amount",
                                                        lines.len(),
                                                        total_qty,
                                                        total_amount
                                                    )}
                                                </div>

                                                <table style="width: 100%; border-collapse: collapse; font-size: 14px;">
                                                    <thead>
                                                        <tr style="background: #f5f5f5;">
                                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: left;">"Shop SKU"</th>
                                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: left;">"Offer ID"</th>
                                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: left;">"Name"</th>
                                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: right;">"Qty"</th>
                                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: right;">"Price"</th>
                                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: right;">"Discount"</th>
                                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: right;">"Effective"</th>
                                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: right;">"Amount"</th>
                                                        </tr>
                                                    </thead>
                                                    <tbody>
                                                        {lines.iter().map(|line| {
                                                            view! {
                                                                <tr>
                                                                    <td style="border: 1px solid #ddd; padding: 8px;">
                                                                        <code style="font-size: 0.85em;">{line.shop_sku.clone()}</code>
                                                                    </td>
                                                                    <td style="border: 1px solid #ddd; padding: 8px;">
                                                                        <code style="font-size: 0.85em;">{line.offer_id.clone()}</code>
                                                                    </td>
                                                                    <td style="border: 1px solid #ddd; padding: 8px;">{line.name.clone()}</td>
                                                                    <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">
                                                                        <strong>{format!("{:.0}", line.qty)}</strong>
                                                                    </td>
                                                                    <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">
                                                                        {line.price_list.map(|p| format!("{:.2}", p)).unwrap_or("—".to_string())}
                                                                    </td>
                                                                    <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">
                                                                        {line.discount_total.map(|d| format!("{:.2}", d)).unwrap_or("—".to_string())}
                                                                    </td>
                                                                    <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">
                                                                        {line.price_effective.map(|p| format!("{:.2}", p)).unwrap_or("—".to_string())}
                                                                    </td>
                                                                    <td style="border: 1px solid #ddd; padding: 8px; text-align: right; font-weight: bold; color: #2e7d32;">
                                                                        {line.amount_line.map(|a| format!("{:.2}", a)).unwrap_or("—".to_string())}
                                                                    </td>
                                                                </tr>
                                                            }
                                                        }).collect_view()}
                                                        <tr style="background: #f5f5f5; font-weight: bold;">
                                                            <td colspan="3" style="border: 1px solid #ddd; padding: 8px; text-align: right;">"Total:"</td>
                                                            <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{format!("{:.0}", total_qty)}</td>
                                                            <td colspan="3" style="border: 1px solid #ddd; padding: 8px;"></td>
                                                            <td style="border: 1px solid #ddd; padding: 8px; text-align: right; color: #2e7d32;">{format!("{:.2}", total_amount)}</td>
                                                        </tr>
                                                    </tbody>
                                                </table>
                                            </div>
                                        }.into_any()
                                    },
                                    "campaign" => {
                                        let campaign_id = order_data.header.campaign_id.clone();

                                        view! {
                                            <div class="campaign-info">
                                                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 15px 20px; align-items: center;">
                                                    <div style="font-weight: 600; color: #555;">"Campaign ID:"</div>
                                                    <div style="display: flex; align-items: center; gap: 8px; font-family: monospace; font-size: 14px;">
                                                        <span style="color: #666;" title=campaign_id.clone()>{campaign_id.clone()}</span>
                                                        <button
                                                            on:click=move |_| {
                                                                let id_copy = campaign_id.clone();
                                                                wasm_bindgen_futures::spawn_local(async move {
                                                                    if let Some(window) = web_sys::window() {
                                                                        let nav = window.navigator().clipboard();
                                                                        let _ = nav.write_text(&id_copy);
                                                                    }
                                                                });
                                                            }
                                                            style="padding: 2px 6px; font-size: 11px; border: 1px solid #ddd; background: white; border-radius: 3px; cursor: pointer;"
                                                            title="Copy to clipboard"
                                                        >
                                                            "📋"
                                                        </button>
                                                    </div>

                                                    <div style="font-weight: 600; color: #555;">"Marketplace:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">
                                                        <span style="padding: 2px 8px; background: #fff3e0; color: #e65100; border-radius: 3px; font-weight: 500;">
                                                            "Yandex Market"
                                                        </span>
                                                    </div>

                                                    <div style="font-weight: 600; color: #555;">"Fetched At:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{order_data.source_meta.fetched_at.clone()}</div>

                                                    <div style="font-weight: 600; color: #555;">"Document Version:"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{order_data.source_meta.document_version}</div>

                                                    <div style="font-weight: 600; color: #555;">"Total Amount (API):"</div>
                                                    <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 16px; font-weight: bold; color: #2e7d32;">
                                                        {order_data.header.total_amount.map(|a| format!("{:.2}", a)).unwrap_or("—".to_string())}
                                                        {order_data.header.currency.as_ref().map(|c| format!(" {}", c)).unwrap_or_default()}
                                                    </div>
                                                </div>
                                            </div>
                                        }.into_any()
                                    },
                                    "json" => view! {
                                        <div class="json-info">
                                            <div style="margin-bottom: 10px;">
                                                <strong>"Raw JSON from Yandex Market API:"</strong>
                                            </div>
                                            {move || {
                                                if let Some(json) = raw_json_from_ym.get() {
                                                    view! {
                                                        <pre style="background: #f5f5f5; padding: 15px; border-radius: 4px; overflow-x: auto; font-size: 0.85em;">
                                                            {json}
                                                        </pre>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <div style="padding: 20px; text-align: center; color: #999;">
                                                            "Loading raw JSON from Yandex Market..."
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
