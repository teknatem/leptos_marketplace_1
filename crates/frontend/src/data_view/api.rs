//! API calls for DataView semantic layer catalog.

use crate::data_view::types::{DataViewMeta, FilterDef};
use crate::shared::api_utils::api_base;
use wasm_bindgen::JsCast;

async fn fetch_json(url: &str) -> Result<serde_json::Value, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or("no window")?;
    let resp: Response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?
        .dyn_into()
        .map_err(|e| format!("{e:?}"))?;

    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let text: String =
        wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
            .await
            .map_err(|e| format!("{e:?}"))?
            .as_string()
            .ok_or("bad text")?;

    serde_json::from_str(&text).map_err(|e| e.to_string())
}

/// Fetch all DataView metadata from the backend.
pub async fn fetch_list() -> Result<Vec<DataViewMeta>, String> {
    let url = format!("{}/api/data-view", api_base());
    let json = fetch_json(&url).await?;
    let views = json["views"]
        .as_array()
        .ok_or("missing 'views' field")?
        .iter()
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .collect();
    Ok(views)
}

/// Fetch the full global filter registry.
///
/// Returns all known FilterDef entries sorted by id.
pub async fn fetch_global_filters() -> Result<Vec<FilterDef>, String> {
    let url = format!("{}/api/data-view/filters", api_base());
    let json = fetch_json(&url).await?;
    let filters = json["filters"]
        .as_array()
        .ok_or("missing 'filters' field")?
        .iter()
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .collect();
    Ok(filters)
}

/// Fetch resolved FilterDef list for a specific DataView.
///
/// Returns filters sorted by their `order` field as defined in the DataView metadata.
pub async fn fetch_view_filters(view_id: &str) -> Result<Vec<FilterDef>, String> {
    let url = format!("{}/api/data-view/{}/filters", api_base(), view_id);
    let json = fetch_json(&url).await?;
    let filters = json["filters"]
        .as_array()
        .ok_or("missing 'filters' field")?
        .iter()
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .collect();
    Ok(filters)
}

/// Fetch metadata for a single DataView by id.
pub async fn fetch_by_id(id: &str) -> Result<DataViewMeta, String> {
    let url = format!("{}/api/data-view/{}", api_base(), id);
    let json = fetch_json(&url).await?;
    serde_json::from_value(json).map_err(|e| e.to_string())
}

#[derive(Debug, Clone)]
pub struct ComputeResult {
    pub value: Option<f64>,
    pub previous_value: Option<f64>,
    pub change_percent: Option<f64>,
    pub status: String,
    pub subtitle: Option<String>,
    pub spark_points: Vec<f64>,
}

/// Compute scalar value for a DataView directly (no indicator needed).
pub async fn compute_view(
    view_id: &str,
    date_from: &str,
    date_to: &str,
    period2_from: Option<&str>,
    period2_to: Option<&str>,
    connection_ids: Vec<String>,
    metric: Option<String>,
) -> Result<ComputeResult, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

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
        "metric": metric,
    });
    let body = serde_json::to_string(&payload).map_err(|e| format!("serialize: {e}"))?;

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body));

    let url = format!("{}/api/data-view/{}/compute", api_base(), view_id);
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or("no window")?;
    let resp: Response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?
        .dyn_into()
        .map_err(|e| format!("{e:?}"))?;

    let text: String =
        wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
            .await
            .map_err(|e| format!("{e:?}"))?
            .as_string()
            .ok_or("bad text")?;

    if !resp.ok() {
        return Err(format!("HTTP {}: {}", resp.status(), text));
    }

    let parsed: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let spark_points = parsed["spark_points"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect())
        .unwrap_or_default();
    Ok(ComputeResult {
        value: parsed["value"].as_f64(),
        previous_value: parsed["previous_value"].as_f64(),
        change_percent: parsed["change_percent"].as_f64(),
        status: parsed["status"].as_str().unwrap_or("Neutral").to_string(),
        subtitle: parsed["subtitle"].as_str().map(|s| s.to_string()),
        spark_points,
    })
}
