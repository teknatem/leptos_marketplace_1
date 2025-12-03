use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmReturnDetailDto {
    pub id: String,
    pub code: String,
    pub description: String,
    pub header: HeaderDto,
    pub lines: Vec<LineDto>,
    pub state: StateDto,
    pub source_meta: SourceMetaDto,
    pub is_posted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderDto {
    pub return_id: i64,
    pub order_id: i64,
    pub connection_id: String,
    pub organization_id: String,
    pub marketplace_id: String,
    pub campaign_id: String,
    pub return_type: String,
    pub amount: Option<f64>,
    pub currency: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineDto {
    pub item_id: i64,
    pub shop_sku: String,
    pub offer_id: String,
    pub name: String,
    pub count: i32,
    pub price: Option<f64>,
    pub return_reason: Option<String>,
    pub decisions: Vec<DecisionDto>,
    #[serde(default)]
    pub photos: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionDto {
    pub decision_type: String,
    pub amount: Option<f64>,
    pub currency: Option<String>,
    pub partner_compensation_amount: Option<f64>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDto {
    pub refund_status: String,
    pub created_at_source: Option<String>,
    pub updated_at_source: Option<String>,
    pub refund_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMetaDto {
    pub raw_payload_ref: String,
    pub fetched_at: String,
    pub document_version: i32,
}

#[component]
pub fn YmReturnDetail(id: String, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let (return_data, set_return_data) = signal::<Option<YmReturnDetailDto>>(None);
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

            let url = format!("http://localhost:3000/api/a016/ym-returns/{}", id);

            match Request::get(&url).send().await {
                Ok(response) => {
                    let status = response.status();
                    if status == 200 {
                        match response.text().await {
                            Ok(text) => {
                                match serde_json::from_str::<YmReturnDetailDto>(&text) {
                                    Ok(data) => {
                                        let raw_payload_ref = data.source_meta.raw_payload_ref.clone();
                                        set_return_data.set(Some(data));
                                        set_loading.set(false);

                                        // Асинхронная загрузка raw JSON
                                        wasm_bindgen_futures::spawn_local(async move {
                                            let raw_url = format!(
                                                "http://localhost:3000/api/a016/raw/{}",
                                                raw_payload_ref
                                            );
                                            match Request::get(&raw_url).send().await {
                                                Ok(resp) => {
                                                    if resp.status() == 200 {
                                                        if let Ok(text) = resp.text().await {
                                                            if let Ok(json_value) =
                                                                serde_json::from_str::<serde_json::Value>(
                                                                    &text,
                                                                )
                                                            {
                                                                if let Ok(formatted) =
                                                                    serde_json::to_string_pretty(
                                                                        &json_value,
                                                                    )
                                                                {
                                                                    set_raw_json_from_ym
                                                                        .set(Some(formatted));
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    log!("Failed to load raw JSON: {:?}", e);
                                                }
                                            }
                                        });
                                    }
                                    Err(e) => {
                                        log!("Failed to parse return: {:?}", e);
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
                    log!("Failed to fetch return: {:?}", e);
                    set_error.set(Some(format!("Failed to fetch: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    });

    view! {
        <div class="return-detail" style="padding: 20px; height: 100%; display: flex; flex-direction: column;">
            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; flex-shrink: 0;">
                <h2 style="margin: 0;">"Yandex Market Return Details"</h2>
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
                    } else if let Some(data) = return_data.get() {
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
                                        "📦 Items"
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
                                                let return_type_label = match data.header.return_type.as_str() {
                                                    "UNREDEEMED" => "Невыкуп",
                                                    "RETURN" => "Возврат",
                                                    _ => data.header.return_type.as_str(),
                                                };
                                                let return_type_style = match data.header.return_type.as_str() {
                                                    "UNREDEEMED" => "background: #fff3e0; color: #e65100;",
                                                    "RETURN" => "background: #e3f2fd; color: #1565c0;",
                                                    _ => "background: #f5f5f5; color: #666;",
                                                };
                                                let refund_status_style = match data.state.refund_status.as_str() {
                                                    "REFUNDED" => "background: #e8f5e9; color: #2e7d32;",
                                                    "NOT_REFUNDED" => "background: #ffebee; color: #c62828;",
                                                    "REFUND_IN_PROGRESS" => "background: #fff3e0; color: #e65100;",
                                                    _ => "background: #f5f5f5; color: #666;",
                                                };

                                                view! {
                                                    <div class="general-info">
                                                        <div style="display: grid; grid-template-columns: 200px 1fr; gap: 15px 20px; align-items: center;">
                                                            <div style="font-weight: 600; color: #555;">"Return №:"</div>
                                                            <div style="font-family: monospace; font-size: 16px; font-weight: bold; color: #1976d2;">{data.header.return_id}</div>

                                                            <div style="font-weight: 600; color: #555;">"Order №:"</div>
                                                            <div style="font-family: monospace; font-size: 14px;">{data.header.order_id}</div>

                                                            <div style="font-weight: 600; color: #555;">"Type:"</div>
                                                            <div>
                                                                <span style={format!("padding: 4px 12px; border-radius: 4px; font-weight: 500; {}", return_type_style)}>
                                                                    {return_type_label}
                                                                </span>
                                                            </div>

                                                            <div style="font-weight: 600; color: #555;">"Refund Status:"</div>
                                                            <div>
                                                                <span style={format!("padding: 4px 12px; border-radius: 4px; font-weight: 500; {}", refund_status_style)}>
                                                                    {data.state.refund_status.clone()}
                                                                </span>
                                                            </div>

                                                            <div style="font-weight: 600; color: #555;">"Amount:"</div>
                                                            <div style="font-size: 16px; font-weight: bold; color: #c62828;">
                                                                {data.header.amount.map(|a| format!("{:.2}", a)).unwrap_or("—".to_string())}
                                                                {data.header.currency.clone().map(|c| format!(" {}", c)).unwrap_or_default()}
                                                            </div>

                                                            <div style="font-weight: 600; color: #555;">"Campaign ID:"</div>
                                                            <div style="font-family: monospace; font-size: 14px;">{data.header.campaign_id.clone()}</div>

                                                            <div style="font-weight: 600; color: #555;">"Created At Source:"</div>
                                                            <div>{data.state.created_at_source.clone().unwrap_or("—".to_string())}</div>

                                                            <div style="font-weight: 600; color: #555;">"Updated At Source:"</div>
                                                            <div>{data.state.updated_at_source.clone().unwrap_or("—".to_string())}</div>

                                                            <div style="font-weight: 600; color: #555;">"Fetched At:"</div>
                                                            <div>{data.source_meta.fetched_at.clone()}</div>

                                                            <div style="font-weight: 600; color: #555;">"Document Version:"</div>
                                                            <div>{data.source_meta.document_version}</div>

                                                            <div style="font-weight: 600; color: #555;">"Is Posted:"</div>
                                                            <div>
                                                                {if data.is_posted {
                                                                    view! { <span style="color: #2e7d32; font-weight: 500;">"✓ Yes"</span> }.into_any()
                                                                } else {
                                                                    view! { <span style="color: #999;">"No"</span> }.into_any()
                                                                }}
                                                            </div>
                                                        </div>
                                                    </div>
                                                }.into_any()
                                            },
                                            "lines" => {
                                                let lines = &data.lines;
                                                let total_items: i32 = lines.iter().map(|l| l.count).sum();
                                                let total_amount: f64 = lines.iter().filter_map(|l| l.price.map(|p| p * l.count as f64)).sum();

                                                view! {
                                                    <div class="lines-info">
                                                        <div style="margin-bottom: 15px; padding: 10px; background: #ffebee; border-radius: 4px;">
                                                            <strong>"Return Summary: "</strong>
                                                            {format!("{} items, {} total units, {:.2} total amount",
                                                                lines.len(),
                                                                total_items,
                                                                total_amount
                                                            )}
                                                        </div>

                                                        <table style="width: 100%; border-collapse: collapse; font-size: 14px;">
                                                            <thead>
                                                                <tr style="background: #f5f5f5;">
                                                                    <th style="border: 1px solid #ddd; padding: 8px; text-align: left;">"Shop SKU"</th>
                                                                    <th style="border: 1px solid #ddd; padding: 8px; text-align: left;">"Name"</th>
                                                                    <th style="border: 1px solid #ddd; padding: 8px; text-align: right;">"Count"</th>
                                                                    <th style="border: 1px solid #ddd; padding: 8px; text-align: right;">"Price"</th>
                                                                    <th style="border: 1px solid #ddd; padding: 8px; text-align: left;">"Reason"</th>
                                                                    <th style="border: 1px solid #ddd; padding: 8px; text-align: left;">"Decision"</th>
                                                                </tr>
                                                            </thead>
                                                            <tbody>
                                                                {lines.iter().map(|line| {
                                                                    let decision_info = line.decisions.first().map(|d| {
                                                                        let amount_str = d.amount.map(|a| format!("{:.2}", a)).unwrap_or("—".to_string());
                                                                        format!("{} ({})", d.decision_type, amount_str)
                                                                    }).unwrap_or("—".to_string());

                                                                    view! {
                                                                        <tr>
                                                                            <td style="border: 1px solid #ddd; padding: 8px;">
                                                                                <code style="font-size: 0.85em;">{line.shop_sku.clone()}</code>
                                                                            </td>
                                                                            <td style="border: 1px solid #ddd; padding: 8px;">{line.name.clone()}</td>
                                                                            <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">
                                                                                <strong>{line.count}</strong>
                                                                            </td>
                                                                            <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">
                                                                                {line.price.map(|p| format!("{:.2}", p)).unwrap_or("—".to_string())}
                                                                            </td>
                                                                            <td style="border: 1px solid #ddd; padding: 8px; font-size: 0.85em;">
                                                                                {line.return_reason.clone().unwrap_or("—".to_string())}
                                                                            </td>
                                                                            <td style="border: 1px solid #ddd; padding: 8px; font-size: 0.85em;">
                                                                                {decision_info}
                                                                            </td>
                                                                        </tr>
                                                                    }
                                                                }).collect_view()}
                                                                <tr style="background: #f5f5f5; font-weight: bold;">
                                                                    <td colspan="2" style="border: 1px solid #ddd; padding: 8px; text-align: right;">"Total:"</td>
                                                                    <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{total_items}</td>
                                                                    <td style="border: 1px solid #ddd; padding: 8px; text-align: right; color: #c62828;">{format!("{:.2}", total_amount)}</td>
                                                                    <td colspan="2" style="border: 1px solid #ddd; padding: 8px;"></td>
                                                                </tr>
                                                            </tbody>
                                                        </table>
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

