//! Host for JavaScript plugin micro-applications.
//!
//! Client code runs as an ES module in a sandboxed iframe. Its `host.invoke()` calls are
//! forwarded through `postMessage` to the authenticated Leptos application and then to the
//! backend `/api/plugin/:id/invoke` endpoint.

use crate::plugins::api;
use crate::plugins::editor::CodeEditor;
use contracts::plugins::{
    PluginBundle, PluginDefinition, PluginHealth, PluginInvokeRequest, PluginRunContext,
    PluginStats, PluginUpsert, PluginValidateReport,
};
use js_sys::{Function, Reflect};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::json;
use std::collections::HashMap;
use thaw::{Tab, TabList};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::HtmlIFrameElement;

struct MessageListenerGuard {
    window: web_sys::Window,
    js_fn: Function,
    _handler: Closure<dyn FnMut(web_sys::MessageEvent)>,
}

impl Drop for MessageListenerGuard {
    fn drop(&mut self) {
        let _ = self
            .window
            .remove_event_listener_with_callback("message", &self.js_fn);
    }
}

const IFRAME_BOOTSTRAP: &str = r#"
const root = document.getElementById("plugin-root");
const pending = new Map();
let currentModule = null;
let currentUrl = null;
let hostContext = {};

const host = Object.freeze({
  get context() { return hostContext; },
  invoke(method, args = {}) {
    const requestId = crypto.randomUUID();
    window.parent.postMessage({
      type: "plugin_invoke",
      instanceId: INSTANCE_ID,
      requestId,
      method,
      args
    }, "*");
    return new Promise((resolve, reject) => {
      pending.set(requestId, { resolve, reject });
    });
  }
});

function showError(error) {
  root.replaceChildren();
  const box = document.createElement("pre");
  box.className = "bootstrap-error";
  box.textContent = error instanceof Error ? `${error.message}\n${error.stack || ""}` : String(error);
  root.append(box);
}

window.addEventListener("message", async event => {
  const message = event.data || {};
  if (message.instanceId !== INSTANCE_ID) return;

  if (message.type === "plugin_invoke_result") {
    const waiter = pending.get(message.requestId);
    if (!waiter) return;
    pending.delete(message.requestId);
    if (message.ok) waiter.resolve(message.result);
    else waiter.reject(new Error(message.error || "Plugin server call failed"));
    return;
  }

  if (message.type !== "plugin_init") return;
  try {
    if (currentModule && typeof currentModule.unmount === "function") {
      await currentModule.unmount();
    }
    if (currentUrl) URL.revokeObjectURL(currentUrl);

    hostContext = message.context || {};
    document.getElementById("plugin-styles").textContent = message.styles || "";
    root.replaceChildren();

    const blob = new Blob([message.clientScript || ""], { type: "text/javascript" });
    currentUrl = URL.createObjectURL(blob);
    currentModule = await import(currentUrl);
    if (typeof currentModule.mount !== "function") {
      throw new Error("client_script must export async function mount(root, host)");
    }
    await currentModule.mount(root, host);
  } catch (error) {
    showError(error);
  }
});

window.parent.postMessage({
  type: "plugin_ready",
  instanceId: INSTANCE_ID
}, "*");
"#;

fn build_srcdoc(instance_id: &str) -> String {
    let instance_json = serde_json::to_string(instance_id).unwrap_or_else(|_| "\"plugin\"".into());
    format!(
        r#"<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <style>
    html, body, #plugin-root {{ min-height: 100%; }}
    body {{ margin: 0; }}
    .bootstrap-error {{
      margin: 16px;
      padding: 14px;
      white-space: pre-wrap;
      color: #b42318;
      background: #fef3f2;
      border: 1px solid #fecdca;
      border-radius: 8px;
    }}
  </style>
  <style id="plugin-styles"></style>
</head>
<body>
  <div id="plugin-root"></div>
  <script type="module">
    const INSTANCE_ID = {instance_json};
    {IFRAME_BOOTSTRAP}
  </script>
</body>
</html>"#
    )
}

fn post_json(iframe: &HtmlIFrameElement, value: serde_json::Value) {
    let Ok(js_value) = serde_wasm_bindgen::to_value(&value) else {
        return;
    };
    if let Some(window) = iframe.content_window() {
        let _ = window.post_message(&js_value, "*");
    }
}

fn string_property(data: &JsValue, name: &str) -> Option<String> {
    Reflect::get(data, &JsValue::from_str(name))
        .ok()
        .and_then(|value| value.as_string())
}

