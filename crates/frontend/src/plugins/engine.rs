//! Клиентский Rhai-движок плагинов (исполняется в браузере, WASM).
//!
//! Модель `Client`-runtime: host сначала тянет данные (DataView), затем прогоняет
//! `client_script` как чистую трансформацию — без I/O внутри скрипта. В области
//! видимости доступны переменные `rows` (массив строк) и `ctx` (параметры запуска).
//! Скрипт возвращает значение (обычно массив объектов), которое host рендерит.

use rhai::{Dynamic, Engine, Scope};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

const MAX_OPERATIONS: u64 = 2_000_000;

/// Найти в client_script все вызовы `call_server("имя")` (литеральные имена),
/// чтобы host заранее выполнил соответствующие серверные функции.
pub fn extract_server_calls(script: &str) -> Vec<String> {
    let pat = "call_server(\"";
    let mut names = Vec::new();
    let mut rest = script;
    while let Some(i) = rest.find(pat) {
        rest = &rest[i + pat.len()..];
        if let Some(end) = rest.find('"') {
            let name = rest[..end].to_string();
            if !names.contains(&name) {
                names.push(name);
            }
            rest = &rest[end + 1..];
        } else {
            break;
        }
    }
    names
}

/// Прогнать `client_script`.
///
/// - `inputs` — именованные переменные в области видимости (`rows`, `ctx` …).
/// - `server_results` — карта имя_функции → результат, полученный host'ом заранее
///   через `/api/plugin/:id/run` (синхронный Rhai в браузере не умеет ждать сеть).
///   В скрипте доступна платформенная функция `call_server(name)`, возвращающая
///   `server_results[name]`.
///
/// Возвращает (JSON-результат, строки вывода `print` для консоли).
pub fn run_transform(
    script: &str,
    inputs: &[(&str, serde_json::Value)],
    server_results: HashMap<String, serde_json::Value>,
) -> Result<(serde_json::Value, Vec<String>), String> {
    let mut engine = Engine::new();
    engine.set_max_operations(MAX_OPERATIONS);

    // Захват вывода print/debug в консоль (wasm однопоточный → Rc<RefCell>).
    let logs: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    {
        let logs = logs.clone();
        engine.on_print(move |s| logs.borrow_mut().push(s.to_string()));
    }
    {
        let logs = logs.clone();
        engine.on_debug(move |s, _src, _pos| logs.borrow_mut().push(format!("[debug] {}", s)));
    }

    // call_server(name) — возвращает заранее полученный результат серверной функции.
    let mut server_map: HashMap<String, Dynamic> = HashMap::new();
    for (k, v) in server_results {
        let dyn_v = rhai::serde::to_dynamic(v).map_err(|e| e.to_string())?;
        server_map.insert(k, dyn_v);
    }
    engine.register_fn("call_server", move |name: &str| -> Dynamic {
        server_map.get(name).cloned().unwrap_or(Dynamic::UNIT)
    });

    let mut scope = Scope::new();
    for (name, value) in inputs {
        let dynamic = rhai::serde::to_dynamic(value.clone()).map_err(|e| e.to_string())?;
        scope.push_dynamic(*name, dynamic);
    }

    let out: Dynamic = engine
        .eval_with_scope(&mut scope, script)
        .map_err(|e| e.to_string())?;

    let captured = logs.borrow().clone();
    let result = rhai::serde::from_dynamic(&out).map_err(|e| e.to_string())?;
    Ok((result, captured))
}
