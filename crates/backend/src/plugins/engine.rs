//! Server-side JavaScript runtime for plugins.
//!
//! A plugin server script is an ES module. Exported async functions are invoked with
//! `(args, host)`, where `host.db.query(sql, params)` provides parameterized read access
//! to the application database and `host.log.*` writes to the invocation log.

use contracts::plugins::{
    is_read_only_sql, PluginCapability, PluginDefinition, PluginError, PluginInvokeRequest,
    PluginValidateReport,
};
use rquickjs::{
    prelude::{Async, Func},
    promise::MaybePromise,
    AsyncContext, AsyncRuntime, CatchResultExt, CaughtError, Function, Module, Object, Value,
};
use sea_orm::{DatabaseBackend, FromQueryResult, Statement, Value as DbValue};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const READ_ROW_LIMIT: usize = 5_000;

/// Жёсткий лимит времени исполнения одного вызова плагина.
const EXEC_TIMEOUT: Duration = Duration::from_secs(5);
/// Лимит памяти JS-рантайма (дефолтный аллокатор QuickJS — лимит действует).
const MEMORY_LIMIT_BYTES: usize = 64 * 1024 * 1024;
/// Лимит стека JS-рантайма (защита от бесконечной рекурсии).
const MAX_STACK_SIZE: usize = 1024 * 1024;

/// Построить ошибку плагина из пойманного JS-исключения, сохранив stack.
fn js_error(stage: &str, caught: &CaughtError) -> PluginError {
    match caught {
        CaughtError::Exception(exception) => PluginError::new(
            stage,
            exception.message().unwrap_or_else(|| exception.to_string()),
        )
        .with_stack(exception.stack()),
        other => PluginError::new(stage, other.to_string()),
    }
}

/// Если исполнение было прервано по таймауту — переразметить этап ошибки в `timeout`.
fn relabel_timeout(deadline: Instant, mut error: PluginError) -> PluginError {
    if Instant::now() >= deadline {
        error.stage = "timeout".to_string();
        error.message = format!(
            "Превышен лимит времени исполнения плагина ({} с)",
            EXEC_TIMEOUT.as_secs()
        );
        error.stack = None;
    }
    error
}

/// Создать JS-рантайм с лимитами времени, памяти и стека.
///
/// Возвращает рантайм и дедлайн (для пост-классификации ошибки как `timeout`).
async fn limited_runtime() -> anyhow::Result<(AsyncRuntime, Instant)> {
    let runtime = AsyncRuntime::new()
        .map_err(|error| anyhow::anyhow!("Failed to create JavaScript runtime: {error}"))?;
    let deadline = Instant::now() + EXEC_TIMEOUT;
    runtime.set_memory_limit(MEMORY_LIMIT_BYTES).await;
    runtime.set_max_stack_size(MAX_STACK_SIZE).await;
    runtime
        .set_interrupt_handler(Some(Box::new(move || Instant::now() >= deadline)))
        .await;
    Ok((runtime, deadline))
}

fn db() -> &'static sea_orm::DatabaseConnection {
    crate::shared::data::db::get_connection()
}

fn json_param_to_db(value: serde_json::Value) -> Result<DbValue, String> {
    match value {
        serde_json::Value::Null => Ok(DbValue::String(None)),
        serde_json::Value::Bool(value) => Ok(DbValue::Bool(Some(value))),
        serde_json::Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                Ok(DbValue::BigInt(Some(value)))
            } else if let Some(value) = value.as_u64() {
                Ok(DbValue::BigUnsigned(Some(value)))
            } else if let Some(value) = value.as_f64() {
                Ok(DbValue::Double(Some(value)))
            } else {
                Err("Unsupported numeric SQL parameter".to_string())
            }
        }
        serde_json::Value::String(value) => Ok(DbValue::String(Some(Box::new(value)))),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            Err("SQL parameters must be scalar JSON values".to_string())
        }
    }
}

