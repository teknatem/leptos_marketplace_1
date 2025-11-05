use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

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
pub struct SalesRegisterListResponse {
    pub items: Vec<SalesRegisterDto>,
    pub total_count: i32,
    pub has_more: bool,
}

#[component]
pub fn SalesRegisterList() -> impl IntoView {
    let (sales, set_sales) = signal(Vec::<SalesRegisterDto>::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(None::<String>);

    // Фильтры
    let (date_from, set_date_from) = signal("2024-01-01".to_string());
    let (date_to, set_date_to) = signal("2024-12-31".to_string());
    let (marketplace_filter, set_marketplace_filter) = signal("".to_string());

    let load_sales = move || {
        set_loading.set(true);
        set_error.set(None);

        let date_from_val = date_from.get();
        let date_to_val = date_to.get();
        let marketplace_val = marketplace_filter.get();

        let mut query_params = format!(
            "?date_from={}&date_to={}&limit=100&offset=0",
            date_from_val, date_to_val
        );

        if !marketplace_val.is_empty() {
            query_params.push_str(&format!("&marketplace={}", marketplace_val));
        }

        spawn_local(async move {
            match fetch_sales(&query_params).await {
                Ok(data) => {
                    set_sales.set(data.items);
                    set_loading.set(false);
                }
                Err(e) => {
                    log!("Failed to fetch sales: {:?}", e);
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    };

    view! {
        <div class="sales-register-list">
            <h2>"Sales Register (P900)"</h2>

            // Фильтры
            <div class="filters" style="margin: 20px 0; padding: 15px; border: 1px solid #ddd; border-radius: 4px;">
                <div style="display: flex; gap: 15px; align-items: flex-end;">
                    <div>
                        <label>"Date From:"</label>
                        <input
                            type="date"
                            prop:value=date_from
                            on:input=move |ev| {
                                set_date_from.set(event_target_value(&ev));
                            }
                            style="margin-left: 10px; padding: 5px;"
                        />
                    </div>

                    <div>
                        <label>"Date To:"</label>
                        <input
                            type="date"
                            prop:value=date_to
                            on:input=move |ev| {
                                set_date_to.set(event_target_value(&ev));
                            }
                            style="margin-left: 10px; padding: 5px;"
                        />
                    </div>

                    <div>
                        <label>"Marketplace:"</label>
                        <select
                            prop:value=marketplace_filter
                            on:change=move |ev| {
                                set_marketplace_filter.set(event_target_value(&ev));
                            }
                            style="margin-left: 10px; padding: 5px;"
                        >
                            <option value="">"All"</option>
                            <option value="OZON">"OZON"</option>
                            <option value="WILDBERRIES">"Wildberries"</option>
                            <option value="YANDEX_MARKET">"Yandex Market"</option>
                        </select>
                    </div>

                    <button
                        on:click=move |_| {
                            load_sales();
                        }
                        style="padding: 8px 16px; background: #4CAF50; color: white; border: none; border-radius: 4px; cursor: pointer;"
                    >
                        "Load Sales"
                    </button>
                </div>
            </div>

            {move || {
                if loading.get() {
                    view! { <div>"Loading..."</div> }.into_any()
                } else if let Some(err) = error.get() {
                    view! { <div style="color: red;">{err}</div> }.into_any()
                } else {
                    view! {
                        <div>
                            <p>"Total: " {sales.get().len()} " records"</p>
                            <table class="data-table" style="width: 100%; border-collapse: collapse; margin-top: 20px;">
                                <thead>
                                    <tr style="background: #f5f5f5;">
                                        <th style="border: 1px solid #ddd; padding: 8px;">"Date"</th>
                                        <th style="border: 1px solid #ddd; padding: 8px;">"Marketplace"</th>
                                        <th style="border: 1px solid #ddd; padding: 8px;">"Document №"</th>
                                        <th style="border: 1px solid #ddd; padding: 8px;">"Product"</th>
                                        <th style="border: 1px solid #ddd; padding: 8px;">"SKU"</th>
                                        <th style="border: 1px solid #ddd; padding: 8px;">"Qty"</th>
                                        <th style="border: 1px solid #ddd; padding: 8px;">"Amount"</th>
                                        <th style="border: 1px solid #ddd; padding: 8px;">"Status"</th>
                                        <th style="border: 1px solid #ddd; padding: 8px;">"Organization"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {sales.get().into_iter().map(|sale| {
                                        let sale_date = sale.sale_date.clone();
                                        let marketplace = sale.marketplace.clone();
                                        let document_no = sale.document_no.clone();
                                        let line_id = sale.line_id.clone();
                                        let title = sale.title.clone().unwrap_or_default();
                                        let seller_sku = sale.seller_sku.clone().unwrap_or_default();
                                        let qty = sale.qty;
                                        let amount_line = sale.amount_line.unwrap_or(0.0);
                                        let status_norm = sale.status_norm.clone();
                                        let org_ref = sale.organization_ref.clone();
                                        let org_ref_short = org_ref[..8.min(org_ref.len())].to_string();
                                        let marketplace_c = marketplace.clone();
                                        let document_no_c = document_no.clone();
                                        let line_id_c = line_id.clone();

                                        view! {
                                            <tr>
                                                <td style="border: 1px solid #ddd; padding: 8px;">{sale_date}</td>
                                                <td style="border: 1px solid #ddd; padding: 8px;">{marketplace}</td>
                                                <td style="border: 1px solid #ddd; padding: 8px;">
                                                    <a href=format!("/p900/details/{}/{}/{}", marketplace_c, document_no_c, line_id_c)>
                                                        {document_no}
                                                    </a>
                                                </td>
                                                <td style="border: 1px solid #ddd; padding: 8px;">{title}</td>
                                                <td style="border: 1px solid #ddd; padding: 8px;">{seller_sku}</td>
                                                <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{format!("{:.2}", qty)}</td>
                                                <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{format!("{:.2}", amount_line)}</td>
                                                <td style="border: 1px solid #ddd; padding: 8px;">{status_norm}</td>
                                                <td style="border: 1px solid #ddd; padding: 8px;">
                                                    // UUID ссылка на организацию
                                                    <span title=org_ref style="font-family: monospace; font-size: 0.85em; color: #666;">
                                                        {org_ref_short}
                                                        "..."
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
            }}
        </div>
    }
}

async fn fetch_sales(query_params: &str) -> Result<SalesRegisterListResponse, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("/api/p900/sales-register{}", query_params);
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
    let data: SalesRegisterListResponse =
        serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}
