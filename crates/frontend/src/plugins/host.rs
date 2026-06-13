//! `PluginHost` — движок плагинов (компилируется один раз).
//!
//! Вкладки (как в a033): «Выполнение» (редакторы server_script/client_script,
//! кнопки Выполнить/Сохранить, консоль) и «Макет» (view_spec).
//!
//! Модель исполнения:
//! - server_script — Rhai на бэкенде; платформенные функции host_read/host_exec/host_query.
//!   Клиент вызывает именованный метод сервера через `call_server("имя")`.
//! - client_script — Rhai в браузере; платформенные функции `call_server(name)` и `print`.
//!   Синхронный Rhai не ждёт сеть, поэтому host заранее выполняет все
//!   `call_server("…")` (POST /api/plugin/:id/run с именем функции и текущим
//!   исходником server_script), кладёт результаты в карту, затем запускает client_script.

use crate::plugins::{api, engine};
use contracts::plugins::{PluginBundle, PluginDefinition, PluginRunContext, PluginUpsert};
use contracts::shared::drilldown::DrilldownResponse;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashMap;
use thaw::{Tab, TabList};

/// Обобщённая таблица для рендера (колонки + строки строковых ячеек).
#[derive(Clone, Default)]
struct GenericTable {
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
}

fn val_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        other => other.to_string(),
    }
}

fn drilldown_to_table(resp: &DrilldownResponse) -> GenericTable {
    let metric_col = format!("{} ({})", resp.metric_label, resp.period1_label);
    GenericTable {
        columns: vec![resp.group_by_label.clone(), metric_col],
        rows: resp
            .rows
            .iter()
            .map(|r| vec![r.label.clone(), format!("{:.2}", r.value1)])
            .collect(),
    }
}

/// JSON-результат скрипта → обобщённая таблица (массив объектов → колонки по ключам).
fn json_to_table(value: serde_json::Value) -> Option<GenericTable> {
    let arr = value.as_array()?;
    if arr.is_empty() {
        return Some(GenericTable::default());
    }
    if let Some(first_obj) = arr[0].as_object() {
        let columns: Vec<String> = first_obj.keys().cloned().collect();
        let rows = arr
            .iter()
            .map(|item| {
                let obj = item.as_object();
                columns
                    .iter()
                    .map(|c| {
                        obj.and_then(|o| o.get(c))
                            .map(val_to_string)
                            .unwrap_or_default()
                    })
                    .collect()
            })
            .collect();
        Some(GenericTable { columns, rows })
    } else {
        Some(GenericTable {
            columns: vec!["value".to_string()],
            rows: arr.iter().map(|v| vec![val_to_string(v)]).collect(),
        })
    }
}

fn split_run_body(body: &serde_json::Value) -> (serde_json::Value, Vec<String>) {
    let result = body.get("result").cloned().unwrap_or(serde_json::Value::Null);
    let logs = body
        .get("logs")
        .and_then(|l| l.as_array())
        .map(|a| {
            a.iter()
                .map(|v| v.as_str().unwrap_or_default().to_string())
                .collect()
        })
        .unwrap_or_default();
    (result, logs)
}

