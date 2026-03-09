use crate::app::ThawThemeContext;
use crate::data_view::api as dv_api;
use crate::data_view::types::{FilterDef, FilterRef};
use crate::data_view::ui::filter_bar::apply_defaults;
use crate::data_view::ui::FilterBar;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::bi_card::{
    available_designs, default_design_name, get_style_css, render_card_html, IndicatorCardParams,
};
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use contracts::shared::data_view::ViewContext;
use gloo_net::http::Request;
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::window_event_listener;
use leptos::prelude::*;
use std::collections::HashMap;
use thaw::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

#[derive(Clone, Debug, serde::Deserialize)]
#[allow(dead_code)]
struct DashboardItem {
    pub indicator_id: String,
    #[serde(default)]
    pub indicator_name: String,
    #[serde(default)]
    pub sort_order: i32,
    #[serde(default = "default_col_class")]
    pub col_class: String,
    #[serde(default)]
    pub param_overrides: HashMap<String, String>,
}

fn default_col_class() -> String {
    "1x1".to_string()
}

#[derive(Clone, Debug, serde::Deserialize)]
struct DashboardGroup {
    #[allow(dead_code)]
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub sort_order: i32,
    #[serde(default)]
    pub items: Vec<DashboardItem>,
    #[serde(default)]
    pub subgroups: Vec<DashboardGroup>,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct DashboardLayout {
    #[serde(default)]
    pub groups: Vec<DashboardGroup>,
}

#[derive(Clone, Debug, Default, serde::Deserialize)]
struct IndicatorViewSpec {
    #[serde(default)]
    pub custom_css: Option<String>,
    #[serde(default)]
    pub format: serde_json::Value,
    #[serde(default)]
    pub preview_values: HashMap<String, String>,
}

#[derive(Clone, Debug, Default, serde::Deserialize)]
#[allow(dead_code)]
struct IndicatorSchemaQuery {
    #[serde(default)]
    pub available_dimensions: Vec<serde_json::Value>,
    #[serde(default)]
    pub metric: serde_json::Value,
}

#[derive(Clone, Debug, Default, serde::Deserialize)]
#[allow(dead_code)]
struct IndicatorDataSpec {
    #[serde(default)]
    pub schema_id: String,
    #[serde(default)]
    pub schema_query: Option<IndicatorSchemaQuery>,
    #[serde(default)]
    pub view_id: Option<String>,
    #[serde(default)]
    pub metric_id: Option<String>,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct IndicatorDef {
    pub id: String,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub view_spec: IndicatorViewSpec,
    #[serde(default)]
    pub data_spec: IndicatorDataSpec,
}

/// Вычисленное значение от /api/indicators/compute
#[derive(Clone, Debug, Default, serde::Deserialize)]
struct ComputedValue {
    /// id сериализуется как строка (IndicatorId — newtype over String)
    pub id: String,
    pub value: Option<f64>,
    pub previous_value: Option<f64>,
    pub change_percent: Option<f64>,
    /// "Good" | "Bad" | "Neutral" | "Warning"
    pub status: Option<String>,
    /// Daily values for period 1 (for sparkline). Empty when not available.
    #[serde(default)]
    pub spark_points: Vec<f64>,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct BiDashboardData {
    #[allow(dead_code)]
    pub id: String,
    pub code: String,
    pub description: String,
    pub layout: DashboardLayout,
    #[serde(default)]
    pub filters: Vec<FilterRef>,
}

/// Minimal local mirror of DimensionMeta for DataView drilldown
#[derive(Clone, Debug, serde::Deserialize)]
struct DataViewDim {
    pub id: String,
    pub label: String,
}

/// Minimal local mirror of DataViewMeta (only fields needed for drilldown)
#[derive(Clone, Debug, serde::Deserialize)]
struct DataViewMetaLocal {
    #[serde(default)]
    pub available_dimensions: Vec<DataViewDim>,
}

async fn fetch_dataview_meta(view_id: &str) -> Result<DataViewMetaLocal, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/data-view/{}", api_base(), view_id);
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

async fn fetch_dashboard(id: &str) -> Result<serde_json::Value, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a025-bi-dashboard/{}", api_base(), id);
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

fn collect_indicator_ids(groups: &[DashboardGroup], out: &mut Vec<String>) {
    for group in groups {
        for item in &group.items {
            if !out.contains(&item.indicator_id) {
                out.push(item.indicator_id.clone());
            }
        }
        collect_indicator_ids(&group.subgroups, out);
    }
}

async fn fetch_indicator_defs(ids: &[String]) -> Result<HashMap<String, IndicatorDef>, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    if ids.is_empty() {
        return Ok(HashMap::new());
    }

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!(
        "{}/api/a024-bi-indicator/list?limit=10000&offset=0&sort_by=code&sort_desc=false",
        api_base()
    );
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
    let parsed: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    let id_set: std::collections::HashSet<&str> = ids.iter().map(|s| s.as_str()).collect();

    let items = parsed["items"].as_array().cloned().unwrap_or_default();
    let mut out = HashMap::new();
    for item in items {
        let Ok(def) = serde_json::from_value::<IndicatorDef>(item) else {
            continue;
        };
        if id_set.contains(def.id.as_str()) {
            out.insert(def.id.clone(), def);
        }
    }

