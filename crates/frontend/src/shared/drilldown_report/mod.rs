//! Drilldown Report (DataView-based) — standalone page with editable filter panel.
//!
//! Сравнительная таблица (П1 vs П2 vs Δ%) с сортировкой, строкой итогов и
//! редактируемой панелью параметров (даты, группировка, кабинеты МП).
//!
//! Tab key: `drilldown__{session_id}` — параметры хранятся в таблице sys_drilldown.

use crate::data_view::api as dv_api;
use crate::data_view::types::FilterDef;
use crate::data_view::ui::FilterBar;
use crate::shared::api_utils::api_base;
use crate::shared::components::field::{Field, FieldContent, FieldGroup, FieldLabel};
use crate::shared::components::field_select::FieldSelect;
use contracts::shared::data_view::ViewContext;
use contracts::shared::drilldown::{DrilldownResponse, DrilldownRow};
use gloo_net::http::Request;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

// ── Request payload (for re-fetch on "Обновить") ─────────────────────────────

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
        // Insert narrow no-break space before every group of 3 digits from the right
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
        Some(d) if d > 0.0 => format!("+{:.1}%", d),
        Some(d) => format!("{:.1}%", d),
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
pub fn DrilldownReportPage(session_id: String, on_close: Option<Callback<()>>) -> impl IntoView {
    // ── Session params (loaded from server once) ──────────────────────────────
    let session_loaded = RwSignal::new(false);
    let title = RwSignal::new(String::new());
    let view_id_sig = RwSignal::new(String::new());

    // ── Editable form params ──────────────────────────────────────────────────
    let view_ctx = RwSignal::new(ViewContext::default());
    let p_group_by = RwSignal::new(String::new());
    let p_metric_id = RwSignal::new(None::<String>);

    // ── Metadata (fetched on mount) ───────────────────────────────────────────
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
    let drawer_open = RwSignal::new(false);

    // ── Load session on mount ─────────────────────────────────────────────────
    {
        let sid = session_id.clone();
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
            view_id_sig.set(record.view_id.clone());

            let df = record.params.date_from.clone();
            let dt = record.params.date_to.clone();

            // If P2 is not stored, compute it as P1 shifted -1 month (mirrors backend logic)
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

            session_loaded.set(true);

            filters_loading.set(true);
            filters_error.set(None);
            match dv_api::fetch_view_filters(&record.view_id).await {
                Ok(defs) => filter_defs.set(defs),
                Err(err) => filters_error.set(Some(err)),
            }
            filters_loading.set(false);

            if let Ok(meta) = dv_api::fetch_by_id(&record.view_id).await {
                dv_dims.set(
                    meta.available_dimensions
                        .into_iter()
                        .map(|dim| (dim.id, dim.label))
                        .collect(),
                );
            }

            // Auto-run report
            fetch_version.update(|n| *n += 1);
        });
    }

    // ── Execute report when fetch_version increments ──────────────────────────
    Effect::new(move |_| {
        let v = fetch_version.get();
        if v == 0 {
            return;
        } // skip initial mount (session not loaded yet)

        let view_id = view_id_sig.get_untracked();
        let group_by = p_group_by.get_untracked();
        if view_id.is_empty() || group_by.is_empty() {
            return;
        }

        let url = format!("{}/api/data-view/{}/drilldown", api_base(), view_id);
        let ctx = view_ctx.get_untracked();
        let req = DvDrilldownRequest {
            date_from: ctx.date_from,
            date_to: ctx.date_to,
            period2_from: ctx.period2_from,
            period2_to: ctx.period2_to,
            group_by,
            connection_mp_refs: ctx.connection_mp_refs,
            metric_id: p_metric_id.get_untracked(),
            params: ctx.params,
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
    let group_by_options = Signal::derive(move || dv_dims.get());
    let group_by_value = Signal::derive(move || p_group_by.get());
    let apply_report = Callback::new(move |_: ()| {
        drawer_open.set(false);
        fetch_version.update(|n| *n += 1);
    });

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
                    <h1 class="page__title">
                        {move || {
                            let base_title = title.get();
                            response
                                .get()
                                .map(|resp| format!("{base_title} ({})", plural_rows(resp.rows.len())))
                                .unwrap_or(base_title)
                        }}
                    </h1>
                </div>

                <div class="page__header-right">
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

                <Show when=move || response.get().is_some() && !loading.get()>
                    {move || {
                        let Some(resp) = response.get() else { return view! { <></> }.into_any() };

                        let p1_label       = resp.period1_label.clone();
                        let p2_label       = resp.period2_label.clone();
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

                        view! {
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

            <Show when=move || session_loaded.get()>
                <OverlayDrawer
                    open=drawer_open
                    position=DrawerPosition::Right
                    size=DrawerSize::Medium
                    close_on_esc=true
                >
                    <DrawerHeader>
                        <div class="drilldown-drawer__header">
                            <DrawerHeaderTitle>
                                <span class="drilldown-drawer__title">"Настройки отчета"</span>
                            </DrawerHeaderTitle>
                            <Button
                                appearance=ButtonAppearance::Primary
                                on_click=move |_| apply_report.run(())
                                disabled=Signal::derive(move || loading.get())
                            >
                                {move || if loading.get() { "Загрузка…" } else { "Применить" }}
                            </Button>
                        </div>
                    </DrawerHeader>
                    <DrawerBody native_scrollbar=true>
                        <div class="drilldown-drawer__body">
                            <div class="drilldown-drawer__content">
                                <FieldGroup class="drilldown-drawer__field-group">
                                    <Field class="field--stretch field--compact">
                                        <FieldContent>
                                            <FieldLabel r#for="drilldown-group-by">"Группировка"</FieldLabel>
                                            <FieldSelect
                                                id="drilldown-group-by"
                                                value=group_by_value
                                                options=group_by_options
                                                placeholder="Выберите группировку"
                                                on_change=Callback::new(move |value: String| p_group_by.set(value))
                                            />
                                        </FieldContent>
                                    </Field>
                                </FieldGroup>

                                {move || {
                                    if filters_loading.get() {
                                        view! {
                                            <div class="drilldown-report__loading">
                                                <span class="spinner" />
                                                " Загрузка фильтров DataView..."
                                            </div>
                                        }.into_any()
                                    } else if let Some(err) = filters_error.get() {
                                        view! { <div class="drilldown-report__error">{err}</div> }.into_any()
                                    } else if filter_defs.get().is_empty() {
                                        view! {
                                            <div class="placeholder placeholder--small">
                                                "Для этого drilldown нет фильтров DataView."
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <div class="drilldown-drawer__filters">
                                                <FilterBar filters=filter_defs.get() ctx=view_ctx />
                                            </div>
                                        }.into_any()
                                    }
                                }}
                            </div>
                        </div>
                    </DrawerBody>
                </OverlayDrawer>
            </Show>
        </div>
    }
}
