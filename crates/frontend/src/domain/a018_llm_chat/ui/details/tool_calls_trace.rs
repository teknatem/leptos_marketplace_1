//! ToolCallsTrace — компактное отображение инструментов в ответе ассистента.
//!
//! Рендерит одну строку с иконками-пилюлями: [🔧 list_data_views 12ms] [✓ execute_query 340ms] ...
//! По клику раскрывается полный список с деталями.

use crate::shared::icons::icon;
use leptos::prelude::*;

/// Структура одного вызова инструмента из tool_trace_json
#[derive(Debug, Clone)]
struct TraceEntry {
    tool: String,
    ok: bool,
    ms: u64,
    summary: String,
}

fn parse_trace(json: &str) -> Vec<TraceEntry> {
    let Ok(arr) = serde_json::from_str::<serde_json::Value>(json) else {
        return vec![];
    };
    let Some(arr) = arr.as_array() else {
        return vec![];
    };
    arr.iter()
        .filter_map(|v| {
            Some(TraceEntry {
                tool: v.get("tool")?.as_str()?.to_string(),
                ok: v.get("ok").and_then(|b| b.as_bool()).unwrap_or(true),
                ms: v.get("ms").and_then(|n| n.as_u64()).unwrap_or(0),
                summary: v
                    .get("summary")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string(),
            })
        })
        .collect()
}

/// Короткое имя инструмента для пилюли
fn short_tool_name(name: &str) -> &str {
    match name {
        "list_entities" => "entities",
        "get_entity_schema" => "schema",
        "get_join_hint" => "join",
        "search_knowledge" => "search",
        "get_knowledge" => "knowledge",
        "list_data_views" => "data_views",
        "execute_query" => "query",
        "create_drilldown_report" => "drilldown",
        other => other,
    }
}

#[component]
#[allow(non_snake_case)]
pub fn ToolCallsTrace(tool_trace: Option<String>) -> impl IntoView {
    let Some(json) = tool_trace else {
        return view! { <></> }.into_any();
    };

    let entries = parse_trace(&json);
    if entries.is_empty() {
        return view! { <></> }.into_any();
    }

    let expanded = RwSignal::new(false);
    let entries = StoredValue::new(entries);

    view! {
        <div class="tool-trace">
            <div
                class="tool-trace__summary"
                on:click=move |_| expanded.update(|v| *v = !*v)
                title="Нажмите чтобы посмотреть детали вызовов инструментов"
            >
                <span class="tool-trace__icon">{icon("wrench")}</span>
                <div class="tool-trace__pills">
                    {entries.get_value().iter().map(|e| {
                        let ok = e.ok;
                        let label = short_tool_name(&e.tool).to_string();
                        let ms = e.ms;
                        view! {
                            <span
                                class=if ok { "tool-trace__pill tool-trace__pill--ok" } else { "tool-trace__pill tool-trace__pill--err" }
                            >
                                {if ok { "✓ " } else { "✗ " }}
                                {label}
                                " "
                                <span class="tool-trace__ms">{format!("{}ms", ms)}</span>
                            </span>
                        }
                    }).collect::<Vec<_>>()}
                </div>
                <span class="tool-trace__toggle">
                    {move || if expanded.get() { icon("chevron-up") } else { icon("chevron-down") }}
                </span>
            </div>

            {move || {
                if expanded.get() {
                    Some(view! {
                        <div class="tool-trace__details">
                            {entries.get_value().iter().map(|e| {
                                let ok = e.ok;
                                let tool = e.tool.clone();
                                let ms = e.ms;
                                let summary = e.summary.clone();
                                view! {
                                    <div class="tool-trace__detail-row">
                                        <span class=if ok { "tool-trace__status tool-trace__status--ok" } else { "tool-trace__status tool-trace__status--err" }>
                                            {if ok { "✓" } else { "✗" }}
                                        </span>
                                        <span class="tool-trace__tool-name">{tool}</span>
                                        <span class="tool-trace__detail-ms">{format!("{}ms", ms)}</span>
                                        <span class="tool-trace__detail-summary">{summary}</span>
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    })
                } else {
                    None
                }
            }}
        </div>
    }.into_any()
}
