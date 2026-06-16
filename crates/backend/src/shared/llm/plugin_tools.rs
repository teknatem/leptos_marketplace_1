//! Инструменты для агента-разработчика плагинов (`AgentType::PluginAdmin`).
//!
//! Позволяют создавать, читать, валидировать, сохранять и тестировать JS-плагины
//! прямо из чата в рантайме. Единица переноса плагина между экземплярами — `bundle`
//! (ключ идентичности — `manifest.code`); локальное состояние (id/version/status)
//! не входит в переносимый артефакт. Любое сохранение атомарно: сначала полная
//! валидация бандла, затем единичный upsert.

use super::types::ToolDefinition;
use crate::plugins::{engine, service};
use contracts::plugins::{PluginBundle, PluginError, PluginInvokeRequest, PluginUpsert};
use serde_json::{json, Value};

/// Имена инструментов разработчика плагинов (для guard'а в диспетчере).
pub const PLUGIN_TOOL_NAMES: &[&str] = &[
    "plugin_list",
    "plugin_get",
    "plugin_validate",
    "plugin_upsert",
    "plugin_invoke",
];

/// Краткое описание формы bundle для всех инструментов, принимающих его на вход.
fn bundle_schema() -> Value {
    json!({
        "type": "object",
        "description": "Самодостаточный переносимый артефакт плагина. Ключ идентичности — manifest.code.",
        "properties": {
            "manifest": {
                "type": "object",
                "properties": {
                    "code": { "type": "string", "description": "Бизнес-код плагина (стабильный идентификатор переноса)." },
                    "title": { "type": "string" },
                    "runtime": { "type": "string", "enum": ["client", "server", "hybrid"] },
                    "api_version": { "type": "string", "description": "Версия API движка плагинов, обычно \"2\"." },
                    "description": { "type": "string" },
                    "capabilities": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["code", "title", "runtime"]
            },
            "client_script": { "type": "string", "description": "ES-модуль iframe; export async function mount(root, host)." },
            "server_script": { "type": "string", "description": "ES-модуль QuickJS; экспортированные async-функции (args, host)." },
            "sql_resources": {
                "type": "object",
                "description": "Имя → SQL (ТОЛЬКО SELECT/WITH). Доступ: host.db.queryResource(name, params).",
                "additionalProperties": { "type": "string" }
            },
            "styles": { "type": "string", "description": "CSS внутри iframe." }
        },
        "required": ["manifest"]
    })
}

/// Определения инструментов для PluginAdmin.
pub fn plugin_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "plugin_list".into(),
            description: "Список плагинов платформы: id, code, title, runtime, status, enabled, version."
                .into(),
            parameters: json!({ "type": "object", "properties": {} }),
        },
        ToolDefinition {
            name: "plugin_get".into(),
            description: "Получить полное определение плагина по id ИЛИ code. Поле `bundle` — \
                          переносимый артефакт (его и правь), `local` — состояние экземпляра \
                          (id/version/status/is_enabled)."
                .into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "UUID плагина." },
                    "code": { "type": "string", "description": "Бизнес-код (manifest.code). Альтернатива id." }
                }
            }),
        },
        ToolDefinition {
            name: "plugin_validate".into(),
            description: "Проверить bundle БЕЗ сохранения: компиляция серверного модуля, перечень \
                          экспортов, проверка SQL. Возвращает { ok, server_exports, errors:[{stage,message,stack}] }. \
                          Вызывай перед plugin_upsert."
                .into(),
            parameters: json!({
                "type": "object",
                "properties": { "bundle": bundle_schema() },
                "required": ["bundle"]
            }),
        },
        ToolDefinition {
            name: "plugin_upsert".into(),
            description: "Создать или обновить плагин. Если id не задан, идентичность берётся по \
                          manifest.code (idempotent upsert-by-code). Бандл валидируется ПЕРЕД \
                          сохранением — невалидный плагин не сохраняется. Возвращает { ok, id, version, validate }."
                .into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "bundle": bundle_schema(),
                    "id": { "type": "string", "description": "UUID для обновления конкретной записи (необязательно)." },
                    "status": { "type": "string", "enum": ["draft", "active", "disabled"], "description": "По умолчанию draft при создании." },
                    "is_enabled": { "type": "boolean" }
                },
                "required": ["bundle"]
            }),
        },
        ToolDefinition {
            name: "plugin_invoke".into(),
            description: "Запустить экспортированный серверный метод плагина для теста (работает и \
                          для draft/disabled — это dev-вызов). Возвращает { result, logs } либо \
                          { error, error_detail:{ stage, message, stack } }."
                .into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "UUID плагина." },
                    "method": { "type": "string", "description": "Имя экспортированной функции server_script." },
                    "args": { "type": "object", "description": "Аргументы метода (первый параметр функции)." }
                },
                "required": ["id", "method"]
            }),
        },
    ]
}

/// Диспетчер инструментов разработчика плагинов.
pub async fn execute_plugin_tool(name: &str, arguments: &str, agent_id: &str) -> Value {
    let args: Value = serde_json::from_str(arguments).unwrap_or_else(|_| json!({}));
    match name {
        "plugin_list" => plugin_list().await,
        "plugin_get" => plugin_get(&args).await,
        "plugin_validate" => plugin_validate(&args).await,
        "plugin_upsert" => plugin_upsert(&args, agent_id).await,
        "plugin_invoke" => plugin_invoke(&args).await,
        _ => json!({ "error": format!("Unknown plugin tool: '{}'", name) }),
    }
}