    Ok(out)
}


fn resolve_dashboard_filters(
    filter_refs: &[FilterRef],
    registry: &[FilterDef],
) -> Vec<FilterDef> {
    let registry_map: HashMap<&str, &FilterDef> =
        registry.iter().map(|def| (def.id.as_str(), def)).collect();

    let mut refs = filter_refs.to_vec();
    refs.sort_by_key(|r| r.order);

    refs.into_iter()
        .filter_map(|filter_ref| {
            let mut def = (*registry_map.get(filter_ref.filter_id.as_str())?).clone();
            if let Some(label_override) = filter_ref.label_override {
                if !label_override.trim().is_empty() {
                    def.label = label_override;
                }
            }
            Some(def)
        })
        .collect()
}

async fn derive_dashboard_filters_from_indicators(
    indicator_defs: &HashMap<String, IndicatorDef>,
) -> Vec<FilterRef> {
    let mut view_ids: Vec<String> = indicator_defs
        .values()
        .filter_map(|def| def.data_spec.view_id.clone())
        .filter(|view_id| !view_id.trim().is_empty())
        .collect();
    view_ids.sort();
    view_ids.dedup();

    let mut merged: Vec<FilterRef> = Vec::new();

    for view_id in view_ids {
        match dv_api::fetch_by_id(&view_id).await {
            Ok(meta) => {
                let mut refs = meta.filters;
                refs.sort_by_key(|filter_ref| filter_ref.order);
                for filter_ref in refs {
                    if let Some(existing) = merged
                        .iter_mut()
                        .find(|existing| existing.filter_id == filter_ref.filter_id)
                    {
                        existing.required |= filter_ref.required;
                        if existing.default_value.as_deref().unwrap_or("").trim().is_empty() {
                            existing.default_value = filter_ref.default_value.clone();
                        }
                        if existing
                            .label_override
                            .as_deref()
                            .unwrap_or("")
                            .trim()
                            .is_empty()
                        {
                            existing.label_override = filter_ref.label_override.clone();
                        }
                        existing.order = existing.order.min(filter_ref.order);
                    } else {
                        merged.push(filter_ref);
                    }
                }
            }
            Err(err) => {
                leptos::logging::warn!(
                    "Failed to derive dashboard filters from DataView {}: {}",
                    view_id,
                    err
                );
            }
        }
    }

    merged.sort_by(|a, b| {
        a.order
            .cmp(&b.order)
            .then_with(|| a.filter_id.cmp(&b.filter_id))
    });
    for (idx, filter_ref) in merged.iter_mut().enumerate() {
        filter_ref.order = idx as u32;
    }
    merged
}

fn default_dashboard_ctx() -> ViewContext {
    use chrono::{Datelike, Duration, NaiveDate, Utc};

    let now = Utc::now().date_naive();
    let month_start = NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap_or(now);
    let month_end = if now.month() == 12 {
        NaiveDate::from_ymd_opt(now.year() + 1, 1, 1)
            .map(|d| d - Duration::days(1))
            .unwrap_or(now)
    } else {
        NaiveDate::from_ymd_opt(now.year(), now.month() + 1, 1)
            .map(|d| d - Duration::days(1))
            .unwrap_or(now)
    };

    ViewContext {
        date_from: month_start.format("%Y-%m-%d").to_string(),
        date_to: month_end.format("%Y-%m-%d").to_string(),
        period2_from: None,
        period2_to: None,
        connection_mp_refs: vec![],
        params: HashMap::new(),
    }
}

fn merge_view_ctx(default_ctx: ViewContext, prev_ctx: ViewContext) -> ViewContext {
    let mut merged = default_ctx;
    if !prev_ctx.date_from.trim().is_empty() {
        merged.date_from = prev_ctx.date_from;
    }
    if !prev_ctx.date_to.trim().is_empty() {
        merged.date_to = prev_ctx.date_to;
    }
    if prev_ctx.period2_from.is_some() {
        merged.period2_from = prev_ctx.period2_from;
    }
    if prev_ctx.period2_to.is_some() {
        merged.period2_to = prev_ctx.period2_to;
    }
    if !prev_ctx.connection_mp_refs.is_empty() {
        merged.connection_mp_refs = prev_ctx.connection_mp_refs;
    }
    merged.params.extend(prev_ctx.params);
    merged
}

/// Вычислить данные индикаторов через /api/indicators/compute
async fn fetch_indicator_data(
    indicator_defs: &HashMap<String, IndicatorDef>,
    ctx: &ViewContext,
) -> HashMap<String, ComputedValue> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    // Собираем пары schema_id -> Vec<indicator_id> для тех, у кого schema_id задан
    let mut schema_to_ids: HashMap<String, Vec<String>> = HashMap::new();
    for (ind_id, def) in indicator_defs {
        if !def.data_spec.schema_id.is_empty() {
            schema_to_ids
                .entry(def.data_spec.schema_id.clone())
                .or_default()
                .push(ind_id.clone());
        }
    }

    if schema_to_ids.is_empty() {
        return HashMap::new();
    }

    // IndicatorId — newtype над String, сериализуется как обычная строка
    let indicator_ids: Vec<&str> = schema_to_ids.keys().map(|k| k.as_str()).collect();

    let body = serde_json::json!({
        "indicator_ids": indicator_ids,
        "context": {
            "date_from": ctx.date_from,
            "date_to": ctx.date_to,
            "connection_mp_refs": ctx.connection_mp_refs,
        }
    });

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);
    let body_str = body.to_string();
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body_str));

    let Some(window) = web_sys::window() else {
        return HashMap::new();
    };
    let Ok(request) =
        Request::new_with_str_and_init(&format!("{}/api/indicators/compute", api_base()), &opts)
    else {
        return HashMap::new();
    };
    let _ = request.headers().set("Accept", "application/json");
    let _ = request.headers().set("Content-Type", "application/json");

    let Ok(resp_val) =
        wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request)).await
    else {
        return HashMap::new();
    };
    let Ok(resp): Result<Response, _> = resp_val.dyn_into() else {
        return HashMap::new();
    };
    if !resp.ok() {
        return HashMap::new();
    }
    let Ok(text_promise) = resp.text() else {
        return HashMap::new();
    };
    let Ok(text_val) = wasm_bindgen_futures::JsFuture::from(text_promise).await else {
        return HashMap::new();
    };
    let Some(text) = text_val.as_string() else {
        return HashMap::new();
    };

    let parsed: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return HashMap::new(),
    };

    let values = parsed["values"].as_array().cloned().unwrap_or_default();

    // Разворачиваем результаты: schema_id → ComputedValue, затем маппируем на indicator_ids
    let mut schema_values: HashMap<String, ComputedValue> = HashMap::new();
    for val in values {
        if let Ok(cv) = serde_json::from_value::<ComputedValue>(val) {
            if !cv.id.is_empty() {
                schema_values.insert(cv.id.clone(), cv);
            }
        }
    }

    let mut result: HashMap<String, ComputedValue> = HashMap::new();
    for (schema_id, ind_ids) in &schema_to_ids {
        if let Some(cv) = schema_values.get(schema_id) {
            for ind_id in ind_ids {
                result.insert(ind_id.clone(), cv.clone());
            }
        }
    }

    result
}

/// Вычислить DataView-индикаторы через /api/a024-bi-indicator/:id/compute
/// (те у которых задан data_spec.view_id)
async fn fetch_indicator_data_view(
    indicator_defs: &HashMap<String, IndicatorDef>,
    ctx: &ViewContext,
) -> HashMap<String, ComputedValue> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let mut result: HashMap<String, ComputedValue> = HashMap::new();

    for (ind_id, def) in indicator_defs {
        let Some(ref _view_id) = def.data_spec.view_id else {
            continue;
        };
        if _view_id.trim().is_empty() {
            continue;
        }

        let body = serde_json::json!({
            "date_from": ctx.date_from,
            "date_to": ctx.date_to,
            "period2_from": ctx.period2_from,
            "period2_to": ctx.period2_to,
            "connection_mp_refs": ctx.connection_mp_refs.join(","),
            "params": ctx.params,
            "metric": def.data_spec.metric_id,
        });

        let opts = RequestInit::new();
        opts.set_method("POST");
        opts.set_mode(RequestMode::Cors);
        let body_str = body.to_string();
        opts.set_body(&wasm_bindgen::JsValue::from_str(&body_str));

        let Some(window) = web_sys::window() else { continue; };
        let url = format!("{}/api/a024-bi-indicator/{}/compute", crate::shared::api_utils::api_base(), ind_id);
        let Ok(request) = Request::new_with_str_and_init(&url, &opts) else { continue; };
        let _ = request.headers().set("Accept", "application/json");
        let _ = request.headers().set("Content-Type", "application/json");

        let Ok(resp_val) = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request)).await else { continue; };
        let Ok(resp): Result<Response, _> = resp_val.dyn_into() else { continue; };
        if !resp.ok() { continue; }
        let Ok(text_promise) = resp.text() else { continue; };
        let Ok(text_val) = wasm_bindgen_futures::JsFuture::from(text_promise).await else { continue; };
        let Some(text) = text_val.as_string() else { continue; };

        let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) else { continue; };

        let cv = ComputedValue {
            id: ind_id.clone(),
            value: parsed["value"].as_f64(),
            previous_value: parsed["previous_value"].as_f64(),
            change_percent: parsed["change_percent"].as_f64(),
            status: parsed["status"].as_str().map(|s| s.to_string()),
            spark_points: parsed["spark_points"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect())
                .unwrap_or_default(),
        };
        result.insert(ind_id.clone(), cv);
    }

    result
}

