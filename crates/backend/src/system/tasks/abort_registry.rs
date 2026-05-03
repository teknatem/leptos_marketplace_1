//! Глобальный реестр `AbortHandle` для принудительного завершения Tokio-задач.
//!
//! Жизненный цикл:
//! 1. При запуске задачи (`worker.rs` или `handlers/tasks.rs`) вызывается `register(session_id, handle)`.
//! 2. При завершении задачи (Ok или Err) вызывается `remove(session_id)`.
//! 3. При нажатии кнопки «Прервать» вызывается `abort(session_id)` — токио-задача получает `JoinError::Cancelled`.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tokio::task::AbortHandle;

static REGISTRY: OnceLock<Mutex<HashMap<String, AbortHandle>>> = OnceLock::new();

fn registry() -> &'static Mutex<HashMap<String, AbortHandle>> {
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Регистрирует `AbortHandle` для сессии.
pub fn register(session_id: &str, handle: AbortHandle) {
    if let Ok(mut map) = registry().lock() {
        map.insert(session_id.to_string(), handle);
    }
}

/// Удаляет запись (вызывать при нормальном завершении задачи).
pub fn remove(session_id: &str) {
    if let Ok(mut map) = registry().lock() {
        map.remove(session_id);
    }
}

/// Прерывает задачу. Возвращает `true`, если запись была найдена и `abort()` вызван.
pub fn abort(session_id: &str) -> bool {
    if let Ok(mut map) = registry().lock() {
        if let Some(handle) = map.remove(session_id) {
            handle.abort();
            return true;
        }
    }
    false
}

/// Список session_id активных задач (для диагностики).
pub fn active_sessions() -> Vec<String> {
    registry()
        .lock()
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default()
}
