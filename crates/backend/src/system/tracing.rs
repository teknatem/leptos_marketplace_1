use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Инициализация системы трассировки (tracing)
///
/// Логи пишутся в:
/// - stdout (с цветами)
/// - target/logs/backend.log (без цветов)
pub fn initialize() -> anyhow::Result<()> {
    // Создаем директорию для логов
    let log_dir = std::path::Path::new("target").join("logs");
    std::fs::create_dir_all(&log_dir)?;

    let log_file_path = log_dir.join("backend.log");
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file_path)?;

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| {
                // Отключаем логи SQL запросов, но оставляем логи приложения
                "info,sqlx=warn,sea_orm=warn".into()
            }),
        ))
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::sync::Arc::new(log_file))
                .with_ansi(false),
        )
        .init();

    Ok(())
}