fn reload_dashboard_data(
    id: String,
    loading: RwSignal<bool>,
    error: RwSignal<Option<String>>,
    dashboard: RwSignal<Option<BiDashboardData>>,
    view_ctx: RwSignal<ViewContext>,
    dashboard_filter_defs: RwSignal<Vec<FilterDef>>,
    indicator_defs: RwSignal<HashMap<String, IndicatorDef>>,
    indicator_values: RwSignal<HashMap<String, ComputedValue>>,
    preserve_session_filters: bool,
) {
    leptos::task::spawn_local(async move {
        loading.set(true);
        error.set(None);

        match fetch_dashboard(&id).await {
            Ok(raw) => match serde_json::from_value::<BiDashboardData>(raw) {
                Ok(data) => {
                    let mut ids = Vec::new();
                    collect_indicator_ids(&data.layout.groups, &mut ids);
                    let defs = fetch_indicator_defs(&ids).await.unwrap_or_default();

                    let effective_filter_refs = if data.filters.is_empty() {
                        derive_dashboard_filters_from_indicators(&defs).await
                    } else {
                        data.filters.clone()
                    };

                    let registry = dv_api::fetch_global_filters().await.unwrap_or_default();
                    let resolved_filters =
                        resolve_dashboard_filters(&effective_filter_refs, &registry);
                    dashboard_filter_defs.set(resolved_filters.clone());

                    let mut next_ctx = default_dashboard_ctx();
                    for filter_ref in &effective_filter_refs {
                        if let Some(default_value) = &filter_ref.default_value {
                            apply_defaults(&mut next_ctx, &filter_ref.filter_id, default_value);
                        }
                    }
                    if preserve_session_filters {
                        next_ctx = merge_view_ctx(next_ctx, view_ctx.get_untracked());
                    }
                    view_ctx.set(next_ctx);

                    // Индикаторы пересчитываются отдельным reactive-effect,
                    // чтобы все карточки обновлялись атомарно на один и тот же ctx.
                    indicator_values.set(HashMap::new());

                    indicator_defs.set(defs);
                    dashboard.set(Some(data));
                }
                Err(e) => error.set(Some(format!("Ошибка парсинга: {}", e))),
            },
            Err(e) => error.set(Some(e)),
        }

        loading.set(false);
    });
}

fn get_app_theme() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        let theme = web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item("app_theme").ok().flatten())
            .unwrap_or_else(|| "dark".to_string());
        if theme == "light" {
            "light".to_string()
        } else {
            "dark".to_string()
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        "dark".to_string()
    }
}

fn get_sidebar_scrollbar_tokens() -> (String, String) {
    let default_thumb = "rgba(0, 0, 0, 0.18)".to_string();
    let default_hover = "rgba(0, 0, 0, 0.28)".to_string();

    #[cfg(target_arch = "wasm32")]
    {
        let Some(window) = web_sys::window() else {
            return (default_thumb, default_hover);
        };
        let Some(document) = window.document() else {
            return (default_thumb, default_hover);
        };
        let Some(root) = document.document_element() else {
            return (default_thumb, default_hover);
        };
        let Ok(Some(style)) = window.get_computed_style(&root) else {
            return (default_thumb, default_hover);
        };

        // Use --list-scrollbar-thumb: it is properly overridden per theme
        // (dark.css defines white-based values; sidebar token is never overridden in dark mode).
        let thumb = style
            .get_property_value("--list-scrollbar-thumb")
            .ok()
            .map(|v| v.trim().to_string())
            .unwrap_or_default();
        let hover = style
            .get_property_value("--list-scrollbar-thumb-hover")
            .ok()
            .map(|v| v.trim().to_string())
            .unwrap_or_default();

        let thumb = if thumb.is_empty() {
            default_thumb
        } else {
            thumb
        };
        let hover = if hover.is_empty() {
            default_hover
        } else {
            hover
        };
        (thumb, hover)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        (default_thumb, default_hover)
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\"', "&quot;")
        .replace('\'', "&#39;")
}

fn item_title(item: &DashboardItem, indicator_defs: &HashMap<String, IndicatorDef>) -> String {
    if !item.indicator_name.trim().is_empty() {
        item.indicator_name.clone()
    } else if let Some(def) = indicator_defs.get(&item.indicator_id) {
        if !def.code.trim().is_empty() && !def.description.trim().is_empty() {
            format!("{} — {}", def.code, def.description)
        } else if !def.description.trim().is_empty() {
            def.description.clone()
        } else if !def.code.trim().is_empty() {
            def.code.clone()
        } else if item.indicator_id.len() > 8 {
            format!("Indicator {}...", &item.indicator_id[..8])
        } else {
            format!("Indicator {}", item.indicator_id)
        }
    } else if item.indicator_id.len() > 8 {
        format!("Индикатор {}…", &item.indicator_id[..8])
    } else {
        format!("Индикатор {}", item.indicator_id)
    }
}

/// Компактное форматирование числового значения (M/K аббревиатуры — идентично detail-view).
fn format_value(value: f64, format_spec: &serde_json::Value) -> String {
    let kind = format_spec["kind"].as_str().unwrap_or("Number");
    let abs = value.abs();
    match kind {
        "Money" => {
            let currency = format_spec["currency"].as_str().unwrap_or("RUB");
            let symbol = if currency == "RUB" { "₽" } else { currency };
            let sign = if value < 0.0 { "-" } else { "" };
            if abs >= 1_000_000_000.0 {
                format!("{sign}{symbol}{:.2}B", abs / 1_000_000_000.0)
            } else if abs >= 1_000_000.0 {
                format!("{sign}{symbol}{:.1}M", abs / 1_000_000.0)
            } else if abs >= 1_000.0 {
                format!("{sign}{symbol}{:.1}K", abs / 1_000.0)
            } else {
                format!("{sign}{symbol}{:.2}", abs)
            }
        }
        "Percent" => {
            let decimals = format_spec["decimals"].as_u64().unwrap_or(1) as usize;
            format!("{:.prec$}%", value, prec = decimals)
        }
        "Integer" => {
            if abs >= 1_000_000_000.0 {
                format!("{:.1}B", value / 1_000_000_000.0)
            } else if abs >= 1_000_000.0 {
                format!("{:.1}M", value / 1_000_000.0)
            } else {
                format!("{}", value as i64)
            }
        }
        _ => {
            let decimals = format_spec["decimals"].as_u64().unwrap_or(2) as usize;
            if abs >= 1_000_000_000.0 {
                format!("{:.1}B", value / 1_000_000_000.0)
            } else if abs >= 1_000_000.0 {
                format!("{:.1}M", value / 1_000_000.0)
            } else {
                format!("{:.prec$}", value, prec = decimals)
            }
        }
    }
}

/// Аббревиатура месяца на русском
fn month_abbr(m: u32) -> &'static str {
    match m {
        1 => "Янв",
        2 => "Фев",
        3 => "Мар",
        4 => "Апр",
        5 => "Май",
        6 => "Июн",
        7 => "Июл",
        8 => "Авг",
        9 => "Сен",
        10 => "Окт",
        11 => "Ноя",
        12 => "Дек",
        _ => "???",
    }
}

/// "YYYY-MM-DD" → "МесYYYY" или пустая строка
fn fmt_date_short(s: &str) -> Option<String> {
    let parts: Vec<&str> = s.splitn(3, '-').collect();
    if parts.len() < 2 {
        return None;
    }
    let year: u32 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;
    if month < 1 || month > 12 {
        return None;
    }
    Some(format!("{} {}", month_abbr(month), year))
}

