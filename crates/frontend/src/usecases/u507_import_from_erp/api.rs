use contracts::usecases::u507_import_from_erp::{ImportProgress, ImportRequest, ImportResponse};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{window, RequestInit, RequestMode, Response};

use crate::shared::api_utils::api_base;

pub async fn start_import(request: ImportRequest) -> Result<ImportResponse, String> {
    let window = window().ok_or("No window object")?;

    let body = serde_json::to_string(&request).map_err(|e| e.to_string())?;

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);
    opts.set_body(&JsValue::from_str(&body));

    let req = web_sys::Request::new_with_str_and_init(
        &format!("{}/api/u507/import/start", api_base()),
        &opts,
    )
    .map_err(|e| format!("Failed to create request: {:?}", e))?;

    req.headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("Failed to set header: {:?}", e))?;

    let resp_val = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|e| format!("Fetch failed: {:?}", e))?;

    let response: Response = resp_val.dyn_into().map_err(|_| "Not a Response")?;

    if !response.ok() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let json = wasm_bindgen_futures::JsFuture::from(
        response
            .json()
            .map_err(|e| format!("Failed to parse JSON: {:?}", e))?,
    )
    .await
    .map_err(|e| format!("Failed to get JSON: {:?}", e))?;

    serde_wasm_bindgen::from_value(json).map_err(|e| e.to_string())
}

pub async fn get_progress(session_id: &str) -> Result<ImportProgress, String> {
    let window = window().ok_or("No window object")?;

    let url = format!(
        "{}/api/u507/import/{}/progress",
        api_base(),
        session_id
    );

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let req = web_sys::Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| format!("Failed to create request: {:?}", e))?;

    let resp_val = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|e| format!("Fetch failed: {:?}", e))?;

    let response: Response = resp_val.dyn_into().map_err(|_| "Not a Response")?;

    if !response.ok() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let json = wasm_bindgen_futures::JsFuture::from(
        response
            .json()
            .map_err(|e| format!("Failed to parse JSON: {:?}", e))?,
    )
    .await
    .map_err(|e| format!("Failed to get JSON: {:?}", e))?;

    serde_wasm_bindgen::from_value(json).map_err(|e| e.to_string())
}

pub async fn get_connections_1c(
) -> Result<Vec<contracts::domain::a001_connection_1c::aggregate::Connection1CDatabase>, String> {
    let window = window().ok_or("No window object")?;

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let req = web_sys::Request::new_with_str_and_init(
        &format!("{}/api/connection_1c", api_base()),
        &opts,
    )
    .map_err(|e| format!("Failed to create request: {:?}", e))?;

    let resp_val = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|e| format!("Fetch failed: {:?}", e))?;

    let response: Response = resp_val.dyn_into().map_err(|_| "Not a Response")?;

    if !response.ok() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let json = wasm_bindgen_futures::JsFuture::from(
        response
            .json()
            .map_err(|e| format!("Failed to parse JSON: {:?}", e))?,
    )
    .await
    .map_err(|e| format!("Failed to get JSON: {:?}", e))?;

    serde_wasm_bindgen::from_value(json).map_err(|e| e.to_string())
}
