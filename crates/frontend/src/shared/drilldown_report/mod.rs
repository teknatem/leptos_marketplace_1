//! Drilldown Report (DataView-based) — standalone page with editable filter panel.
//!
//! Сравнительная таблица (П1 vs П2 vs Δ%) с сортировкой, строкой итогов и
//! редактируемой панелью параметров (даты, группировка, кабинеты МП).
//!
//! Tab key: `drilldown__{session_id}` — параметры хранятся в таблице sys_drilldown.

use crate::shared::api_utils::api_base;
use contracts::shared::data_view::DataViewMeta;
use contracts::shared::drilldown::{DrilldownResponse, DrilldownRow};
use gloo_net::http::Request;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use thaw::*;

// ── Local types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Deserialize)]
struct ConnItem {
    pub id: String,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub description: String,
}

impl ConnItem {
    fn display_name(&self) -> String {
        if !self.code.is_empty() {
            self.code.clone()
        } else if !self.description.is_empty() {
            self.description.clone()
        } else {
            self.id.clone()
        }
    }
}

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
    pub group_by_label: String,
    pub date_from: String,
    pub date_to: String,
    #[serde(default)]
    pub period2_from: Option<String>,
    #[serde(default)]
    pub period2_to: Option<String>,
    #[serde(default)]
    pub connection_mp_refs: Vec<String>,
}

// ── Request payload (for re-fetch on "Сформировать") ─────────────────────────

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
}

// ── Sorting ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum SortCol {
    Label,
    Value1,
    Value2,
    Delta,
}

fn sort_rows(rows: &[DrilldownRow], col: SortCol, asc: bool) -> Vec<DrilldownRow> {
    let mut sorted = rows.to_vec();
    sorted.sort_by(|a, b| {
        let ord = match col {
            SortCol::Label => a.label.cmp(&b.label),
            SortCol::Value1 => a
                .value1
                .partial_cmp(&b.value1)
                .unwrap_or(std::cmp::Ordering::Equal),
            SortCol::Value2 => a
                .value2
                .partial_cmp(&b.value2)
                .unwrap_or(std::cmp::Ordering::Equal),
            SortCol::Delta => a
                .delta_pct
                .unwrap_or(f64::NEG_INFINITY)
                .partial_cmp(&b.delta_pct.unwrap_or(f64::NEG_INFINITY))
                .unwrap_or(std::cmp::Ordering::Equal),
        };
        if asc { ord } else { ord.reverse() }
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
        // Insert narrow no-break space before every group of 3 digits from the right
        if i > 0 && (n - i) % 3 == 0 {
            result.push('\u{202F}');
        }
        result.push(ch);
    }
    if v < 0.0 { format!("-{}", result) } else { result }
}

/// Shift a "YYYY-MM-DD" date by `months` (same logic as backend dv001).
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
            if (ny % 4 == 0 && ny % 100 != 0) || ny % 400 == 0 { 29 } else { 28 }
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
        Some(d) if d > 0.0 => format!("+{:.1}%", d),
        Some(d) => format!("{:.1}%", d),
        None => "—".to_string(),
    }
}

fn sort_icon(current: SortCol, col: SortCol, asc: bool) -> &'static str {
    if current != col {
        "⇅"
    } else if asc {
        "↑"
    } else {
        "↓"
    }
}

// ── Component ─────────────────────────────────────────────────────────────────