/// Компактный хинт для meta_1: "Янв – Фев 2026 · 4 каб."
fn compact_filter_hint(
    ctx: &ViewContext,
) -> String {
    let mut parts: Vec<String> = Vec::new();

    // Диапазон дат
    let from = ctx.date_from.as_str();
    let to = ctx.date_to.as_str();
    let date_part = match (fmt_date_short(from), fmt_date_short(to)) {
        (Some(f), Some(t)) if f == t => f,
        (Some(f), Some(t)) => format!("{f} – {t}"),
        (Some(f), None) => format!("с {f}"),
        (None, Some(t)) => format!("до {t}"),
        _ => String::new(),
    };
    if !date_part.is_empty() {
        parts.push(date_part);
    }

    // Кол-во кабинетов
    let selected_count = ctx.connection_mp_refs.len();
    if selected_count > 0 {
        parts.push(format!("{} каб.", selected_count));
    }

    parts.join(" · ")
}

fn has_custom_css_for_all(indicator_defs: &HashMap<String, IndicatorDef>) -> bool {
    !indicator_defs.is_empty()
        && indicator_defs.values().all(|def| {
            def.view_spec
                .custom_css
                .as_deref()
                .map(|css| !css.trim().is_empty())
                .unwrap_or(false)
        })
}

fn render_indicator_html(
    item: &DashboardItem,
    view_ctx: &ViewContext,
    indicator_defs: &HashMap<String, IndicatorDef>,
    indicator_values: &HashMap<String, ComputedValue>,
    theme: &str,
    design_key: &str,
) -> String {
    let def = indicator_defs.get(&item.indicator_id);
    let computed = indicator_values.get(&item.indicator_id);
    let preview_values = def.map(|d| &d.view_spec.preview_values);
    let preview = |key: &str| -> String {
        preview_values
            .and_then(|pv| pv.get(key))
            .cloned()
            .unwrap_or_default()
    };
    let hidden: std::collections::HashSet<String> = preview("_hidden")
        .split(',')
        .filter_map(|k| {
            let key = k.trim();
            if key.is_empty() {
                None
            } else {
                Some(key.to_string())
            }
        })
        .collect();
    let is_hidden = |key: &str| hidden.contains(key);

    let format_spec = def
        .map(|d| d.view_spec.format.clone())
        .unwrap_or(serde_json::Value::Null);

    // For fields with a live source: use computed value, fallback to preview("key") if missing.
    // For fields with source="—": always use preview("key").
    let value_str = computed
        .and_then(|cv| cv.value)
        .map(|v| format_value(v, &format_spec))
        .unwrap_or_else(|| {
            let pv = preview("value");
            if !pv.is_empty() { pv } else { "—".to_string() }
        });

    let change_pct = computed.and_then(|cv| cv.change_percent);
    let delta_str = change_pct
        .map(|pct| {
            if pct > 0.0 {
                format!("{:+.1}%", pct)
            } else if pct < 0.0 {
                format!("{:.1}%", pct)
            } else {
                "0.0%".to_string()
            }
        })
        .unwrap_or_else(|| {
            let pv = preview("delta");
            if !pv.is_empty() { pv } else { "—".to_string() }
        });
    let delta_dir: String = change_pct
        .map(|pct| {
            if pct > 0.0 { "up".to_string() }
            else if pct < 0.0 { "down".to_string() }
            else { "flat".to_string() }
        })
        .unwrap_or_else(|| {
            let pv = preview("delta_dir");
            if !pv.is_empty() { pv } else { "flat".to_string() }
        });

    let status: String = computed
        .and_then(|cv| cv.status.as_deref())
        .map(|s| match s {
            "Good"    => "ok",
            "Bad"     => "bad",
            "Warning" => "warn",
            _         => "neutral",
        })
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            let pv = preview("status");
            if !pv.is_empty() { pv } else { "neutral".to_string() }
        });

    let mut graph_type = preview("graph_type")
        .parse::<u8>()
        .ok()
        .map(|v| v.min(2))
        .unwrap_or_else(|| {
            let progress = preview("progress").parse::<u8>().unwrap_or(0);
            let has_spark = !preview("spark_points").trim().is_empty();
            if progress > 0 {
                1
            } else if has_spark {
                2
            } else {
                2
            }
        });
    if (graph_type == 1 && is_hidden("progress")) || (graph_type == 2 && is_hidden("spark")) {
        graph_type = 0;
    }
    let progress = if graph_type == 1 && !is_hidden("progress") {
        preview("progress").parse::<u8>().unwrap_or(0).min(100)
    } else {
        0
    };
    let spark_points = if graph_type == 2 && !is_hidden("spark") {
        let from_computed = computed
            .filter(|cv| !cv.spark_points.is_empty())
            .map(|cv| cv.spark_points.clone());
        from_computed.unwrap_or_else(|| {
            preview("spark_points")
                .split(',')
                .filter_map(|p| p.trim().parse::<f64>().ok())
                .collect()
        })
    } else {
        vec![]
    };
    let meta_1 = if is_hidden("meta_1") {
        String::new()
    } else {
        let val = preview("meta_1");
        if val.trim().is_empty() {
            compact_filter_hint(view_ctx)
        } else {
            val
        }
    };
    let meta_2 = if is_hidden("meta_2") {
        String::new()
    } else {
        preview("meta_2")
    };

    // name: use preview("name") if set, otherwise fallback to code/description
    let card_name = {
        let pv = preview("name");
        if !pv.trim().is_empty() { pv } else { item_title(item, indicator_defs) }
    };

    let params = IndicatorCardParams {
        style_name: design_key.to_string(),
        theme: theme.to_string(),
        name: card_name,
        value: value_str,
        unit: if is_hidden("unit") {
            String::new()
        } else {
            preview("unit")
        },
        delta: delta_str,
        delta_dir,
        status,
        chip: if is_hidden("chip") {
            String::new()
        } else {
            preview("chip")
        },
        col_class: "col-12".to_string(),
        graph_type,
        progress,
        spark_points,
        meta_1,
        meta_2,
        hint: if is_hidden("hint") {
            String::new()
        } else {
            preview("hint")
        },
        footer_1: if is_hidden("footer_1") {
            String::new()
        } else {
            preview("footer_1")
        },
        footer_2: if is_hidden("footer_2") {
            String::new()
        } else {
            preview("footer_2")
        },
        custom_html: None,
        custom_css: if design_key == "custom" {
            def.and_then(|d| d.view_spec.custom_css.clone())
        } else {
            None
        },
    };

    render_card_html(&params)
}

fn sort_groups_recursive(groups: &mut Vec<DashboardGroup>) {
    groups.sort_by_key(|g| g.sort_order);
    for group in groups {
        group.items.sort_by_key(|i| i.sort_order);
        sort_groups_recursive(&mut group.subgroups);
    }
}