fn normalize_ident(raw: &str) -> Option<String> {
    let ident = raw
        .trim_matches(|c: char| {
            matches!(
                c,
                '"' | '\'' | '`' | '[' | ']' | '(' | ')' | ',' | ';' | '\n' | '\r' | '\t'
            )
        })
        .split('.')
        .last()
        .unwrap_or(raw)
        .trim()
        .to_ascii_lowercase();
    if ident.is_empty()
        || ident.starts_with("select")
        || ident.starts_with('$')
        || ident
            .chars()
            .any(|c| !(c.is_ascii_alphanumeric() || c == '_'))
    {
        None
    } else {
        Some(ident)
    }
}

fn sql_table_refs(sql: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let tokens: Vec<&str> = sql
        .split(|c: char| c.is_whitespace() || matches!(c, ',' | ';'))
        .filter(|token| !token.trim().is_empty())
        .collect();
    for pair in tokens.windows(2) {
        let head = pair[0].trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '_');
        if matches!(head.to_ascii_uppercase().as_str(), "FROM" | "JOIN") {
            if let Some(table) = normalize_ident(pair[1]) {
                if !refs.contains(&table) {
                    refs.push(table);
                }
            }
        }
    }
    refs
}

fn table_scopes(table: &str) -> Vec<String> {
    let table = table.to_ascii_lowercase();
    let mut scopes = vec![table.clone()];
    let tags: &[&str] =
        if table.starts_with("a017") || table.starts_with("a018") || table.starts_with("a019") {
            &["llm"]
        } else if table.starts_with("a024") || table.starts_with("a025") {
            &["bi", "dashboard"]
        } else if table.starts_with("sys_general_ledger") {
            &["gl", "accounting"]
        } else if table.starts_with("a013") {
            &["ym", "sales"]
        } else if table.starts_with("a002")
            || table.starts_with("a004")
            || table.starts_with("a005")
            || table.starts_with("a006")
        {
            &["ref"]
        } else if table.starts_with("a012")
            || table.starts_with("a015")
            || table.starts_with("a020")
            || table.starts_with("a026")
            || table.starts_with("p9")
        {
            &["wb", "projection"]
        } else {
            &[]
        };
    scopes.extend(tags.iter().map(|tag| tag.to_string()));
    scopes
}

fn capability_allows_table(capabilities: &[PluginCapability], table: &str) -> bool {
    let scopes = table_scopes(table);
    capabilities.iter().any(|capability| match capability {
        PluginCapability::DbReadAll => true,
        PluginCapability::DbRead(scope) => scopes.iter().any(|item| item == scope),
        _ => false,
    })
}

fn enforce_sql_capabilities(sql: &str, capabilities: &[PluginCapability]) -> Result<(), String> {
    let tables = sql_table_refs(sql);
    if tables.is_empty() {
        return Ok(());
    }
    let blocked: Vec<String> = tables
        .into_iter()
        .filter(|table| !capability_allows_table(capabilities, table))
        .collect();
    if blocked.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Plugin manifest capabilities do not allow reading table(s): {}. Add db:read:<table>, db:read:<tag>, or db:read:*.",
            blocked.join(", ")
        ))
    }
}

fn limit_read_sql(sql: &str) -> String {
    let statement = sql.trim().trim_end_matches(';').trim();
    format!("SELECT * FROM ({statement}) AS plugin_limited_result LIMIT {READ_ROW_LIMIT}")
}

async fn read_sql(
    sql: &str,
    params: Vec<serde_json::Value>,
    capabilities: &[PluginCapability],
) -> Result<Vec<serde_json::Value>, String> {
    let trimmed = sql.trim();
    if !is_read_only_sql(trimmed) {
        return Err("host.db.query allows only SELECT/WITH statements".to_string());
    }
    enforce_sql_capabilities(trimmed, capabilities)?;

    let values = params
        .into_iter()
        .map(json_param_to_db)
        .collect::<Result<Vec<_>, _>>()?;
    let limited = limit_read_sql(trimmed);
    let stmt = Statement::from_sql_and_values(DatabaseBackend::Sqlite, limited, values);
    let rows = serde_json::Value::find_by_statement(stmt)
        .all(db())
        .await
        .map_err(|error| format!("SQL error: {error}"))?;

    Ok(rows)
}