async fn plugin_list() -> Value {
    match service::list_all().await {
        Ok(defs) => json!({
            "plugins": defs.iter().map(|d| json!({
                "id": d.id,
                "code": d.bundle.manifest.code,
                "title": d.bundle.manifest.title,
                "runtime": d.bundle.manifest.runtime.as_str(),
                "status": d.status.as_str(),
                "is_enabled": d.is_enabled,
                "version": d.version,
            })).collect::<Vec<_>>()
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn plugin_get(args: &Value) -> Value {
    let found = if let Some(id) = args.get("id").and_then(Value::as_str) {
        service::get_by_id(id).await
    } else if let Some(code) = args.get("code").and_then(Value::as_str) {
        service::get_by_code(code).await
    } else {
        return json!({ "error": "Укажите id или code" });
    };

    match found {
        Ok(Some(def)) => json!({
            "bundle": def.bundle,
            "local": {
                "id": def.id,
                "version": def.version,
                "status": def.status.as_str(),
                "is_enabled": def.is_enabled,
                "created_by_agent_id": def.created_by_agent_id,
                "updated_at": def.updated_at,
            },
            "hint": "Единица переноса — поле `bundle` (ключ идентичности manifest.code). \
                     Локальное состояние в `local` не переносится между экземплярами."
        }),
        Ok(None) => json!({ "error": "Плагин не найден" }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

fn parse_bundle(args: &Value) -> Result<PluginBundle, Value> {
    let raw = args
        .get("bundle")
        .ok_or_else(|| json!({ "error": "Отсутствует поле bundle" }))?;
    serde_json::from_value::<PluginBundle>(raw.clone())
        .map_err(|e| json!({ "error": format!("Некорректный bundle: {e}") }))
}

async fn plugin_validate(args: &Value) -> Value {
    let bundle = match parse_bundle(args) {
        Ok(b) => b,
        Err(e) => return e,
    };
    json!({ "validate": service::validate(&bundle).await })
}

async fn plugin_upsert(args: &Value, agent_id: &str) -> Value {
    let bundle = match parse_bundle(args) {
        Ok(b) => b,
        Err(e) => return e,
    };

    // Атомарность: невалидный плагин не сохраняется.
    let report = service::validate(&bundle).await;
    if !report.ok {
        return json!({ "ok": false, "error": "Валидация не пройдена", "validate": report });
    }

    // Идентичность по code: если id не задан, но плагин с таким code уже есть — обновляем его.
    let mut id = args
        .get("id")
        .and_then(Value::as_str)
        .map(str::to_string);
    let mut version = None;
    if id.is_none() {
        if let Ok(Some(existing)) = service::get_by_code(&bundle.manifest.code).await {
            id = Some(existing.id);
            version = Some(existing.version);
        }
    }

    let dto = PluginUpsert {
        id,
        bundle,
        status: args
            .get("status")
            .and_then(Value::as_str)
            .map(str::to_string),
        is_enabled: args.get("is_enabled").and_then(Value::as_bool),
        owner_user_id: None,
        created_by_agent_id: Some(agent_id.to_string()),
        version,
    };

    match service::upsert(dto).await {
        Ok(saved_id) => {
            let saved_version = service::get_by_id(&saved_id)
                .await
                .ok()
                .flatten()
                .map(|d| d.version);
            json!({
                "ok": true,
                "id": saved_id,
                "version": saved_version,
                "validate": report,
            })
        }
        Err(e) => json!({ "ok": false, "error": e.to_string() }),
    }
}

async fn plugin_invoke(args: &Value) -> Value {
    let Some(id) = args.get("id").and_then(Value::as_str) else {
        return json!({ "error": "Отсутствует id" });
    };
    let Some(method) = args.get("method").and_then(Value::as_str) else {
        return json!({ "error": "Отсутствует method" });
    };

    // Dev-вызов: тестируем плагин даже в статусе draft/disabled (минуя гейт активности).
    let def = match service::get_by_id(id).await {
        Ok(Some(def)) => def,
        Ok(None) => return json!({ "error": "Плагин не найден" }),
        Err(e) => return json!({ "error": e.to_string() }),
    };

    let request = PluginInvokeRequest {
        method: method.to_string(),
        args: args.get("args").cloned().unwrap_or(Value::Null),
        context: Default::default(),
    };

    match engine::invoke_server_method(def, request).await {
        Ok((result, logs)) => json!({ "ok": true, "result": result, "logs": logs }),
        Err(e) => {
            let detail = e.downcast_ref::<PluginError>().cloned();
            json!({ "ok": false, "error": e.to_string(), "error_detail": detail })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Атомарность: невалидный server_script отклоняется до обращения к БД
    /// (валидация раньше upsert), плагин не сохраняется.
    #[tokio::test]
    async fn upsert_rejects_invalid_bundle_before_persist() {
        let args = json!({
            "bundle": {
                "manifest": { "code": "T-BROKEN", "title": "T", "runtime": "server", "api_version": "2" },
                "server_script": "export async function broken( {"
            }
        });
        let result = execute_plugin_tool("plugin_upsert", &args.to_string(), "agent-1").await;
        assert_eq!(result["ok"], json!(false));
        assert_eq!(result["validate"]["ok"], json!(false));
        assert!(result.get("id").is_none(), "битый плагин не должен сохраняться");
    }
}
