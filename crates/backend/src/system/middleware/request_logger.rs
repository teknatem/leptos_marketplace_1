use axum::body::to_bytes;
use axum::body::Body;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use chrono::Utc;

use crate::shared::format::format_number;

/// Middleware для логирования HTTP запросов
///
/// Выводит в консоль:
/// - Timestamp (MSK, UTC+3)
/// - Длительность (ms)
/// - Размер ответа (форматированный)
/// - Статус код
/// - Метод и путь
pub async fn request_logger(req: Request<Body>, next: Next) -> Response {
    let start = std::time::Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();

    let response = next.run(req).await;
    let (parts, body) = response.into_parts();

    // Читаем тело ответа, чтобы узнать реальный размер
    let bytes = match to_bytes(body, usize::MAX).await {
        Ok(b) => b,
        Err(_) => {
            let duration = start.elapsed();
            let timestamp = Utc::now() + chrono::Duration::hours(3);
            // Ошибка - используем коричневый цвет
            println!(
                "\x1b[33m{}\x1b[0m | {:>5}ms | {:>12} | {} {:>6} {}",
                timestamp.format("%H:%M:%S"),
                duration.as_millis(),
                "error",
                parts.status.as_u16(),
                method,
                uri.path()
            );
            return Response::from_parts(parts, Body::default());
        }
    };

    let size = bytes.len();
    let duration = start.elapsed();
    let timestamp = Utc::now() + chrono::Duration::hours(3);

    // Выбираем цвет для времени: голубой для 200, коричневый для остальных
    let color_code = if parts.status.as_u16() == 200 {
        "36"
    } else {
        "33"
    };

    println!(
        "\x1b[{}m{}\x1b[0m | {:>5}ms | {:>12} | {} {:>6} {}",
        color_code,
        timestamp.format("%H:%M:%S"),
        duration.as_millis(),
        format_number(size),
        parts.status.as_u16(),
        method,
        uri.path()
    );

    // Создаем новый ответ с прочитанным телом
    Response::from_parts(parts, Body::from(bytes))
}
