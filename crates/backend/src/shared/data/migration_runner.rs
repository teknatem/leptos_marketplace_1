use crate::shared::config;
use sqlx::sqlite::SqlitePool;
use std::path::{Path, PathBuf};

fn build_sqlite_url(path: &Path) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    let needs_leading_slash = !normalized.starts_with('/') && normalized.contains(':');
    let prefix = if needs_leading_slash { "/" } else { "" };
    format!("sqlite://{}{}?mode=rwc", prefix, normalized)
}

fn candidate_migrations_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            dirs.push(exe_dir.join("migrations"));
        }
    }

    dirs.push(PathBuf::from("migrations"));
    dirs.push(PathBuf::from("../migrations"));
    dirs.push(PathBuf::from("../../migrations"));
    dirs.push(PathBuf::from("../../../migrations"));

    dirs
}

async fn has_table(pool: &SqlitePool, table_name: &str) -> anyhow::Result<bool> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(1) FROM sqlite_master WHERE type='table' AND name = ?1")
            .bind(table_name)
            .fetch_one(pool)
            .await?;
    Ok(count > 0)
}

pub async fn run_migrations() -> anyhow::Result<()> {
    let cfg = config::load_config()?;
    let db_path = config::get_database_path(&cfg)?;
    let db_url = build_sqlite_url(&db_path);

    let pool = SqlitePool::connect(&db_url).await?;

    let has_migrations_table = has_table(&pool, "_sqlx_migrations").await?;
    let has_core_table = has_table(&pool, "a001_connection_1c_database").await?;
    if !has_migrations_table && has_core_table {
        tracing::info!(
            "Legacy database detected (business tables exist, _sqlx_migrations absent). Running baseline migration in idempotent mode."
        );
    }

    let migrations_dir = candidate_migrations_dirs()
        .into_iter()
        .find(|p| p.exists() && p.is_dir())
        .ok_or_else(|| anyhow::anyhow!("migrations directory not found"))?;

    tracing::info!("Using migrations directory: {}", migrations_dir.display());

    let migrator = sqlx::migrate::Migrator::new(migrations_dir.as_path()).await?;
    migrator.run(&pool).await?;

    ensure_a015_dealer_price_ut_column(&pool).await?;

    tracing::info!("Database migrations applied successfully");
    Ok(())
}

async fn has_column(pool: &SqlitePool, table: &str, column: &str) -> anyhow::Result<bool> {
    use sqlx::Row;
    // PRAGMA не принимает bind-параметры — имя таблицы подставляем напрямую (оно из кода, не из ввода).
    let rows = sqlx::query(&format!("PRAGMA table_info({table})"))
        .fetch_all(pool)
        .await?;
    Ok(rows.iter().any(|row| {
        row.try_get::<String, _>("name")
            .map(|name| name == column)
            .unwrap_or(false)
    }))
}

/// Идемпотентно гарантирует наличие денормализованного столбца
/// `a015_wb_orders.dealer_price_ut` (зеркало `line_json.$.dealer_price_ut`).
///
/// Через обычную sqlx-миграцию это сделать нельзя: на части баз столбец был заведён
/// вне миграций, и `ALTER TABLE ... ADD COLUMN` там падает с `duplicate column name`,
/// а в SQLite нет `ADD COLUMN IF NOT EXISTS`. Поэтому делаем программно и идемпотентно:
/// добавляем столбец, если его нет; бэкфиллим пустые значения из `line_json`; создаём индекс.
/// Дальше столбец поддерживается при каждом сохранении документа.
async fn ensure_a015_dealer_price_ut_column(pool: &SqlitePool) -> anyhow::Result<()> {
    if !has_table(pool, "a015_wb_orders").await? {
        return Ok(());
    }

    if !has_column(pool, "a015_wb_orders", "dealer_price_ut").await? {
        sqlx::query("ALTER TABLE a015_wb_orders ADD COLUMN dealer_price_ut REAL")
            .execute(pool)
            .await?;
        tracing::info!("Added column a015_wb_orders.dealer_price_ut");
    }

    // Бэкфилл только пустых зеркал — на повторных стартах ничего не находит и стоит дёшево.
    sqlx::query(
        "UPDATE a015_wb_orders \
         SET dealer_price_ut = json_extract(line_json, '$.dealer_price_ut') \
         WHERE dealer_price_ut IS NULL \
           AND json_extract(line_json, '$.dealer_price_ut') IS NOT NULL",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_a015_dealer_price_ut \
         ON a015_wb_orders(dealer_price_ut)",
    )
    .execute(pool)
    .await?;

    Ok(())
}
