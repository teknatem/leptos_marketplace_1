use contracts::usecases::u501_import_from_ut::{
    ImportRequest, ImportResponse, ImportProgress,
};
use serde_json;
use wasm_bindgen::{JsValue, JsCast};
use web_sys::{window, RequestInit, RequestMode, Response};

/// API клиент для UseCase u501
pub async fn start_import(request: ImportRequest) -> Result<ImportResponse, String> {
    let window = window().ok_or("No window object")?;

    let body = serde_json::to_string(&request).map_err(|e| e.to_string())?;

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);
    opts.set_body(&JsValue::from_str(&body));

    let request = web_sys::Request::new_with_str_and_init(
        "http://localhost:3000/api/u501/import/start",
        &opts,
    )
    .map_err(|e| format!("Failed to create request: {:?}", e))?;

    request
        .headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("Failed to set header: {:?}", e))?;

    let response_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("Fetch failed: {:?}", e))?;

    let response: Response = response_value.dyn_into().map_err(|_| "Not a Response")?;

    if !response.ok() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let json = wasm_bindgen_futures::JsFuture::from(
        response.json().map_err(|e| format!("Failed to parse JSON: {:?}", e))?,
    )
    .await
    .map_err(|e| format!("Failed to get JSON: {:?}", e))?;

    let response: ImportResponse =
        serde_wasm_bindgen::from_value(json).map_err(|e| e.to_string())?;

    Ok(response)
}

/// Получить прогресс импорта
pub async fn get_progress(session_id: &str) -> Result<ImportProgress, String> {
    let window = window().ok_or("No window object")?;

    let url = format!("http://localhost:3000/api/u501/import/{}/progress", session_id);

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let request = web_sys::Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| format!("Failed to create request: {:?}", e))?;

    let response_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("Fetch failed: {:?}", e))?;

    let response: Response = response_value.dyn_into().map_err(|_| "Not a Response")?;

    if !response.ok() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let json = wasm_bindgen_futures::JsFuture::from(
        response.json().map_err(|e| format!("Failed to parse JSON: {:?}", e))?,
    )
    .await
    .map_err(|e| format!("Failed to get JSON: {:?}", e))?;

    let progress: ImportProgress =
        serde_wasm_bindgen::from_value(json).map_err(|e| e.to_string())?;

    Ok(progress)
}

/// Получить список подключений 1С
pub async fn get_connections() -> Result<Vec<contracts::domain::a001_connection_1c::aggregate::Connection1CDatabase>, String> {
    let window = window().ok_or("No window object")?;

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let request = web_sys::Request::new_with_str_and_init(
        "http://localhost:3000/api/connection_1c",
        &opts,
    )
    .map_err(|e| format!("Failed to create request: {:?}", e))?;

    let response_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("Fetch failed: {:?}", e))?;

    let response: Response = response_value.dyn_into().map_err(|_| "Not a Response")?;

    if !response.ok() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let json = wasm_bindgen_futures::JsFuture::from(
        response.json().map_err(|e| format!("Failed to parse JSON: {:?}", e))?,
    )
    .await
    .map_err(|e| format!("Failed to get JSON: {:?}", e))?;

    let connections =
        serde_wasm_bindgen::from_value(json).map_err(|e| e.to_string())?;

    Ok(connections)
}