fn render_group_html(
    group: &DashboardGroup,
    view_ctx: &ViewContext,
    indicator_defs: &HashMap<String, IndicatorDef>,
    indicator_values: &HashMap<String, ComputedValue>,
    theme: &str,
    design_key: &str,
    depth: usize,
) -> String {
    let title = if group.title.trim().is_empty() {
        "Без названия".to_string()
    } else {
        group.title.clone()
    };

    let title_class = if depth == 0 {
        "group__title group__title--root"
    } else {
        "group__title group__title--sub"
    };

    let cards_html = if group.items.is_empty() {
        "<div class=\"cards-empty\">Нет индикаторов</div>".to_string()
    } else {
        group
            .items
            .iter()
            .map(|item| {
                let card_html = render_indicator_html(
                    item,
                    view_ctx,
                    indicator_defs,
                    indicator_values,
                    theme,
                    design_key,
                );
                format!(r#"<div class="card-slot" data-indicator-id="{}">{card_html}</div>"#, item.indicator_id)
            })
            .collect::<Vec<_>>()
            .join("")
    };

    let subgroups_html = group
        .subgroups
        .iter()
        .map(|sub| {
            render_group_html(
                sub,
                view_ctx,
                indicator_defs,
                indicator_values,
                theme,
                design_key,
                depth + 1,
            )
        })
        .collect::<Vec<_>>()
        .join("");

    format!(
        r#"<section class="group">
<div class="{title_class}">{title}</div>
<div class="cards">{cards_html}</div>
{subgroups_html}
</section>"#,
        title_class = title_class,
        title = escape_html(&title),
        cards_html = cards_html,
        subgroups_html = subgroups_html
    )
}

fn build_dashboard_srcdoc(
    groups: &[DashboardGroup],
    view_ctx: &ViewContext,
    theme: &str,
    design_key: &str,
    indicator_defs: &HashMap<String, IndicatorDef>,
    indicator_values: &HashMap<String, ComputedValue>,
    sidebar_scrollbar_thumb: &str,
    sidebar_scrollbar_thumb_hover: &str,
) -> String {
    let groups_html = if groups.is_empty() {
        "<div class=\"empty\">Дашборд пуст. Добавьте группы и индикаторы в редакторе.</div>"
            .to_string()
    } else {
        groups
            .iter()
            .map(|g| {
                render_group_html(
                    g,
                    view_ctx,
                    indicator_defs,
                    indicator_values,
                    theme,
                    design_key,
                    0,
                )
            })
            .collect::<Vec<_>>()
            .join("")
    };

    let style_css = get_style_css(design_key);
    let css = format!(
        r#"
{style_css}
html,body{{margin:0;padding:0;}}
:root{{
  --sb-thumb: {sidebar_scrollbar_thumb};
  --sb-thumb-hover: {sidebar_scrollbar_thumb_hover};
  --bi-primary:#3b82f6;
  --bi-success:#22c55e;
  --bi-danger:#ef4444;
  --bi-warning:#f59e0b;
  --bi-text:#1e293b;
  --bi-text-secondary:#64748b;
  --bi-bg:#ffffff;
  --bi-bg-secondary:#f8fafc;
  --bi-border:#e2e8f0;
}}
body[data-theme="dark"]{{
  --bi-text:#e5e7eb;
  --bi-text-secondary:#9aa4b2;
  --bi-bg:#0b1220;
  --bi-bg-secondary:#0f1a2e;
  --bi-border:rgba(255,255,255,.12);
}}
body{{
  background:transparent !important;
  min-height:100% !important;
  display:block !important;
  justify-content:initial !important;
  align-items:initial !important;
  padding:0 !important;
}}
html{{
  overflow:auto;
  scrollbar-width:thin;
  scrollbar-color:var(--sb-thumb) transparent;
}}
html::-webkit-scrollbar{{width:6px;height:6px;}}
html::-webkit-scrollbar-track{{background:transparent;}}
html::-webkit-scrollbar-thumb{{
  background:var(--sb-thumb);
  border-radius:3px;
}}
html::-webkit-scrollbar-thumb:hover{{
  background:var(--sb-thumb-hover);
}}
.dashboard{{
  margin:12px 110px 110px;
  display:flex;
  flex-direction:column;
  gap:14px;
}}
.group{{
  display:flex;
  flex-direction:column;
  gap:10px;
}}
.group__title{{
  font-weight:700;
  padding:0;
  margin:0;
  background:none !important;
  border:none !important;
  color:var(--text,var(--bi-text));
}}
.group__title--root{{font-size:16px;}}
.group__title--sub{{font-size:14px;opacity:.9;margin-top:2px;}}
.cards{{
  display:flex;
  flex-wrap:wrap;
  gap:12px;
  align-items:stretch;
}}
.card-slot{{
  flex:0 0 280px;
  width:280px;
  min-width:280px;
}}
.card-slot .indicator-card{{
  width:100%;
  min-height:124px;
}}
.cards-empty,.empty{{
  color:var(--muted,var(--bi-text-secondary));
  font-size:13px;
  padding:2px 0;
}}
"#,
        sidebar_scrollbar_thumb = sidebar_scrollbar_thumb,
        sidebar_scrollbar_thumb_hover = sidebar_scrollbar_thumb_hover,
    );

    let mut html = String::new();
    html.push_str("<!DOCTYPE html><html><head><meta charset=\"UTF-8\"><style>");
    html.push_str(&css);
    html.push_str(concat!(
        "</style><script>(function(){",
        "var ac=null;",
        "document.addEventListener('click',function(e){",
          "var btn=e.target.closest('.indicator-card');if(!btn)return;",
          "var slot=btn.closest('[data-indicator-id]');if(!slot)return;",
          "if(ac&&ac!==btn){ac.style.cssText='';}",
          "ac=btn;",
          "var r=btn.getBoundingClientRect();",
          "var cx=r.left+r.width/2,cy=r.top+r.height/2;",
          "var dx=(window.innerWidth/2-cx)*0.3,dy=(window.innerHeight/2-cy)*0.3;",
          "btn.style.transition='transform 0.22s cubic-bezier(0.4,0,0.2,1),opacity 0.22s ease';",
          "btn.style.transform='translate('+dx+'px,'+dy+'px) scale(1.08)';",
          "btn.style.opacity='0';btn.style.pointerEvents='none';",
          "setTimeout(function(){",
            "window.parent.postMessage({type:'indicator_click',id:slot.dataset.indicatorId,cx:cx,cy:cy},'*');",
          "},230);",
        "});",
        "window.addEventListener('message',function(e){",
          "if(!e.data||e.data.type!=='indicator_restore')return;",
          "if(!ac)return;",
          "var c=ac;ac=null;",
          "c.style.transition='transform 0.28s cubic-bezier(0.2,0.9,0.2,1),opacity 0.28s ease';",
          "c.style.transform='';c.style.opacity='';c.style.pointerEvents='';",
          "setTimeout(function(){c.style.transition='';},300);",
        "});",
        "})();</script></head><body data-theme=\""
    ));
    html.push_str(if theme == "light" { "light" } else { "dark" });
    html.push_str("\"><div class=\"dashboard\">");
    html.push_str(&groups_html);
    html.push_str("</div></body></html>");
    html
}

/// State passed from the iframe postMessage to drive the detail modal.
#[derive(Clone, Debug)]
struct IndicatorSelection {
    id: String,
    /// Horizontal offset of the card center from the viewport center (px).
    from_x: f64,
    /// Vertical offset of the card center from the viewport center (px).
    from_y: f64,
}

#[component]
pub fn BiDashboardView(id: String) -> impl IntoView {
    let tabs_ctx = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let loading: RwSignal<bool> = RwSignal::new(false);
    let error: RwSignal<Option<String>> = RwSignal::new(None);
    let dashboard: RwSignal<Option<BiDashboardData>> = RwSignal::new(None);
    let view_ctx: RwSignal<ViewContext> = RwSignal::new(ViewContext::default());
    let rendered_ctx: RwSignal<ViewContext> = RwSignal::new(ViewContext::default());
    let dashboard_filter_defs: RwSignal<Vec<FilterDef>> = RwSignal::new(vec![]);
    let indicator_defs: RwSignal<HashMap<String, IndicatorDef>> = RwSignal::new(HashMap::new());
    let indicator_values: RwSignal<HashMap<String, ComputedValue>> = RwSignal::new(HashMap::new());
    let indicator_refreshing: RwSignal<bool> = RwSignal::new(false);
    let indicator_refresh_seq: RwSignal<u64> = RwSignal::new(0);
    let dashboard_design: RwSignal<String> = RwSignal::new(default_design_name().to_string());
    let thaw_theme_ctx = leptos::context::use_context::<ThawThemeContext>();
    let selected_indicator: RwSignal<Option<IndicatorSelection>> = RwSignal::new(None);

    // Listen for postMessage events from the indicator cards iframe.
    let _ = window_event_listener(leptos::ev::message, move |ev: web_sys::MessageEvent| {
        let data = ev.data();
        let get_str = |key: &str| -> Option<String> {
            js_sys::Reflect::get(&data, &wasm_bindgen::JsValue::from_str(key))
                .ok()
                .and_then(|v| v.as_string())
        };
        let get_f64 = |key: &str| -> Option<f64> {
            js_sys::Reflect::get(&data, &wasm_bindgen::JsValue::from_str(key))
                .ok()
                .and_then(|v| v.as_f64())
        };
        if get_str("type").as_deref() != Some("indicator_click") {
            return;
        }
        let Some(indicator_id) = get_str("id") else {
            return;
        };
        let cx_in_iframe = get_f64("cx").unwrap_or(0.0);
        let cy_in_iframe = get_f64("cy").unwrap_or(0.0);

        let (from_x, from_y) = {
            let win = web_sys::window().unwrap();
            let doc = win.document().unwrap();
            let iframe_el = doc.query_selector(".dashboard-viewer__iframe").ok().flatten();
            if let Some(el) = iframe_el {
                let rect = el.get_bounding_client_rect();
                let vw = win.inner_width().unwrap().as_f64().unwrap_or(1280.0);
                let vh = win.inner_height().unwrap().as_f64().unwrap_or(800.0);
                let cx_vp = rect.left() + cx_in_iframe;
                let cy_vp = rect.top() + cy_in_iframe;
                (cx_vp - vw / 2.0, cy_vp - vh / 2.0)
            } else {
                (0.0, 0.0)
            }
        };

        selected_indicator.set(Some(IndicatorSelection {
            id: indicator_id,
            from_x,
            from_y,
        }));
    });

    let current_theme = Signal::derive(move || {
        if let Some(ctx) = thaw_theme_ctx {
            let theme = ctx.0.get();
            if theme.name == "light" {
                "light".to_string()
            } else {
                "dark".to_string()
            }
        } else {
            get_app_theme()
        }
    });

    let dashboard_design_options = Signal::derive(move || {
        let defs = indicator_defs.get();
        available_designs(has_custom_css_for_all(&defs))
    });

    let is_filter_expanded = RwSignal::new(true);

    let active_filters_count = Signal::derive(move || dashboard_filter_defs.get().len());

    reload_dashboard_data(
        id.clone(),
        loading,
        error,
        dashboard,
        view_ctx,
        dashboard_filter_defs,
        indicator_defs,
        indicator_values,
        false,
    );

    // Реактивный эффект: пересчитываем данные индикаторов при смене фильтров
    Effect::new(move |_| {
        let ctx = view_ctx.get();
        let defs = indicator_defs.get();
        let request_id = indicator_refresh_seq.get_untracked().wrapping_add(1);
        indicator_refresh_seq.set(request_id);
        if defs.is_empty() {
            rendered_ctx.set(ctx);
            indicator_values.set(HashMap::new());
            indicator_refreshing.set(false);
            return;
        }
        indicator_refreshing.set(true);
        leptos::task::spawn_local(async move {
            let computed_schema = fetch_indicator_data(&defs, &ctx).await;
            let computed_view = fetch_indicator_data_view(&defs, &ctx).await;
            let mut computed = computed_schema;
            computed.extend(computed_view);
            if indicator_refresh_seq.get_untracked() != request_id {
                return;
            }
            rendered_ctx.set(ctx);
            indicator_values.set(computed);
            indicator_refreshing.set(false);
        });
    });

    Effect::new(move |_| {
        let current = dashboard_design.get();
        let allowed = dashboard_design_options.get();
        if !allowed.iter().any(|entry| entry.key == current.as_str()) {
            dashboard_design.set(default_design_name().to_string());
        }
    });

    let srcdoc = Signal::derive(move || {
        dashboard
            .get()
            .map(|data| {
                let mut groups = data.layout.groups.clone();
                sort_groups_recursive(&mut groups);
                let (thumb, hover) = get_sidebar_scrollbar_tokens();
                build_dashboard_srcdoc(
                    &groups,
                    &rendered_ctx.get(),
                    &current_theme.get(),
                    &dashboard_design.get(),
                    &indicator_defs.get(),
                    &indicator_values.get(),
                    &thumb,
                    &hover,
                )
            })
            .unwrap_or_default()
    });

    view! {
        <PageFrame page_id="a025_bi_dashboard--view" category="dashboard">
            {move || if loading.get() {
                view! { <div class="placeholder">"Загрузка дашборда..."</div> }.into_any()
            } else if let Some(e) = error.get() {
                view! {
                    <div class="warning-box">
                        <span class="warning-box__icon">"⚠"</span>
                        <span class="warning-box__text">{e}</span>
                    </div>
                }.into_any()
            } else if let Some(data) = dashboard.get() {
                let title = data.description.clone();
                let code = data.code.clone();
                let detail_tab_key = format!("a025_bi_dashboard_details_{}", id.clone());
                let detail_tab_title = format!("Дашборд · {}", code.clone());
                let tabs_ctx_edit = tabs_ctx;
                let refresh_id = id.clone();

                view! {
                    <div class="page__header">
                        <div class="page__header-left">
                            <h1 class="page__title">{title}</h1>
                            <span class="text-muted" style="margin-left: 8px">{code}</span>
                        </div>
                        <div class="page__header-right">
                            <div style="display:flex; align-items:center; gap:8px; margin-right: 6px;">
                                <span class="text-muted" style="font-size: 12px;">"Дизайн"</span>
                                <select
                                    class="form__select form__select--sm"
                                    prop:value=move || dashboard_design.get()
                                    on:change=move |ev| {
                                        let target = ev.target().unwrap();
                                        let sel: &web_sys::HtmlSelectElement = target.unchecked_ref();
                                        dashboard_design.set(sel.value());
                                    }
                                >
                                    {move || {
                                        dashboard_design_options
                                            .get()
                                            .into_iter()
                                            .map(|entry| {
                                                view! { <option value=entry.key>{entry.label}</option> }
                                            })
                                            .collect_view()
                                    }}
                                </select>
                            </div>
                            <Button
                                appearance=ButtonAppearance::Secondary
                                on_click=move |_| {
                                    tabs_ctx_edit.open_tab(&detail_tab_key, &detail_tab_title);
                                }
                            >
                                {icon("edit-2")} " Изменить"
                            </Button>
                            <Button
                                appearance=ButtonAppearance::Secondary
                                on_click=move |_| {
                                    reload_dashboard_data(
                                        refresh_id.clone(),
                                        loading,
                                        error,
                                        dashboard,
                                        view_ctx,
                                        dashboard_filter_defs,
                                        indicator_defs,
                                        indicator_values,
                                        true,
                                    );
                                }
                            >
                                {icon("refresh")} " Обновить"
                            </Button>
                        </div>
                    </div>

                    <div class="filter-panel">
                        <div class="filter-panel-header">
                            <div
                                class="filter-panel-header__left"
                                on:click=move |_| is_filter_expanded.update(|e| *e = !*e)
                            >
                                <svg
                                    width="16" height="16" viewBox="0 0 24 24"
                                    fill="none" stroke="currentColor" stroke-width="2"
                                    stroke-linecap="round" stroke-linejoin="round"
                                    class=move || if is_filter_expanded.get() {
                                        "filter-panel__chevron filter-panel__chevron--expanded"
                                    } else {
                                        "filter-panel__chevron"
                                    }
                                >
                                    <polyline points="6 9 12 15 18 9"></polyline>
                                </svg>
                                {icon("filter")}
                                <span class="filter-panel__title">"Фильтры"</span>
                                {move || {
                                    let count = active_filters_count.get();
                                    if count > 0 {
                                        view! { <span class="filter-panel__badge">{count}</span> }.into_any()
                                    } else {
                                        view! { <></> }.into_any()
                                    }
                                }}
                            </div>
                            <div class="filter-panel-header__right" />
                        </div>

                        <Show when=move || is_filter_expanded.get()>
                            <div class="filter-panel-content">
                                {move || {
                                    let filters = dashboard_filter_defs.get();
                                    if filters.is_empty() {
                                        view! {
                                            <div class="placeholder placeholder--small">
                                                "Для этого дашборда не настроены фильтры."
                                            </div>
                                        }
                                            .into_any()
                                    } else {
                                        view! { <FilterBar filters=filters ctx=view_ctx /> }.into_any()
                                    }
                                }}
                            </div>
                        </Show>
                    </div>

                    <div class="dashboard-content" style="position: relative;">
                        <iframe
                            class="dashboard-viewer__iframe"
                            sandbox="allow-scripts"
                            srcdoc=move || srcdoc.get()
                        />
                        <Show when=move || indicator_refreshing.get()>
                            <div class="loading-overlay">
                                <div class="loading-overlay__spinner">
                                    <span class="spinner spinner--sm" />
                                    <span>"Обновление индикаторов..."</span>
                                </div>
                            </div>
                        </Show>
                    </div>
                }.into_any()
            } else {
                view! { <div class="placeholder">"Дашборд не найден"</div> }.into_any()
            }}

            {move || {
                let Some(sel) = selected_indicator.get() else {
                    return view! { <></> }.into_any();
                };
                let on_close = Callback::new(move |_| selected_indicator.set(None));
                view! {
                    <IndicatorDetailModal
                        sel=sel
                        indicator_defs=indicator_defs
                        indicator_values=indicator_values
                        on_close=on_close
                        ctx=rendered_ctx.get_untracked()
                    />
                }.into_any()
            }}
        </PageFrame>
    }
}

/// Sends an `indicator_restore` postMessage to the dashboard iframe,
/// telling it to animate the previously selected card back into place.
fn send_indicator_restore() {
    if let Some(win) = web_sys::window() {
        if let Some(doc) = win.document() {
            if let Ok(Some(el)) = doc.query_selector(".dashboard-viewer__iframe") {
                let iframe: &web_sys::HtmlIFrameElement = el.unchecked_ref();
                if let Some(cw) = iframe.content_window() {
                    let msg = js_sys::Object::new();
                    let _ = js_sys::Reflect::set(
                        &msg,
                        &wasm_bindgen::JsValue::from_str("type"),
                        &wasm_bindgen::JsValue::from_str("indicator_restore"),
                    );
                    let _ = cw.post_message(&msg, "*");
                }
            }
        }
    }
}

#[component]
fn IndicatorDetailModal(
    sel: IndicatorSelection,
    indicator_defs: RwSignal<HashMap<String, IndicatorDef>>,
    indicator_values: RwSignal<HashMap<String, ComputedValue>>,
    on_close: Callback<()>,
    ctx: ViewContext,
) -> impl IntoView {
    let def = indicator_defs.get_untracked().get(&sel.id).cloned();
    let computed = indicator_values.get_untracked().get(&sel.id).cloned();

    let name = def.as_ref().map(|d| {
        if d.description.trim().is_empty() {
            d.code.clone()
        } else {
            d.description.clone()
        }
    }).unwrap_or_else(|| sel.id.clone());

    let code = def.as_ref().map(|d| d.code.clone()).unwrap_or_default();
    let schema_id = def.as_ref().map(|d| d.data_spec.schema_id.clone()).unwrap_or_default();
    let format_spec = def.as_ref().map(|d| d.view_spec.format.clone()).unwrap_or(serde_json::Value::Null);

    // Drilldown: only available when indicator has a DataView (view_id)
    let view_id_opt = def.as_ref().and_then(|d| d.data_spec.view_id.clone());
    let metric_id_opt = def.as_ref().and_then(|d| d.data_spec.metric_id.clone());
    let has_drilldown = view_id_opt.is_some();

    // Async-загружаемые измерения из DataViewMeta
    let dv_dims: RwSignal<Option<Vec<(String, String)>>> = RwSignal::new(None);
    if let Some(vid) = view_id_opt.clone() {
        let vid2 = vid.clone();
        spawn_local(async move {
            match fetch_dataview_meta(&vid2).await {
                Ok(meta) => {
                    let dims = meta.available_dimensions
                        .into_iter()
                        .map(|d| (d.id, d.label))
                        .collect();
                    dv_dims.set(Some(dims));
                }
                Err(e) => {
                    leptos::logging::warn!("DataView meta fetch failed for {}: {}", vid2, e);
                    dv_dims.set(Some(vec![]));
                }
            }
        });
    }

    let selected_dim: RwSignal<Option<String>> = RwSignal::new(None);

    let value_str = computed.as_ref()
        .and_then(|cv| cv.value)
        .map(|v| format_value(v, &format_spec))
        .unwrap_or_else(|| "—".to_string());

    let prev_str = computed.as_ref()
        .and_then(|cv| cv.previous_value)
        .map(|v| format_value(v, &format_spec))
        .unwrap_or_else(|| "—".to_string());

    let change_pct = computed.as_ref().and_then(|cv| cv.change_percent);
    let delta_str = change_pct.map(|pct| {
        if pct > 0.0 { format!("+{:.1}%", pct) } else { format!("{:.1}%", pct) }
    }).unwrap_or_else(|| "—".to_string());

    let delta_class = match change_pct {
        Some(p) if p > 0.0 => "indicator-detail__delta--up",
        Some(p) if p < 0.0 => "indicator-detail__delta--down",
        _ => "indicator-detail__delta--flat",
    };

    let status = computed.as_ref()
        .and_then(|cv| cv.status.clone())
        .unwrap_or_else(|| "Neutral".to_string());

    let status_class = match status.as_str() {
        "Good" => "indicator-detail__status--good",
        "Bad" => "indicator-detail__status--bad",
        "Warning" => "indicator-detail__status--warning",
        _ => "indicator-detail__status--neutral",
    };

    let modal_style = format!(
        "--from-x: {}px; --from-y: {}px;",
        sel.from_x as i32,
        sel.from_y as i32
    );

    // Closing state: triggers reverse animation before the modal is removed from DOM.
    let is_closing = RwSignal::new(false);

    let do_close = Callback::new(move |_: ()| {
        if is_closing.get_untracked() { return; }
        is_closing.set(true);
        // Tell the iframe to restore the card immediately (animations overlap naturally).
        send_indicator_restore();
        spawn_local(async move {
            TimeoutFuture::new(220).await;
            on_close.run(());
        });
    });

    // Mouse-down tracking so that dragging out of the overlay does not close it.
    let overlay_mousedown = RwSignal::new(false);

    let is_direct = |ev: &leptos::ev::MouseEvent| -> bool {
        matches!((ev.target(), ev.current_target()), (Some(t), Some(ct)) if t == ct)
    };

    view! {
        <div
            class=move || {
                if is_closing.get() {
                    "modal-overlay modal-overlay--indicator modal-overlay--closing".to_string()
                } else {
                    "modal-overlay modal-overlay--indicator".to_string()
                }
            }
            style="z-index: 1000;"
            on:mousedown=move |ev: leptos::ev::MouseEvent| {
                overlay_mousedown.set(is_direct(&ev));
            }
            on:click=move |ev: leptos::ev::MouseEvent| {
                if overlay_mousedown.get() && is_direct(&ev) {
                    overlay_mousedown.set(false);
                    do_close.run(());
                }
            }
        >
            <div
                class=move || {
                    if is_closing.get() {
                        "modal indicator-detail-modal indicator-detail-modal--closing".to_string()
                    } else {
                        "modal indicator-detail-modal".to_string()
                    }
                }
                style=modal_style
                on:click=|ev: leptos::ev::MouseEvent| ev.stop_propagation()
            >
                <div class="modal-header">
                    <div class="modal-header__left">
                        <span class="modal-title">{name.clone()}</span>
                        {if !code.is_empty() {
                            view! { <span class="indicator-detail__code-badge">{code}</span> }.into_any()
                        } else {
                            view! { <></> }.into_any()
                        }}
                    </div>
                    <button
                        class="modal__close"
                        on:click=move |_| do_close.run(())
                        aria-label="Закрыть"
                    >
                        {icon("x")}
                    </button>
                </div>
                <div class="modal-body indicator-detail__body">
                    <div class="indicator-detail__value-row">
                        <span class="indicator-detail__value">{value_str}</span>
                        <span class=format!("indicator-detail__status {status_class}")>{status.clone()}</span>
                    </div>
                    <div class="indicator-detail__meta">
                        <div class="indicator-detail__meta-row">
                            <span class="indicator-detail__meta-label">"Изменение"</span>
                            <span class=format!("indicator-detail__delta {delta_class}")>{delta_str}</span>
                        </div>
                        <div class="indicator-detail__meta-row">
                            <span class="indicator-detail__meta-label">"Предыдущий период"</span>
                            <span class="indicator-detail__meta-value">{prev_str}</span>
                        </div>
                        {if !schema_id.is_empty() {
                            view! {
                                <div class="indicator-detail__meta-row">
                                    <span class="indicator-detail__meta-label">"Источник данных"</span>
                                    <span class="indicator-detail__meta-value">{schema_id.clone()}</span>
                                </div>
                            }.into_any()
                        } else {
                            view! { <></> }.into_any()
                        }}
                    </div>

                    // Drilldown section — visible only when indicator has a DataView (view_id)
                    {if has_drilldown {
                        let indicator_id_c = sel.id.clone();
                        let indicator_name_c = name.clone();
                        let view_id_c = view_id_opt.clone().unwrap_or_default();
                        let metric_id_c = metric_id_opt.clone();
                        let ctx_c = ctx.clone();
                        let tabs_store = leptos::context::use_context::<AppGlobalContext>();
                        let date_range_label = format!("{} — {}", ctx.date_from, ctx.date_to);

                        view! {
                            <div class="drill-picker">
                                <div class="drill-picker__header">
                                    <span class="drill-picker__title">"Детализация"</span>
                                    <span class="drill-picker__subtitle">{date_range_label}</span>
                                </div>

                                // Reactive: show loading or options depending on async fetch state
                                {move || {
                                    match dv_dims.get() {
                                        None => view! {
                                            <div class="drill-picker__loading">
                                                <span class="spinner spinner--sm" />
                                                " Загрузка измерений..."
                                            </div>
                                        }.into_any(),

                                        Some(dims_list) => {
                                            // Clone per reactive call so inner `move` closures can capture
                                            let id    = indicator_id_c.clone();
                                            let vid   = view_id_c.clone();
                                            let metric = metric_id_c.clone();
                                            let drill_ctx = ctx_c.clone();
                                            let iname = indicator_name_c.clone();
                                            let ts    = tabs_store;

                                            view! {
                                                <div class="drill-picker__list">
                                                    {dims_list.into_iter().map(|(field_id, label)| {
                                                        let fid_sel = field_id.clone();
                                                        let fid_click = field_id.clone();
                                                        view! {
                                                            <div
                                                                class=move || {
                                                                    if selected_dim.get().as_deref() == Some(fid_sel.as_str()) {
                                                                        "drill-picker__item drill-picker__item--selected"
                                                                    } else {
                                                                        "drill-picker__item"
                                                                    }
                                                                }
                                                                on:click=move |_| {
                                                                    selected_dim.set(Some(fid_click.clone()));
                                                                }
                                                            >
                                                                <span class="drill-picker__radio">
                                                                    <span class="drill-picker__radio-dot" />
                                                                </span>
                                                                <span class="drill-picker__item-label">{label}</span>
                                                            </div>
                                                        }
                                                    }).collect_view()}
                                                </div>

                                                <button
                                                    class="btn btn--primary drill-picker__submit"
                                                    disabled=move || selected_dim.get().is_none()
                                                    on:click=move |_| {
                                                        let Some(dim) = selected_dim.get_untracked() else { return };
                                                        let dim_label = dv_dims.get_untracked()
                                                            .unwrap_or_default()
                                                            .into_iter()
                                                            .find(|(fid, _)| fid == &dim)
                                                            .map(|(_, lbl)| lbl)
                                                            .unwrap_or_else(|| dim.clone());

                                                        let tab_title = format!("{} — {}", iname, dim_label);
                                                        let store_opt = ts.clone();

                                                        // Clone for async block (can't move out of FnMut)
                                                        let vid2   = vid.clone();
                                                        let id2    = id.clone();
                                                        let iname2 = iname.clone();
                                                        let metric2 = metric.clone();
                                                        let ctx2   = drill_ctx.clone();

                                                        spawn_local(async move {
                                                            if let Some(session_id) = post_drilldown_session(
                                                                vid2,
                                                                id2,
                                                                iname2,
                                                                metric2,
                                                                dim,
                                                                dim_label,
                                                                ctx2,
                                                            ).await {
                                                                let tab_key = format!("drilldown__{}", session_id);
                                                                if let Some(ref store) = store_opt {
                                                                    store.open_tab(&tab_key, &tab_title);
                                                                }
                                                            }
                                                            do_close.run(());
                                                        });
                                                    }
                                                >
                                                    "Сформировать отчёт"
                                                </button>
                                            }.into_any()
                                        }
                                    }
                                }}
                            </div>
                        }.into_any()
                    } else {
                        view! { <></> }.into_any()
                    }}
                </div>
            </div>
        </div>
    }
}

// ── Drilldown session helper ──────────────────────────────────────────────────

async fn post_drilldown_session(
    view_id: String,
    indicator_id: String,
    indicator_name: String,
    metric_id: Option<String>,
    group_by: String,
    group_by_label: String,
    ctx: ViewContext,
) -> Option<String> {
    let body = serde_json::json!({
        "view_id": view_id,
        "indicator_id": indicator_id,
        "indicator_name": indicator_name,
        "metric_id": metric_id,
        "group_by": group_by,
        "group_by_label": group_by_label,
        "date_from": ctx.date_from,
        "date_to": ctx.date_to,
        "period2_from": ctx.period2_from,
        "period2_to": ctx.period2_to,
        "connection_mp_refs": ctx.connection_mp_refs,
        "params": ctx.params,
    });

    let url = format!("{}/api/sys-drilldown", api_base());
    let resp = Request::post(&url)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .ok()?
        .send()
        .await
        .ok()?;

    if !resp.ok() {
        leptos::logging::error!("post_drilldown_session: HTTP {}", resp.status());
        return None;
    }

    let json: serde_json::Value = resp.json().await.ok()?;
    json["session_id"].as_str().map(String::from)
}
