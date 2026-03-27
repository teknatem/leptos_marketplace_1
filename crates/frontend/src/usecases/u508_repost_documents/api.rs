use contracts::usecases::u508_repost_documents::{
    aggregate::AggregateOption, aggregate_request::AggregateRepostRequest,
    progress::RepostProgress, projection::ProjectionOption, request::RepostRequest,
    response::RepostResponse,
};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{window, RequestInit, RequestMode, Response};

use crate::shared::api_utils::api_base;

pub async fn get_projections() -> Result<Vec<ProjectionOption>, String> {
    let window = window().ok_or("No window object")?;

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let req = web_sys::Request::new_with_str_and_init(
        &format!("{}/api/u508/repost/projections", api_base()),
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

pub async fn get_aggregates() -> Result<Vec<AggregateOption>, String> {
    let window = window().ok_or("No window object")?;

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let req = web_sys::Request::new_with_str_and_init(
        &format!("{}/api/u508/repost/aggregates", api_base()),
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

pub async fn start_repost(request: RepostRequest) -> Result<RepostResponse, String> {
    let window = window().ok_or("No window object")?;

    let body = serde_json::to_string(&request).map_err(|e| e.to_string())?;

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);
    opts.set_body(&JsValue::from_str(&body));

    let req = web_sys::Request::new_with_str_and_init(
        &format!("{}/api/u508/repost/start", api_base()),
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

pub async fn start_aggregate_repost(
    request: AggregateRepostRequest,
) -> Result<RepostResponse, String> {
    let window = window().ok_or("No window object")?;

    let body = serde_json::to_string(&request).map_err(|e| e.to_string())?;

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);
    opts.set_body(&JsValue::from_str(&body));

    let req = web_sys::Request::new_with_str_and_init(
        &format!("{}/api/u508/repost/aggregate/start", api_base()),
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

pub async fn get_progress(session_id: &str) -> Result<RepostProgress, String> {
    let window = window().ok_or("No window object")?;

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let req = web_sys::Request::new_with_str_and_init(
        &format!("{}/api/u508/repost/{}/progress", api_base(), session_id),
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
