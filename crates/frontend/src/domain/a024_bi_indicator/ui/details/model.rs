use crate::shared::api_utils::api_base;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use web_sys::{Request, RequestInit, RequestMode, Response};

/// Frontend save DTO that matches backend `BiIndicatorDto` structure.
/// Uses nested `data_spec`, `params`, `view_spec`, `drill_spec` to correctly
/// map to the backend service layer.
#[derive(Debug, Clone, Serialize)]
pub struct BiIndicatorSaveDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub code: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    pub status: String,
    pub owner_user_id: String,
    pub is_public: bool,
    pub version: i64,
    /// DataSpec as raw JSON (backend deserializes into DataSpec)
    pub data_spec: serde_json::Value,
    /// Params as raw JSON array
    pub params: serde_json::Value,
    /// ViewSpec as structured object (matches contracts::ViewSpec)
    pub view_spec: serde_json::Value,
    /// DrillSpec as raw JSON or null
    pub drill_spec: serde_json::Value,
}

pub async fn fetch_by_id(id: &str) -> Result<serde_json::Value, String> {
    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a024-bi-indicator/{}", api_base(), id);
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

pub async fn save_indicator(dto: BiIndicatorSaveDto) -> Result<String, String> {
    let fallback_id = dto.id.clone();
    let body = serde_json::to_string(&dto).map_err(|e| format!("serialize error: {e}"))?;

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body));

    let url = format!("{}/api/a024-bi-indicator/upsert", api_base());
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
        let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
            .await
            .map_err(|e| format!("{e:?}"))?;
        let text = text.as_string().unwrap_or_default();
        return Err(format!("HTTP {}: {}", resp.status(), text));
    }

    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;

    let parsed: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    if let Some(id) = parsed["id"].as_str() {
        Ok(id.to_string())
    } else if let Some(id) = fallback_id {
        Ok(id)
    } else {
        Err("No id in response".to_string())
    }
}

/// Minimal schema entry from the indicator catalog.
#[derive(Debug, Clone, Deserialize)]
pub struct IndicatorSchemaMeta {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
}

/// Fetch the indicator catalog from /api/indicators/meta.
/// Returns a flat list of (id, label, description).
pub async fn fetch_indicator_catalog() -> Result<Vec<IndicatorSchemaMeta>, String> {
    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/indicators/meta", api_base());
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

    // Response: { indicators: [{ id: { "0": "sales_revenue" }, label, description, ... }] }
    let parsed: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    let items = parsed["indicators"]
        .as_array()
        .ok_or_else(|| "No indicators array".to_string())?;

    let result = items
        .iter()
        .filter_map(|v| {
            // IndicatorId serializes as { "0": "sales_revenue" } because it's a newtype
            // but contracts says it should serialize as plain string — check both
            let id = if let Some(s) = v["id"].as_str() {
                s.to_string()
            } else if let Some(s) = v["id"]["0"].as_str() {
                s.to_string()
            } else {
                return None;
            };
            Some(IndicatorSchemaMeta {
                id,
                label: v["label"].as_str().unwrap_or("").to_string(),
                description: v["description"].as_str().map(|s| s.to_string()),
            })
        })
        .collect();
    Ok(result)
}

/// Result from /api/indicators/compute for a single indicator.
#[derive(Debug, Clone, Deserialize)]
pub struct ComputedIndicatorValue {
    pub value: Option<f64>,
    pub previous_value: Option<f64>,
    pub change_percent: Option<f64>,
    pub status: String,
    pub subtitle: Option<String>,
}

/// Compute indicator value using its own UUID and data_spec
/// (supports DataView, DataSourceConfig, IndicatorRegistry paths).
/// This calls POST /api/a024-bi-indicator/:id/compute.
pub async fn compute_indicator_by_id(
    indicator_id: &str,
    date_from: &str,
    date_to: &str,
    period2_from: Option<&str>,
    period2_to: Option<&str>,
    connection_ids: Vec<String>,
) -> Result<ComputedIndicatorValue, String> {
    let connection_mp_refs = if connection_ids.is_empty() {
        None
    } else {
        Some(connection_ids.join(","))
    };
    let payload = serde_json::json!({
        "date_from": date_from,
        "date_to": date_to,
        "period2_from": period2_from,
        "period2_to": period2_to,
        "connection_mp_refs": connection_mp_refs,
    });
    let body = serde_json::to_string(&payload).map_err(|e| format!("serialize: {e}"))?;

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body));

    let url = format!("{}/api/a024-bi-indicator/{}/compute", api_base(), indicator_id);
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

    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;

    if !resp.ok() {
        return Err(format!("HTTP {}: {}", resp.status(), text));
    }

    let parsed: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(ComputedIndicatorValue {
        value: parsed["value"].as_f64(),
        previous_value: parsed["previous_value"].as_f64(),
        change_percent: parsed["change_percent"].as_f64(),
        status: parsed["status"].as_str().unwrap_or("Neutral").to_string(),
        subtitle: parsed["subtitle"].as_str().map(|s| s.to_string()),
    })
}

pub async fn compute_indicator_schema(
    schema_id: &str,
    date_from: &str,
    date_to: &str,
    connection_ids: Vec<String>,
) -> Result<ComputedIndicatorValue, String> {
    let payload = serde_json::json!({
        "indicator_ids": [schema_id],
        "context": {
            "date_from": date_from,
            "date_to": date_to,
            "connection_mp_refs": connection_ids,
        }
    });
    let body = serde_json::to_string(&payload).map_err(|e| format!("serialize: {e}"))?;

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body));

    let url = format!("{}/api/indicators/compute", api_base());
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

    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;

    if !resp.ok() {
        return Err(format!("HTTP {}: {}", resp.status(), text));
    }

    let parsed: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    let first = parsed["values"]
        .as_array()
        .and_then(|arr| arr.first())
        .ok_or_else(|| "No values in response".to_string())?;

    Ok(ComputedIndicatorValue {
        value: first["value"].as_f64(),
        previous_value: first["previous_value"].as_f64(),
        change_percent: first["change_percent"].as_f64(),
        status: first["status"].as_str().unwrap_or("Neutral").to_string(),
        subtitle: first["subtitle"].as_str().map(|s| s.to_string()),
    })
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateViewResp {
    pub custom_html: String,
    pub custom_css: String,
    pub explanation: String,
}

pub async fn generate_view(
    prompt: &str,
    current_html: Option<&str>,
    current_css: Option<&str>,
    indicator_description: &str,
) -> Result<GenerateViewResp, String> {
    let payload = serde_json::json!({
        "prompt": prompt,
        "current_html": current_html,
        "current_css": current_css,
        "indicator_description": indicator_description,
    });
    let body = serde_json::to_string(&payload).map_err(|e| format!("serialize: {e}"))?;

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body));

    let url = format!("{}/api/a024-bi-indicator/generate-view", api_base());
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
        let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
            .await
            .map_err(|e| format!("{e:?}"))?;
        let text = text.as_string().unwrap_or_default();
        return Err(format!("HTTP {}: {}", resp.status(), text));
    }

    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("{e}"))
}