fn commit_selected_sql(
    resources: RwSignal<HashMap<String, String>>,
    selected_name: RwSignal<Option<String>>,
    sql_source: RwSignal<String>,
) {
    if let Some(name) = selected_name.get_untracked() {
        resources.update(|items| {
            items.insert(name, sql_source.get_untracked());
        });
    }
}

fn sorted_resource_names(resources: &HashMap<String, String>) -> Vec<String> {
    let mut names = resources.keys().cloned().collect::<Vec<_>>();
    names.sort();
    names
}

/// Собрать актуальный bundle из редактируемых сигналов (для save / validate / runner).
fn build_current_bundle(
    def: ReadSignal<Option<PluginDefinition>>,
    client_src: RwSignal<String>,
    server_src: RwSignal<String>,
    styles_src: RwSignal<String>,
    sql_resources: RwSignal<HashMap<String, String>>,
    selected_sql_name: RwSignal<Option<String>>,
    sql_src: RwSignal<String>,
) -> Option<PluginBundle> {
    commit_selected_sql(sql_resources, selected_sql_name, sql_src);
    let current = def.get_untracked()?;
    let mut bundle = current.bundle.clone();
    bundle.client_script = Some(client_src.get_untracked());
    bundle.server_script = Some(server_src.get_untracked());
    bundle.styles = Some(styles_src.get_untracked());
    bundle.sql_resources = sql_resources.get_untracked();
    Some(bundle)
}

/// Контекст запуска (период) — общий для client_script (`host.context`) и серверных вызовов.
fn run_context(date_from: &str, date_to: &str) -> PluginRunContext {
    PluginRunContext {
        date_from: (!date_from.trim().is_empty()).then(|| date_from.trim().to_string()),
        date_to: (!date_to.trim().is_empty()).then(|| date_to.trim().to_string()),
        connection_mp_refs: Vec::new(),
        group_by: None,
        params: HashMap::new(),
    }
}

/// Сериализованный контекст для отправки в iframe (`plugin_init.context`).
fn run_context_json(date_from: &str, date_to: &str) -> serde_json::Value {
    serde_json::to_value(run_context(date_from, date_to)).unwrap_or_else(|_| json!({}))
}

/// Подпись и CSS-модификатор бейджа «здоровья» плагина.
fn health_badge(health: PluginHealth) -> (&'static str, &'static str) {
    match health {
        PluginHealth::Ok => ("OK", "ok"),
        PluginHealth::Warn => ("Внимание", "warn"),
        PluginHealth::Crit => ("Критично", "crit"),
        PluginHealth::NoData => ("Нет данных", "nodata"),
    }
}

