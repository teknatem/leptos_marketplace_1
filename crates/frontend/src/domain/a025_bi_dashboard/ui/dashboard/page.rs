use crate::app::ThawThemeContext;
use crate::data_view::api as dv_api;
use crate::data_view::types::{FilterDef, FilterRef};
use crate::data_view::ui::filter_bar::apply_defaults;
use crate::data_view::ui::FilterBar;
use crate::general_ledger::api::fetch_gl_dimensions;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::bi_card::{
    available_designs, default_design_name, get_style_css, render_card_html, IndicatorCardParams,
};
use crate::shared::icons::icon;
use crate::shared::indicator_format::{format_int_with_triads, format_money_with_format_spec};
use crate::shared::page_frame::PageFrame;
use chrono::NaiveDate;
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
struct IndicatorDataSpec {
    #[serde(default)]
    pub view_id: Option<String>,
    #[serde(default)]
    pub metric_id: Option<String>,
}

#[derive(Clone, Debug, Default, serde::Deserialize)]
struct IndicatorParamDef {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub default_value: Option<String>,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct IndicatorDef {
    pub id: String,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(default)]
    pub view_spec: IndicatorViewSpec,
    #[serde(default)]
    pub data_spec: IndicatorDataSpec,
    #[serde(default)]
    pub params: Vec<IndicatorParamDef>,
}

/// Computed value from /api/a024-bi-indicator/compute-batch
#[derive(Clone, Debug, Default, serde::Deserialize)]
struct ComputedValue {
    /// id сериализуется как строка (IndicatorId — newtype over String)
    pub id: String,
    pub value: Option<f64>,
    pub previous_value: Option<f64>,
    pub change_percent: Option<f64>,
    /// "Good" | "Bad" | "Neutral" | "Warning"
    pub status: Option<String>,
    #[serde(default)]
    pub subtitle: Option<String>,
    #[serde(default)]
    pub details: Vec<String>,
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct DrillDimensionGroup {
    title: &'static str,
    subtitle: &'static str,
    items: Vec<(String, String)>,
}

const GL_TURNOVER_DATA_VIEW_ID_LOCAL: &str = "dv004_general_ledger_turnovers";

fn indicator_default_params(def: &IndicatorDef) -> HashMap<String, String> {
    def.params
        .iter()
        .filter_map(|param| {
            let value = param.default_value.as_ref()?.trim();
            if param.key.trim().is_empty() || value.is_empty() {
                None
            } else {
                Some((param.key.clone(), value.to_string()))
            }
        })
        .collect()
}

fn parse_gl_turnover_items(params: &HashMap<String, String>) -> Vec<String> {
    if let Some(items) = params
        .get("turnover_items")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        let mut seen = std::collections::HashSet::new();
        return items
            .split(|ch| ch == ',' || ch == ';' || ch == '\n' || ch == '\r')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .filter_map(|token| {
                let code = match token.chars().next() {
                    Some('+') | Some('-') => token[1..].trim(),
                    _ => token,
                };
                if code.is_empty() || !seen.insert(code.to_string()) {
                    None
                } else {
                    Some(code.to_string())
                }
            })
            .collect();
    }

    params
        .get("turnover_code")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| vec![value.to_string()])
        .unwrap_or_default()
}

async fn fetch_indicator_drill_dimensions(
    def: &IndicatorDef,
    params: &HashMap<String, String>,
) -> Result<Vec<(String, String)>, String> {
    let Some(view_id) = def.data_spec.view_id.as_deref() else {
        return Ok(vec![]);
    };

    if view_id != GL_TURNOVER_DATA_VIEW_ID_LOCAL {
        let meta = fetch_dataview_meta(view_id).await?;
        return Ok(meta
            .available_dimensions
            .into_iter()
            .map(|dim| (dim.id, dim.label))
            .collect());
    }

    let turnover_codes = parse_gl_turnover_items(params);
    if turnover_codes.is_empty() {
        return Ok(vec![]);
    }

    let mut common_dims: Option<Vec<(String, String)>> = None;
    for turnover_code in turnover_codes {
        let response = fetch_gl_dimensions(&turnover_code).await?;
        let dims = response
            .dimensions
            .into_iter()
            .map(|dim| (dim.id, dim.label))
            .collect::<Vec<_>>();
        common_dims = Some(match common_dims.take() {
            None => dims,
            Some(current) => current
                .into_iter()
                .filter(|(id, _)| dims.iter().any(|(candidate_id, _)| candidate_id == id))
                .collect(),
        });
    }

    Ok(common_dims.unwrap_or_default())
}

