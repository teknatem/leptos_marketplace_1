use crate::shared::api_utils::api_base;
use crate::system::auth::storage;
use contracts::system::tasks::metadata::{TaskConfigFieldDto, TaskConfigFieldTypeDto};
use gloo_net::http::Request;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashMap;
use thaw::*;

// ============================================================================
// Cron Editor
// ============================================================================

const CRON_PRESETS: &[(&str, &str)] = &[
    ("5 мин", "0 */5 * * * *"),
    ("15 мин", "0 */15 * * * *"),
    ("30 мин", "0 */30 * * * *"),
    ("Каждый час", "0 0 * * * *"),
    ("Каждые 2 часа", "0 0 */2 * * *"),
    ("Каждый день 00:00", "0 0 0 * * *"),
    ("Каждый день 02:00", "0 0 2 * * *"),
];

/// Visual cron editor with preset buttons and a free-form input.
#[component]
pub fn CronEditor(value: RwSignal<String>) -> impl IntoView {
    view! {
        <div>
            <div style="display:flex;flex-wrap:wrap;gap:6px;margin-bottom:8px;">
                {CRON_PRESETS.iter().map(|(label, cron)| {
                    let cron_s   = cron.to_string();
                    let label_s  = label.to_string();
                    let cron_cmp = cron.to_string();
                    view! {
                        <button
                            type="button"
                            style=move || {
                                if value.get() == cron_cmp {
                                    "padding:4px 10px;border-radius:var(--radius-sm);\
                                     border:1px solid var(--colorBrandStroke1);\
                                     background:var(--colorBrandBackground2);\
                                     color:var(--colorBrandForeground1);\
                                     cursor:pointer;font-size:12px;font-weight:600;"
                                } else {
                                    "padding:4px 10px;border-radius:var(--radius-sm);\
                                     border:1px solid var(--color-border);\
                                     background:var(--colorNeutralBackground1);\
                                     color:var(--color-text);\
                                     cursor:pointer;font-size:12px;"
                                }
                            }
                            on:click=move |_| value.set(cron_s.clone())
                        >
                            {label_s}
                        </button>
                    }
                }).collect_view()}
            </div>
            <Input value placeholder="0 */5 * * * *" />
            <div style="font-size:11px;color:var(--color-text-tertiary);margin-top:4px;">
                "Формат: сек мин час день месяц день_нед  •  Пример: «0 0 2 * * *» — каждый день в 2:00"
            </div>
        </div>
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn parse_config_json(json: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Ok(serde_json::Value::Object(obj)) = serde_json::from_str::<serde_json::Value>(json) {
        for (k, v) in obj {
            let str_val = match &v {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                _ => String::new(),
            };
            map.insert(k, str_val);
        }
    }
    map
}

/// Synchronously compute config JSON from current signal values.
/// Uses `get_untracked()` — safe to call outside reactive context (e.g. event handlers).
fn compute_config_json(
    signals_sv: StoredValue<Vec<(TaskConfigFieldDto, RwSignal<String>)>>,
) -> String {
    signals_sv.with_value(|fields| {
        let parts: Vec<String> = fields
            .iter()
            .map(|(field, sig)| {
                let val = sig.get_untracked();
                let json_val = if field.field_type == TaskConfigFieldTypeDto::Integer {
                    if val.is_empty() {
                        field
                            .default_value
                            .clone()
                            .unwrap_or_else(|| "0".to_string())
                    } else {
                        val
                    }
                } else {
                    format!("\"{}\"", val.replace('"', "\\\""))
                };
                format!("\"{}\":{}", field.key, json_val)
            })
            .collect();
        format!("{{{}}}", parts.join(","))
    })
}

// ============================================================================
// TaskConfigEditor
// ============================================================================

/// Schema-driven config editor.
///
/// Keeps `config_json` updated **synchronously** via DOM event bubbling
/// (not via a deferred Leptos Effect), so the value is always current when
/// the save handler reads it with `get_untracked()`.
#[component]
pub fn TaskConfigEditor(
    config_json: RwSignal<String>,
    schema: Vec<TaskConfigFieldDto>,
) -> impl IntoView {
    // Parse existing JSON once (untracked – no reactive subscription to config_json)
    let initial = parse_config_json(&config_json.get_untracked());

    // One signal per field, initialised from current JSON or schema default
    let field_signals: Vec<(TaskConfigFieldDto, RwSignal<String>)> = schema
        .into_iter()
        .map(|field| {
            let default_val = field.default_value.clone().unwrap_or_default();
            let init_val = initial.get(&field.key).cloned().unwrap_or(default_val);
            let sig = RwSignal::new(init_val);
            (field, sig)
        })
        .collect();

    let signals_sv = StoredValue::new(field_signals);

    // Build initial config_json synchronously right now (no async dependency)
    config_json.set(compute_config_json(signals_sv));

    // Load WB connections if any ConnectionMp field is in the schema
    let connections: RwSignal<Vec<(String, String)>> = RwSignal::new(Vec::new());

    let needs_connections = signals_sv.with_value(|f| {
        f.iter()
            .any(|(fd, _)| fd.field_type == TaskConfigFieldTypeDto::ConnectionMp)
    });

    if needs_connections {
        Effect::new(move |_| {
            let auth = storage::get_access_token().map(|t| format!("Bearer {}", t));
            spawn_local(async move {
                let Some(auth_header) = auth else { return };
                let url = format!("{}/api/connection_mp", api_base());
                let Ok(resp) = Request::get(&url)
                    .header("Authorization", &auth_header)
                    .send()
                    .await
                else {
                    return;
                };
                if !resp.ok() {
                    return;
                }
                let Ok(text) = resp.text().await else { return };
                let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(&text) else {
                    return;
                };
                let entries: Vec<(String, String)> = items
                    .into_iter()
                    .filter_map(|item| {
                        let id = item.get("id")?.as_str()?.to_string();
                        let desc = item
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let code = item
                            .get("code")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let label = if desc.is_empty() { code } else { desc };
                        Some((id, label))
                    })
                    .collect();
                connections.set(entries);
            });
        });
    }

    view! {
        <div style="display:flex;flex-direction:column;gap:16px;">
            {signals_sv.with_value(|fields| {
                fields.iter().map(|(field, sig)| {
                    let sig        = *sig;
                    let label      = field.label.to_string();
                    let hint       = field.hint.to_string();
                    let min_s      = field.min_value.map(|v| v.to_string()).unwrap_or_default();
                    let max_s      = field.max_value.map(|v| v.to_string()).unwrap_or_default();
                    let field_type = field.field_type.clone();
                    let required   = field.required;

                    view! {
                        <div class="form__group">
                            <label class="form__label" style="display:flex;align-items:center;gap:4px;">
                                {label}
                                {if required {
                                    view! { <span style="color:var(--color-error);">"*"</span> }.into_any()
                                } else {
                                    view! {
                                        <span style="font-size:11px;color:var(--color-text-tertiary);">
                                            "(опц.)"
                                        </span>
                                    }.into_any()
                                }}
                            </label>

                            {match field_type {
                                TaskConfigFieldTypeDto::ConnectionMp => {
                                    // Wrap in a div to catch bubbled `change` events.
                                    // The Select is rendered ONLY after connections have loaded,
                                    // so Thaw mounts it with the matching option already present
                                    // and correctly displays the saved value on first render.
                                    view! {
                                        <div on:change=move |_| {
                                            config_json.set(compute_config_json(signals_sv));
                                        }>
                                            {move || {
                                                let conns = connections.get();
                                                if conns.is_empty() {
                                                    view! {
                                                        <span style="font-size:12px;color:var(--color-text-tertiary);">
                                                            "⏳ Загрузка кабинетов..."
                                                        </span>
                                                    }.into_any()
                                                } else {
                                                    let current = sig.get_untracked();
                                                    let has_match = conns.iter().any(|(id, _)| id == &current);
                                                    view! {
                                                        <Select value=sig>
                                                            <option value="">"— Выберите кабинет —"</option>
                                                            {conns.into_iter().map(|(id, lbl)| {
                                                                let selected = id == current && has_match;
                                                                view! {
                                                                    <option
                                                                        value=id.clone()
                                                                        selected=selected
                                                                    >{lbl}</option>
                                                                }
                                                            }).collect_view()}
                                                        </Select>
                                                    }.into_any()
                                                }
                                            }}
                                        </div>
                                    }.into_any()
                                }

                                TaskConfigFieldTypeDto::Integer => {
                                    view! {
                                        <div on:input=move |_| {
                                            config_json.set(compute_config_json(signals_sv));
                                        }>
                                            <Input
                                                value=sig
                                                attr:r#type="number"
                                                attr:min=min_s.clone()
                                                attr:max=max_s.clone()
                                            />
                                        </div>
                                    }.into_any()
                                }

                                TaskConfigFieldTypeDto::Text => {
                                    view! {
                                        <div on:input=move |_| {
                                            config_json.set(compute_config_json(signals_sv));
                                        }>
                                            <Input value=sig />
                                        </div>
                                    }.into_any()
                                }

                                TaskConfigFieldTypeDto::Date => {
                                    view! {
                                        <input
                                            type="date"
                                            prop:value=move || sig.get()
                                            on:input=move |ev| {
                                                sig.set(event_target_value(&ev));
                                                config_json.set(compute_config_json(signals_sv));
                                            }
                                            style="width:100%;padding:6px 10px;border:1px solid var(--color-border);border-radius:var(--radius-sm);background:var(--colorNeutralBackground1);color:var(--color-text);font-size:var(--font-size-base);"
                                        />
                                    }.into_any()
                                }
                            }}

                            {if !hint.is_empty() {
                                view! {
                                    <div style="font-size:11px;color:var(--color-text-tertiary);margin-top:3px;">
                                        {hint}
                                    </div>
                                }.into_any()
                            } else {
                                view! { <></> }.into_any()
                            }}
                        </div>
                    }
                }).collect_view()
            })}
        </div>
    }
}
