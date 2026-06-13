//! Серверный Rhai-движок плагинов.
//!
//! Исполняет `server_script` плагина в песочнице (лимиты операций/глубины/строк).
//! Скрипту доступны host-функции с **полным доступом** к данным системы (admin-only):
//! - `host_read(sql)` — выполнить SELECT, вернуть массив строк (карт колонка→значение);
//! - `host_exec(sql)` — выполнить мутацию (INSERT/UPDATE/DELETE), вернуть число изменённых строк;
//! - `host_query(view_id, group_by)` — drilldown по DataView, вернуть массив строк;
//! - `log(msg)` — запись в server-лог.
//!
//! В скрипте доступна переменная `ctx` (карта: date_from/date_to/connection_mp_refs/params).
//! Скрипт возвращает значение, которое сериализуется в JSON и отдаётся клиенту.
//!
//! Async-БД вызывается из синхронного Rhai через `Handle::block_on` внутри `spawn_blocking`.

use contracts::plugins::{PluginDefinition, PluginRunContext};
use rhai::{Dynamic, Engine, Scope};
use sea_orm::{ConnectionTrait, DatabaseBackend, FromQueryResult, Statement};

const MAX_OPERATIONS: u64 = 5_000_000;
const MAX_STRING_SIZE: usize = 2_000_000;
const MAX_ARRAY_SIZE: usize = 200_000;
const READ_ROW_LIMIT: usize = 5_000;

fn db() -> &'static sea_orm::DatabaseConnection {
    crate::shared::data::db::get_connection()
}

/// Выполнить SELECT и вернуть строки как JSON (имена колонок → значения).
async fn read_sql(sql: &str) -> Result<Vec<serde_json::Value>, String> {
    let trimmed = sql.trim();
    let upper = trimmed.to_uppercase();
    if !(upper.starts_with("SELECT") || upper.starts_with("WITH")) {
        return Err("host_read разрешает только SELECT/WITH".to_string());
    }
    let stmt = Statement::from_string(DatabaseBackend::Sqlite, trimmed.to_string());
    let rows = serde_json::Value::find_by_statement(stmt)
        .all(db())
        .await
        .map_err(|e| format!("SQL error: {}", e))?;
    Ok(rows.into_iter().take(READ_ROW_LIMIT).collect())
}

/// Выполнить мутацию, вернуть число затронутых строк.
async fn exec_sql(sql: &str) -> Result<u64, String> {
    let stmt = Statement::from_string(DatabaseBackend::Sqlite, sql.trim().to_string());
    let res = db()
        .execute(stmt)
        .await
        .map_err(|e| format!("SQL error: {}", e))?;
    Ok(res.rows_affected())
}

/// Drilldown по DataView → строки JSON.
async fn query_view(
    view_id: &str,
    group_by: &str,
    ctx: &PluginRunContext,
) -> Result<Vec<serde_json::Value>, String> {
    use crate::data_view::DataViewRegistry;
    use contracts::shared::data_view::ViewContext;

    let view_ctx = ViewContext {
        date_from: ctx.date_from.clone().unwrap_or_default(),
        date_to: ctx.date_to.clone().unwrap_or_default(),
        period2_from: None,
        period2_to: None,
        connection_mp_refs: ctx.connection_mp_refs.clone(),
        params: ctx.params.clone(),
    };
    let registry = DataViewRegistry::new();
    let resp = registry
        .compute_drilldown(view_id, &view_ctx, group_by, &[])
        .await
        .map_err(|e| format!("DataView '{}' error: {}", view_id, e))?;

    Ok(resp
        .rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "group_key": r.group_key,
                "label": r.label,
                "value1": r.value1,
                "value2": r.value2,
                "delta_pct": r.delta_pct,
            })
        })
        .collect())
}