async fn host_db_query(
    sql: String,
    params_json: String,
    capabilities_json: String,
) -> rquickjs::Result<String> {
    let params: Vec<serde_json::Value> = serde_json::from_str(&params_json).map_err(|error| {
        rquickjs::Error::new_from_js_message(
            "JSON",
            "SQL parameters",
            format!("Invalid parameter array: {error}"),
        )
    })?;
    let capabilities: Vec<PluginCapability> =
        serde_json::from_str(&capabilities_json).map_err(|error| {
            rquickjs::Error::new_from_js_message(
                "JSON",
                "plugin capabilities",
                format!("Invalid capability array: {error}"),
            )
        })?;
    let rows = read_sql(&sql, params, &capabilities)
        .await
        .map_err(|error| rquickjs::Error::new_into_js_message("database", "JavaScript", error))?;
    serde_json::to_string(&rows).map_err(|error| {
        rquickjs::Error::new_into_js_message("database result", "JSON", error.to_string())
    })
}

const HOST_FACTORY: &str = r#"
(() => ({
    db: Object.freeze({
    query: async (sql, params = []) => {
      const json = await __hostDbQuery(String(sql), JSON.stringify(params), __hostCapabilitiesJson);
      return JSON.parse(json);
    },
    queryResource: async (name, params = []) => {
      const key = String(name);
      const sql = __hostSqlResources[key];
      if (typeof sql !== "string") {
        throw new Error(`SQL resource '${key}' is not defined`);
      }
      const json = await __hostDbQuery(sql, JSON.stringify(params), __hostCapabilitiesJson);
      return JSON.parse(json);
    }
  }),
  log: Object.freeze({
    info: (...values) => __hostLog(values.map(formatLogValue).join(" ")),
    warn: (...values) => __hostLog("[warn] " + values.map(formatLogValue).join(" ")),
    error: (...values) => __hostLog("[error] " + values.map(formatLogValue).join(" "))
  })
}))()

function formatLogValue(value) {
  if (typeof value === "string") return value;
  try { return JSON.stringify(value); } catch (_) { return String(value); }
}
"#;

