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
pub struct SalesRegisterDetailDto {
    pub sale: SalesRegisterDto,
    pub organization_name: Option<String>,
    pub connection_mp_name: Option<String>,
    pub marketplace_product_name: Option<String>,
}

#[component]
pub fn SalesRegisterDetails(
    marketplace: String,
    document_no: String,
    line_id: String,
) -> impl IntoView {
    let (sale, set_sale) = signal(None::<SalesRegisterDetailDto>);
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(None::<String>);

    Effect::new(move |_| {
        let marketplace = marketplace.clone();
        let document_no = document_no.clone();
        let line_id = line_id.clone();

        spawn_local(async move {
            match fetch_sale_detail(&marketplace, &document_no, &line_id).await {
                Ok(data) => {
                    set_sale.set(Some(data));
                    set_loading.set(false);
                }
                Err(e) => {
                    log!("Failed to fetch sale detail: {:?}", e);
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    });

    view! {
        <div class="sales-register-details">
            <h2>"Sales Register Details"</h2>

            <div style="margin-bottom: 20px;">
                <a href="/p900/list">"← Back to List"</a>
            </div>

            {move || {
                if loading.get() {
                    view! { <div>"Loading..."</div> }.into_any()
                } else if let Some(err) = error.get() {
                    view! { <div style="color: red;">{err}</div> }.into_any()
                } else if let Some(detail) = sale.get() {
                    let s = detail.sale;
                    view! {
                        <div class="details-container" style="border: 1px solid #ddd; border-radius: 4px; padding: 20px;">
                            <h3>"Basic Information"</h3>
                            <table style="width: 100%; margin-bottom: 20px;">
                                <tr>
                                    <td style="font-weight: bold; padding: 8px; width: 200px;">"Marketplace:"</td>
                                    <td style="padding: 8px;">{s.marketplace.clone()}</td>
                                </tr>
                                <tr>
                                    <td style="font-weight: bold; padding: 8px;">"Document №:"</td>
                                    <td style="padding: 8px;">{s.document_no.clone()}</td>
                                </tr>
                                <tr>
                                    <td style="font-weight: bold; padding: 8px;">"Line ID:"</td>
                                    <td style="padding: 8px;">{s.line_id.clone()}</td>
                                </tr>
                                <tr>
                                    <td style="font-weight: bold; padding: 8px;">"Sale Date:"</td>
                                    <td style="padding: 8px;">{s.sale_date.clone()}</td>
                                </tr>
                                <tr>
                                    <td style="font-weight: bold; padding: 8px;">"Status:"</td>
                                    <td style="padding: 8px;">{s.status_norm.clone()}</td>
                                </tr>
                            </table>

                            <h3>"Product Information"</h3>
                            <table style="width: 100%; margin-bottom: 20px;">
                                <tr>
                                    <td style="font-weight: bold; padding: 8px; width: 200px;">"Title:"</td>
                                    <td style="padding: 8px;">{s.title.clone().unwrap_or_default()}</td>
                                </tr>
                                <tr>
                                    <td style="font-weight: bold; padding: 8px;">"Seller SKU:"</td>
                                    <td style="padding: 8px;">{s.seller_sku.clone().unwrap_or_default()}</td>
                                </tr>
                                <tr>
                                    <td style="font-weight: bold; padding: 8px;">"MP Item ID:"</td>
                                    <td style="padding: 8px;">{s.mp_item_id.clone()}</td>
                                </tr>
                                <tr>
                                    <td style="font-weight: bold; padding: 8px;">"Barcode:"</td>
                                    <td style="padding: 8px;">{s.barcode.clone().unwrap_or_default()}</td>
                                </tr>
                            </table>

                            <h3>"Financial Information"</h3>
                            <table style="width: 100%; margin-bottom: 20px;">
                                <tr>
                                    <td style="font-weight: bold; padding: 8px; width: 200px;">"Quantity:"</td>
                                    <td style="padding: 8px;">{format!("{:.2}", s.qty)}</td>
                                </tr>
                                <tr>
                                    <td style="font-weight: bold; padding: 8px;">"Price List:"</td>
                                    <td style="padding: 8px;">{s.price_list.map(|v| format!("{:.2}", v)).unwrap_or_default()}</td>
                                </tr>
                                <tr>
                                    <td style="font-weight: bold; padding: 8px;">"Discount:"</td>
                                    <td style="padding: 8px;">{s.discount_total.map(|v| format!("{:.2}", v)).unwrap_or_default()}</td>
                                </tr>
                                <tr>
                                    <td style="font-weight: bold; padding: 8px;">"Price Effective:"</td>
                                    <td style="padding: 8px;">{s.price_effective.map(|v| format!("{:.2}", v)).unwrap_or_default()}</td>
                                </tr>
                                <tr>
                                    <td style="font-weight: bold; padding: 8px;">"Amount Line:"</td>
                                    <td style="padding: 8px; font-weight: bold; font-size: 1.1em;">
                                        {s.amount_line.map(|v| format!("{:.2}", v)).unwrap_or_default()}
                                        " "
                                        {s.currency_code.clone().unwrap_or_default()}
                                    </td>
                                </tr>
                            </table>

                            <h3>"References (UUID Links)"</h3>
                            <table style="width: 100%; margin-bottom: 20px;">
                                <tr>
                                    <td style="font-weight: bold; padding: 8px; width: 200px;">"Organization:"</td>
                                    <td style="padding: 8px;">
                                        <code style="background: #f5f5f5; padding: 4px 8px; border-radius: 4px; font-size: 0.9em;">
                                            {s.organization_ref.clone()}
                                        </code>
                                    </td>
                                </tr>
                                <tr>
                                    <td style="font-weight: bold; padding: 8px;">"Connection MP:"</td>
                                    <td style="padding: 8px;">
                                        <code style="background: #f5f5f5; padding: 4px 8px; border-radius: 4px; font-size: 0.9em;">
                                            {s.connection_mp_ref.clone()}
                                        </code>
                                    </td>
                                </tr>
                                <tr>
                                    <td style="font-weight: bold; padding: 8px;">"Marketplace Product:"</td>
                                    <td style="padding: 8px;">
                                        <code style="background: #f5f5f5; padding: 4px 8px; border-radius: 4px; font-size: 0.9em;">
                                            {s.marketplace_product_ref.clone().unwrap_or_else(|| "Not linked".to_string())}
                                        </code>
                                    </td>
                                </tr>
                                <tr>
                                    <td style="font-weight: bold; padding: 8px;">"Registrator:"</td>
                                    <td style="padding: 8px;">
                                        <code style="background: #f5f5f5; padding: 4px 8px; border-radius: 4px; font-size: 0.9em;">
                                            {s.registrator_ref.clone()}
                                        </code>
                                    </td>
                                </tr>
                            </table>

                            <h3>"Technical Information"</h3>
                            <table style="width: 100%;">
                                <tr>
                                    <td style="font-weight: bold; padding: 8px; width: 200px;">"Document Type:"</td>
                                    <td style="padding: 8px;">{s.document_type.clone()}</td>
                                </tr>
                                <tr>
                                    <td style="font-weight: bold; padding: 8px;">"Document Version:"</td>
                                    <td style="padding: 8px;">{s.document_version}</td>
                                </tr>
                                <tr>
                                    <td style="font-weight: bold; padding: 8px;">"Scheme:"</td>
                                    <td style="padding: 8px;">{s.scheme.clone().unwrap_or_default()}</td>
                                </tr>
                                <tr>
                                    <td style="font-weight: bold; padding: 8px;">"Event Time Source:"</td>
                                    <td style="padding: 8px;">{s.event_time_source.clone()}</td>
                                </tr>
                                <tr>
                                    <td style="font-weight: bold; padding: 8px;">"Loaded At:"</td>
                                    <td style="padding: 8px;">{s.loaded_at_utc.clone()}</td>
                                </tr>
                            </table>
                        </div>
                    }.into_any()
                } else {
                    view! { <div>"No data"</div> }.into_any()
                }
            }}
        </div>
    }
}

async fn fetch_sale_detail(
    marketplace: &str,
    document_no: &str,
    line_id: &str,
) -> Result<SalesRegisterDetailDto, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!(
        "/api/p900/sales-register/{}/{}/{}",
        marketplace, document_no, line_id
    );
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
    let data: SalesRegisterDetailDto = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}