fn merge_indicator_params(
    defaults: &HashMap<String, String>,
    overrides: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut merged = defaults.clone();
    merged.extend(overrides.clone());
    merged
}

fn group_drill_dimensions(dims: &[(String, String)]) -> Vec<DrillDimensionGroup> {
    let classify = |id: &str| match id {
        "entry_date" | "connection_mp_ref" | "layer" => 0,
        "registrator_type" | "registrator_ref" => 1,
        "nomenclature" | "dim1_category" | "dim2_line" | "dim3_model" | "dim4_format"
        | "dim5_sink" | "dim6_size" => 2,
        _ => 3,
    };

    let mut buckets: [Vec<(String, String)>; 4] = [vec![], vec![], vec![], vec![]];
    for (id, label) in dims {
        buckets[classify(id)].push((id.clone(), label.clone()));
    }

    let specs = [
        ("Основные", "Период, кабинет и слой учета", 0),
        ("Документы", "Тип и конкретный документ-регистратор", 1),
        ("Номенклатура", "Товар и товарные аналитики", 2),
        ("Другое", "Редко используемые измерения", 3),
    ];

    specs
        .into_iter()
        .filter_map(|(title, subtitle, index)| {
            let items = std::mem::take(&mut buckets[index]);
            if items.is_empty() {
                None
            } else {
                Some(DrillDimensionGroup {
                    title,
                    subtitle,
                    items,
                })
            }
        })
        .collect()
}

/// Reads a non-OK HTTP response and returns a human-readable error string.
/// For 403 responses that carry the backend's `access_denied` JSON body,
/// formats the scope name and required access level in Russian.
async fn read_http_error(resp: web_sys::Response) -> String {
    let status = resp.status();
    if status == 403 {
        if let Ok(promise) = resp.text() {
            if let Ok(val) = wasm_bindgen_futures::JsFuture::from(promise).await {
                if let Some(text) = val.as_string() {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        if json["error"].as_str() == Some("access_denied") {
                            let scope = json["scope_id"].as_str().unwrap_or("неизвестен");
                            let access = match json["required_access"].as_str().unwrap_or("all") {
                                "read" => "чтение",
                                _ => "полный доступ",
                            };
                            return format!(
                                "Доступ запрещён: недостаточно прав для «{}» (требуется: {})",
                                scope, access
                            );
                        }
                    }
                }
            }
        }
        return "Доступ запрещён (403 Forbidden)".to_string();
    }
    format!("Ошибка HTTP {}", status)
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
        return Err(read_http_error(resp).await);
    }

    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("{e}"))
}

fn collect_indicator_ids(
    groups: &[DashboardGroup],
    out: &mut Vec<String>,
    seen: &mut std::collections::HashSet<String>,
) {
    for group in groups {
        for item in &group.items {
            if seen.insert(item.indicator_id.clone()) {
                out.push(item.indicator_id.clone());
            }
        }
        collect_indicator_ids(&group.subgroups, out, seen);
    }
}

async fn fetch_indicator_defs(ids: &[String]) -> Result<HashMap<String, IndicatorDef>, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    if ids.is_empty() {
        return Ok(HashMap::new());
    }

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);
    let body = serde_json::json!({ "ids": ids });
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body.to_string()));

    let url = format!("{}/api/a024-bi-indicator/resolve-batch", api_base());
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
        return Err(read_http_error(resp).await);
    }

    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    let items: Vec<serde_json::Value> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    let mut out = HashMap::new();
    for item in items {
        let Ok(def) = serde_json::from_value::<IndicatorDef>(item) else {
            continue;
        };
        out.insert(def.id.clone(), def);
    }

    Ok(out)
}

