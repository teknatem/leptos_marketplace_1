use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

use crate::shared::date_utils::format_datetime;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator};
use crate::shared::table_utils::init_column_resize;

const TABLE_ID_LINES: &str = "a016-ym-return-lines-table";
const TABLE_ID_PROJECTIONS: &str = "a016-ym-return-p904-table";
const COLUMN_WIDTHS_KEY_LINES: &str = "a016_ym_return_lines_column_widths";
const COLUMN_WIDTHS_KEY_PROJECTIONS: &str = "a016_ym_return_p904_column_widths";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmReturnDetailDto {
    pub id: String,
    pub code: String,
    pub description: String,
    pub header: HeaderDto,
    pub lines: Vec<LineDto>,
    pub state: StateDto,
    pub source_meta: SourceMetaDto,
    pub is_posted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderDto {
    pub return_id: i64,
    pub order_id: i64,
    pub connection_id: String,
    pub organization_id: String,
    pub marketplace_id: String,
    pub campaign_id: String,
    pub return_type: String,
    pub amount: Option<f64>,
    pub currency: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineDto {
    pub item_id: i64,
    pub shop_sku: String,
    pub offer_id: String,
    pub name: String,
    pub count: i32,
    pub price: Option<f64>,
    pub return_reason: Option<String>,
    pub decisions: Vec<DecisionDto>,
    #[serde(default)]
    pub photos: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionDto {
    pub decision_type: String,
    pub amount: Option<f64>,
    pub currency: Option<String>,
    pub partner_compensation_amount: Option<f64>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDto {
    pub refund_status: String,
    pub created_at_source: Option<String>,
    pub updated_at_source: Option<String>,
    pub refund_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMetaDto {
    pub raw_payload_ref: String,
    pub fetched_at: String,
    pub document_version: i32,
}

#[component]
pub fn YmReturnDetail(id: String, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let (return_data, set_return_data) = signal::<Option<YmReturnDetailDto>>(None);
    let (raw_json_from_ym, set_raw_json_from_ym) = signal::<Option<String>>(None);
    let (projections, set_projections) = signal::<Option<serde_json::Value>>(None);
    let (projections_loading, set_projections_loading) = signal(false);
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal::<Option<String>>(None);
    let (active_tab, set_active_tab) = signal("general");
    
    // Sort state for lines table
    let (lines_sort_column, set_lines_sort_column) = signal::<Option<&'static str>>(None);
    let (lines_sort_asc, set_lines_sort_asc) = signal(true);
    
    // Sort state for projections table
    let (proj_sort_column, set_proj_sort_column) = signal::<Option<&'static str>>(None);
    let (proj_sort_asc, set_proj_sort_asc) = signal(true);

    // Загрузить детальные данные
    Effect::new(move || {
        let id = id.clone();
        wasm_bindgen_futures::spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let url = format!("http://localhost:3000/api/a016/ym-returns/{}", id);

            match Request::get(&url).send().await {
                Ok(response) => {
                    let status = response.status();
                    if status == 200 {
                        match response.text().await {
                            Ok(text) => {
                                match serde_json::from_str::<YmReturnDetailDto>(&text) {
                                    Ok(data) => {
                                        let raw_payload_ref = data.source_meta.raw_payload_ref.clone();
                                        let return_id = data.id.clone();
                                        set_return_data.set(Some(data));
                                        set_loading.set(false);

                                        // Асинхронная загрузка raw JSON
                                        wasm_bindgen_futures::spawn_local(async move {
                                            let raw_url = format!(
                                                "http://localhost:3000/api/a016/raw/{}",
                                                raw_payload_ref
                                            );
                                            match Request::get(&raw_url).send().await {
                                                Ok(resp) => {
                                                    if resp.status() == 200 {
                                                        if let Ok(text) = resp.text().await {
                                                            if let Ok(json_value) =
                                                                serde_json::from_str::<serde_json::Value>(
                                                                    &text,
                                                                )
                                                            {
                                                                if let Ok(formatted) =
                                                                    serde_json::to_string_pretty(
                                                                        &json_value,
                                                                    )
                                                                {
                                                                    set_raw_json_from_ym
                                                                        .set(Some(formatted));
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    log!("Failed to load raw JSON: {:?}", e);
                                                }
                                            }
                                        });

                                        // Асинхронная загрузка проекций
                                        wasm_bindgen_futures::spawn_local(async move {
                                            set_projections_loading.set(true);
                                            let projections_url = format!(
                                                "http://localhost:3000/api/a016/ym-returns/{}/projections",
                                                return_id
                                            );
                                            match Request::get(&projections_url).send().await {
                                                Ok(resp) => {
                                                    if resp.status() == 200 {
                                                        if let Ok(text) = resp.text().await {
                                                            if let Ok(proj_data) =
                                                                serde_json::from_str::<serde_json::Value>(&text)
                                                            {
                                                                set_projections.set(Some(proj_data));
                                                            }
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    log!("Failed to load projections: {:?}", e);
                                                }
                                            }
                                            set_projections_loading.set(false);
                                        });
                                    }
                                    Err(e) => {
                                        log!("Failed to parse return: {:?}", e);
                                        set_error.set(Some(format!("Failed to parse: {}", e)));
                                        set_loading.set(false);
                                    }
                                }
                            }
                            Err(e) => {
                                log!("Failed to read response: {:?}", e);
                                set_error.set(Some(format!("Failed to read response: {}", e)));
                                set_loading.set(false);
                            }
                        }
                    } else {
                        set_error.set(Some(format!("Server error: {}", status)));
                        set_loading.set(false);
                    }
                }
                Err(e) => {
                    log!("Failed to fetch return: {:?}", e);
                    set_error.set(Some(format!("Failed to fetch: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    });

    view! {
        <div class="detail-form">
            <div class="detail-form-header">
                <div class="detail-form-header-left">
                    <h2>"Yandex Market Return"</h2>
                </div>
                <div class="detail-form-header-right">
                    <button
                        class="btn btn-secondary"
                        on:click=move |_| on_close.run(())
                    >
                        "✕ Закрыть"
                    </button>
                </div>
            </div>

            <div class="detail-form-content">
                {move || {
                    if loading.get() {
                        view! {
                            <div style="text-align: center; padding: var(--space-2xl);">
                                <p style="font-size: var(--font-size-sm);">"Загрузка..."</p>
                            </div>
                        }.into_any()
                    } else if let Some(err) = error.get() {
                        view! {
                            <div style="padding: var(--space-lg); background: var(--color-error-bg); border: 1px solid var(--color-error-border); border-radius: var(--radius-sm); color: var(--color-error-text); margin: var(--space-lg); font-size: var(--font-size-sm);">
                                <strong>"Ошибка: "</strong>{err}
                            </div>
                        }.into_any()
                    } else if let Some(data) = return_data.get() {
                        view! {
                            <div>
                                <div class="detail-tabs">
                                    <button
                                        class="detail-tab"
                                        class:active=move || active_tab.get() == "general"
                                        on:click=move |_| set_active_tab.set("general")
                                    >
                                        "Общие данные"
                                    </button>
                                    <button
                                        class="detail-tab"
                                        class:active=move || active_tab.get() == "lines"
                                        on:click=move |_| set_active_tab.set("lines")
                                    >
                                        "Товары"
                                    </button>
                                    <button
                                        class="detail-tab"
                                        class:active=move || active_tab.get() == "projections"
                                        on:click=move |_| set_active_tab.set("projections")
                                    >
                                        {move || {
                                            let count = projections.get().as_ref().map(|p| {
                                                p["p904_sales_data"].as_array().map(|a| a.len()).unwrap_or(0)
                                            }).unwrap_or(0);
                                            format!("Проекции ({})", count)
                                        }}
                                    </button>
                                    <button
                                        class="detail-tab"
                                        class:active=move || active_tab.get() == "json"
                                        on:click=move |_| set_active_tab.set("json")
                                    >
                                        "Raw JSON"
                                    </button>
                                </div>

                                <div style="padding-top: var(--space-lg);">
                                    {move || {
                                        let tab = active_tab.get();
                                        match tab.as_ref() {
                                            "general" => {
                                                let return_type_label = match data.header.return_type.as_str() {
                                                    "UNREDEEMED" => "Невыкуп",
                                                    "RETURN" => "Возврат",
                                                    _ => data.header.return_type.as_str(),
                                                };
                                                let (return_type_class, return_type_extra_style) = match data.header.return_type.as_str() {
                                                    "UNREDEEMED" => ("badge", "background: #fff3e0; color: #e65100;"),
                                                    "RETURN" => ("badge", "background: #e3f2fd; color: #1565c0;"),
                                                    _ => ("badge", "background: #f5f5f5; color: #666;"),
                                                };
                                                let (refund_status_class, refund_extra_style) = match data.state.refund_status.as_str() {
                                                    "REFUNDED" => ("badge badge-success", ""),
                                                    "NOT_REFUNDED" => ("badge badge-error", ""),
                                                    "REFUND_IN_PROGRESS" => ("badge", "background: #fff3e0; color: #e65100;"),
                                                    _ => ("badge", "background: #f5f5f5; color: #666;"),
                                                };

                                                view! {
                                                    <div class="general-info" style="max-width: 1400px;">
                                                        <div style="background: var(--color-bg-body); padding: var(--space-xl); border-radius: var(--radius-md); border: 1px solid var(--color-border-lighter);">
                                                            <div style="display: grid; grid-template-columns: 180px 1fr; gap: var(--space-md); align-items: start; font-size: var(--font-size-sm);">
                                                                <div class="field-label">"Return №:"</div>
                                                                <div style="font-family: monospace; font-size: var(--font-size-base); font-weight: var(--font-weight-semibold); color: #1976d2;">{data.header.return_id}</div>

                                                                <div class="field-label">"Order №:"</div>
                                                                <div style="font-family: monospace;">{data.header.order_id}</div>

                                                                <div class="field-label">"Type:"</div>
                                                                <div>
                                                                    <span class={return_type_class} style={return_type_extra_style}>
                                                                        {return_type_label}
                                                                    </span>
                                                                </div>

                                                                <div class="field-label">"Refund Status:"</div>
                                                                <div>
                                                                    <span class={refund_status_class} style={refund_extra_style}>
                                                                        {data.state.refund_status.clone()}
                                                                    </span>
                                                                </div>

                                                                <div class="field-label">"Amount:"</div>
                                                                <div style="font-size: var(--font-size-base); font-weight: var(--font-weight-semibold); color: #c62828;">
                                                                    {data.header.amount.map(|a| format!("{:.2}", a)).unwrap_or("—".to_string())}
                                                                    {data.header.currency.clone().map(|c| format!(" {}", c)).unwrap_or_default()}
                                                                </div>

                                                                <div class="field-label">"Campaign ID:"</div>
                                                                <div style="font-family: monospace;">{data.header.campaign_id.clone()}</div>

                                                                <div class="field-label">"Created At Source:"</div>
                                                                <div class="field-value">{data.state.created_at_source.as_ref().map(|d| format_datetime(d)).unwrap_or("—".to_string())}</div>

                                                                <div class="field-label">"Updated At Source:"</div>
                                                                <div class="field-value">{data.state.updated_at_source.as_ref().map(|d| format_datetime(d)).unwrap_or("—".to_string())}</div>

                                                                <div class="field-label">"Refund Date:"</div>
                                                                <div class="field-value">{data.state.refund_date.as_ref().map(|d| format_datetime(d)).unwrap_or("—".to_string())}</div>

                                                                <div class="field-label">"Fetched At:"</div>
                                                                <div class="field-value">{format_datetime(&data.source_meta.fetched_at)}</div>

                                                                <div class="field-label">"Document Version:"</div>
                                                                <div class="field-value">{data.source_meta.document_version}</div>

                                                                <div class="field-label">"Is Posted:"</div>
                                                                <div>
                                                                    {if data.is_posted {
                                                                        view! { <span style="color: var(--color-success); font-weight: var(--font-weight-medium);">"✓ Yes"</span> }.into_any()
                                                                    } else {
                                                                        view! { <span style="color: var(--color-text-muted);">"No"</span> }.into_any()
                                                                    }}
                                                                </div>
                                                            </div>
                                                        </div>
                                                    </div>
                                                }.into_any()
                                            },
                                            "lines" => {
                                                // Clone and sort lines
                                                let mut lines = data.lines.clone();
                                                if let Some(col) = lines_sort_column.get() {
                                                    let asc = lines_sort_asc.get();
                                                    lines.sort_by(|a, b| {
                                                        let cmp = match col {
                                                            "shop_sku" => a.shop_sku.cmp(&b.shop_sku),
                                                            "name" => a.name.cmp(&b.name),
                                                            "count" => a.count.cmp(&b.count),
                                                            "price" => {
                                                                let a_price = a.price.unwrap_or(0.0);
                                                                let b_price = b.price.unwrap_or(0.0);
                                                                a_price.partial_cmp(&b_price).unwrap_or(Ordering::Equal)
                                                            },
                                                            _ => Ordering::Equal,
                                                        };
                                                        if asc { cmp } else { cmp.reverse() }
                                                    });
                                                }
                                                
                                                let total_items: i32 = lines.iter().map(|l| l.count).sum();
                                                let total_amount: f64 = lines.iter().filter_map(|l| l.price.map(|p| p * l.count as f64)).sum();

                                                // Sort handler
                                                let handle_lines_sort = move |column: &'static str| {
                                                    if lines_sort_column.get() == Some(column) {
                                                        set_lines_sort_asc.set(!lines_sort_asc.get());
                                                    } else {
                                                        set_lines_sort_column.set(Some(column));
                                                        set_lines_sort_asc.set(true);
                                                    }
                                                };

                                                view! {
                                                    <div class="lines-info">
                                                        <div style="margin-bottom: var(--space-lg); padding: var(--space-lg); background: var(--color-error-bg); border-radius: var(--radius-sm); font-size: var(--font-size-sm);">
                                                            <strong>"Сводка по возврату: "</strong>
                                                            {format!("{} позиций, {} шт. всего, {:.2} сумма",
                                                                lines.len(),
                                                                total_items,
                                                                total_amount
                                                            )}
                                                        </div>

                                                        <div class="table-container">
                                                            <table class="data-table" id=TABLE_ID_LINES>
                                                                <thead>
                                                                    <tr>
                                                                        <th class="resizable" on:click=move |_| handle_lines_sort("shop_sku")>
                                                                            <span class="sortable-header">
                                                                                "Shop SKU"
                                                                                <span class={move || get_sort_class(lines_sort_column.get().unwrap_or(""), "shop_sku")}>
                                                                                    {move || get_sort_indicator(lines_sort_column.get().unwrap_or(""), "shop_sku", lines_sort_asc.get())}
                                                                                </span>
                                                                            </span>
                                                                        </th>
                                                                        <th class="resizable" on:click=move |_| handle_lines_sort("name")>
                                                                            <span class="sortable-header">
                                                                                "Наименование"
                                                                                <span class={move || get_sort_class(lines_sort_column.get().unwrap_or(""), "name")}>
                                                                                    {move || get_sort_indicator(lines_sort_column.get().unwrap_or(""), "name", lines_sort_asc.get())}
                                                                                </span>
                                                                            </span>
                                                                        </th>
                                                                        <th class="resizable text-right" on:click=move |_| handle_lines_sort("count")>
                                                                            <span class="sortable-header">
                                                                                "Кол-во"
                                                                                <span class={move || get_sort_class(lines_sort_column.get().unwrap_or(""), "count")}>
                                                                                    {move || get_sort_indicator(lines_sort_column.get().unwrap_or(""), "count", lines_sort_asc.get())}
                                                                                </span>
                                                                            </span>
                                                                        </th>
                                                                        <th class="resizable text-right" on:click=move |_| handle_lines_sort("price")>
                                                                            <span class="sortable-header">
                                                                                "Цена"
                                                                                <span class={move || get_sort_class(lines_sort_column.get().unwrap_or(""), "price")}>
                                                                                    {move || get_sort_indicator(lines_sort_column.get().unwrap_or(""), "price", lines_sort_asc.get())}
                                                                                </span>
                                                                            </span>
                                                                        </th>
                                                                        <th class="resizable">"Причина"</th>
                                                                        <th class="resizable">"Тип решения"</th>
                                                                        <th class="resizable">"Комментарий"</th>
                                                                    </tr>
                                                                </thead>
                                                            <tbody>
                                                                {lines.iter().map(|line| {
                                                                    let decision_type = line.decisions.first()
                                                                        .map(|d| d.decision_type.clone())
                                                                        .unwrap_or("—".to_string());
                                                                    let comment = line.decisions.first()
                                                                        .and_then(|d| d.comment.clone())
                                                                        .unwrap_or("—".to_string());

                                                                    view! {
                                                                        <tr>
                                                                            <td>
                                                                                <code style="font-size: var(--font-size-xs);">{line.shop_sku.clone()}</code>
                                                                            </td>
                                                                            <td>{line.name.clone()}</td>
                                                                            <td class="text-right">
                                                                                <strong>{line.count}</strong>
                                                                            </td>
                                                                            <td class="text-right">
                                                                                {line.price.map(|p| format!("{:.2}", p)).unwrap_or("—".to_string())}
                                                                            </td>
                                                                            <td style="font-size: var(--font-size-xs);">
                                                                                {line.return_reason.clone().unwrap_or("—".to_string())}
                                                                            </td>
                                                                            <td style="font-size: var(--font-size-xs);">
                                                                                {decision_type}
                                                                            </td>
                                                                            <td style="font-size: var(--font-size-xs);">
                                                                                {comment}
                                                                            </td>
                                                                        </tr>
                                                                    }
                                                                }).collect_view()}
                                                                <tr style="background: var(--color-bg-secondary); font-weight: var(--font-weight-semibold);">
                                                                    <td colspan="2" class="text-right">"Итого:"</td>
                                                                    <td class="text-right">{total_items}</td>
                                                                    <td class="text-right" style="color: #c62828;">{format!("{:.2}", total_amount)}</td>
                                                                    <td colspan="3"></td>
                                                                </tr>
                                                            </tbody>
                                                        </table>
                                                        </div>
                                                        {
                                                            // Initialize column resize after table renders
                                                            Effect::new(move || {
                                                                init_column_resize(TABLE_ID_LINES, COLUMN_WIDTHS_KEY_LINES);
                                                            });
                                                        }
                                                    </div>
                                                }.into_any()
                                            },
                                            "projections" => view! {
                                                <div class="projections-info">
                                                    {move || {
                                                        if projections_loading.get() {
                                                            view! {
                                                                <div style="padding: var(--space-xl); text-align: center; color: var(--color-text-muted); font-size: var(--font-size-sm);">
                                                                    "Загрузка проекций..."
                                                                </div>
                                                            }.into_any()
                                                        } else if let Some(proj_data) = projections.get() {
                                                            let mut p904_items = proj_data["p904_sales_data"].as_array().cloned().unwrap_or_default();
                                                            
                                                            // Sort p904 items
                                                            if let Some(col) = proj_sort_column.get() {
                                                                let asc = proj_sort_asc.get();
                                                                p904_items.sort_by(|a, b| {
                                                                    let cmp = match col {
                                                                        "article" => {
                                                                            let a_val = a["article"].as_str().unwrap_or("");
                                                                            let b_val = b["article"].as_str().unwrap_or("");
                                                                            a_val.cmp(b_val)
                                                                        },
                                                                        "date" => {
                                                                            let a_val = a["date"].as_str().unwrap_or("");
                                                                            let b_val = b["date"].as_str().unwrap_or("");
                                                                            a_val.cmp(b_val)
                                                                        },
                                                                        "price_list" => {
                                                                            let a_val = a["price_list"].as_f64().unwrap_or(0.0);
                                                                            let b_val = b["price_list"].as_f64().unwrap_or(0.0);
                                                                            a_val.partial_cmp(&b_val).unwrap_or(Ordering::Equal)
                                                                        },
                                                                        "price_return" => {
                                                                            let a_val = a["price_return"].as_f64().unwrap_or(0.0);
                                                                            let b_val = b["price_return"].as_f64().unwrap_or(0.0);
                                                                            a_val.partial_cmp(&b_val).unwrap_or(Ordering::Equal)
                                                                        },
                                                                        "customer_out" => {
                                                                            let a_val = a["customer_out"].as_f64().unwrap_or(0.0);
                                                                            let b_val = b["customer_out"].as_f64().unwrap_or(0.0);
                                                                            a_val.partial_cmp(&b_val).unwrap_or(Ordering::Equal)
                                                                        },
                                                                        "total" => {
                                                                            let a_val = a["total"].as_f64().unwrap_or(0.0);
                                                                            let b_val = b["total"].as_f64().unwrap_or(0.0);
                                                                            a_val.partial_cmp(&b_val).unwrap_or(Ordering::Equal)
                                                                        },
                                                                        _ => Ordering::Equal,
                                                                    };
                                                                    if asc { cmp } else { cmp.reverse() }
                                                                });
                                                            }
                                                            
                                                            // Sort handler for projections
                                                            let handle_proj_sort = move |column: &'static str| {
                                                                if proj_sort_column.get() == Some(column) {
                                                                    set_proj_sort_asc.set(!proj_sort_asc.get());
                                                                } else {
                                                                    set_proj_sort_column.set(Some(column));
                                                                    set_proj_sort_asc.set(true);
                                                                }
                                                            };

                                                            view! {
                                                                <div style="display: flex; flex-direction: column; gap: var(--space-lg);">
                                                                    <div style="background: var(--color-bg-body); padding: var(--space-lg); border-radius: var(--radius-md); box-shadow: var(--shadow-sm); border: 1px solid var(--color-border-lighter);">
                                                                        <h3 style="margin: 0 0 var(--space-lg) 0; color: var(--color-text-primary); font-size: var(--font-size-base); font-weight: var(--font-weight-semibold); border-bottom: 2px solid var(--color-primary); padding-bottom: var(--space-md);">
                                                                            {format!("📈 Sales Data (p904) — {} записей", p904_items.len())}
                                                                        </h3>
                                                                        {if !p904_items.is_empty() {
                                                                            view! {
                                                                                <div class="table-container">
                                                                                    <table class="data-table" id=TABLE_ID_PROJECTIONS>
                                                                                        <thead>
                                                                                            <tr>
                                                                                                <th class="resizable" on:click=move |_| handle_proj_sort("article")>
                                                                                                    <span class="sortable-header">
                                                                                                        "Артикул"
                                                                                                        <span class={move || get_sort_class(proj_sort_column.get().unwrap_or(""), "article")}>
                                                                                                            {move || get_sort_indicator(proj_sort_column.get().unwrap_or(""), "article", proj_sort_asc.get())}
                                                                                                        </span>
                                                                                                    </span>
                                                                                                </th>
                                                                                                <th class="resizable" on:click=move |_| handle_proj_sort("date")>
                                                                                                    <span class="sortable-header">
                                                                                                        "Дата"
                                                                                                        <span class={move || get_sort_class(proj_sort_column.get().unwrap_or(""), "date")}>
                                                                                                            {move || get_sort_indicator(proj_sort_column.get().unwrap_or(""), "date", proj_sort_asc.get())}
                                                                                                        </span>
                                                                                                    </span>
                                                                                                </th>
                                                                                                <th class="resizable text-right" on:click=move |_| handle_proj_sort("price_list") title="price_list">
                                                                                                    <span class="sortable-header">
                                                                                                        "Цена прайс"
                                                                                                        <span class={move || get_sort_class(proj_sort_column.get().unwrap_or(""), "price_list")}>
                                                                                                            {move || get_sort_indicator(proj_sort_column.get().unwrap_or(""), "price_list", proj_sort_asc.get())}
                                                                                                        </span>
                                                                                                    </span>
                                                                                                </th>
                                                                                                <th class="resizable text-right" on:click=move |_| handle_proj_sort("price_return") title="price_return">
                                                                                                    <span class="sortable-header">
                                                                                                        "Цена возврат"
                                                                                                        <span class={move || get_sort_class(proj_sort_column.get().unwrap_or(""), "price_return")}>
                                                                                                            {move || get_sort_indicator(proj_sort_column.get().unwrap_or(""), "price_return", proj_sort_asc.get())}
                                                                                                        </span>
                                                                                                    </span>
                                                                                                </th>
                                                                                                <th class="resizable text-right" on:click=move |_| handle_proj_sort("customer_out") title="customer_out (отрицательное значение - возврат)">
                                                                                                    <span class="sortable-header">
                                                                                                        "К клиенту"
                                                                                                        <span class={move || get_sort_class(proj_sort_column.get().unwrap_or(""), "customer_out")}>
                                                                                                            {move || get_sort_indicator(proj_sort_column.get().unwrap_or(""), "customer_out", proj_sort_asc.get())}
                                                                                                        </span>
                                                                                                    </span>
                                                                                                </th>
                                                                                                <th class="resizable text-right" on:click=move |_| handle_proj_sort("total") title="total">
                                                                                                    <span class="sortable-header">
                                                                                                        "Итого"
                                                                                                        <span class={move || get_sort_class(proj_sort_column.get().unwrap_or(""), "total")}>
                                                                                                            {move || get_sort_indicator(proj_sort_column.get().unwrap_or(""), "total", proj_sort_asc.get())}
                                                                                                        </span>
                                                                                                    </span>
                                                                                                </th>
                                                                                            </tr>
                                                                                        </thead>
                                                                                        <tbody>
                                                                                            {p904_items.iter().map(|item| {
                                                                                                let article = item["article"].as_str().unwrap_or("—");
                                                                                                let date = item["date"].as_str().unwrap_or("—");
                                                                                                let date_formatted = if date.len() > 10 { &date[..10] } else { date };
                                                                                                let price_list = item["price_list"].as_f64().unwrap_or(0.0);
                                                                                                let price_return = item["price_return"].as_f64().unwrap_or(0.0);
                                                                                                let customer_out = item["customer_out"].as_f64().unwrap_or(0.0);
                                                                                                let total = item["total"].as_f64().unwrap_or(0.0);

                                                                                                view! {
                                                                                                    <tr>
                                                                                                        <td style="font-family: monospace; font-size: var(--font-size-xs);">{article}</td>
                                                                                                        <td>{date_formatted}</td>
                                                                                                        <td class="text-right">{format!("{:.2}", price_list)}</td>
                                                                                                        <td class="text-right" style="color: #e65100;">{format!("{:.2}", price_return)}</td>
                                                                                                        <td class="text-right" style="color: #c62828; background: var(--color-error-bg); font-weight: var(--font-weight-semibold);">{format!("{:.2}", customer_out)}</td>
                                                                                                        <td class="text-right font-medium">{format!("{:.2}", total)}</td>
                                                                                                    </tr>
                                                                                                }
                                                                                            }).collect::<Vec<_>>()}
                                                                                        </tbody>
                                                                                    </table>
                                                                                </div>
                                                                                {
                                                                                    // Initialize column resize after table renders
                                                                                    Effect::new(move || {
                                                                                        init_column_resize(TABLE_ID_PROJECTIONS, COLUMN_WIDTHS_KEY_PROJECTIONS);
                                                                                    });
                                                                                }
                                                                            }.into_any()
                                                                        } else {
                                                                            view! {
                                                                                <p style="text-align: center; padding: var(--space-lg); color: var(--color-text-muted); font-size: var(--font-size-sm);">
                                                                                    "Нет записей. Документ должен иметь статус REFUNDED и быть проведён."
                                                                                </p>
                                                                            }.into_any()
                                                                        }}
                                                                    </div>
                                                                </div>
                                                            }.into_any()
                                                        } else {
                                                            view! {
                                                                <div style="padding: var(--space-xl); text-align: center; color: var(--color-text-muted); font-size: var(--font-size-sm);">
                                                                    "Нет данных проекций"
                                                                </div>
                                                            }.into_any()
                                                        }
                                                    }}
                                                </div>
                                            }.into_any(),
                                            "json" => view! {
                                                <div class="json-info">
                                                    <div style="margin-bottom: var(--space-lg); font-size: var(--font-size-sm); font-weight: var(--font-weight-semibold);">
                                                        "Raw JSON from Yandex Market API:"
                                                    </div>
                                                    {move || {
                                                        if let Some(json) = raw_json_from_ym.get() {
                                                            view! {
                                                                <pre style="background: var(--color-bg-secondary); padding: var(--space-lg); border-radius: var(--radius-sm); overflow-x: auto; font-size: var(--font-size-xs); border: 1px solid var(--color-border-lighter);">
                                                                    {json}
                                                                </pre>
                                                            }.into_any()
                                                        } else {
                                                            view! {
                                                                <div style="padding: var(--space-xl); text-align: center; color: var(--color-text-muted); font-size: var(--font-size-sm);">
                                                                    "Загрузка raw JSON из Yandex Market..."
                                                                </div>
                                                            }.into_any()
                                                        }
                                                    }}
                                                </div>
                                            }.into_any(),
                                            _ => view! { <div>"Unknown tab"</div> }.into_any()
                                        }
                                    }}
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! { <div>"No data"</div> }.into_any()
                    }
                }}
            </div>
        </div>
    }
}