/// Invoke one exported server function and return its JSON result plus captured log lines.
pub async fn invoke_server_method(
    def: PluginDefinition,
    request: PluginInvokeRequest,
) -> anyhow::Result<(serde_json::Value, Vec<String>)> {
    let script = def
        .bundle
        .server_script
        .ok_or_else(|| anyhow::anyhow!("Plugin has no server_script"))?;
    let sql_resources = def.bundle.sql_resources;
    let capabilities = def.bundle.manifest.parsed_capabilities();
    if request.method.trim().is_empty() {
        return Err(anyhow::anyhow!("Plugin method must not be empty"));
    }

    let method = request.method;
    let args = request.args;
    let context = request.context;
    let logs = Arc::new(Mutex::new(Vec::<String>::new()));
    let logs_for_runtime = logs.clone();

    let (runtime, deadline) = limited_runtime().await?;
    let js_context = AsyncContext::full(&runtime)
        .await
        .map_err(|error| anyhow::anyhow!("Failed to create JavaScript context: {error}"))?;

    let result: Result<serde_json::Value, PluginError> = js_context
        .async_with(async move |ctx| {
            let globals = ctx.globals();
            globals
                .set("__hostDbQuery", Func::from(Async(host_db_query)))
                .catch(&ctx)
                .map_err(|error| js_error("module_eval", &error))?;
            let sql_resources_value = rquickjs_serde::to_value(ctx.clone(), sql_resources)
                .map_err(|error| PluginError::new("module_eval", error.to_string()))?;
            globals
                .set("__hostSqlResources", sql_resources_value)
                .catch(&ctx)
                .map_err(|error| js_error("module_eval", &error))?;
            let capabilities_json = serde_json::to_string(&capabilities)
                .map_err(|error| PluginError::new("module_eval", error.to_string()))?;
            globals
                .set("__hostCapabilitiesJson", capabilities_json)
                .catch(&ctx)
                .map_err(|error| js_error("module_eval", &error))?;

            let log_fn = move |message: String| {
                logs_for_runtime.lock().unwrap().push(message);
            };
            globals
                .set("__hostLog", Func::from(log_fn))
                .catch(&ctx)
                .map_err(|error| js_error("module_eval", &error))?;

            let declared = Module::declare(ctx.clone(), "plugin-server.js", script)
                .catch(&ctx)
                .map_err(|error| js_error("module_eval", &error))?;
            let (module, evaluation) = declared
                .eval()
                .catch(&ctx)
                .map_err(|error| js_error("module_eval", &error))?;
            evaluation
                .into_future::<()>()
                .await
                .catch(&ctx)
                .map_err(|error| js_error("module_eval", &error))?;

            let function: Function = module.get(method.as_str()).catch(&ctx).map_err(|_| {
                PluginError::new(
                    "missing_export",
                    format!("Server method '{method}' is not exported"),
                )
            })?;
            let args_value = rquickjs_serde::to_value(ctx.clone(), args)
                .map_err(|error| PluginError::new("invoke", error.to_string()))?;
            let context_value = rquickjs_serde::to_value(ctx.clone(), context)
                .map_err(|error| PluginError::new("invoke", error.to_string()))?;
            let host: Object = ctx
                .eval(HOST_FACTORY)
                .catch(&ctx)
                .map_err(|error| js_error("invoke", &error))?;
            host.set("context", context_value)
                .catch(&ctx)
                .map_err(|error| js_error("invoke", &error))?;

            let promise: MaybePromise = function
                .call((args_value, host))
                .catch(&ctx)
                .map_err(|error| js_error("invoke", &error))?;
            let value: Value = promise
                .into_future()
                .await
                .catch(&ctx)
                .map_err(|error| js_error("runtime", &error))?;
            rquickjs_serde::from_value(value)
                .map_err(|error| PluginError::new("deserialize", error.to_string()))
        })
        .await;

    runtime.idle().await;
    let captured = logs.lock().unwrap().clone();
    result
        .map(|value| (value, captured))
        .map_err(|error| anyhow::Error::new(relabel_timeout(deadline, error)))
}

/// Скомпилировать ES-модуль и перечислить его экспорты **без вызова** функций.
///
/// `stage_prefix` подставляется в `stage` ошибок (пусто для серверного модуля,
/// `client_` для клиентского), чтобы агент отличал, какой скрипт не собрался.
/// Stub-глобалы (`__hostDbQuery`/`__hostLog`) позволяют исполнить верхний уровень
/// модуля без доступа к БД и журналу; DOM не мокается — обращение к нему на верхнем
/// уровне справедливо считается ошибкой.
async fn compile_module_exports(
    script: &str,
    stage_prefix: &str,
) -> Result<Vec<String>, PluginError> {
    let stage = format!("{stage_prefix}module_eval");
    let (runtime, deadline) = limited_runtime()
        .await
        .map_err(|error| PluginError::new(stage.clone(), error.to_string()))?;
    let js_context = AsyncContext::full(&runtime)
        .await
        .map_err(|error| PluginError::new(stage.clone(), error.to_string()))?;

    let script = script.to_string();
    let stage_for_block = stage.clone();
    let result: Result<Vec<String>, PluginError> = js_context
        .async_with(async move |ctx| {
            let stage = stage_for_block.as_str();
            let globals = ctx.globals();
            globals
                .set(
                    "__hostDbQuery",
                    Func::from(|_: String, _: String| "[]".to_string()),
                )
                .catch(&ctx)
                .map_err(|error| js_error(stage, &error))?;
            globals
                .set("__hostLog", Func::from(|_: String| {}))
                .catch(&ctx)
                .map_err(|error| js_error(stage, &error))?;

            let declared = Module::declare(ctx.clone(), "plugin-module.js", script)
                .catch(&ctx)
                .map_err(|error| js_error(stage, &error))?;
            let (module, evaluation) = declared
                .eval()
                .catch(&ctx)
                .map_err(|error| js_error(stage, &error))?;
            evaluation
                .into_future::<()>()
                .await
                .catch(&ctx)
                .map_err(|error| js_error(stage, &error))?;

            let namespace = module
                .namespace()
                .catch(&ctx)
                .map_err(|error| js_error(stage, &error))?;
            let mut exports = namespace
                .keys::<String>()
                .collect::<rquickjs::Result<Vec<String>>>()
                .catch(&ctx)
                .map_err(|error| js_error(stage, &error))?;
            exports.sort();
            Ok(exports)
        })
        .await;

    runtime.idle().await;
    result.map_err(|error| relabel_timeout(deadline, error))
}

