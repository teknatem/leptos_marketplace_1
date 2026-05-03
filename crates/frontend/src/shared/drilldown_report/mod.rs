//! Drilldown Report (DataView-based) — standalone page with editable filter panel.
//!
//! Сравнительная таблица (П1 vs П2 vs Δ%) или multi-resource таблица (один ресурс = колонка).
//!
//! Tab key:
//!   `drilldown__{session_id}` — параметры из таблицы sys_drilldown (режим дашборда).
//!   `drilldown__new`          — ручной режим, session_id = None, drawer открывается сразу.

use crate::data_view::api as dv_api;
use crate::data_view::types::{
    DataViewMeta, DrilldownCapabilitiesResponse, DrilldownDimensionCapability, FilterDef,
    ResourceMeta,
};
use crate::data_view::ui::FilterBar;
use crate::shared::api_utils::api_base;
use crate::shared::icons::icon;
use contracts::shared::data_view::ViewContext;
use contracts::shared::drilldown::{DrilldownResponse, DrilldownRow, MetricValues};
use gloo_net::http::Request;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thaw::*;

// ── Local types ───────────────────────────────────────────────────────────────

/// Десериализованная запись из GET /api/sys-drilldown/:id
#[derive(Debug, Clone, Deserialize)]
struct DrilldownSessionRecord {
    pub view_id: String,
    pub indicator_name: String,
    pub params: DrilldownSessionParams,
}

#[derive(Debug, Clone, Deserialize)]
struct DrilldownSessionParams {
    pub group_by: String,
    #[serde(default)]
    pub metric_id: Option<String>,
    #[serde(default)]
    pub metric_ids: Vec<String>,
    pub date_from: String,
    pub date_to: String,
    #[serde(default)]
    pub period2_from: Option<String>,
    #[serde(default)]
    pub period2_to: Option<String>,
    #[serde(default)]
    pub connection_mp_refs: Vec<String>,
    #[serde(default)]
    pub params: HashMap<String, String>,
}

// ── Request payload ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
struct DvDrilldownRequest {
    pub date_from: String,
    pub date_to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period2_from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period2_to: Option<String>,
    pub group_by: String,
    pub connection_mp_refs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metric_id: Option<String>,
    #[serde(default)]
    pub params: HashMap<String, String>,
    /// Multi-resource режим: список выбранных resource id.
    #[serde(default)]
    pub metric_ids: Vec<String>,
}

// ── Payload for saving manual session ────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
struct DrilldownSessionSave {
    pub view_id: String,
    pub indicator_name: String,
    pub metric_id: Option<String>,
    pub metric_ids: Vec<String>,
    pub group_by: String,
    pub date_from: String,
    pub date_to: String,
    pub period2_from: Option<String>,
    pub period2_to: Option<String>,
    pub connection_mp_refs: Vec<String>,
    pub params: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
struct SessionCreateResponse {
    pub session_id: String,
}

// ── Sorting ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum SortCol {
    Label,
    /// Single-metric: "value1" / "value2" / "delta". Multi: resource id.
    Named(String),
}

fn sort_rows(rows: &[DrilldownRow], col: &SortCol, asc: bool) -> Vec<DrilldownRow> {
    let mut sorted = rows.to_vec();
    sorted.sort_by(|a, b| {
        let ord = match col {
            SortCol::Label => a.label.cmp(&b.label),
            SortCol::Named(n) if n == "value1" => a
                .value1
                .partial_cmp(&b.value1)
                .unwrap_or(std::cmp::Ordering::Equal),
            SortCol::Named(n) if n == "value2" => a
                .value2
                .partial_cmp(&b.value2)
                .unwrap_or(std::cmp::Ordering::Equal),
            SortCol::Named(n) if n == "delta" => a
                .delta_pct
                .unwrap_or(f64::NEG_INFINITY)
                .partial_cmp(&b.delta_pct.unwrap_or(f64::NEG_INFINITY))
                .unwrap_or(std::cmp::Ordering::Equal),
            // Multi-resource sort keys: "m_v1:{id}", "m_v2:{id}", "m_delta:{id}"
            SortCol::Named(n) if n.starts_with("m_v1:") => {
                let id = &n["m_v1:".len()..];
                let va = a.metric_values.get(id).map(|mv| mv.value1).unwrap_or(0.0);
                let vb = b.metric_values.get(id).map(|mv| mv.value1).unwrap_or(0.0);
                va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
            }
            SortCol::Named(n) if n.starts_with("m_v2:") => {
                let id = &n["m_v2:".len()..];
                let va = a.metric_values.get(id).map(|mv| mv.value2).unwrap_or(0.0);
                let vb = b.metric_values.get(id).map(|mv| mv.value2).unwrap_or(0.0);
                va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
            }
            SortCol::Named(n) if n.starts_with("m_delta:") => {
                let id = &n["m_delta:".len()..];
                let va = a
                    .metric_values
                    .get(id)
                    .map(|mv| mv.delta_pct.unwrap_or(f64::NEG_INFINITY))
                    .unwrap_or(f64::NEG_INFINITY);
                let vb = b
                    .metric_values
                    .get(id)
                    .map(|mv| mv.delta_pct.unwrap_or(f64::NEG_INFINITY))
                    .unwrap_or(f64::NEG_INFINITY);
                va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
            }
            SortCol::Named(metric_id) => {
                let va = a
                    .metric_values
                    .get(metric_id.as_str())
                    .map(|mv| mv.value1)
                    .unwrap_or(0.0);
                let vb = b
                    .metric_values
                    .get(metric_id.as_str())
                    .map(|mv| mv.value1)
                    .unwrap_or(0.0);
                va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
            }
        };
        if asc {
            ord
        } else {
            ord.reverse()
        }
    });
    sorted
}