#[component]
pub fn PluginHost(plugin_id: String) -> impl IntoView {
    let (def, set_def) = signal(None::<PluginDefinition>);
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(None::<String>);

    // Редактируемые исходники (можно менять и запускать без сохранения).
    let server_src = RwSignal::new(String::new());
    let client_src = RwSignal::new(String::new());
    let (version, set_version) = signal(1i32);

    // Запуск.
    let (table, set_table) = signal(None::<GenericTable>);
    let (running, set_running) = signal(false);
    let (run_error, set_run_error) = signal(None::<String>);
    let (console, set_console) = signal(Vec::<String>::new());

    // Сохранение.
    let (saving, set_saving) = signal(false);
    let (save_msg, set_save_msg) = signal(None::<String>);

    let selected_tab = RwSignal::new("run".to_string());

    // Загрузка определения.
    {
        let id = plugin_id.clone();
        spawn_local(async move {
            match api::get_by_id(&id).await {
                Ok(d) => {
                    server_src.set(d.bundle.server_script.clone().unwrap_or_default());
                    client_src.set(d.bundle.client_script.clone().unwrap_or_default());
                    set_version.set(d.version);
                    set_def.set(Some(d));
                }
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    }

    let base_ctx = || PluginRunContext {
        date_from: None,
        date_to: None,
        connection_mp_refs: vec![],
        group_by: None,
        params: Default::default(),
        function: None,
        script_override: None,
    };

    // «Выполнить».
    let run = {
        let plugin_id = plugin_id.clone();
        move |_| {
            let id = plugin_id.clone();
            let client_code = client_src.get_untracked();
            let server_code = server_src.get_untracked();
            let has_view = def
                .get_untracked()
                .and_then(|d| d.bundle.data.view_id.clone())
                .is_some();
            set_running.set(true);
            set_run_error.set(None);
            set_table.set(None);
            set_console.set(Vec::new());
            let mut lines: Vec<String> = Vec::new();

            spawn_local(async move {
                // Случай 1: есть client_script — он драйвит, вызывая методы сервера.
                if !client_code.trim().is_empty() {
                    let names = engine::extract_server_calls(&client_code);
                    let mut server_results: HashMap<String, serde_json::Value> = HashMap::new();

                    for name in names {
                        lines.push(format!(
                            "→ call_server(\"{}\") → POST /api/plugin/:id/run",
                            name
                        ));
                        set_console.set(lines.clone());

                        let mut ctx = base_ctx();
                        ctx.function = Some(name.clone());
                        ctx.script_override = Some(server_code.clone());
                        match api::run_script(&id, &ctx).await {
                            Ok(body) => {
                                let (result, logs) = split_run_body(&body);
                                for l in logs {
                                    lines.push(format!("   [сервер] {}", l));
                                }
                                lines.push(format!("   ← \"{}\" вернул результат", name));
                                server_results.insert(name, result);
                            }
                            Err(e) => {
                                lines.push(format!("   [сервер][ошибка] {}", e));
                                server_results.insert(name, serde_json::Value::Null);
                            }
                        }
                        set_console.set(lines.clone());
                    }

                    lines.push("→ выполняется client_script (в браузере)".to_string());
                    set_console.set(lines.clone());

                    let ctx_json =
                        serde_json::to_value(base_ctx()).unwrap_or(serde_json::Value::Null);
                    let inputs = vec![("ctx", ctx_json)];
                    match engine::run_transform(&client_code, &inputs, server_results) {
                        Ok((out, logs)) => {
                            for l in logs {
                                lines.push(l);
                            }
                            set_console.set(lines.clone());
                            set_table.set(json_to_table(out));
                        }
                        Err(e) => set_run_error.set(Some(format!("Ошибка client_script: {}", e))),
                    }
                }
                // Случай 2: только server_script — выполняем его тело.
                else if !server_code.trim().is_empty() {
                    lines.push("→ выполняется server_script (на сервере)".to_string());
                    set_console.set(lines.clone());
                    let mut ctx = base_ctx();
                    ctx.script_override = Some(server_code.clone());
                    match api::run_script(&id, &ctx).await {
                        Ok(body) => {
                            let (result, logs) = split_run_body(&body);
                            for l in logs {
                                lines.push(format!("[сервер] {}", l));
                            }
                            set_console.set(lines.clone());
                            set_table.set(json_to_table(result));
                        }
                        Err(e) => set_run_error.set(Some(e)),
                    }
                }
                // Случай 3: декларативный плагин на DataView.
                else if has_view {
                    let ctx = base_ctx();
                    match api::run_data(&id, &ctx).await {
                        Ok(resp) => set_table.set(Some(drilldown_to_table(&resp))),
                        Err(e) => set_run_error.set(Some(e)),
                    }
                } else {
                    set_run_error.set(Some("Нет кода для исполнения".into()));
                }
                set_running.set(false);
            });
        }
    };

    // «Сохранить» — записать отредактированные скрипты в плагин (upsert).
    let save = {
        let plugin_id = plugin_id.clone();
        move |_| {
            let id = plugin_id.clone();
            let Some(d) = def.get_untracked() else {
                return;
            };
            let server_code = server_src.get_untracked();
            let client_code = client_src.get_untracked();
            let mut bundle: PluginBundle = d.bundle.clone();
            bundle.server_script = (!server_code.trim().is_empty()).then_some(server_code);
            bundle.client_script = (!client_code.trim().is_empty()).then_some(client_code);

            let dto = PluginUpsert {
                id: Some(id),
                bundle,
                status: Some(d.status.as_str().to_string()),
                is_enabled: Some(d.is_enabled),
                owner_user_id: None,
                created_by_agent_id: None,
                version: Some(version.get_untracked()),
            };

            set_saving.set(true);
            set_save_msg.set(None);
            spawn_local(async move {
                match api::upsert(&dto).await {
                    Ok(_) => {
                        set_version.update(|v| *v += 1);
                        set_save_msg.set(Some("Сохранено".to_string()));
                    }
                    Err(e) => set_save_msg.set(Some(format!("Ошибка: {}", e))),
                }
                set_saving.set(false);
            });
        }
    };

    let root_class = move || match def.get() {
        Some(d) => format!("plugin-host plugin-{}", d.bundle.manifest.code),
        None => "plugin-host".to_string(),
    };
    let styles = move || {
        def.get()
            .and_then(|d| d.bundle.styles.clone())
            .unwrap_or_default()
    };
    let view_spec_pretty = move || {
        def.get()
            .map(|d| {
                serde_json::to_string_pretty(&d.bundle.view_spec)
                    .unwrap_or_else(|_| "{}".to_string())
            })
            .unwrap_or_default()
    };

    view! {
        <div class=root_class>
            <style>{styles}</style>

            {move || loading.get().then(|| view! {
                <div class="plugin-host__state">"Загрузка плагина…"</div>
            })}
            {move || error.get().map(|e| view! {
                <div class="plugin-host__alert plugin-host__alert--error">{e}</div>
            })}

            // Заголовок
            <div class="plugin-host__header">
                <div class="plugin-host__title-row">
                    <h2 class="plugin-host__title">
                        {move || def.get().map(|d| d.bundle.manifest.title.clone()).unwrap_or_default()}
                    </h2>
                    <span class="plugin-host__chip">
                        {move || def.get().map(|d| d.bundle.manifest.runtime.as_str().to_string()).unwrap_or_default()}
                    </span>
                    <span class="plugin-host__code">
                        {move || def.get().map(|d| d.bundle.manifest.code.clone()).unwrap_or_default()}
                    </span>
                </div>
                {move || def.get()
                    .and_then(|d| d.bundle.manifest.description.clone())
                    .filter(|s| !s.is_empty())
                    .map(|desc| view! { <p class="plugin-host__desc">{desc}</p> })}
            </div>

            // Вкладки
            <div class="plugin-host__tabs">
                <TabList selected_value=selected_tab>
                    <Tab value="run".to_string()>"Выполнение"</Tab>
                    <Tab value="view".to_string()>"Макет"</Tab>
                </TabList>
            </div>

            // ── Вкладка «Выполнение» ─────────────────────────────────────────
            <div
                class="plugin-host__pane"
                class:plugin-host__hidden=move || selected_tab.get() != "run"
            >
                    <div class="plugin-host__editor-block">
                        <div class="plugin-host__editor-label">
                            "server_script — Rhai на сервере (host_read / host_exec / host_query)"
                        </div>
                        <textarea
                            class="plugin-host__editor"
                            rows="9"
                            spellcheck="false"
                            prop:value=move || server_src.get()
                            on:input=move |ev| server_src.set(event_target_value(&ev))
                        />
                    </div>

                    <div class="plugin-host__editor-block">
                        <div class="plugin-host__editor-label">
                            "client_script — Rhai в браузере (call_server(\"имя\"), print)"
                        </div>
                        <textarea
                            class="plugin-host__editor"
                            rows="9"
                            spellcheck="false"
                            prop:value=move || client_src.get()
                            on:input=move |ev| client_src.set(event_target_value(&ev))
                        />
                    </div>

                    <div class="plugin-host__toolbar">
                        <button
                            class="plugin-host__run"
                            on:click=run
                            disabled=Signal::derive(move || running.get())
                        >
                            {move || if running.get() { "Выполняется…" } else { "Выполнить" }}
                        </button>
                        <button
                            class="plugin-host__run plugin-host__run--server"
                            on:click=save
                            disabled=Signal::derive(move || saving.get())
                        >
                            {move || if saving.get() { "Сохранение…" } else { "Сохранить" }}
                        </button>
                        {move || save_msg.get().map(|m| view! {
                            <span class="plugin-host__save-msg">{m}</span>
                        })}
                    </div>

                    {move || run_error.get().map(|e| view! {
                        <div class="plugin-host__alert plugin-host__alert--error">{e}</div>
                    })}

                    // Консоль
                    {move || {
                        let lines = console.get();
                        (!lines.is_empty()).then(|| view! {
                            <div class="plugin-host__console">
                                <div class="plugin-host__console-head">
                                    <span class="plugin-host__console-title">"Консоль"</span>
                                    <button
                                        class="plugin-host__console-clear"
                                        on:click=move |_| set_console.set(Vec::new())
                                    >
                                        "Очистить"
                                    </button>
                                </div>
                                <div class="plugin-host__console-body">
                                    {lines.into_iter().map(|l| view! {
                                        <div class="plugin-host__console-line">{l}</div>
                                    }).collect_view()}
                                </div>
                            </div>
                        })
                    }}

                    // Таблица результата
                    {move || table.get().map(|t| {
                        if t.rows.is_empty() {
                            return view! { <div class="plugin-host__state">"Пусто."</div> }.into_any();
                        }
                        let columns = t.columns.clone();
                        view! {
                            <div class="plugin-host__card">
                                <table class="plugin-host__table">
                                    <thead>
                                        <tr>
                                            {columns.into_iter().map(|c| view! { <th>{c}</th> }).collect_view()}
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {t.rows.into_iter().map(|row| view! {
                                            <tr>
                                                {row.into_iter().map(|cell| view! { <td>{cell}</td> }).collect_view()}
                                            </tr>
                                        }).collect_view()}
                                    </tbody>
                                </table>
                            </div>
                        }.into_any()
                    })}
            </div>

            // ── Вкладка «Макет» ──────────────────────────────────────────────
            <div
                class="plugin-host__pane"
                class:plugin-host__hidden=move || selected_tab.get() != "view"
            >
                <div class="plugin-host__editor-label">"view_spec (макет вывода)"</div>
                <pre class="plugin-host__code-block">{view_spec_pretty}</pre>
            </div>
        </div>
    }
}