#[component]
pub fn DrilldownReportPage(
    session_id: String,
    on_close: Option<Callback<()>>,
) -> impl IntoView {
    // ── Session params (loaded from server once) ──────────────────────────────
    let session_loaded = RwSignal::new(false);
    let title          = RwSignal::new(String::new());
    let view_id_sig    = RwSignal::new(String::new());

    // ── Editable form params ──────────────────────────────────────────────────
    let p_date_from = RwSignal::new(String::new());
    let p_date_to   = RwSignal::new(String::new());
    let p_p2_from   = RwSignal::new(String::new());
    let p_p2_to     = RwSignal::new(String::new());
    let p_group_by  = RwSignal::new(String::new());
    let p_mp_refs: RwSignal<Vec<String>> = RwSignal::new(vec![]);

    // ── Metadata (fetched on mount) ───────────────────────────────────────────
    let dv_dims: RwSignal<Vec<(String, String)>> = RwSignal::new(vec![]);
    let connections: RwSignal<Vec<ConnItem>>     = RwSignal::new(vec![]);

    // ── Report state ─────────────────────────────────────────────────────────
    let loading   = RwSignal::new(false);
    let error_msg = RwSignal::new(None::<String>);
    let response  = RwSignal::new(None::<DrilldownResponse>);

    // ── Fetch trigger ─────────────────────────────────────────────────────────
    let fetch_version = RwSignal::new(0u32);

    // ── Load session on mount ─────────────────────────────────────────────────
    {
        let sid = session_id.clone();
        spawn_local(async move {
            let url = format!("{}/api/sys-drilldown/{}", api_base(), sid);
            let Ok(resp) = Request::get(&url).send().await else { return };
            if !resp.ok() { return; }
            let Ok(record) = resp.json::<DrilldownSessionRecord>().await else { return };

            title.set(record.indicator_name.clone());
            view_id_sig.set(record.view_id.clone());

            let df = record.params.date_from.clone();
            let dt = record.params.date_to.clone();

            // If P2 is not stored, compute it as P1 shifted -1 month (mirrors backend logic)
            let p2f = record.params.period2_from
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| shift_month(&df, -1));
            let p2t = record.params.period2_to
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| shift_month(&dt, -1));

            p_date_from.set(df);
            p_date_to.set(dt);
            p_p2_from.set(p2f);
            p_p2_to.set(p2t);
            p_group_by.set(record.params.group_by.clone());
            p_mp_refs.set(record.params.connection_mp_refs.clone());

            session_loaded.set(true);

            // Load DataView dimensions
            let url2 = format!("{}/api/data-view/{}", api_base(), record.view_id);
            if let Ok(r) = Request::get(&url2).send().await {
                if r.ok() {
                    if let Ok(meta) = r.json::<DataViewMeta>().await {
                        dv_dims.set(
                            meta.available_dimensions
                                .into_iter()
                                .map(|d| (d.id, d.label))
                                .collect(),
                        );
                    }
                }
            }

            // Load connections
            let url3 = format!("{}/api/connection_mp", api_base());
            if let Ok(r) = Request::get(&url3).send().await {
                if r.ok() {
                    if let Ok(conns) = r.json::<Vec<ConnItem>>().await {
                        connections.set(conns);
                    }
                }
            }

            // Auto-run report
            fetch_version.update(|n| *n += 1);
        });
    }

    // ── Execute report when fetch_version increments ──────────────────────────
    Effect::new(move |_| {
        let v = fetch_version.get();
        if v == 0 { return; } // skip initial mount (session not loaded yet)

        let view_id = view_id_sig.get_untracked();
        if view_id.is_empty() { return; }

        let url = format!("{}/api/data-view/{}/drilldown", api_base(), view_id);
        let req = DvDrilldownRequest {
            date_from: p_date_from.get_untracked(),
            date_to:   p_date_to.get_untracked(),
            period2_from: {
                let v = p_p2_from.get_untracked();
                if v.is_empty() { None } else { Some(v) }
            },
            period2_to: {
                let v = p_p2_to.get_untracked();
                if v.is_empty() { None } else { Some(v) }
            },
            group_by: p_group_by.get_untracked(),
            connection_mp_refs: p_mp_refs.get_untracked(),
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
    let sort_col = RwSignal::new(SortCol::Value1);
    let sort_asc = RwSignal::new(false);

    let toggle_sort = move |col: SortCol| {
        if sort_col.get_untracked() == col {
            sort_asc.update(|a| *a = !*a);
        } else {
            sort_col.set(col);
            sort_asc.set(col == SortCol::Label);
        }
    };

    view! {
        <div class="page drilldown-report">

            // ── Page header ──────────────────────────────────────────────────
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">{move || title.get()}</h1>
                </div>
                {on_close.map(|cb| view! {
                    <div class="page__header-right">
                        <button class="btn btn--ghost" on:click=move |_| cb.run(())>
                            "Закрыть"
                        </button>
                    </div>
                })}
            </div>

            // ── Loading skeleton until session params arrive ──────────────────
            <Show when=move || !session_loaded.get()>
                <div class="drilldown-report__loading">
                    <span class="spinner" />
                    " Загрузка параметров…"
                </div>
            </Show>

            <Show when=move || session_loaded.get()>

                // ── Filter panel ─────────────────────────────────────────────
                <div class="drilldown-report__filters">

                    // Row 1: period inputs + grouping + submit
                    <div class="drilldown-report__filters-row">

                        <div class="drilldown-report__filter-group">
                            <label class="drilldown-report__filter-label">"П1 с"</label>
                            <input
                                type="date"
                                class="drilldown-report__filter-input"
                                prop:value=move || p_date_from.get()
                                on:change=move |ev| p_date_from.set(event_target_value(&ev))
                            />
                        </div>

                        <div class="drilldown-report__filter-group">
                            <label class="drilldown-report__filter-label">"по"</label>
                            <input
                                type="date"
                                class="drilldown-report__filter-input"
                                prop:value=move || p_date_to.get()
                                on:change=move |ev| p_date_to.set(event_target_value(&ev))
                            />
                        </div>

                        <span class="drilldown-report__filter-sep">"·"</span>

                        <div class="drilldown-report__filter-group">
                            <label class="drilldown-report__filter-label">"П2 с"</label>
                            <input
                                type="date"
                                class="drilldown-report__filter-input"
                                placeholder="авто"
                                prop:value=move || p_p2_from.get()
                                on:change=move |ev| p_p2_from.set(event_target_value(&ev))
                            />
                        </div>

                        <div class="drilldown-report__filter-group">
                            <label class="drilldown-report__filter-label">"по"</label>
                            <input
                                type="date"
                                class="drilldown-report__filter-input"
                                placeholder="авто"
                                prop:value=move || p_p2_to.get()
                                on:change=move |ev| p_p2_to.set(event_target_value(&ev))
                            />
                        </div>

                        <span class="drilldown-report__filter-sep">"·"</span>

                        <div class="drilldown-report__filter-group">
                            <label class="drilldown-report__filter-label">"Группировка"</label>
                            <select
                                class="drilldown-report__filter-select"
                                on:change=move |ev| p_group_by.set(event_target_value(&ev))
                            >
                                {move || {
                                    let current = p_group_by.get();
                                    dv_dims.get().into_iter().map(|(id, label)| {
                                        let sel = id == current;
                                        view! {
                                            <option value=id.clone() selected=sel>{label}</option>
                                        }
                                    }).collect_view()
                                }}
                            </select>
                        </div>

                        <button
                            class="btn btn--primary drilldown-report__filter-submit"
                            on:click=move |_| fetch_version.update(|n| *n += 1)
                            disabled=move || loading.get()
                        >
                            {move || if loading.get() { "Загрузка…" } else { "Сформировать" }}
                        </button>
                    </div>

                    // Row 2: connections (only when available)
                    <Show when=move || !connections.get().is_empty()>
                        <div class="drilldown-report__filter-connections">
                            <span class="drilldown-report__filter-label">"Кабинеты:"</span>
                            {move || {
                                connections.get().into_iter().map(|conn| {
                                    let id1 = conn.id.clone();
                                    let id2 = conn.id.clone();
                                    let label = conn.display_name();
                                    view! {
                                        <label class="drilldown-report__mp-check">
                                            <input
                                                type="checkbox"
                                                checked=move || p_mp_refs.get().contains(&id1)
                                                on:change=move |ev| {
                                                    let checked = event_target_checked(&ev);
                                                    p_mp_refs.update(|v| {
                                                        if checked {
                                                            if !v.contains(&id2) {
                                                                v.push(id2.clone());
                                                            }
                                                        } else {
                                                            v.retain(|x| x != &id2);
                                                        }
                                                    });
                                                }
                                            />
                                            " " {label}
                                        </label>
                                    }
                                }).collect_view()
                            }}
                        </div>
                    </Show>
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

                <Show when=move || response.get().is_some() && !loading.get()>
                    {move || {
                        let Some(resp) = response.get() else { return view! { <></> }.into_any() };

                        let p1_label       = resp.period1_label.clone();
                        let p2_label       = resp.period2_label.clone();
                        let metric_label   = resp.metric_label.clone();
                        let group_by_label = resp.group_by_label.clone();

                        let rows_sorted = Signal::derive(move || {
                            let Some(r) = response.get() else { return vec![] };
                            sort_rows(&r.rows, sort_col.get(), sort_asc.get())
                        });

                        // Totals
                        let total1: f64 = resp.rows.iter().map(|r| r.value1).sum();
                        let total2: f64 = resp.rows.iter().map(|r| r.value2).sum();
                        let total_delta = if total2.abs() > 0.01 {
                            Some(((total1 - total2) / total2.abs()) * 100.0)
                        } else {
                            None
                        };
                        let total_delta_cls = delta_class(total_delta).to_string();
                        let total_delta_str = fmt_delta(total_delta);
                        let row_count = resp.rows.len();

                        view! {
                            // Info bar
                            <div class="drilldown-report__meta">
                                <span class="drilldown-report__period-badge drilldown-report__period-badge--1">
                                    "П1: " {p1_label.clone()}
                                </span>
                                <span class="drilldown-report__period-badge drilldown-report__period-badge--2">
                                    "П2: " {p2_label.clone()}
                                </span>
                                <span class="drilldown-report__metric-badge">
                                    {metric_label.clone()}
                                </span>
                                <span class="drilldown-report__count-badge">
                                    {format!("{} строк", row_count)}
                                </span>
                            </div>

                            // Sortable table
                            <div class="table-wrapper" style="overflow-x: auto;">
                            <Table attr:style="width: 100%;" attr:class="drilldown-report__table">
                                <TableHeader>
                                    <TableRow>
                                        <TableHeaderCell class="drill-th">
                                            <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort(SortCol::Label)>
                                                {group_by_label.clone()}
                                                " "
                                                <span class="drill-sort-icon">
                                                    {move || sort_icon(sort_col.get(), SortCol::Label, sort_asc.get())}
                                                </span>
                                            </div>
                                        </TableHeaderCell>
                                        <TableHeaderCell class="drill-th">
                                            <div class="table__sortable-header" style="cursor: pointer; text-align: right;" on:click=move |_| toggle_sort(SortCol::Value1)>
                                                {p1_label.clone()}
                                                " "
                                                <span class="drill-sort-icon">
                                                    {move || sort_icon(sort_col.get(), SortCol::Value1, sort_asc.get())}
                                                </span>
                                            </div>
                                        </TableHeaderCell>
                                        <TableHeaderCell class="drill-th">
                                            <div class="table__sortable-header" style="cursor: pointer; text-align: right;" on:click=move |_| toggle_sort(SortCol::Value2)>
                                                {p2_label.clone()}
                                                " "
                                                <span class="drill-sort-icon">
                                                    {move || sort_icon(sort_col.get(), SortCol::Value2, sort_asc.get())}
                                                </span>
                                            </div>
                                        </TableHeaderCell>
                                        <TableHeaderCell class="drill-th">
                                            <div class="table__sortable-header" style="cursor: pointer; text-align: right;" on:click=move |_| toggle_sort(SortCol::Delta)>
                                                "Δ%"
                                                " "
                                                <span class="drill-sort-icon">
                                                    {move || sort_icon(sort_col.get(), SortCol::Delta, sort_asc.get())}
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
                                                    <td class="data-table__cell">
                                                        {row.label.clone()}
                                                    </td>
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
                                        <td class="data-table__cell drilldown-report__total-label">
                                            "Итого"
                                        </td>
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
        </div>
    }
}