fn resolve_dashboard_filters(filter_refs: &[FilterRef], registry: &[FilterDef]) -> Vec<FilterDef> {
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
                        if existing
                            .default_value
                            .as_deref()
                            .unwrap_or("")
                            .trim()
                            .is_empty()
                        {
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
    let current_month_start = NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap_or(now);
    let current_month_end = if now.month() == 12 {
        NaiveDate::from_ymd_opt(now.year() + 1, 1, 1)
            .map(|d| d - Duration::days(1))
            .unwrap_or(now)
    } else {
        NaiveDate::from_ymd_opt(now.year(), now.month() + 1, 1)
            .map(|d| d - Duration::days(1))
            .unwrap_or(now)
    };
    let (prev_year, prev_month) = if now.month() == 1 {
        (now.year() - 1, 12)
    } else {
        (now.year(), now.month() - 1)
    };
    let previous_month_start =
        NaiveDate::from_ymd_opt(prev_year, prev_month, 1).unwrap_or(current_month_start);
    let previous_month_end = current_month_start - Duration::days(1);

    ViewContext {
        date_from: current_month_start.format("%Y-%m-%d").to_string(),
        date_to: current_month_end.format("%Y-%m-%d").to_string(),
        period2_from: Some(previous_month_start.format("%Y-%m-%d").to_string()),
        period2_to: Some(previous_month_end.format("%Y-%m-%d").to_string()),
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

/// Compute dashboard indicator values through /api/a024-bi-indicator/compute-batch.
/// Returns `Err` with a human-readable message on HTTP errors (including 403 access denied).
async fn fetch_indicator_data(
    indicator_defs: &HashMap<String, IndicatorDef>,
    ctx: &ViewContext,
) -> Result<HashMap<String, ComputedValue>, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let indicator_ids: Vec<String> = indicator_defs.keys().cloned().collect();
    if indicator_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let body = serde_json::json!({
        "indicator_ids": indicator_ids,
        "date_from": ctx.date_from,
        "date_to": ctx.date_to,
        "period2_from": ctx.period2_from,
        "period2_to": ctx.period2_to,
        "connection_mp_refs": ctx.connection_mp_refs.join(","),
        "params": ctx.params,
    });

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);
    let body_str = body.to_string();
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body_str));

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let request = Request::new_with_str_and_init(
        &format!("{}/api/a024-bi-indicator/compute-batch", api_base()),
        &opts,
    )
    .map_err(|e| format!("{e:?}"))?;
    let _ = request.headers().set("Accept", "application/json");
    let _ = request.headers().set("Content-Type", "application/json");

    let resp_val = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_val.dyn_into().map_err(|e| format!("{e:?}"))?;

    if !resp.ok() {
        return Err(read_http_error(resp).await);
    }

    let text_promise = resp.text().map_err(|e| format!("{e:?}"))?;
    let text_val = wasm_bindgen_futures::JsFuture::from(text_promise)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text = text_val.as_string().ok_or_else(|| "bad text".to_string())?;

    let parsed: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;

    let values = parsed["values"].as_array().cloned().unwrap_or_default();
    let mut result: HashMap<String, ComputedValue> = HashMap::new();
    for val in values {
        if let Ok(cv) = serde_json::from_value::<ComputedValue>(val) {
            if !cv.id.is_empty() {
                result.insert(cv.id.clone(), cv);
            }
        }
    }
    Ok(result)
}

/// Вычислить DataView-индикаторы через /api/a024-bi-indicator/:id/compute
/// (те у которых задан data_spec.view_id)
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
                    let mut seen = std::collections::HashSet::new();
                    collect_indicator_ids(&data.layout.groups, &mut ids, &mut seen);
                    let defs = match fetch_indicator_defs(&ids).await {
                        Ok(d) => d,
                        Err(e) => {
                            error.set(Some(e));
                            loading.set(false);
                            return;
                        }
                    };

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
        "Money" => format_money_with_format_spec(value, format_spec),
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

fn format_full_value(value: f64, format_spec: &serde_json::Value) -> String {
    let kind = format_spec["kind"].as_str().unwrap_or("Number");
    match kind {
        "Money" => format_money_with_format_spec(value, format_spec),
        "Integer" => format_int_with_triads(value.round() as i64),
        "Percent" => {
            let decimals = format_spec["decimals"].as_u64().unwrap_or(1) as usize;
            format!("{:.prec$}%", value, prec = decimals)
        }
        _ => {
            let decimals = format_spec["decimals"].as_u64().unwrap_or(2) as usize;
            let formatted = format!("{:.prec$}", value, prec = decimals);
            let mut parts = formatted.splitn(2, '.');
            let whole = parts
                .next()
                .and_then(|s| s.parse::<i64>().ok())
                .map(format_int_with_triads)
                .unwrap_or_else(|| formatted.clone());
            match parts.next() {
                Some(frac) => format!("{whole}.{frac}"),
                None => whole,
            }
        }
    }
}

fn push_detail_line(lines: &mut Vec<String>, value: impl Into<String>) {
    let value = value.into();
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return;
    }
    if !lines.iter().any(|item| item == trimmed) {
        lines.push(trimmed.to_string());
    }
}

fn build_indicator_description(def: Option<&IndicatorDef>) -> Option<String> {
    let def = def?;
    if let Some(comment) = def.comment.as_ref().map(|value| value.trim()) {
        if !comment.is_empty() {
            return Some(comment.to_string());
        }
    }

    match (
        def.data_spec.view_id.as_deref(),
        def.data_spec.metric_id.as_deref(),
    ) {
        (Some(view_id), Some(metric_id)) if !view_id.trim().is_empty() => Some(format!(
            "Индикатор рассчитывается через {} по метрике {}.",
            view_id, metric_id
        )),
        (Some(view_id), None) if !view_id.trim().is_empty() => {
            Some(format!("Индикатор рассчитывается через {}.", view_id))
        }
        _ => None,
    }
}

