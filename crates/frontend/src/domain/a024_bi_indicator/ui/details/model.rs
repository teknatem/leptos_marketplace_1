use crate::shared::api_utils::api_base;
use serde::Serialize;
use wasm_bindgen::JsCast;
use web_sys::{Request, RequestInit, RequestMode, Response};

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
    pub data_spec_schema_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_spec_sql_artifact_id: Option<String>,
    pub data_spec_query_config_json: String,
    pub params_json: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_spec_custom_html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_spec_custom_css: Option<String>,
    pub view_spec_format_json: String,
    pub view_spec_thresholds_json: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drill_spec_json: Option<String>,
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
        let text = wasm_bindgen_futures::JsFuture::from(
            resp.text().map_err(|e| format!("{e:?}"))?,
        )
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
    parsed["id"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "No id in response".to_string())
}