// ── Formatting ────────────────────────────────────────────────────────────────

fn fmt_value(v: f64) -> String {
    let s = format!("{:.0}", v.abs());
    let digits: Vec<char> = s.chars().collect();
    let n = digits.len();
    let mut result = String::with_capacity(n + n / 3 + 1);
    for (i, &ch) in digits.iter().enumerate() {
        if i > 0 && (n - i) % 3 == 0 {
            result.push('\u{202F}');
        }
        result.push(ch);
    }
    if v < 0.0 {
        format!("-{}", result)
    } else {
        result
    }
}

fn shift_month(d: &str, months: i32) -> String {
    let parts: Vec<&str> = d.split('-').collect();
    if parts.len() < 3 {
        return d.to_string();
    }
    let y: i32 = parts[0].parse().unwrap_or(2025);
    let m: i32 = parts[1].parse().unwrap_or(1);
    let day: i32 = parts[2].parse().unwrap_or(1);
    let total = y * 12 + (m - 1) + months;
    let ny = total / 12;
    let nm = total % 12 + 1;
    let max_day = match nm {
        2 => {
            if (ny % 4 == 0 && ny % 100 != 0) || ny % 400 == 0 {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    format!("{:04}-{:02}-{:02}", ny, nm, day.min(max_day))
}

fn delta_class(delta: Option<f64>) -> &'static str {
    match delta {
        Some(d) if d > 0.0 => "drill-cell--delta-up",
        Some(d) if d < 0.0 => "drill-cell--delta-down",
        _ => "drill-cell--delta-flat",
    }
}

fn fmt_delta(delta: Option<f64>) -> String {
    match delta {
        Some(d) if d > 0.0 => format!("+{:.1}", d),
        Some(d) => format!("{:.1}", d),
        None => "—".to_string(),
    }
}

fn plural_rows(count: usize) -> String {
    let rem100 = count % 100;
    let rem10 = count % 10;
    let suffix = if (11..=14).contains(&rem100) {
        "строк"
    } else {
        match rem10 {
            1 => "строка",
            2..=4 => "строки",
            _ => "строк",
        }
    };
    format!("{count} {suffix}")
}

fn sort_icon(current: &SortCol, col: &SortCol, asc: bool) -> &'static str {
    if current != col {
        "⇅"
    } else if asc {
        "↑"
    } else {
        "↓"
    }
}

fn format_dimension_option(capability: &DrilldownDimensionCapability) -> (String, String) {
    let label = match capability.mode.as_str() {
        "partial" => format!(
            "{} [partial {}% + Прочее]",
            capability.label,
            capability.coverage_pct.unwrap_or(0.0)
        ),
        _ => format!("{} [100% safe]", capability.label),
    };
    (capability.id.clone(), label)
}

fn capabilities_to_dimension_options(
    capabilities: DrilldownCapabilitiesResponse,
) -> Vec<(String, String)> {
    capabilities
        .safe_dimensions
        .into_iter()
        .chain(capabilities.partial_dimensions)
        .map(|capability| format_dimension_option(&capability))
        .collect()
}

// ── Component ─────────────────────────────────────────────────────────────────

#[component]
pub fn DrilldownReportPage(
    /// Session id from dashboard. None = manual mode (no pre-loaded session).
    session_id: Option<String>,
    on_close: Option<Callback<()>>,
) -> impl IntoView {
    let is_manual = session_id.is_none();

    // ── Session state ─────────────────────────────────────────────────────────
    let session_loaded = RwSignal::new(is_manual);
    let title = RwSignal::new(String::new());
    let view_id_sig = RwSignal::new(String::new());
    let saved_session_id: RwSignal<Option<String>> = RwSignal::new(session_id.clone());

    // ── Editable form params ──────────────────────────────────────────────────
    let view_ctx = RwSignal::new(ViewContext::default());
    let p_group_by = RwSignal::new(String::new());
    let p_metric_id = RwSignal::new(None::<String>);

    // ── DataView list ─────────────────────────────────────────────────────────
    let view_list: RwSignal<Vec<DataViewMeta>> = RwSignal::new(vec![]);

    // ── Resources: HashSet for CheckboxGroup, Vec for request ─────────────────
    let dv_resources: RwSignal<Vec<ResourceMeta>> = RwSignal::new(vec![]);
    let selected_resources_set: RwSignal<HashSet<String>> = RwSignal::new(HashSet::new());

    // Sync HashSet → sorted Vec for the request payload
    let selected_resources = Signal::derive(move || {
        let mut v: Vec<String> = selected_resources_set.get().into_iter().collect();
        v.sort();
        v
    });

    // ── Metadata ──────────────────────────────────────────────────────────────
    let filter_defs: RwSignal<Vec<FilterDef>> = RwSignal::new(vec![]);
    let filters_loading = RwSignal::new(false);
    let filters_error = RwSignal::new(None::<String>);
    let dv_dims: RwSignal<Vec<(String, String)>> = RwSignal::new(vec![]);

    // ── Report state ─────────────────────────────────────────────────────────
    let loading = RwSignal::new(false);
    let error_msg = RwSignal::new(None::<String>);
    let response = RwSignal::new(None::<DrilldownResponse>);

    // ── Fetch trigger ─────────────────────────────────────────────────────────
    let fetch_version = RwSignal::new(0u32);
    let drawer_open = RwSignal::new(is_manual);

    // ── Load DataView list on mount ───────────────────────────────────────────
    spawn_local(async move {
        if let Ok(views) = dv_api::fetch_list().await {
            view_list.set(views);
        }
    });

    // ── Load session (dashboard mode only) ───────────────────────────────────
    if let Some(sid) = session_id.clone() {
        spawn_local(async move {
            let url = format!("{}/api/sys-drilldown/{}", api_base(), sid);
            let Ok(resp) = Request::get(&url).send().await else {
                return;
            };
            if !resp.ok() {
                return;
            }
            let Ok(record) = resp.json::<DrilldownSessionRecord>().await else {
                return;
            };

            title.set(record.indicator_name.clone());
            let vid = record.view_id.clone();

            let df = record.params.date_from.clone();
            let dt = record.params.date_to.clone();
            let p2f = record
                .params
                .period2_from
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| shift_month(&df, -1));
            let p2t = record
                .params
                .period2_to
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| shift_month(&dt, -1));

            view_ctx.set(ViewContext {
                date_from: df,
                date_to: dt,
                period2_from: Some(p2f),
                period2_to: Some(p2t),
                connection_mp_refs: record.params.connection_mp_refs.clone(),
                params: record.params.params.clone(),
            });
            p_group_by.set(record.params.group_by.clone());
            p_metric_id.set(record.params.metric_id.clone());
            selected_resources_set.set(record.params.metric_ids.into_iter().collect());

            view_id_sig.set(vid.clone());
            session_loaded.set(true);

            load_view_metadata(
                vid,
                view_ctx.get_untracked(),
                record.params.metric_id.clone(),
                filter_defs,
                filters_loading,
                filters_error,
                dv_dims,
                dv_resources,
            )
            .await;

            fetch_version.update(|n| *n += 1);
        });
    }

    // ── DataView picker change handler (used in the drawer) ──────────────────
    // NOTE: We do NOT use a reactive Effect for view_id changes to avoid a
    // race between the session spawn_local and Thaw's Select two-way binding.
    // Instead, the drawer uses a native <select> with an explicit on:change
    // callback that only fires on real user interaction.
    let on_view_id_change = move |new_vid: String| {
        if new_vid.is_empty() || new_vid == view_id_sig.get_untracked() {
            return;
        }
        view_id_sig.set(new_vid.clone());
        response.set(None);
        selected_resources_set.set(HashSet::new());
        p_group_by.set(String::new());
        filter_defs.set(vec![]);
        dv_dims.set(vec![]);
        dv_resources.set(vec![]);

        spawn_local(async move {
            load_view_metadata(
                new_vid,
                view_ctx.get_untracked(),
                p_metric_id.get_untracked(),
                filter_defs,
                filters_loading,
                filters_error,
                dv_dims,
                dv_resources,
            )
            .await;
        });
    };

    // ── Execute report when fetch_version increments ──────────────────────────
    Effect::new(move |_| {
        let v = fetch_version.get();
        if v == 0 {
            return;
        }

        let view_id = view_id_sig.get_untracked();
        let group_by = p_group_by.get_untracked();
        if view_id.is_empty() || group_by.is_empty() {
            return;
        }

        let url = format!("{}/api/data-view/{}/drilldown", api_base(), view_id);
        let ctx = view_ctx.get_untracked();
        let metric_ids = selected_resources.get_untracked();
        let req = DvDrilldownRequest {
            date_from: ctx.date_from,
            date_to: ctx.date_to,
            period2_from: ctx.period2_from,
            period2_to: ctx.period2_to,
            group_by,
            connection_mp_refs: ctx.connection_mp_refs,
            metric_id: if metric_ids.is_empty() {
                p_metric_id.get_untracked()
            } else {
                None
            },
            params: ctx.params,
            metric_ids,
        };
        loading.set(true);
        error_msg.set(None);

        spawn_local(async move {
            let body = match serde_json::to_string(&req) {
                Ok(b) => b,
                Err(e) => {
                    error_msg.set(Some(format!("Ошибка сериализации: {}", e)));
                    loading.set(false);
                    return;
                }
            };
            match Request::post(&url)
                .header("Content-Type", "application/json")
                .body(body)
                .unwrap()
                .send()
                .await
            {
                Ok(resp) if resp.ok() => match resp.json::<DrilldownResponse>().await {
                    Ok(data) => response.set(Some(data)),
                    Err(e) => error_msg.set(Some(format!("Ошибка разбора: {}", e))),
                },
                Ok(resp) => error_msg.set(Some(format!("Ошибка сервера: {}", resp.status()))),
                Err(e) => error_msg.set(Some(format!("Ошибка сети: {}", e))),
            }
            loading.set(false);
        });
    });

    // ── Sort ─────────────────────────────────────────────────────────────────
    let sort_col = RwSignal::new(SortCol::Named("value1".to_string()));
    let sort_asc = RwSignal::new(false);
    let group_by_options = Signal::derive(move || dv_dims.get());

    let apply_report = Callback::new(move |_: ()| {
        // In manual mode: save session to backend
        if is_manual && saved_session_id.get_untracked().is_none() {
            let vid = view_id_sig.get_untracked();
            let ctx = view_ctx.get_untracked();
            let group_by = p_group_by.get_untracked();
            let metric_ids = selected_resources.get_untracked();
            let view_name = view_list
                .get_untracked()
                .into_iter()
                .find(|v| v.id == vid)
                .map(|v| v.name)
                .unwrap_or_else(|| vid.clone());

            let save_body = DrilldownSessionSave {
                view_id: vid,
                indicator_name: view_name,
                metric_id: p_metric_id.get_untracked(),
                metric_ids,
                group_by,
                date_from: ctx.date_from.clone(),
                date_to: ctx.date_to.clone(),
                period2_from: ctx.period2_from.clone(),
                period2_to: ctx.period2_to.clone(),
                connection_mp_refs: ctx.connection_mp_refs.clone(),
                params: ctx.params.clone(),
            };

            spawn_local(async move {
                if let Ok(body_str) = serde_json::to_string(&save_body) {
                    let url = format!("{}/api/sys-drilldown", api_base());
                    if let Ok(resp) = Request::post(&url)
                        .header("Content-Type", "application/json")
                        .body(body_str)
                        .unwrap()
                        .send()
                        .await
                    {
                        if let Ok(data) = resp.json::<SessionCreateResponse>().await {
                            saved_session_id.set(Some(data.session_id));
                        }
                    }
                }
            });
        }

        drawer_open.set(false);
        fetch_version.update(|n| *n += 1);
    });

    let toggle_sort = move |col: SortCol| {
        if sort_col.get_untracked() == col {
            sort_asc.update(|a| *a = !*a);
        } else {
            let default_asc = col == SortCol::Label;
            sort_col.set(col);
            sort_asc.set(default_asc);
        }
    };

    view! {
        <div class="page drilldown-report">

            // ── Page header ──────────────────────────────────────────────────
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">
                        {move || {
                            let base_title = if title.get().is_empty() {
                                view_list.get()
                                    .into_iter()
                                    .find(|v| v.id == view_id_sig.get())
                                    .map(|v| v.name)
                                    .unwrap_or_else(|| "Детализация".to_string())
                            } else {
                                title.get()
                            };
                            response
                                .get()
                                .map(|resp| format!("{base_title} ({})", plural_rows(resp.rows.len())))
                                .unwrap_or(base_title)
                        }}
                    </h1>
                </div>

                <div class="page__header-right">
                    <Show when=move || response.get().is_some() && !loading.get()>
                        <Button
                            appearance=ButtonAppearance::Subtle
                            on_click=move |_| {
                                let Some(resp) = response.get_untracked() else { return; };
                                let t = title.get_untracked();
                                let base = if t.is_empty() {
                                    view_list.get_untracked()
                                        .into_iter()
                                        .find(|v| v.id == view_id_sig.get_untracked())
                                        .map(|v| v.name)
                                        .unwrap_or_else(|| "Детализация".to_string())
                                } else {
                                    t
                                };
                                export_drilldown_csv(&resp, &base);
                            }
                        >
                            {icon("download")}
                            "Excel (csv)"
                        </Button>
                    </Show>
                    <Button
                        appearance=ButtonAppearance::Subtle
                        on_click=move |_| drawer_open.set(true)
                    >
                        "Настройки"
                    </Button>
                    {on_close.map(|cb| view! {
                        <Button
                            appearance=ButtonAppearance::Subtle
                            on_click=move |_| cb.run(())
                        >
                            "Закрыть"
                        </Button>
                    })}
                </div>
            </div>

            // ── Loading skeleton until session params arrive ──────────────────
            <Show when=move || !session_loaded.get()>
                <div class="drilldown-report__loading">
                    <span class="spinner" />
                    " Загрузка параметров…"
                </div>
            </Show>

            // ── Report content ───────────────────────────────────────────────
            <div class="page__content">

                <Show when=move || loading.get()>
                    <div class="drilldown-report__loading">
                        <span class="spinner" />
                        " Загрузка данных…"
                    </div>
                </Show>

                <Show when=move || error_msg.get().is_some()>
                    <div class="drilldown-report__error">
                        {move || error_msg.get().unwrap_or_default()}
                    </div>
                </Show>

                // ── Multi-resource table (двухуровневая шапка: метрика / П1·П2·Δ%) ─
                <Show when=move || {
                    response
                        .get()
                        .and_then(|resp| resp.coverage.clone())
                        .is_some()
                        && !loading.get()
                }>
                    {move || {
                        let Some(resp) = response.get() else { return view! { <></> }.into_any() };
                        let Some(coverage) = resp.coverage.clone() else { return view! { <></> }.into_any() };
                        let title = if coverage.mode == "partial" {
                            format!(
                                "Partial coverage: {}% / {}%",
                                coverage.coverage_pct_period1.unwrap_or(0.0),
                                coverage.coverage_pct_period2.unwrap_or(0.0)
                            )
                        } else {
                            "100% safe coverage".to_string()
                        };
                        view! {
                            <div class="warning-box warning-box--info">
                                <span class="warning-box__text">
                                    {title}
                                    {format!(
                                        " · covered {} / {} · Прочее {} / {}",
                                        fmt_value(coverage.covered_value1),
                                        fmt_value(coverage.covered_value2),
                                        fmt_value(coverage.other_value1),
                                        fmt_value(coverage.other_value2)
                                    )}
                                </span>
                            </div>
                        }.into_any()
                    }}
                </Show>

                <Show when=move || {
                    response.get().map(|r| !r.metric_columns.is_empty()).unwrap_or(false)
                        && !loading.get()
                }>
                    {move || {
                        let Some(resp) = response.get() else { return view! { <></> }.into_any() };
                        let cols = resp.metric_columns.clone();
                        let group_by_label = resp.group_by_label.clone();
                        let p1_label = resp.period1_label.clone();
                        let p2_label = resp.period2_label.clone();

                        let rows_sorted = Signal::derive(move || {
                            let Some(r) = response.get() else { return vec![] };
                            sort_rows(&r.rows, &sort_col.get(), sort_asc.get())
                        });

                        // Pre-compute totals (P1 + P2) per metric for the footer
                        let totals: HashMap<String, (f64, f64)> = cols.iter().map(|col| {
                            let sums = resp.rows.iter().fold((0.0_f64, 0.0_f64), |(s1, s2), r| {
                                let mv = r.metric_values.get(&col.id);
                                (s1 + mv.map(|v| v.value1).unwrap_or(0.0),
                                 s2 + mv.map(|v| v.value2).unwrap_or(0.0))
                            });
                            (col.id.clone(), sums)
                        }).collect();

                        let cols_h1 = cols.clone();
                        let cols_h2 = cols.clone();
                        let cols_rows = cols.clone();
                        let cols_total = cols.clone();

                        view! {
                            <div class="drilldown-report__table-wrap">
                            <table class="drilldown-report__table">
                                <thead>
                                    // ── Row 1: group label (rowspan=2) + metric group headers (colspan=3) ──
                                    <tr>
                                        <th class="drill-th drill-th--group data-table__cell data-table__cell--header"
                                            rowspan="2">
                                            <div class="drilldown-report__sort-trigger"
                                                on:click=move |_| toggle_sort(SortCol::Label)>
                                                {group_by_label.clone()}
                                                " "
                                                <span class="drill-sort-icon">
                                                    {move || sort_icon(&sort_col.get(), &SortCol::Label, sort_asc.get())}
                                                </span>
                                            </div>
                                        </th>
                                        {cols_h1.into_iter().map(|col| view! {
                                            <th class="drill-th drill-th--metric-group data-table__cell data-table__cell--header"
                                                colspan="3">
                                                {col.label.clone()}
                                            </th>
                                        }).collect_view()}
                                    </tr>
                                    // ── Row 2: П1 / П2 / Δ% sub-headers per metric ──────────────────
                                    <tr>
                                        {cols_h2.into_iter().map(|col| {
                                            let id = col.id.clone();
                                            let p1 = p1_label.clone();
                                            let p2 = p2_label.clone();
                                            let sk_v1 = SortCol::Named(format!("m_v1:{}", id));
                                            let sk_v2 = SortCol::Named(format!("m_v2:{}", id));
                                            let sk_d  = SortCol::Named(format!("m_delta:{}", id));
                                            let id1 = id.clone();
                                            let id2 = id.clone();
                                            let id3 = id.clone();
                                            view! {
                                                <>
                                                    <th class="drill-th drill-th--sub drill-cell--p1 data-table__cell data-table__cell--header">
                                                        <div class="drilldown-report__sort-trigger" data-align="end"
                                                            on:click=move |_| toggle_sort(SortCol::Named(format!("m_v1:{}", id1)))>
                                                            {p1.clone()}
                                                            " "
                                                            <span class="drill-sort-icon">
                                                                {move || sort_icon(&sort_col.get(), &sk_v1, sort_asc.get())}
                                                            </span>
                                                        </div>
                                                    </th>
                                                    <th class="drill-th drill-th--sub drill-th--muted data-table__cell data-table__cell--header">
                                                        <div class="drilldown-report__sort-trigger" data-align="end"
                                                            on:click=move |_| toggle_sort(SortCol::Named(format!("m_v2:{}", id2)))>
                                                            {p2.clone()}
                                                            " "
                                                            <span class="drill-sort-icon">
                                                                {move || sort_icon(&sort_col.get(), &sk_v2, sort_asc.get())}
                                                            </span>
                                                        </div>
                                                    </th>
                                                    <th class="drill-th drill-th--sub data-table__cell data-table__cell--header">
                                                        <div class="drilldown-report__sort-trigger" data-align="end"
                                                            on:click=move |_| toggle_sort(SortCol::Named(format!("m_delta:{}", id3)))>
                                                            "Δ%"
                                                            " "
                                                            <span class="drill-sort-icon">
                                                                {move || sort_icon(&sort_col.get(), &sk_d, sort_asc.get())}
                                                            </span>
                                                        </div>
                                                    </th>
                                                </>
                                            }
                                        }).collect_view()}
                                    </tr>
                                </thead>
                                <tbody>
                                    <For
                                        each=move || rows_sorted.get()
                                        key=|row| row.group_key.clone()
                                        children=move |row: DrilldownRow| {
                                            let cols = cols_rows.clone();
                                            view! {
                                                <tr class="data-table__row">
                                                    <td class="data-table__cell">{row.label.clone()}</td>
                                                    {cols.into_iter().map(|col| {
                                                        let mv: MetricValues = row.metric_values
                                                            .get(&col.id)
                                                            .cloned()
                                                            .unwrap_or_default();
                                                        view! {
                                                            <>
                                                                <td class="data-table__cell data-table__cell--num drill-cell--p1">
                                                                    {fmt_value(mv.value1)}
                                                                </td>
                                                                <td class="data-table__cell data-table__cell--num drill-cell--muted">
                                                                    {fmt_value(mv.value2)}
                                                                </td>
                                                                <td class={format!("data-table__cell data-table__cell--num {}",
                                                                    delta_class(mv.delta_pct))}>
                                                                    {fmt_delta(mv.delta_pct)}
                                                                </td>
                                                            </>
                                                        }
                                                    }).collect_view()}
                                                </tr>
                                            }
                                        }
                                    />
                                </tbody>
                                <tfoot>
                                    <tr class="drilldown-report__total-row">
                                        <td class="data-table__cell drilldown-report__total-label">"Итого"</td>
                                        {cols_total.into_iter().map(|col| {
                                            let (t1, t2) = totals.get(&col.id).copied().unwrap_or((0.0, 0.0));
                                            let td = if t2.abs() > 0.01 {
                                                Some(((t1 - t2) / t2.abs()) * 100.0)
                                            } else { None };
                                            view! {
                                                <>
                                                    <td class="data-table__cell data-table__cell--num drilldown-report__total-value drill-cell--p1">
                                                        {fmt_value(t1)}
                                                    </td>
                                                    <td class="data-table__cell data-table__cell--num drilldown-report__total-value drill-cell--muted">
                                                        {fmt_value(t2)}
                                                    </td>
                                                    <td class={format!("data-table__cell data-table__cell--num drilldown-report__total-value {}",
                                                        delta_class(td))}>
                                                        {fmt_delta(td)}
                                                    </td>
                                                </>
                                            }
                                        }).collect_view()}
                                    </tr>
                                </tfoot>
                            </table>
                            </div>
                        }.into_any()
                    }}
                </Show>

                // ── Single-metric table (П1 / П2 / Δ%) ──────────────────────
                <Show when=move || {
                    response.get().map(|r| r.metric_columns.is_empty()).unwrap_or(false)
                        && !loading.get()
                }>
                    {move || {
                        let Some(resp) = response.get() else { return view! { <></> }.into_any() };

                        let p1_label       = resp.period1_label.clone();
                        let p2_label       = resp.period2_label.clone();
                        let group_by_label = resp.group_by_label.clone();

                        let rows_sorted = Signal::derive(move || {
                            let Some(r) = response.get() else { return vec![] };
                            sort_rows(&r.rows, &sort_col.get(), sort_asc.get())
                        });

                        let total1: f64 = resp.rows.iter().map(|r| r.value1).sum();
                        let total2: f64 = resp.rows.iter().map(|r| r.value2).sum();
                        let total_delta = if total2.abs() > 0.01 {
                            Some(((total1 - total2) / total2.abs()) * 100.0)
                        } else { None };
                        let total_delta_cls = delta_class(total_delta).to_string();
                        let total_delta_str = fmt_delta(total_delta);

                        view! {
                            <div class="drilldown-report__table-wrap">
                            <Table attr:class="drilldown-report__table">
                                <TableHeader>
                                    <TableRow>
                                        <TableHeaderCell class="drill-th">
                                            <div class="drilldown-report__sort-trigger"
                                                on:click=move |_| toggle_sort(SortCol::Label)>
                                                {group_by_label.clone()}
                                                " "
                                                <span class="drill-sort-icon">
                                                    {move || sort_icon(&sort_col.get(), &SortCol::Label, sort_asc.get())}
                                                </span>
                                            </div>
                                        </TableHeaderCell>
                                        <TableHeaderCell class="drill-th">
                                            <div class="drilldown-report__sort-trigger" data-align="end"
                                                on:click=move |_| toggle_sort(SortCol::Named("value1".to_string()))>
                                                {p1_label.clone()}
                                                " "
                                                <span class="drill-sort-icon">
                                                    {move || sort_icon(&sort_col.get(), &SortCol::Named("value1".to_string()), sort_asc.get())}
                                                </span>
                                            </div>
                                        </TableHeaderCell>
                                        <TableHeaderCell class="drill-th">
                                            <div class="drilldown-report__sort-trigger" data-align="end"
                                                on:click=move |_| toggle_sort(SortCol::Named("value2".to_string()))>
                                                {p2_label.clone()}
                                                " "
                                                <span class="drill-sort-icon">
                                                    {move || sort_icon(&sort_col.get(), &SortCol::Named("value2".to_string()), sort_asc.get())}
                                                </span>
                                            </div>
                                        </TableHeaderCell>
                                        <TableHeaderCell class="drill-th">
                                            <div class="drilldown-report__sort-trigger" data-align="end"
                                                on:click=move |_| toggle_sort(SortCol::Named("delta".to_string()))>
                                                "Δ%"
                                                " "
                                                <span class="drill-sort-icon">
                                                    {move || sort_icon(&sort_col.get(), &SortCol::Named("delta".to_string()), sort_asc.get())}
                                                </span>
                                            </div>
                                        </TableHeaderCell>
                                    </TableRow>
                                </TableHeader>
                                <tbody>
                                    <For
                                        each=move || rows_sorted.get()
                                        key=|row| row.group_key.clone()
                                        children=|row: DrilldownRow| {
                                            let delta_cls = delta_class(row.delta_pct).to_string();
                                            let delta_str = fmt_delta(row.delta_pct);
                                            view! {
                                                <tr class="data-table__row">
                                                    <td class="data-table__cell">{row.label.clone()}</td>
                                                    <td class="data-table__cell data-table__cell--num">
                                                        {fmt_value(row.value1)}
                                                    </td>
                                                    <td class="data-table__cell data-table__cell--num data-table__cell--muted">
                                                        {fmt_value(row.value2)}
                                                    </td>
                                                    <td class=format!("data-table__cell data-table__cell--num {}", delta_cls)>
                                                        {delta_str}
                                                    </td>
                                                </tr>
                                            }
                                        }
                                    />
                                </tbody>
                                <tfoot>
                                    <tr class="drilldown-report__total-row">
                                        <td class="data-table__cell drilldown-report__total-label">"Итого"</td>
                                        <td class="data-table__cell data-table__cell--num drilldown-report__total-value">
                                            {fmt_value(total1)}
                                        </td>
                                        <td class="data-table__cell data-table__cell--num data-table__cell--muted drilldown-report__total-value">
                                            {fmt_value(total2)}
                                        </td>
                                        <td class=format!("data-table__cell data-table__cell--num drilldown-report__total-value {}", total_delta_cls)>
                                            {total_delta_str}
                                        </td>
                                    </tr>
                                </tfoot>
                            </Table>
                            </div>
                        }.into_any()
                    }}
                </Show>
            </div>

            // ── Settings drawer ───────────────────────────────────────────────
            <OverlayDrawer
                open=drawer_open
                position=DrawerPosition::Right
                size=DrawerSize::Small
                close_on_esc=true
            >
                <DrawerHeader>
                    <div style="display:flex;align-items:center;justify-content:space-between;gap:12px;width:100%">
                        <DrawerHeaderTitle>"Настройки отчета"</DrawerHeaderTitle>
                        <Button
                            appearance=ButtonAppearance::Primary
                            on_click=move |_| apply_report.run(())
                            disabled=Signal::derive(move || {
                                loading.get()
                                    || view_id_sig.get().is_empty()
                                    || p_group_by.get().is_empty()
                            })
                        >
                            {move || if loading.get() { "Загрузка…" } else { "Применить" }}
                        </Button>
                    </div>
                </DrawerHeader>
                <DrawerBody native_scrollbar=true>
                    <div style="display:flex;flex-direction:column;gap:16px">

                        // ── DataView picker ──────────────────────────────────
                        <div class="form__group">
                            <label class="form__label">"DataView"</label>
                            {move || {
                                if view_list.get().is_empty() {
                                    view! {
                                        <div class="placeholder placeholder--small">
                                            <span class="spinner" />
                                            " Загрузка списка…"
                                        </div>
                                    }.into_any()
                                } else {
                                    // Native <select> with one-way prop:value binding.
                                    // on:change only fires on real user interaction,
                                    // never on programmatic signal updates — this prevents
                                    // the Select from accidentally overwriting view_id_sig
                                    // (which would corrupt p_group_by via the old Effect).
                                    view! {
                                        <select
                                            class="thaw-select"
                                            prop:value=move || view_id_sig.get()
                                            on:change=move |ev| {
                                                let val = event_target_value(&ev);
                                                on_view_id_change(val);
                                            }
                                        >
                                            <option value="">"— выберите DataView —"</option>
                                            {move || view_list.get().into_iter().map(|v| {
                                                view! { <option value=v.id.clone()>{v.name.clone()}</option> }
                                            }).collect_view()}
                                        </select>
                                    }.into_any()
                                }
                            }}
                        </div>

                        // ── Группировка ──────────────────────────────────────
                        <Show when=move || !view_id_sig.get().is_empty()>
                            <div class="form__group">
                                <label class="form__label">"Группировка"</label>
                                {move || {
                                    if dv_dims.get().is_empty() {
                                        view! {
                                            <div class="placeholder placeholder--small">
                                                <span class="spinner" />
                                                " Загрузка измерений…"
                                            </div>
                                        }.into_any()
                                    } else {
                                        // Native <select> with selected=true on the matching option.
                                        // This is reliable regardless of mount order — the option
                                        // itself declares it is selected, so no race with prop:value.
                                        view! {
                                            <select
                                                class="thaw-select"
                                                on:change=move |ev| {
                                                    p_group_by.set(event_target_value(&ev));
                                                }
                                            >
                                                <option
                                                    value=""
                                                    selected=move || p_group_by.get().is_empty()
                                                >
                                                    "— выберите измерение —"
                                                </option>
                                                {move || {
                                                    let current = p_group_by.get();
                                                    group_by_options.get().into_iter().map(move |(v, l)| {
                                                        let is_sel = v == current;
                                                        view! {
                                                            <option value=v.clone() selected=is_sel>{l}</option>
                                                        }
                                                    }).collect_view()
                                                }}
                                            </select>
                                        }.into_any()
                                    }
                                }}
                            </div>
                        </Show>

                        // ── Ресурсы (метрики) ────────────────────────────────
                        <Show when=move || !dv_resources.get().is_empty()>
                            <div class="form__group">
                                <label class="form__label">"Ресурсы (метрики)"</label>
                                <CheckboxGroup value=selected_resources_set>
                                    <div style="display:flex;flex-direction:column;gap:6px">
                                        <For
                                            each=move || dv_resources.get()
                                            key=|res| res.id.clone()
                                            children=|res: ResourceMeta| view! {
                                                <Checkbox value=res.id label=res.label />
                                            }
                                        />
                                    </div>
                                </CheckboxGroup>
                            </div>
                        </Show>

                        // ── Фильтры ──────────────────────────────────────────
                        {move || {
                            if view_id_sig.get().is_empty() {
                                view! { <></> }.into_any()
                            } else if filters_loading.get() {
                                view! {
                                    <div class="placeholder placeholder--small">
                                        <span class="spinner" />
                                        " Загрузка фильтров…"
                                    </div>
                                }.into_any()
                            } else if let Some(err) = filters_error.get() {
                                view! { <p>{err}</p> }.into_any()
                            } else if filter_defs.get().is_empty() {
                                view! {
                                    <div class="placeholder placeholder--small">
                                        "Для этого DataView нет фильтров."
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <FilterBar filters=filter_defs.get() ctx=view_ctx single_col=true />
                                }.into_any()
                            }
                        }}
                    </div>
                </DrawerBody>
            </OverlayDrawer>
        </div>
    }
}

