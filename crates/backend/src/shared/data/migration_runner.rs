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
    dirs.push(PathBuf::from("../../migrations"));
    dirs.push(PathBuf::from("../../../migrations"));

    dirs
}

async fn has_table(pool: &SqlitePool, table_name: &str) -> anyhow::Result<bool> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(1) FROM sqlite_master WHERE type='table' AND name = ?1",
    )
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

    tracing::info!("Database migrations applied successfully");
    Ok(())
}
