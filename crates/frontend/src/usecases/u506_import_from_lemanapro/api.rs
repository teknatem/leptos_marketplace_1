use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::a005_marketplace::aggregate::Marketplace;
use contracts::usecases::u506_import_from_lemanapro::progress::ImportProgress;
use contracts::usecases::u506_import_from_lemanapro::request::ImportRequest;
use contracts::usecases::u506_import_from_lemanapro::response::ImportResponse;
use wasm_bindgen::JsCast;
use web_sys::{Request, RequestInit, RequestMode, Response};

fn api_base() -> String {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return String::new(),
    };
    let location = window.location();
    let protocol = location.protocol().unwrap_or_else(|_| "http:".to_string());
    let hostname = location
        .hostname()
        .unwrap_or_else(|_| "127.0.0.1".to_string());
    format!("{}//{}:3000", protocol, hostname)
}

pub async fn get_connections() -> Result<Vec<ConnectionMP>, String> {
    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/connection_mp", api_base());
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("{e}"))
}

pub async fn get_marketplaces() -> Result<Vec<Marketplace>, String> {
    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/marketplace", api_base());
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("{e}"))
}

pub async fn start_import(request: ImportRequest) -> Result<ImportResponse, String> {
    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    let body = serde_json::to_string(&request).map_err(|e| format!("{e}"))?;
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body));

    let url = format!("{}/api/u506/import/start", api_base());
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() {
        let error_text = wasm_bindgen_futures::JsFuture::from(
            resp.text().map_err(|e| format!("{e:?}"))?,
        )
        .await
        .map_err(|e| format!("{e:?}"))?;
        let error_string = error_text
            .as_string()
            .unwrap_or_else(|| "Unknown error".to_string());
        return Err(format!("HTTP {}: {}", resp.status(), error_string));
    }
    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("{e}"))
}

pub async fn get_progress(session_id: &str) -> Result<ImportProgress, String> {
    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/u506/import/{}/progress", api_base(), session_id);
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("{e}"))
}