fn build_indicator_details(
    def: Option<&IndicatorDef>,
    computed: Option<&ComputedValue>,
    effective_params: &HashMap<String, String>,
) -> Vec<String> {
    let mut lines = Vec::new();

    if let Some(computed) = computed {
        if let Some(subtitle) = computed.subtitle.as_deref() {
            push_detail_line(&mut lines, format!("Схема расчёта: {}", subtitle));
        }
        for detail in &computed.details {
            push_detail_line(&mut lines, detail.clone());
        }
    }

    if let Some(def) = def {
        if let Some(view_id) = def
            .data_spec
            .view_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            push_detail_line(&mut lines, format!("Источник данных: {}", view_id));
        }
        if let Some(metric_id) = def
            .data_spec
            .metric_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            push_detail_line(&mut lines, format!("Показатель: {}", metric_id));
        }
    }

    let mut param_pairs: Vec<_> = effective_params
        .iter()
        .filter(|(key, value)| {
            let key = key.as_str();
            !value.trim().is_empty()
                && key != "metric"
                && key != "period2_from"
                && key != "period2_to"
        })
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    param_pairs.sort_by(|left, right| left.0.cmp(&right.0));

    for (key, value) in param_pairs.into_iter().take(6) {
        push_detail_line(&mut lines, format!("Параметр: {} = {}", key, value));
    }

    lines
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

fn fmt_date_label(s: &str) -> Option<String> {
    let date = NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()?;
    Some(date.format("%d.%m.%Y").to_string())
}

fn period_label(from: &str, to: &str) -> String {
    match (fmt_date_label(from), fmt_date_label(to)) {
        (Some(f), Some(t)) if f == t => f,
        (Some(f), Some(t)) => format!("{f} — {t}"),
        (Some(f), None) => format!("с {f}"),
        (None, Some(t)) => format!("до {t}"),
        _ => "Период не задан".to_string(),
    }
}

/// Компактный хинт для meta_1: "Янв – Фев 2026 · 4 каб."
fn compact_filter_hint(ctx: &ViewContext) -> String {
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
            if !pv.is_empty() {
                pv
            } else {
                "—".to_string()
            }
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
            if !pv.is_empty() {
                pv
            } else {
                "—".to_string()
            }
        });
    let delta_dir: String = change_pct
        .map(|pct| {
            if pct > 0.0 {
                "up".to_string()
            } else if pct < 0.0 {
                "down".to_string()
            } else {
                "flat".to_string()
            }
        })
        .unwrap_or_else(|| {
            let pv = preview("delta_dir");
            if !pv.is_empty() {
                pv
            } else {
                "flat".to_string()
            }
        });

    let status: String = computed
        .and_then(|cv| cv.status.as_deref())
        .map(|s| match s {
            "Good" => "ok",
            "Bad" => "bad",
            "Warning" => "warn",
            _ => "neutral",
        })
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            let pv = preview("status");
            if !pv.is_empty() {
                pv
            } else {
                "neutral".to_string()
            }
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
        if !pv.trim().is_empty() {
            pv
        } else {
            item_title(item, indicator_defs)
        }
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
                format!(
                    r#"<div class="card-slot" data-indicator-id="{}">{card_html}</div>"#,
                    item.indicator_id
                )
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

