use contracts::usecases::u505_match_nomenclature::{MatchProgress, MatchRequest, MatchResponse};
use serde_json;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{window, RequestInit, RequestMode, Response};

/// API клиент для UseCase u505
pub async fn start_matching(request: MatchRequest) -> Result<MatchResponse, String> {
    let window = window().ok_or("No window object")?;

    let body = serde_json::to_string(&request).map_err(|e| e.to_string())?;

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);
    opts.set_body(&JsValue::from_str(&body));

    let request = web_sys::Request::new_with_str_and_init(
        "http://localhost:3000/api/u505/match/start",
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
        response
            .json()
            .map_err(|e| format!("Failed to parse JSON: {:?}", e))?,
    )
    .await
    .map_err(|e| format!("Failed to get JSON: {:?}", e))?;

    let response: MatchResponse =
        serde_wasm_bindgen::from_value(json).map_err(|e| e.to_string())?;

    Ok(response)
}

/// Получить прогресс сопоставления
pub async fn get_progress(session_id: &str) -> Result<MatchProgress, String> {
    let window = window().ok_or("No window object")?;

    let url = format!(
        "http://localhost:3000/api/u505/match/{}/progress",
        session_id
    );

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
        response
            .json()
            .map_err(|e| format!("Failed to parse JSON: {:?}", e))?,
    )
    .await
    .map_err(|e| format!("Failed to get JSON: {:?}", e))?;

    let progress: MatchProgress =
        serde_wasm_bindgen::from_value(json).map_err(|e| e.to_string())?;

    Ok(progress)
}
