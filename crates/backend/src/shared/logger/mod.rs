pub mod repository;

use repository::log_event_internal;

/// Логирование события на сервере
///
/// # Примеры
/// ```
/// logger::log("startup", "Сервер запущен");
/// logger::log("api", "Получен запрос к /api/marketplace");
/// ```
pub fn log(category: &str, message: &str) {
    log_event_internal("server", category, message);
}
