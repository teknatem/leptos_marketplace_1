use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Инициализация системы трассировки (tracing)
///
/// Логи пишутся в:
/// - stdout (с цветами)
/// - target/logs/backend.log (без цветов)
pub fn initialize() -> anyhow::Result<()> {
    println!("========================================");
    println!("  LOGGING SYSTEM INITIALIZATION");
    println!("========================================\n");

    // Получаем директорию исполняемого файла
    let log_dir = if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let dir = exe_dir.join("logs");
            println!("✓ Log directory (next to exe): {}", dir.display());
            dir
        } else {
            let dir = std::path::Path::new("target").join("logs");
            println!("ℹ Using default log directory: {}", dir.display());
            dir
        }
    } else {
        let dir = std::path::Path::new("target").join("logs");
        println!("ℹ Using default log directory: {}", dir.display());
        dir
    };

    // Создаем директорию для логов
    println!("\nCreating log directory if needed...");
    match std::fs::create_dir_all(&log_dir) {
        Ok(_) => println!("✓ Log directory ready"),
        Err(e) => {
            println!("✗ ERROR: Cannot create log directory: {}", e);
            println!("  Error kind: {:?}", e.kind());
            println!("  Possible causes:");
            println!("  - Insufficient permissions");
            println!("  - Invalid path");
            println!("  - Disk is full or read-only\n");
            println!("========================================\n");
            return Err(anyhow::anyhow!("Cannot create log directory: {}", e));
        }
    }

    let log_file_path = log_dir.join("backend.log");
    println!("✓ Log file path: {}\n", log_file_path.display());

    println!("Opening log file for writing...");
    let log_file = match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)
    {
        Ok(f) => {
            println!("✓ Log file opened successfully");
            f
        }
        Err(e) => {
            println!("✗ ERROR: Cannot open log file: {}", e);
            println!("  Error kind: {:?}", e.kind());
            println!("  Path: {}", log_file_path.display());
            println!("  Possible causes:");
            println!("  - Insufficient permissions");
            println!("  - File is locked by another process");
            println!("  - Directory is read-only\n");
            println!("========================================\n");
            return Err(anyhow::anyhow!("Cannot open log file: {}", e));
        }
    };

    println!("\nInitializing tracing subscriber...");
    let log_level =
        std::env::var("RUST_LOG").unwrap_or_else(|_| "info,sqlx=warn,sea_orm=warn".into());
    println!("✓ Log level: {}", log_level);

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(log_level))
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::sync::Arc::new(log_file))
                .with_ansi(false),
        )
        .init();

    println!("✓ Tracing subscriber initialized");
    println!("========================================\n");

    Ok(())
}