fn json_rows_to_dynamic(rows: Result<Vec<serde_json::Value>, String>) -> Dynamic {
    match rows {
        Ok(list) => rhai::serde::to_dynamic(list).unwrap_or(Dynamic::UNIT),
        Err(e) => {
            let mut map = rhai::Map::new();
            map.insert("error".into(), e.into());
            Dynamic::from_map(map)
        }
    }
}

/// Запустить `server_script` плагина. Возвращает (JSON-результат, строки вывода `print`).
pub async fn run_server_script(
    def: PluginDefinition,
    ctx: PluginRunContext,
) -> anyhow::Result<(serde_json::Value, Vec<String>)> {
    // Инлайн-override (отредактированный код) имеет приоритет над сохранённым.
    let script = ctx
        .script_override
        .clone()
        .or_else(|| def.bundle.server_script.clone())
        .ok_or_else(|| anyhow::anyhow!("У плагина нет server_script"))?;
    let function = ctx.function.clone();

    let handle = tokio::runtime::Handle::current();

    let result: Result<(serde_json::Value, Vec<String>), String> =
        tokio::task::spawn_blocking(move || {
        let mut engine = Engine::new();
        engine.set_max_operations(MAX_OPERATIONS);
        engine.set_max_expr_depths(64, 64);
        engine.set_max_string_size(MAX_STRING_SIZE);
        engine.set_max_array_size(MAX_ARRAY_SIZE);

        // Захват вывода print/debug (sync-фича rhai требует Send+Sync → Arc<Mutex>).
        let logs = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        {
            let logs = logs.clone();
            engine.on_print(move |s| {
                logs.lock().unwrap().push(s.to_string());
            });
        }
        {
            let logs = logs.clone();
            engine.on_debug(move |s, _src, _pos| {
                logs.lock().unwrap().push(format!("[debug] {}", s));
            });
        }

        // host_read(sql) -> Array
        {
            let h = handle.clone();
            engine.register_fn("host_read", move |sql: &str| -> Dynamic {
                json_rows_to_dynamic(h.block_on(read_sql(sql)))
            });
        }
        // host_exec(sql) -> i64
        {
            let h = handle.clone();
            engine.register_fn("host_exec", move |sql: &str| -> i64 {
                h.block_on(exec_sql(sql)).map(|n| n as i64).unwrap_or(-1)
            });
        }
        // host_query(view_id, group_by) -> Array
        {
            let h = handle.clone();
            let ctx_for_query = ctx.clone();
            engine.register_fn(
                "host_query",
                move |view_id: &str, group_by: &str| -> Dynamic {
                    json_rows_to_dynamic(
                        h.block_on(query_view(view_id, group_by, &ctx_for_query)),
                    )
                },
            );
        }
        // Контекст в скрипте: переменная `ctx`.
        let mut scope = Scope::new();
        let ctx_json = serde_json::to_value(&ctx).unwrap_or(serde_json::Value::Null);
        let ctx_dyn = rhai::serde::to_dynamic(ctx_json).unwrap_or(Dynamic::UNIT);
        scope.push_dynamic("ctx", ctx_dyn);

        // Компилируем; либо вызываем именованную функцию, либо исполняем тело.
        let ast = match engine.compile(&script) {
            Ok(a) => a,
            Err(e) => return Err(format!("Ошибка компиляции: {}", e)),
        };
        let eval_result = match &function {
            Some(fname) => engine.call_fn::<Dynamic>(&mut scope, &ast, fname.as_str(), ()),
            None => engine.eval_ast_with_scope::<Dynamic>(&mut scope, &ast),
        };
        let captured = logs.lock().unwrap().clone();

        match eval_result {
            Ok(out) => {
                let json: serde_json::Value =
                    rhai::serde::from_dynamic(&out).unwrap_or(serde_json::Value::Null);
                Ok((json, captured))
            }
            Err(e) => Err(e.to_string()),
        }
    })
    .await
    .map_err(|e| anyhow::anyhow!("Ошибка исполнения (join): {}", e))?;

    result.map_err(|e| anyhow::anyhow!("Ошибка скрипта: {}", e))
}
