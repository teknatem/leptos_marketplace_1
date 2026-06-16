use crate::shared::config;
use sha2::{Digest, Sha384};
use sqlx::sqlite::SqlitePool;
use sqlx::Row;
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

fn migration_checksum(contents: &[u8]) -> Vec<u8> {
    Sha384::digest(contents).to_vec()
}

fn normalize_line_endings(contents: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let text = String::from_utf8_lossy(contents);
    let lf = text.replace("\r\n", "\n").replace('\r', "\n");
    let crlf = lf.replace('\n', "\r\n");
    (lf.into_bytes(), crlf.into_bytes())
}

fn find_migration_path(migrations_dir: &Path, version: i64) -> anyhow::Result<Option<PathBuf>> {
    let prefix = format!("{version:04}_");
    Ok(std::fs::read_dir(migrations_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(&prefix) && name.ends_with(".sql"))
        }))
}

async fn repair_line_ending_checksums(
    pool: &SqlitePool,
    migrations_dir: &Path,
) -> anyhow::Result<()> {
    let applied = sqlx::query("SELECT version, checksum FROM _sqlx_migrations WHERE success = 1")
        .fetch_all(pool)
        .await?;

    for row in applied {
        let version: i64 = row.try_get("version")?;
        let stored_checksum: Vec<u8> = row.try_get("checksum")?;
        let migration_path = find_migration_path(migrations_dir, version)?;

        let Some(migration_path) = migration_path else {
            continue;
        };

        let contents = std::fs::read(&migration_path)?;
        let current_checksum = migration_checksum(&contents);
        if stored_checksum == current_checksum {
            continue;
        }

        let (lf, crlf) = normalize_line_endings(&contents);
        let is_line_ending_only = stored_checksum == migration_checksum(&lf)
            || stored_checksum == migration_checksum(&crlf);
        if !is_line_ending_only {
            continue;
        }

        sqlx::query("UPDATE _sqlx_migrations SET checksum = ?1 WHERE version = ?2")
            .bind(current_checksum)
            .bind(version)
            .execute(pool)
            .await?;
        tracing::info!(
            "Repaired line-ending-only checksum mismatch for migration {}",
            version
        );
    }

    Ok(())
}

async fn repair_known_legacy_checksums(
    pool: &SqlitePool,
    migrations_dir: &Path,
) -> anyhow::Result<()> {
    const LEGACY_MIGRATION_73_CHECKSUM: &str =
        "292C5C2F13A02F029BF59AAABB290D69A0376DB48BBDE5AFDA8019C53467BEF6C4C2F1115E16B0578DA0E8E118038B0C";

    let Some(stored_checksum) = sqlx::query_scalar::<_, Vec<u8>>(
        "SELECT checksum FROM _sqlx_migrations WHERE version = 73 AND success = 1",
    )
    .fetch_optional(pool)
    .await?
    else {
        return Ok(());
    };

    let stored_hex = stored_checksum
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<String>();
    if stored_hex != LEGACY_MIGRATION_73_CHECKSUM {
        return Ok(());
    }

    let migration_74_applied: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM _sqlx_migrations WHERE version = 74 AND success = 1)",
    )
    .fetch_one(pool)
    .await?;
    if !migration_74_applied || !has_table(pool, "a029_wb_supply").await? {
        return Ok(());
    }

    let Some(migration_path) = find_migration_path(migrations_dir, 73)? else {
        return Ok(());
    };
    let current_checksum = migration_checksum(&std::fs::read(migration_path)?);
    sqlx::query("UPDATE _sqlx_migrations SET checksum = ?1 WHERE version = 73")
        .bind(current_checksum)
        .execute(pool)
        .await?;
    tracing::info!("Repaired known legacy checksum for migration 73");

    Ok(())
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

    repair_line_ending_checksums(&pool, &migrations_dir).await?;
    repair_known_legacy_checksums(&pool, &migrations_dir).await?;

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