/// Скомпилировать серверный ES-модуль и перечислить экспортированные функции
/// **без вызова** какой-либо из них. Используется `POST /api/plugin/validate`
/// для быстрой петли обратной связи (в т.ч. при доработке плагина из чата).
pub async fn validate_server_script(script: &str) -> PluginValidateReport {
    match compile_module_exports(script, "").await {
        Ok(server_exports) => PluginValidateReport {
            ok: true,
            server_exports,
            ..Default::default()
        },
        Err(error) => PluginValidateReport {
            ok: false,
            errors: vec![error],
            ..Default::default()
        },
    }
}

/// Скомпилировать клиентский ES-модуль (UI iframe) и убедиться, что он экспортирует
/// `mount`. Реального рендера нет (в QuickJS нет DOM) — это статическая проверка
/// контракта для самопроверки агента до передачи плагина пользователю.
pub async fn validate_client_script(script: &str) -> PluginValidateReport {
    match compile_module_exports(script, "client_").await {
        Ok(client_exports) => {
            if client_exports.iter().any(|name| name == "mount") {
                PluginValidateReport {
                    ok: true,
                    client_exports,
                    ..Default::default()
                }
            } else {
                PluginValidateReport {
                    ok: false,
                    errors: vec![PluginError::new(
                        "client_missing_export",
                        "client_script должен экспортировать async function mount(root, host)",
                    )],
                    client_exports,
                    ..Default::default()
                }
            }
        }
        Err(error) => PluginValidateReport {
            ok: false,
            errors: vec![error],
            ..Default::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use contracts::plugins::{
        DataBinding, PluginBundle, PluginManifest, PluginRunContext, PluginRuntime, PluginStatus,
        ViewSpec,
    };

    fn test_plugin(script: &str) -> PluginDefinition {
        PluginDefinition {
            id: "test".to_string(),
            bundle: PluginBundle {
                manifest: PluginManifest {
                    code: "TEST".to_string(),
                    title: "Test".to_string(),
                    runtime: PluginRuntime::Server,
                    api_version: "2".to_string(),
                    description: None,
                    capabilities: vec!["db:read:*".into()],
                },
                params: vec![],
                data: DataBinding::default(),
                client_script: None,
                server_script: Some(script.to_string()),
                view_spec: ViewSpec::default(),
                styles: None,
                sql_resources: Default::default(),
                assets: Default::default(),
            },
            status: PluginStatus::Active,
            is_enabled: true,
            owner_user_id: None,
            created_by_agent_id: None,
            version: 1,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn invokes_async_export_and_captures_log() {
        let def = test_plugin(
            r#"
export async function echo(args, host) {
  host.log.info("echo", args.value);
  return { value: args.value, contextValue: host.context.params.test };
}
"#,
        );
        let request = PluginInvokeRequest {
            method: "echo".to_string(),
            args: serde_json::json!({ "value": 42 }),
            context: PluginRunContext {
                params: [("test".to_string(), "ok".to_string())]
                    .into_iter()
                    .collect(),
                ..Default::default()
            },
        };

        let (result, logs) = invoke_server_method(def, request).await.unwrap();
        assert_eq!(
            result,
            serde_json::json!({ "value": 42, "contextValue": "ok" })
        );
        assert_eq!(logs, vec!["echo 42"]);
    }

    #[tokio::test]
    async fn reports_unknown_sql_resource() {
        let def = test_plugin(
            r#"
export async function run(_args, host) {
  return await host.db.queryResource("missing");
}
"#,
        );
        let request = PluginInvokeRequest {
            method: "run".to_string(),
            args: serde_json::Value::Null,
            context: PluginRunContext::default(),
        };

        let error = invoke_server_method(def, request).await.unwrap_err();
        assert!(error.to_string().contains("SQL resource 'missing'"));
    }

    #[tokio::test]
    async fn times_out_on_infinite_loop() {
        let def = test_plugin("export function spin() { while (true) {} }");
        let request = PluginInvokeRequest {
            method: "spin".to_string(),
            args: serde_json::Value::Null,
            context: PluginRunContext::default(),
        };

        let error = invoke_server_method(def, request).await.unwrap_err();
        let detail = error
            .downcast_ref::<PluginError>()
            .expect("error should carry PluginError");
        assert_eq!(detail.stage, "timeout");
    }

    #[tokio::test]
    async fn invoke_error_has_stage_and_stack() {
        let def = test_plugin(
            r#"
export function boom() {
  throw new Error("kaboom");
}
"#,
        );
        let request = PluginInvokeRequest {
            method: "boom".to_string(),
            args: serde_json::Value::Null,
            context: PluginRunContext::default(),
        };

        let error = invoke_server_method(def, request).await.unwrap_err();
        let detail = error
            .downcast_ref::<PluginError>()
            .expect("error should carry PluginError");
        assert_eq!(detail.stage, "invoke");
        assert!(detail.message.contains("kaboom"));
        assert!(detail.stack.is_some(), "expected a JS stack trace");
    }

    #[tokio::test]
    async fn validate_lists_exports() {
        let report = validate_server_script(
            r#"
export async function alpha(args, host) { return 1; }
export function beta() { return 2; }
"#,
        )
        .await;
        assert!(report.ok, "errors: {:?}", report.errors);
        assert_eq!(report.server_exports, vec!["alpha", "beta"]);
    }

    #[tokio::test]
    async fn validate_reports_syntax_error() {
        let report = validate_server_script("export async function broken( {").await;
        assert!(!report.ok);
        assert_eq!(
            report.errors.first().map(|e| e.stage.as_str()),
            Some("module_eval")
        );
    }

    #[tokio::test]
    async fn validate_client_lists_exports_and_accepts_mount() {
        let report = validate_client_script(
            r#"
export async function mount(root, host) {
  const rows = await host.invoke("load");
  root.textContent = JSON.stringify(rows);
}
export function unmount() {}
"#,
        )
        .await;
        assert!(report.ok, "errors: {:?}", report.errors);
        assert_eq!(report.client_exports, vec!["mount", "unmount"]);
    }

    #[tokio::test]
    async fn validate_client_requires_mount_export() {
        let report = validate_client_script("export function render() {}").await;
        assert!(!report.ok);
        assert_eq!(
            report.errors.first().map(|e| e.stage.as_str()),
            Some("client_missing_export")
        );
    }

    #[tokio::test]
    async fn validate_client_reports_syntax_error_with_prefix() {
        let report = validate_client_script("export async function mount( {").await;
        assert!(!report.ok);
        assert_eq!(
            report.errors.first().map(|e| e.stage.as_str()),
            Some("client_module_eval")
        );
    }

    #[test]
    fn extracts_table_refs_for_capability_checks() {
        assert_eq!(
            sql_table_refs("SELECT * FROM a004_nomenclature n JOIN p900_x x ON 1=1"),
            vec!["a004_nomenclature".to_string(), "p900_x".to_string()]
        );
    }

    #[test]
    fn capability_blocks_unauthorized_tables() {
        let caps = vec![PluginCapability::DbRead("ref".into())];
        assert!(enforce_sql_capabilities("SELECT * FROM a004_nomenclature", &caps).is_ok());
        let error = enforce_sql_capabilities("SELECT * FROM plugin", &caps).unwrap_err();
        assert!(error.contains("plugin"), "got: {error}");
    }

    #[test]
    fn read_sql_is_wrapped_with_hard_limit() {
        assert_eq!(
            limit_read_sql("SELECT 1 AS value;"),
            format!(
                "SELECT * FROM (SELECT 1 AS value) AS plugin_limited_result LIMIT {READ_ROW_LIMIT}"
            )
        );
    }
}