#[component]
fn IndicatorRefreshOverlay(
    #[prop(into)] card_count: Signal<usize>,
    #[prop(into)] filter_hint: Signal<String>,
) -> impl IntoView {
    view! {
        <div class="loading-overlay loading-overlay--dashboard">
            <div class="indicator-refresh">
                <div class="indicator-refresh__badge">
                    {move || {
                        let count = card_count.get();
                        if count == 0 {
                            "Подготавливаем макет".to_string()
                        } else {
                            format!("{count} карточек")
                        }
                    }}
                </div>
                <div class="indicator-refresh__headline">
                    <span class="indicator-refresh__pulse"></span>
                    <div class="indicator-refresh__titles">
                        <strong>"Формируем индикаторы"</strong>
                        <span>
                            {move || {
                                let hint = filter_hint.get();
                                if hint.trim().is_empty() {
                                    "Собираем новые значения, сравниваем период и обновляем витрину.".to_string()
                                } else {
                                    format!("{hint} • обновляем значения и сравнение периодов")
                                }
                            }}
                        </span>
                    </div>
                </div>

                <div class="indicator-refresh__timeline">
                    <div class="indicator-refresh__step">
                        <span class="indicator-refresh__dot"></span>
                        <span>"Читаем DataView и GL"</span>
                    </div>
                    <div class="indicator-refresh__step">
                        <span class="indicator-refresh__dot"></span>
                        <span>"Считаем сравнение с прошлым периодом"</span>
                    </div>
                    <div class="indicator-refresh__step">
                        <span class="indicator-refresh__dot"></span>
                        <span>"Перерисовываем карточки дашборда"</span>
                    </div>
                </div>

                <div class="indicator-refresh__cards" aria-hidden="true">
                    <div class="indicator-refresh__card indicator-refresh__card--lg">
                        <span class="indicator-refresh__line indicator-refresh__line--short"></span>
                        <span class="indicator-refresh__line indicator-refresh__line--value"></span>
                        <span class="indicator-refresh__line indicator-refresh__line--medium"></span>
                    </div>
                    <div class="indicator-refresh__card">
                        <span class="indicator-refresh__line indicator-refresh__line--short"></span>
                        <span class="indicator-refresh__line indicator-refresh__line--value"></span>
                        <span class="indicator-refresh__line indicator-refresh__line--short"></span>
                    </div>
                    <div class="indicator-refresh__card indicator-refresh__card--accent">
                        <span class="indicator-refresh__line indicator-refresh__line--short"></span>
                        <span class="indicator-refresh__line indicator-refresh__line--value"></span>
                        <span class="indicator-refresh__line indicator-refresh__line--medium"></span>
                    </div>
                </div>
            </div>
        </div>
    }
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
    // The handle must be stored until cleanup — WindowListenerHandle has no Drop impl,
    // so `let _ = ...` would drop it without removing the listener, leaking the closure
    // (which captures `selected_indicator`) past this component's lifetime.
    let msg_handle =
        window_event_listener(leptos::ev::message, move |ev: web_sys::MessageEvent| {
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
                let iframe_el = doc
                    .query_selector(".dashboard-viewer__iframe")
                    .ok()
                    .flatten();
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
    on_cleanup(move || msg_handle.remove());

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
            let computed = match fetch_indicator_data(&defs, &ctx).await {
                Ok(c) => c,
                Err(e) => {
                    indicator_refreshing.set(false);
                    error.set(Some(e));
                    return;
                }
            };
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
                            <IndicatorRefreshOverlay
                                card_count=move || indicator_defs.get().len()
                                filter_hint=move || compact_filter_hint(&view_ctx.get())
                            />
                        </Show>
                        <Show when=move || false && indicator_refreshing.get()>
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

    let name = def
        .as_ref()
        .map(|d| {
            if d.description.trim().is_empty() {
                d.code.clone()
            } else {
                d.description.clone()
            }
        })
        .unwrap_or_else(|| sel.id.clone());

    let code = def.as_ref().map(|d| d.code.clone()).unwrap_or_default();
    let view_id = def
        .as_ref()
        .and_then(|d| d.data_spec.view_id.clone())
        .unwrap_or_default();
    let metric_id = def
        .as_ref()
        .and_then(|d| d.data_spec.metric_id.clone())
        .unwrap_or_default();
    let indicator_default_params = def
        .as_ref()
        .map(indicator_default_params)
        .unwrap_or_default();
    let effective_indicator_params = merge_indicator_params(&indicator_default_params, &ctx.params);
    let format_spec = def
        .as_ref()
        .map(|d| d.view_spec.format.clone())
        .unwrap_or(serde_json::Value::Null);

    // Drilldown: only available when indicator has a DataView (view_id)
    let view_id_opt = def.as_ref().and_then(|d| d.data_spec.view_id.clone());
    let metric_id_opt = def.as_ref().and_then(|d| d.data_spec.metric_id.clone());
    let has_drilldown = view_id_opt.is_some();
    let user_description = build_indicator_description(def.as_ref());
    let computation_details =
        build_indicator_details(def.as_ref(), computed.as_ref(), &effective_indicator_params);
    let active_tab = RwSignal::new("overview".to_string());
    let tabs_store = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let about_description = user_description.clone();
    let about_details = computation_details.clone();

    // Async-загружаемые измерения из DataViewMeta
    let dv_dims: RwSignal<Option<Vec<(String, String)>>> = RwSignal::new(None);
    if let Some(def_for_dims) = def.clone() {
        let params_for_dims = effective_indicator_params.clone();
        spawn_local(async move {
            match fetch_indicator_drill_dimensions(&def_for_dims, &params_for_dims).await {
                Ok(dims) => {
                    dv_dims.set(Some(dims));
                }
                Err(e) => {
                    let failed_view_id = def_for_dims.data_spec.view_id.clone().unwrap_or_default();
                    leptos::logging::warn!(
                        "Drilldown dimensions fetch failed for {}: {}",
                        failed_view_id,
                        e
                    );
                    dv_dims.set(Some(vec![]));
                }
            }
        });
    }

    let value_full_str = computed
        .as_ref()
        .and_then(|cv| cv.value)
        .map(|v| format_full_value(v, &format_spec))
        .unwrap_or_else(|| "—".to_string());

    let prev_full_str = computed
        .as_ref()
        .and_then(|cv| cv.previous_value)
        .map(|v| format_full_value(v, &format_spec))
        .unwrap_or_else(|| "—".to_string());

    let change_pct = computed.as_ref().and_then(|cv| cv.change_percent);
    let delta_str = change_pct
        .map(|pct| {
            if pct > 0.0 {
                format!("+{:.1}%", pct)
            } else {
                format!("{:.1}%", pct)
            }
        })
        .unwrap_or_else(|| "—".to_string());

    let delta_class = match change_pct {
        Some(p) if p > 0.0 => "indicator-detail__delta--up",
        Some(p) if p < 0.0 => "indicator-detail__delta--down",
        _ => "indicator-detail__delta--flat",
    };

    let status = computed
        .as_ref()
        .and_then(|cv| cv.status.clone())
        .unwrap_or_else(|| "Neutral".to_string());

    let status_class = match status.as_str() {
        "Good" => "indicator-detail__status--good",
        "Bad" => "indicator-detail__status--bad",
        "Warning" => "indicator-detail__status--warning",
        _ => "indicator-detail__status--neutral",
    };

    let current_period_label = period_label(&ctx.date_from, &ctx.date_to);
    let has_period2 = ctx.period2_from.is_some() || ctx.period2_to.is_some();
    let comparison_period_title = if has_period2 {
        "Период сравнения"
    } else {
        "Предыдущий период"
    };
    let comparison_period_label = match (ctx.period2_from.as_deref(), ctx.period2_to.as_deref()) {
        (Some(from), Some(to)) => period_label(from, to),
        (Some(from), None) => period_label(from, ""),
        (None, Some(to)) => period_label("", to),
        (None, None) => "Автоматическое сравнение".to_string(),
    };

    let overview_current_period_label = current_period_label.clone();
    let overview_value_full_str = value_full_str.clone();
    let overview_comparison_period_title = comparison_period_title.to_string();
    let overview_comparison_period_label = comparison_period_label.clone();
    let overview_prev_full_str = prev_full_str.clone();
    let overview_delta_str = delta_str.clone();
    let overview_view_id = view_id.clone();
    let overview_delta_class = delta_class.to_string();
    let overview_subtitle = computed.as_ref().and_then(|value| value.subtitle.clone());
    let overview_effective_indicator_params = StoredValue::new(effective_indicator_params.clone());

    let modal_style = format!(
        "--from-x: {}px; --from-y: {}px;",
        sel.from_x as i32, sel.from_y as i32
    );

    // Closing state: triggers reverse animation before the modal is removed from DOM.
    let is_closing = RwSignal::new(false);

    let do_close = Callback::new(move |_: ()| {
        if is_closing.get_untracked() {
            return;
        }
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

    let open_edit = {
        let tabs_store = tabs_store.clone();
        let indicator_id = sel.id.clone();
        let code = code.clone();
        let name = name.clone();
        let do_close = do_close.clone();
        move |_| {
            use crate::layout::tabs::{detail_tab_label, pick_identifier};
            use contracts::domain::a024_bi_indicator::ENTITY_METADATA as A024;

            let identifier = pick_identifier(None, Some(&code), Some(&name), &indicator_id);
            let title = detail_tab_label(A024.ui.element_name, identifier);
            tabs_store.open_tab(
                &format!("a024_bi_indicator_details_{}", indicator_id),
                &title,
            );
            do_close.run(());
        }
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
                <div class="modal-header indicator-detail__header">
                    <div class="modal-header__left indicator-detail__header-main">
                        <div class="indicator-detail__title-block">
                            <span class="modal-title">{name.clone()}</span>
                            {user_description.as_ref().map(|text| view! {
                                <span class="indicator-detail__title-subtitle">{text.clone()}</span>
                            })}
                        </div>
                        <div class="indicator-detail__header-meta">
                            {if !code.is_empty() {
                                view! { <span class="indicator-detail__code-badge">{code.clone()}</span> }.into_any()
                            } else {
                                view! { <></> }.into_any()
                            }}
                            {if !view_id.is_empty() {
                                view! { <span class="indicator-detail__header-chip">{view_id.clone()}</span> }.into_any()
                            } else {
                                view! { <></> }.into_any()
                            }}
                            {if !metric_id.is_empty() {
                                view! { <span class="indicator-detail__header-chip">{format!("metric: {}", metric_id.clone())}</span> }.into_any()
                            } else {
                                view! { <></> }.into_any()
                            }}
                            <span class=format!("indicator-detail__status {}", status_class)>{status.clone()}</span>
                        </div>
                    </div>
                    <div class="indicator-detail__header-actions">
                        <button
                            type="button"
                            class="indicator-detail__edit-link"
                            on:click=open_edit
                        >
                            "Редактировать"
                        </button>
                        <button
                            class="modal__close"
                            on:click=move |_| do_close.run(())
                            aria-label="Закрыть"
                        >
                            {icon("x")}
                        </button>
                    </div>
                </div>
                <div class="modal-body indicator-detail__body">
                    <div class="indicator-detail__tabs">
                        <button
                            type="button"
                            class=move || {
                                if active_tab.get() == "overview" {
                                    "indicator-detail__tab indicator-detail__tab--active".to_string()
                                } else {
                                    "indicator-detail__tab".to_string()
                                }
                            }
                            on:click=move |_| active_tab.set("overview".to_string())
                        >
                            "Обзор"
                        </button>
                        <button
                            type="button"
                            class=move || {
                                if active_tab.get() == "about" {
                                    "indicator-detail__tab indicator-detail__tab--active".to_string()
                                } else {
                                    "indicator-detail__tab".to_string()
                                }
                            }
                            on:click=move |_| active_tab.set("about".to_string())
                        >
                            "Описание и расчёт"
                        </button>
                    </div>

                    {move || if active_tab.get() == "about" {
                        view! {
                            <div class="indicator-detail__about">
                                <section class="indicator-detail__section">
                                    <span class="indicator-detail__section-eyebrow">"Краткое описание"</span>
                                    {if let Some(text) = about_description.clone() {
                                        view! { <p class="indicator-detail__description">{text}</p> }.into_any()
                                    } else {
                                        view! { <div class="indicator-detail__empty">"Описание для этого индикатора пока не заполнено."</div> }.into_any()
                                    }}
                                </section>
                                <section class="indicator-detail__section">
                                    <span class="indicator-detail__section-eyebrow">"Подробности расчёта"</span>
                                    {if about_details.is_empty() {
                                        view! { <div class="indicator-detail__empty">"Ключевые детали расчёта пока не сформированы."</div> }.into_any()
                                    } else {
                                        view! {
                                            <div class="indicator-detail__details-block">
                                                {about_details.iter().cloned().map(|line| view! {
                                                    <div class="indicator-detail__details-line">{line}</div>
                                                }).collect_view()}
                                            </div>
                                        }.into_any()
                                    }}
                                </section>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="indicator-detail__periods">
                                <div class="indicator-detail__period-card">
                                    <span class="indicator-detail__period-caption">"Текущий период"</span>
                                    <span class="indicator-detail__period-range">{overview_current_period_label.clone()}</span>
                                    <span class="indicator-detail__period-value">{overview_value_full_str.clone()}</span>
                                </div>
                                <div class="indicator-detail__period-card">
                                    <span class="indicator-detail__period-caption">{overview_comparison_period_title.clone()}</span>
                                    <span class="indicator-detail__period-range">{overview_comparison_period_label.clone()}</span>
                                    <span class="indicator-detail__period-value">{overview_prev_full_str.clone()}</span>
                                </div>
                            </div>
                            <div class="indicator-detail__meta">
                                <div class="indicator-detail__meta-row">
                                    <span class="indicator-detail__meta-label">"Изменение"</span>
                                    <span class=format!("indicator-detail__delta {}", overview_delta_class)>{overview_delta_str.clone()}</span>
                                </div>
                                {if !overview_view_id.is_empty() {
                                    view! {
                                        <div class="indicator-detail__meta-row">
                                            <span class="indicator-detail__meta-label">"Источник данных"</span>
                                            <span class="indicator-detail__meta-value">{overview_view_id.clone()}</span>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <></> }.into_any()
                                }}
                                {overview_subtitle.clone().map(|subtitle| view! {
                                    <div class="indicator-detail__meta-row">
                                        <span class="indicator-detail__meta-label">"Схема расчёта"</span>
                                        <span class="indicator-detail__meta-value">{subtitle}</span>
                                    </div>
                                })}
                            </div>

                            // Drilldown section — visible only when indicator has a DataView (view_id)
                            {if has_drilldown {
                        let indicator_id_c = sel.id.clone();
                        let indicator_name_c = name.clone();
                        let view_id_c = view_id_opt.clone().unwrap_or_default();
                        let metric_id_c = metric_id_opt.clone();
                        let ctx_c = ctx.clone();
                        let tabs_store = Some(tabs_store.clone());

                        view! {
                            <div class="drill-picker">
                                <div class="drill-picker__header">
                                    <span class="drill-picker__title">"Показать детализацию"</span>
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
                                            let id = indicator_id_c.clone();
                                            let vid = view_id_c.clone();
                                            let metric = metric_id_c.clone();
                                            let drill_ctx = ctx_c.clone();
                                            let iname = indicator_name_c.clone();
                                            let ts = tabs_store.clone();
                                            let grouped_dims = group_drill_dimensions(&dims_list);
                                            let drill_params = overview_effective_indicator_params.get_value();

                                            view! {
                                                <div class="drill-picker__groups">
                                                    {if dims_list.is_empty() {
                                                        view! {
                                                            <div class="drill-picker__empty">
                                                                "Нет общих измерений для выбранных оборотов."
                                                            </div>
                                                        }.into_any()
                                                    } else {
                                                        grouped_dims.into_iter().map(|group| {
                                                            let group_title = group.title;
                                                            let group_subtitle = group.subtitle;
                                                            let group_items = group.items;
                                                            let id_group = id.clone();
                                                            let vid_group = vid.clone();
                                                            let metric_group = metric.clone();
                                                            let drill_ctx_group = drill_ctx.clone();
                                                            let iname_group = iname.clone();
                                                            let ts_group = ts.clone();
                                                            let params_group = drill_params.clone();

                                                            view! {
                                                                <section class="drill-picker__group">
                                                                    <div class="drill-picker__group-header">
                                                                        <span class="drill-picker__group-title">{group_title}</span>
                                                                        <span class="drill-picker__group-subtitle">{group_subtitle}</span>
                                                                    </div>
                                                                    <div class="drill-picker__list">
                                                                        {group_items.into_iter().map(|(field_id, label)| {
                                                                            let dim = field_id.clone();
                                                                            let dim_code = field_id.clone();
                                                                            let dim_label = label.clone();
                                                                            let tab_title = format!("{} - {}", iname_group, dim_label);
                                                                            let store_opt = ts_group.clone();
                                                                            let vid2 = vid_group.clone();
                                                                            let id2 = id_group.clone();
                                                                            let iname2 = iname_group.clone();
                                                                            let metric2 = metric_group.clone();
                                                                            let ctx2 = drill_ctx_group.clone();
                                                                            let params2 = params_group.clone();
                                                                            view! {
                                                                                <button
                                                                                    type="button"
                                                                                    class="drill-picker__item drill-picker__item--button"
                                                                                    on:click=move |_| {
                                                                                        let store_opt = store_opt.clone();
                                                                                        let dim = dim.clone();
                                                                                        let dim_label = dim_label.clone();
                                                                                        let tab_title = tab_title.clone();
                                                                                        let vid2 = vid2.clone();
                                                                                        let id2 = id2.clone();
                                                                                        let iname2 = iname2.clone();
                                                                                        let metric2 = metric2.clone();
                                                                                        let ctx2 = ctx2.clone();
                                                                                        let params2 = params2.clone();

                                                                                        spawn_local(async move {
                                                                                            if let Some(session_id) = post_drilldown_session(
                                                                                                vid2,
                                                                                                id2,
                                                                                                iname2,
                                                                                                metric2,
                                                                                                dim,
                                                                                                dim_label,
                                                                                                ctx2,
                                                                                                params2,
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
                                                                                    <span class="drill-picker__item-main">
                                                                                        <span class="drill-picker__item-label">{label}</span>
                                                                                        <span class="drill-picker__item-hint">{dim_code}</span>
                                                                                    </span>
                                                                                    <span class="drill-picker__item-arrow">"→"</span>
                                                                                </button>
                                                                            }
                                                                        }).collect_view()}
                                                                    </div>
                                                                </section>
                                                            }
                                                        }).collect_view().into_any()
                                                    }}
                                                </div>
                                            }.into_any()
                                        }
                                    }
                                }}
                            </div>
                        }.into_any()
                            } else {
                                view! { <></> }.into_any()
                            }}
                        }.into_any()
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
    params: HashMap<String, String>,
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
        "params": params,
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