/// Сформировать человекочитаемый вывод runner'а из полного тела ответа invoke
/// (результат либо ошибка со stage/stack + журнал host.log.*).
fn format_invoke_body(body: &serde_json::Value) -> String {
    let mut out = String::new();

    if body.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
        let result = body.get("result").cloned().unwrap_or(serde_json::Value::Null);
        out.push_str(&format!(
            "✓ Результат:\n{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        ));
    } else {
        if let Some(err) = body.get("error").and_then(|v| v.as_str()) {
            out.push_str(&format!("✗ {err}\n"));
        }
        if let Some(detail) = body.get("error_detail").filter(|d| !d.is_null()) {
            let stage = detail.get("stage").and_then(|v| v.as_str()).unwrap_or("");
            let message = detail.get("message").and_then(|v| v.as_str()).unwrap_or("");
            out.push_str(&format!("\nstage: {stage}\n{message}\n"));
            if let Some(stack) = detail.get("stack").and_then(|v| v.as_str()) {
                out.push_str(&format!("\n{stack}\n"));
            }
        }
        if out.is_empty() {
            out = "Неизвестная ошибка".to_string();
        }
    }

    if let Some(logs) = body.get("logs").and_then(|v| v.as_array()) {
        let lines: Vec<String> = logs
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect();
        if !lines.is_empty() {
            out.push_str(&format!("\n— журнал —\n{}", lines.join("\n")));
        }
    }
    out
}

#[component]
pub fn PluginHost(plugin_id: String) -> impl IntoView {
    let (def, set_def) = signal(None::<PluginDefinition>);
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(None::<String>);
    let client_src = RwSignal::new(String::new());
    let server_src = RwSignal::new(String::new());
    let styles_src = RwSignal::new(String::new());
    let sql_resources = RwSignal::new(HashMap::<String, String>::new());
    let selected_sql_name = RwSignal::new(None::<String>);
    let sql_name_input = RwSignal::new(String::new());
    let sql_src = RwSignal::new(String::new());
    let (version, set_version) = signal(1i32);
    let (saving, set_saving) = signal(false);
    let (save_msg, set_save_msg) = signal(None::<String>);
    let (console, set_console) = signal(Vec::<String>::new());
    let restart_generation = RwSignal::new(0u64);
    let selected_tab = RwSignal::new("app".to_string());

    // Контекст запуска (период) — отдаётся client_script через host.context и серверным вызовам.
    let run_date_from = RwSignal::new(String::new());
    let run_date_to = RwSignal::new(String::new());

    // Серверный runner (вкладка «Сервер»).
    let runner_method = RwSignal::new(String::new());
    let runner_args = RwSignal::new("{}".to_string());
    let runner_output = RwSignal::new(None::<String>);
    let runner_busy = RwSignal::new(false);
    let validate_report = RwSignal::new(None::<PluginValidateReport>);

    // Статистика запусков (вкладка «Статистика»).
    let stats = RwSignal::new(None::<PluginStats>);
    let stats_busy = RwSignal::new(false);
    let stats_error = RwSignal::new(None::<String>);

    // Видимость вкладок зависит от наличия скриптов.
    let has_client = Signal::derive(move || !client_src.get().trim().is_empty());
    let has_server = Signal::derive(move || !server_src.get().trim().is_empty());
    let iframe_element = StoredValue::new_local(None::<HtmlIFrameElement>);
    let instance_base = uuid::Uuid::new_v4().to_string();

    {
        let id = plugin_id.clone();
        spawn_local(async move {
            match api::get_by_id(&id).await {
                Ok(plugin) => {
                    client_src.set(plugin.bundle.client_script.clone().unwrap_or_default());
                    server_src.set(plugin.bundle.server_script.clone().unwrap_or_default());
                    styles_src.set(plugin.bundle.styles.clone().unwrap_or_default());
                    // Server-only плагин не имеет UI — открываем сразу вкладку «Сервер».
                    let client_empty = plugin
                        .bundle
                        .client_script
                        .as_deref()
                        .map(|s| s.trim().is_empty())
                        .unwrap_or(true);
                    let server_present = plugin
                        .bundle
                        .server_script
                        .as_deref()
                        .map(|s| !s.trim().is_empty())
                        .unwrap_or(false);
                    if client_empty && server_present {
                        selected_tab.set("server".to_string());
                    }
                    let resources = plugin.bundle.sql_resources.clone();
                    let first_name = sorted_resource_names(&resources).into_iter().next();
                    let first_sql = first_name
                        .as_ref()
                        .and_then(|name| resources.get(name))
                        .cloned()
                        .unwrap_or_default();
                    sql_resources.set(resources);
                    selected_sql_name.set(first_name.clone());
                    sql_name_input.set(first_name.unwrap_or_default());
                    sql_src.set(first_sql);
                    set_version.set(plugin.version);
                    set_def.set(Some(plugin));
                    restart_generation.update(|value| *value += 1);
                }
                Err(message) => set_error.set(Some(message)),
            }
            set_loading.set(false);
        });
    }

    let listener_plugin_id = plugin_id.clone();
    let listener_instance_base = instance_base.clone();
    let _message_listener = StoredValue::new_local(web_sys::window().map(|window| {
        let handler = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
            let data = event.data();
            let Some(message_type) = string_property(&data, "type") else {
                return;
            };
            let Some(instance_id) = string_property(&data, "instanceId") else {
                return;
            };
            let expected_instance = format!(
                "{}-{}",
                listener_instance_base,
                restart_generation.get_untracked()
            );
            if instance_id != expected_instance {
                return;
            }

            if message_type == "plugin_ready" {
                // Server-only плагин не имеет client_script — iframe оставляем пустым.
                if client_src.get_untracked().trim().is_empty() {
                    return;
                }
                if let Some(iframe) = iframe_element.get_value() {
                    post_json(
                        &iframe,
                        json!({
                            "type": "plugin_init",
                            "instanceId": instance_id,
                            "clientScript": client_src.get_untracked(),
                            "styles": styles_src.get_untracked(),
                            "context": run_context_json(
                                &run_date_from.get_untracked(),
                                &run_date_to.get_untracked(),
                            )
                        }),
                    );
                }
                return;
            }

            if message_type != "plugin_invoke" {
                return;
            }
            let Some(request_id) = string_property(&data, "requestId") else {
                return;
            };
            let Some(method) = string_property(&data, "method") else {
                return;
            };
            let args = Reflect::get(&data, &JsValue::from_str("args"))
                .ok()
                .and_then(|value| serde_wasm_bindgen::from_value(value).ok())
                .unwrap_or(serde_json::Value::Null);
            let id = listener_plugin_id.clone();

            spawn_local(async move {
                let request = PluginInvokeRequest {
                    method,
                    args,
                    context: run_context(
                        &run_date_from.get_untracked(),
                        &run_date_to.get_untracked(),
                    ),
                };
                let response = api::invoke(&id, &request).await;
                let message = match response {
                    Ok(body) => {
                        let logs = body
                            .get("logs")
                            .and_then(|value| value.as_array())
                            .map(|items| {
                                items
                                    .iter()
                                    .filter_map(|value| value.as_str().map(str::to_string))
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default();
                        if !logs.is_empty() {
                            set_console.update(|current| current.extend(logs));
                        }
                        json!({
                            "type": "plugin_invoke_result",
                            "instanceId": instance_id,
                            "requestId": request_id,
                            "ok": true,
                            "result": body.get("result").cloned().unwrap_or(serde_json::Value::Null)
                        })
                    }
                    Err(message) => json!({
                        "type": "plugin_invoke_result",
                        "instanceId": instance_id,
                        "requestId": request_id,
                        "ok": false,
                        "error": message
                    }),
                };
                if let Some(iframe) = iframe_element.get_value() {
                    post_json(&iframe, message);
                }
            });
        }) as Box<dyn FnMut(_)>);

        let js_fn = handler.as_ref().unchecked_ref::<Function>().clone();
        let _ = window.add_event_listener_with_callback("message", &js_fn);
        MessageListenerGuard {
            window,
            js_fn,
            _handler: handler,
        }
    }));

    let save = {
        let id = plugin_id.clone();
        move |_| {
            let Some(current) = def.get_untracked() else {
                return;
            };
            let Some(bundle) = build_current_bundle(
                def,
                client_src,
                server_src,
                styles_src,
                sql_resources,
                selected_sql_name,
                sql_src,
            ) else {
                return;
            };
            let saved_bundle = bundle.clone();
            let dto = PluginUpsert {
                id: Some(id.clone()),
                bundle,
                status: Some(current.status.as_str().to_string()),
                is_enabled: Some(current.is_enabled),
                owner_user_id: None,
                created_by_agent_id: None,
                version: Some(version.get_untracked()),
            };

            set_saving.set(true);
            set_save_msg.set(None);
            spawn_local(async move {
                match api::upsert(&dto).await {
                    Ok(_) => {
                        set_version.update(|value| *value += 1);
                        set_def.update(|value| {
                            if let Some(plugin) = value {
                                plugin.bundle = saved_bundle;
                                plugin.version += 1;
                            }
                        });
                        set_save_msg.set(Some("Сохранено".to_string()));
                        restart_generation.update(|value| *value += 1);
                    }
                    Err(message) => set_save_msg.set(Some(format!("Ошибка: {message}"))),
                }
                set_saving.set(false);
            });
        }
    };

    let restart = move |_| {
        set_console.set(Vec::new());
        restart_generation.update(|value| *value += 1);
    };

    let select_sql = move |event| {
        commit_selected_sql(sql_resources, selected_sql_name, sql_src);
        let name = event_target_value(&event);
        let source = sql_resources
            .with_untracked(|items| items.get(&name).cloned())
            .unwrap_or_default();
        selected_sql_name.set(Some(name.clone()));
        sql_name_input.set(name);
        sql_src.set(source);
    };

    let add_sql = move |_| {
        commit_selected_sql(sql_resources, selected_sql_name, sql_src);
        let name = sql_resources.with_untracked(|items| {
            (1..)
                .map(|index| format!("query{index}"))
                .find(|candidate| !items.contains_key(candidate))
                .unwrap()
        });
        sql_resources.update(|items| {
            items.insert(name.clone(), "SELECT 1 AS value".to_string());
        });
        selected_sql_name.set(Some(name.clone()));
        sql_name_input.set(name);
        sql_src.set("SELECT 1 AS value".to_string());
        set_save_msg.set(None);
    };

    let rename_sql = move |_| {
        let Some(old_name) = selected_sql_name.get_untracked() else {
            return;
        };
        let new_name = sql_name_input.get_untracked().trim().to_string();
        if new_name.is_empty() {
            set_save_msg.set(Some("Имя SQL-ресурса не может быть пустым".to_string()));
            return;
        }
        if new_name != old_name
            && sql_resources.with_untracked(|items| items.contains_key(&new_name))
        {
            set_save_msg.set(Some(format!("SQL-ресурс '{new_name}' уже существует")));
            return;
        }

        commit_selected_sql(sql_resources, selected_sql_name, sql_src);
        if new_name != old_name {
            sql_resources.update(|items| {
                let source = items.remove(&old_name).unwrap_or_default();
                items.insert(new_name.clone(), source);
            });
            selected_sql_name.set(Some(new_name.clone()));
            sql_name_input.set(new_name);
        }
        set_save_msg.set(None);
    };

    let delete_sql = move |_| {
        let Some(name) = selected_sql_name.get_untracked() else {
            return;
        };
        sql_resources.update(|items| {
            items.remove(&name);
        });
        let next_name =
            sql_resources.with_untracked(|items| sorted_resource_names(items).into_iter().next());
        let next_source = next_name
            .as_ref()
            .and_then(|next| sql_resources.with_untracked(|items| items.get(next).cloned()))
            .unwrap_or_default();
        selected_sql_name.set(next_name.clone());
        sql_name_input.set(next_name.unwrap_or_default());
        sql_src.set(next_source);
        set_save_msg.set(None);
    };

    let run_validate = move |_| {
        let Some(bundle) = build_current_bundle(
            def,
            client_src,
            server_src,
            styles_src,
            sql_resources,
            selected_sql_name,
            sql_src,
        ) else {
            return;
        };
        runner_busy.set(true);
        runner_output.set(None);
        spawn_local(async move {
            match api::validate(&bundle).await {
                Ok(report) => {
                    if runner_method.get_untracked().trim().is_empty() {
                        if let Some(first) = report.server_exports.first() {
                            runner_method.set(first.clone());
                        }
                    }
                    validate_report.set(Some(report));
                }
                Err(message) => runner_output.set(Some(format!("Ошибка валидации: {message}"))),
            }
            runner_busy.set(false);
        });
    };

    let invoke_plugin_id = plugin_id.clone();
    let run_invoke = move |_| {
        let method = runner_method.get_untracked().trim().to_string();
        if method.is_empty() {
            runner_output.set(Some("Укажите имя метода".to_string()));
            return;
        }
        let args: serde_json::Value = match serde_json::from_str(&runner_args.get_untracked()) {
            Ok(value) => value,
            Err(error) => {
                runner_output.set(Some(format!("Некорректный JSON аргументов: {error}")));
                return;
            }
        };
        let request = PluginInvokeRequest {
            method,
            args,
            context: run_context(&run_date_from.get_untracked(), &run_date_to.get_untracked()),
        };
        let id = invoke_plugin_id.clone();
        runner_busy.set(true);
        runner_output.set(None);
        spawn_local(async move {
            let text = match api::invoke_raw(&id, &request).await {
                Ok(body) => format_invoke_body(&body),
                Err(message) => format!("Сетевая ошибка: {message}"),
            };
            runner_output.set(Some(text));
            runner_busy.set(false);
        });
    };

    let export_plugin_id = plugin_id.clone();
    let export_plugin = move |_| {
        let url = format!("/api/plugin/{}/export", export_plugin_id);
        let fallback = def
            .get_untracked()
            .map(|d| format!("{}.plugin.zip", d.bundle.manifest.code))
            .unwrap_or_else(|| "plugin.zip".to_string());
        spawn_local(async move {
            if let Err(message) =
                crate::shared::auth_download::download_authenticated_file(&url, &fallback).await
            {
                set_error.set(Some(format!("Экспорт не удался: {message}")));
            }
        });
    };

    let stats_plugin_id = plugin_id.clone();
    let load_stats = Callback::new(move |_: ()| {
        if stats_busy.get_untracked() {
            return;
        }
        let id = stats_plugin_id.clone();
        stats_busy.set(true);
        stats_error.set(None);
        spawn_local(async move {
            match api::get_stats(&id, 7).await {
                Ok(value) => stats.set(Some(value)),
                Err(message) => stats_error.set(Some(message)),
            }
            stats_busy.set(false);
        });
    });

    // Ленивая загрузка при первом открытии вкладки «Статистика».
    Effect::new(move |_| {
        if selected_tab.get() == "stats"
            && stats.get_untracked().is_none()
            && !stats_busy.get_untracked()
        {
            load_stats.run(());
        }
    });

    let iframe_instance_base = instance_base.clone();
    let iframe_srcdoc = move || {
        build_srcdoc(&format!(
            "{}-{}",
            iframe_instance_base,
            restart_generation.get()
        ))
    };

    view! {
        <div class="plugin-host">
            {move || loading.get().then(|| view! {
                <div class="plugin-host__state">"Загрузка плагина..."</div>
            })}
            {move || error.get().map(|message| view! {
                <div class="plugin-host__alert plugin-host__alert--error">{message}</div>
            })}

            <div class="plugin-host__header">
                <div class="plugin-host__title-row">
                    <h2 class="plugin-host__title">
                        {move || def.get().map(|plugin| plugin.bundle.manifest.title).unwrap_or_default()}
                    </h2>
                    <span class="plugin-host__chip">"JavaScript"</span>
                    <span class="plugin-host__code">
                        {move || def.get().map(|plugin| plugin.bundle.manifest.code).unwrap_or_default()}
                    </span>
                    <button
                        class="plugin-host__run plugin-host__run--server plugin-host__export"
                        on:click=export_plugin
                    >
                        "Экспорт .zip"
                    </button>
                </div>
                {move || def.get()
                    .and_then(|plugin| plugin.bundle.manifest.description)
                    .map(|description| view! { <p class="plugin-host__desc">{description}</p> })}
            </div>

            <div class="plugin-host__context">
                <label class="plugin-host__context-field">
                    "Период с"
                    <input
                        type="date"
                        prop:value=move || run_date_from.get()
                        on:input=move |e| run_date_from.set(event_target_value(&e))
                    />
                </label>
                <label class="plugin-host__context-field">
                    "по"
                    <input
                        type="date"
                        prop:value=move || run_date_to.get()
                        on:input=move |e| run_date_to.set(event_target_value(&e))
                    />
                </label>
                <span class="plugin-host__context-hint">
                    "Контекст запуска → host.context и серверные вызовы. Для client_script нажмите «Перезапустить»."
                </span>
            </div>

            <div class="plugin-host__tabs">
                <TabList selected_value=selected_tab>
                    <Tab value="app".to_string()>"Приложение"</Tab>
                    <Tab value="server".to_string()>"Сервер"</Tab>
                    <Tab value="stats".to_string()>"Статистика"</Tab>
                    <Tab value="code".to_string()>"Код"</Tab>
                </TabList>
            </div>

            <div
                class="plugin-host__pane"
                class:plugin-host__hidden=move || selected_tab.get() != "app"
            >
                <div class="plugin-host__toolbar">
                    <button class="plugin-host__run plugin-host__run--server" on:click=restart>
                        "Перезапустить"
                    </button>
                </div>
                {move || (!has_client.get()).then(|| view! {
                    <div class="plugin-host__state">
                        "У плагина нет client_script — откройте вкладку «Сервер» для вызова методов."
                    </div>
                })}
                <iframe
                    class="plugin-host__iframe"
                    sandbox="allow-scripts"
                    srcdoc=iframe_srcdoc
                    on:load=move |event| {
                        let iframe = event
                            .target()
                            .and_then(|target| target.dyn_into::<HtmlIFrameElement>().ok());
                        if let Some(iframe) = iframe {
                            let instance_id = format!(
                                "{}-{}",
                                instance_base,
                                restart_generation.get_untracked()
                            );
                            if !client_src.get_untracked().trim().is_empty() {
                                post_json(
                                    &iframe,
                                    json!({
                                        "type": "plugin_init",
                                        "instanceId": instance_id,
                                        "clientScript": client_src.get_untracked(),
                                        "styles": styles_src.get_untracked(),
                                        "context": run_context_json(
                                            &run_date_from.get_untracked(),
                                            &run_date_to.get_untracked(),
                                        )
                                    }),
                                );
                            }
                            iframe_element.set_value(Some(iframe));
                        }
                    }
                ></iframe>
                {move || {
                    let lines = console.get();
                    (!lines.is_empty()).then(|| view! {
                        <div class="plugin-host__console">
                            <div class="plugin-host__console-head">
                                <span class="plugin-host__console-title">"Серверный журнал"</span>
                                <button
                                    class="plugin-host__console-clear"
                                    on:click=move |_| set_console.set(Vec::new())
                                >
                                    "Очистить"
                                </button>
                            </div>
                            <div class="plugin-host__console-body">
                                {lines.into_iter().map(|line| view! {
                                    <div class="plugin-host__console-line">{line}</div>
                                }).collect_view()}
                            </div>
                        </div>
                    })
                }}
            </div>

            <div
                class="plugin-host__pane"
                class:plugin-host__hidden=move || selected_tab.get() != "server"
            >
                <div class="plugin-host__toolbar">
                    <button
                        class="plugin-host__run plugin-host__run--server"
                        on:click=run_validate
                        disabled=Signal::derive(move || runner_busy.get())
                    >
                        "Проверить (компиляция + экспорты)"
                    </button>
                </div>
                {move || (!has_server.get()).then(|| view! {
                    <div class="plugin-host__state">
                        "У плагина нет server_script. Добавьте его на вкладке «Код»."
                    </div>
                })}
                {move || validate_report.get().map(|report| {
                    let exports = report.server_exports.join(", ");
                    let errors = report.errors.clone();
                    view! {
                        <div class="plugin-host__card plugin-host__runner-report">
                            <div class="plugin-host__runner-line">
                                <span class="plugin-host__runner-key">"Статус: "</span>
                                {if report.ok { "✓ валиден" } else { "✗ ошибки" }}
                            </div>
                            <div class="plugin-host__runner-line">
                                <span class="plugin-host__runner-key">"Экспорты: "</span>
                                {if exports.is_empty() { "—".to_string() } else { exports }}
                            </div>
                            {(!errors.is_empty()).then(|| view! {
                                <div class="plugin-host__runner-errors">
                                    {errors.into_iter().map(|e| view! {
                                        <div class="plugin-host__runner-line">
                                            {format!("[{}] {}", e.stage, e.message)}
                                        </div>
                                    }).collect_view()}
                                </div>
                            })}
                        </div>
                    }
                })}
                <div class="plugin-host__runner-form">
                    <label class="plugin-host__runner-field">
                        "Метод (export server_script)"
                        <input
                            class="plugin-host__resource-name"
                            placeholder="напр. loadReport"
                            prop:value=move || runner_method.get()
                            on:input=move |e| runner_method.set(event_target_value(&e))
                        />
                    </label>
                    <label class="plugin-host__runner-field">
                        "Аргументы (JSON)"
                        <textarea
                            class="plugin-host__runner-args"
                            prop:value=move || runner_args.get()
                            on:input=move |e| runner_args.set(event_target_value(&e))
                        ></textarea>
                    </label>
                    <button
                        class="plugin-host__run"
                        on:click=run_invoke
                        disabled=Signal::derive(move || runner_busy.get())
                    >
                        {move || if runner_busy.get() { "Выполнение..." } else { "Вызвать" }}
                    </button>
                </div>
                {move || runner_output.get().map(|text| view! {
                    <pre class="plugin-host__runner-output">{text}</pre>
                })}
            </div>

            <div
                class="plugin-host__pane"
                class:plugin-host__hidden=move || selected_tab.get() != "stats"
            >
                <div class="plugin-host__toolbar">
                    <button
                        class="plugin-host__run plugin-host__run--server"
                        on:click=move |_| load_stats.run(())
                        disabled=Signal::derive(move || stats_busy.get())
                    >
                        {move || if stats_busy.get() { "Загрузка..." } else { "Обновить (7 дней)" }}
                    </button>
                </div>
                {move || stats_error.get().map(|message| view! {
                    <div class="plugin-host__alert plugin-host__alert--error">{message}</div>
                })}
                {move || stats.get().map(|data| {
                    let s = data.summary;
                    let (label, modifier) = health_badge(s.health);
                    let recent = data.recent;
                    view! {
                        <div class="plugin-host__card plugin-host__stats">
                            <div class="plugin-host__stats-grid">
                                <div class="plugin-host__stat">
                                    <span class="plugin-host__runner-key">"Здоровье"</span>
                                    <span class=format!("plugins-health plugins-health--{modifier}")>{label}</span>
                                </div>
                                <div class="plugin-host__stat">
                                    <span class="plugin-host__runner-key">"Запусков (7д)"</span>{s.total}
                                </div>
                                <div class="plugin-host__stat">
                                    <span class="plugin-host__runner-key">"Доля ошибок"</span>
                                    {format!("{} ({:.0}%)", s.errors, s.error_rate * 100.0)}
                                </div>
                                <div class="plugin-host__stat">
                                    <span class="plugin-host__runner-key">"Таймауты"</span>{s.timeouts}
                                </div>
                                <div class="plugin-host__stat">
                                    <span class="plugin-host__runner-key">"avg / max"</span>
                                    {format!("{} / {} мс", s.avg_ms, s.max_ms)}
                                </div>
                            </div>
                            {(!s.by_stage.is_empty()).then(|| view! {
                                <div class="plugin-host__runner-line">
                                    <span class="plugin-host__runner-key">"Ошибки по стадиям: "</span>
                                    {s.by_stage.into_iter()
                                        .map(|sc| format!("{}×{}", sc.stage, sc.count))
                                        .collect::<Vec<_>>()
                                        .join(", ")}
                                </div>
                            })}
                            {if recent.is_empty() {
                                view! { <div class="plugin-host__state">"Запусков пока нет."</div> }.into_any()
                            } else {
                                view! {
                                    <table class="plugin-host__table">
                                        <thead>
                                            <tr>
                                                <th>"Время"</th>
                                                <th>"Метод"</th>
                                                <th>"Статус"</th>
                                                <th class="plugin-host__num">"мс"</th>
                                                <th class="plugin-host__num">"строк"</th>
                                                <th>"Стадия"</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {recent.into_iter().map(|r| view! {
                                                <tr>
                                                    <td>{r.started_at}</td>
                                                    <td>{r.method}</td>
                                                    <td>{r.status}</td>
                                                    <td class="plugin-host__num">{r.duration_ms}</td>
                                                    <td class="plugin-host__num">
                                                        {r.row_count.map(|v| v.to_string()).unwrap_or_default()}
                                                    </td>
                                                    <td>{r.error_stage.unwrap_or_default()}</td>
                                                </tr>
                                            }).collect_view()}
                                        </tbody>
                                    </table>
                                }.into_any()
                            }}
                        </div>
                    }
                })}
            </div>

            <div
                class="plugin-host__pane"
                class:plugin-host__hidden=move || selected_tab.get() != "code"
            >
                <div class="plugin-host__editor-block">
                    <div class="plugin-host__editor-label">
                        "client_script: ES-модуль в iframe; export async function mount(root, host)"
                    </div>
                    <CodeEditor
                        language="javascript"
                        value=client_src
                        class="plugin-code-editor--large"
                    />
                </div>
                <div class="plugin-host__editor-block">
                    <div class="plugin-host__editor-label">
                        "server_script: ES-модуль QuickJS; экспортированные async-функции"
                    </div>
                    <CodeEditor
                        language="javascript"
                        value=server_src
                        class="plugin-code-editor--large"
                    />
                </div>
                <div class="plugin-host__editor-block">
                    <div class="plugin-host__editor-label">
                        "SQL-ресурсы: host.db.queryResource(name, params)"
                    </div>
                    <div class="plugin-host__resource-toolbar">
                        <select
                            class="plugin-host__resource-select"
                            prop:value=move || selected_sql_name.get().unwrap_or_default()
                            on:change=select_sql
                        >
                            {move || sorted_resource_names(&sql_resources.get())
                                .into_iter()
                                .map(|name| view! {
                                    <option value=name.clone()>{name.clone()}</option>
                                })
                                .collect_view()}
                        </select>
                        <input
                            class="plugin-host__resource-name"
                            placeholder="Имя SQL-ресурса"
                            prop:value=move || sql_name_input.get()
                            on:input=move |event| sql_name_input.set(event_target_value(&event))
                        />
                        <button type="button" class="plugin-host__resource-action" on:click=rename_sql>
                            "Переименовать"
                        </button>
                        <button type="button" class="plugin-host__resource-action" on:click=add_sql>
                            "Добавить"
                        </button>
                        <button
                            type="button"
                            class="plugin-host__resource-action plugin-host__resource-action--danger"
                            on:click=delete_sql
                            disabled=Signal::derive(move || selected_sql_name.get().is_none())
                        >
                            "Удалить"
                        </button>
                    </div>
                    {move || if selected_sql_name.get().is_some() {
                        view! {
                            <CodeEditor
                                language="sql"
                                value=sql_src
                                class="plugin-code-editor--sql"
                            />
                        }.into_any()
                    } else {
                        view! {
                            <div class="plugin-host__state">
                                "SQL-ресурсов пока нет. Нажмите «Добавить»."
                            </div>
                        }.into_any()
                    }}
                </div>
                <div class="plugin-host__editor-block">
                    <div class="plugin-host__editor-label">"styles: CSS внутри iframe"</div>
                    <CodeEditor
                        language="css"
                        value=styles_src
                        class="plugin-code-editor--medium"
                    />
                </div>
                <div class="plugin-host__toolbar">
                    <button
                        class="plugin-host__run"
                        on:click=save
                        disabled=Signal::derive(move || saving.get())
                    >
                        {move || if saving.get() { "Сохранение..." } else { "Сохранить и запустить" }}
                    </button>
                    {move || save_msg.get().map(|message| view! {
                        <span class="plugin-host__save-msg">{message}</span>
                    })}
                </div>
            </div>
        </div>
    }
}
