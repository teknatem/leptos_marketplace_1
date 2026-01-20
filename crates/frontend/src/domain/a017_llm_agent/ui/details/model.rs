//! LLM Agent Details - Model Layer
//!
//! DTOs and API functions for LLM Agent details

use crate::shared::api_utils::api_base;
use contracts::domain::a017_llm_agent::aggregate::LlmAgent;
use serde::Deserialize;
use wasm_bindgen::JsCast;
use web_sys::{Request, RequestInit, RequestMode, Response};

/// Response DTO for test connection endpoint
#[derive(Deserialize)]
pub struct TestConnectionResponse {
    pub success: bool,
    pub message: String,
}

/// Response DTO for fetch models endpoint
#[derive(Deserialize)]
pub struct FetchModelsResponse {
    pub success: bool,
    pub models: Vec<serde_json::Value>,
    pub count: usize,
    pub message: String,
}

/// Fetch LLM agent by ID from API
pub async fn fetch_agent(id: &str) -> Result<LlmAgent, String> {
    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a017-llm-agent/{}", api_base(), id);
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
    let agent: LlmAgent = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    
    Ok(agent)
}

/// Save (create or update) LLM agent via API
pub async fn save_agent(dto: serde_json::Value) -> Result<(), String> {
    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    let body = serde_json::to_string(&dto).map_err(|e| format!("{e}"))?;
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body));

    let url = format!("{}/api/a017-llm-agent", api_base());
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    
    Ok(())
}

/// Test LLM agent connection via API
pub async fn test_agent_connection(id: &str) -> Result<TestConnectionResponse, String> {
    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a017-llm-agent/{}/test", api_base(), id);
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
    let result: TestConnectionResponse = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    
    Ok(result)
}

/// Fetch available models from LLM provider API
pub async fn fetch_models_from_api(id: &str) -> Result<FetchModelsResponse, String> {
    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a017-llm-agent/{}/fetch-models", api_base(), id);
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
    let result: FetchModelsResponse = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    
    Ok(result)
}
