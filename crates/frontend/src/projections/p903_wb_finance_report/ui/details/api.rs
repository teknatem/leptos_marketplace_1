use contracts::projections::p903_wb_finance_report::dto::WbFinanceReportDetailResponse;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

// Simplified WbSales structure for links display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbSalesLink {
    pub id: String,
    pub header: WbSalesHeaderLink,
    pub line: WbSalesLineLink,
    pub state: WbSalesStateLink,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbSalesHeaderLink {
    pub document_no: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbSalesLineLink {
    pub nm_id: i64,
    pub supplier_article: String,
    pub name: String,
    pub qty: f64,
    pub total_price: Option<f64>,
    pub payment_sale_amount: Option<f64>,
    pub price_effective: Option<f64>,
    pub amount_line: Option<f64>,
    pub finished_price: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbSalesStateLink {
    pub sale_dt: String,
}

pub async fn fetch_detail(id: &str) -> Result<WbFinanceReportDetailResponse, String> {
    let window = web_sys::window().ok_or("No window object")?;
    let url = format!("/api/p903/finance-report/by-id/{}", urlencoding::encode(id));

    let resp_value = JsFuture::from(window.fetch_with_str(&url))
        .await
        .map_err(|e| format!("Fetch failed: {:?}", e))?;

    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|_| "Failed to cast to Response")?;

    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }

    let json = JsFuture::from(resp.json().map_err(|_| "Failed to get JSON")?)
        .await
        .map_err(|e| format!("Failed to parse JSON: {:?}", e))?;

    serde_wasm_bindgen::from_value(json).map_err(|e| format!("Failed to deserialize: {:?}", e))
}

pub async fn post_detail(id: &str) -> Result<WbFinanceReportDetailResponse, String> {
    use web_sys::{Request, RequestInit, RequestMode};

    let window = web_sys::window().ok_or("No window object")?;
    let url = format!(
        "/api/p903/finance-report/by-id/{}/post",
        urlencoding::encode(id)
    );

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| format!("Request init failed: {:?}", e))?;

    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("Fetch failed: {:?}", e))?;

    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|_| "Failed to cast to Response")?;

    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }

    let json = JsFuture::from(resp.json().map_err(|_| "Failed to get JSON")?)
        .await
        .map_err(|e| format!("Failed to parse JSON: {:?}", e))?;

    serde_wasm_bindgen::from_value(json).map_err(|e| format!("Failed to deserialize: {:?}", e))
}

pub async fn fetch_linked_sales(srid: &str) -> Result<Vec<WbSalesLink>, String> {
    let window = web_sys::window().ok_or("No window object")?;
    let url = format!("/api/a012/wb-sales/search-by-srid?srid={}", srid);

    let resp_value = JsFuture::from(window.fetch_with_str(&url))
        .await
        .map_err(|e| format!("Fetch failed: {:?}", e))?;

    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|_| "Failed to cast to Response")?;

    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }

    let json = JsFuture::from(resp.json().map_err(|_| "Failed to get JSON")?)
        .await
        .map_err(|e| format!("Failed to parse JSON: {:?}", e))?;

    serde_wasm_bindgen::from_value(json).map_err(|e| format!("Failed to deserialize: {:?}", e))
}