// ── Helper: Excel/CSV export ──────────────────────────────────────────────────

fn csv_escape(s: &str) -> String {
    if s.contains(';') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn fmt_value_raw(v: f64) -> String {
    format!("{:.0}", v)
}

fn fmt_delta_raw(delta: Option<f64>) -> String {
    match delta {
        Some(d) => format!("{:.1}", d),
        None => String::new(),
    }
}

fn export_drilldown_csv(resp: &DrilldownResponse, base_title: &str) {
    use wasm_bindgen::JsCast;
    use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

    let mut csv = String::from("\u{FEFF}"); // UTF-8 BOM

    if resp.metric_columns.is_empty() {
        // Single-metric: Group | П1 | П2 | Δ%
        csv.push_str(&format!(
            "{};{};{};Δ%\n",
            csv_escape(&resp.group_by_label),
            csv_escape(&resp.period1_label),
            csv_escape(&resp.period2_label),
        ));
        for row in &resp.rows {
            csv.push_str(&format!(
                "{};{};{};{}\n",
                csv_escape(&row.label),
                fmt_value_raw(row.value1),
                fmt_value_raw(row.value2),
                fmt_delta_raw(row.delta_pct),
            ));
        }
        let t1: f64 = resp.rows.iter().map(|r| r.value1).sum();
        let t2: f64 = resp.rows.iter().map(|r| r.value2).sum();
        let td = if t2.abs() > 0.01 {
            Some(((t1 - t2) / t2.abs()) * 100.0)
        } else {
            None
        };
        csv.push_str(&format!(
            "Итого;{};{};{}\n",
            fmt_value_raw(t1),
            fmt_value_raw(t2),
            fmt_delta_raw(td)
        ));
    } else {
        // Multi-resource: flat headers per metric
        let mut header = csv_escape(&resp.group_by_label);
        for col in &resp.metric_columns {
            header.push_str(&format!(
                ";{} {};{} {};{} Δ%",
                csv_escape(&col.label),
                csv_escape(&resp.period1_label),
                csv_escape(&col.label),
                csv_escape(&resp.period2_label),
                csv_escape(&col.label),
            ));
        }
        csv.push_str(&header);
        csv.push('\n');

        for row in &resp.rows {
            let mut line = csv_escape(&row.label);
            for col in &resp.metric_columns {
                let mv = row.metric_values.get(&col.id).cloned().unwrap_or_default();
                line.push_str(&format!(
                    ";{};{};{}",
                    fmt_value_raw(mv.value1),
                    fmt_value_raw(mv.value2),
                    fmt_delta_raw(mv.delta_pct)
                ));
            }
            csv.push_str(&line);
            csv.push('\n');
        }

        let mut total_line = "Итого".to_string();
        for col in &resp.metric_columns {
            let (t1, t2) = resp.rows.iter().fold((0.0_f64, 0.0_f64), |(s1, s2), r| {
                let mv = r.metric_values.get(&col.id);
                (
                    s1 + mv.map(|v| v.value1).unwrap_or(0.0),
                    s2 + mv.map(|v| v.value2).unwrap_or(0.0),
                )
            });
            let td = if t2.abs() > 0.01 {
                Some(((t1 - t2) / t2.abs()) * 100.0)
            } else {
                None
            };
            total_line.push_str(&format!(
                ";{};{};{}",
                fmt_value_raw(t1),
                fmt_value_raw(t2),
                fmt_delta_raw(td)
            ));
        }
        csv.push_str(&total_line);
        csv.push('\n');
    }

    // Build safe filename
    let safe = base_title
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect::<String>();
    let filename = format!("{}.csv", safe);

    // Trigger download via hidden <a>
    let array = js_sys::Array::new();
    array.push(&wasm_bindgen::JsValue::from_str(&csv));
    let props = BlobPropertyBag::new();
    props.set_type("text/csv;charset=utf-8;");
    let Ok(blob) = Blob::new_with_str_sequence_and_options(&array, &props) else {
        return;
    };
    let Ok(url) = Url::create_object_url_with_blob(&blob) else {
        return;
    };
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let Ok(el) = document.create_element("a") else {
        return;
    };
    let Ok(a) = el.dyn_into::<HtmlAnchorElement>() else {
        return;
    };
    a.set_href(&url);
    a.set_download(&filename);
    let _ = a.set_attribute("style", "display:none");
    if let Some(body) = document.body() {
        let _ = body.append_child(&a);
        a.click();
        let _ = body.remove_child(&a);
    }
    let _ = Url::revoke_object_url(&url);
}

// ── Helper: load view metadata ────────────────────────────────────────────────

async fn load_view_metadata(
    view_id: String,
    view_ctx: ViewContext,
    metric_id: Option<String>,
    filter_defs: RwSignal<Vec<FilterDef>>,
    filters_loading: RwSignal<bool>,
    filters_error: RwSignal<Option<String>>,
    dv_dims: RwSignal<Vec<(String, String)>>,
    dv_resources: RwSignal<Vec<ResourceMeta>>,
) {
    filters_loading.set(true);
    filters_error.set(None);

    match dv_api::fetch_view_filters(&view_id).await {
        Ok(defs) => filter_defs.set(defs),
        Err(err) => filters_error.set(Some(err)),
    }
    filters_loading.set(false);

    if let Ok(meta) = dv_api::fetch_by_id(&view_id).await {
        let dims = match dv_api::fetch_drilldown_capabilities(&view_id, &view_ctx, metric_id).await
        {
            Ok(capabilities) => capabilities_to_dimension_options(capabilities),
            Err(_) => meta
                .available_dimensions
                .iter()
                .map(|dim| (dim.id.clone(), dim.label.clone()))
                .collect(),
        };
        dv_dims.set(dims);
        dv_resources.set(meta.available_resources);
    }
}
