use crate::shared::config;
use sha2::{Digest, Sha384};
use sqlx::sqlite::SqlitePool;
use sqlx::{Executor, Row};
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
    ensure_llm_chat_agent_fk_to_a038(&pool).await?;

    tracing::info!("Database migrations applied successfully");
    Ok(())
}

/// Текущий номер (наибольшая успешно применённая версия) миграции БД этого инстанса —
/// для ручной сверки с `PluginManifest.built_for_migration` на странице разработки плагина.
pub async fn current_migration_version() -> anyhow::Result<i64> {
    use sea_orm::{ConnectionTrait, Statement};

    let db = crate::shared::data::db::get_connection();
    let stmt = Statement::from_string(
        sea_orm::DatabaseBackend::Sqlite,
        "SELECT MAX(version) as version FROM _sqlx_migrations WHERE success = 1".to_string(),
    );
    let row = db.query_one(stmt).await?;
    Ok(row
        .and_then(|row| row.try_get::<i64>("", "version").ok())
        .unwrap_or(0))
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

/// Проверяет, ссылается ли FK `<table>.agent_id` на указанную родительскую таблицу.
async fn agent_fk_references(
    pool: &SqlitePool,
    table: &str,
    parent: &str,
) -> anyhow::Result<bool> {
    // PRAGMA не принимает bind-параметры — имя таблицы из кода, не из ввода.
    let rows = sqlx::query(&format!("PRAGMA foreign_key_list({table})"))
        .fetch_all(pool)
        .await?;
    Ok(rows.iter().any(|row| {
        let from: String = row.try_get("from").unwrap_or_default();
        let ref_table: String = row.try_get("table").unwrap_or_default();
        from == "agent_id" && ref_table == parent
    }))
}

/// Идемпотентно перенаправляет внешние ключи `a018_llm_chat.agent_id` и
/// `a019_llm_artifact.agent_id` с ретайр-таблицы `a017_llm_agent` на `a038_llm_connection`.
///
/// Миграция 0165 засеяла a038 из a017 с теми же id, но НЕ перенастроила FK у чата/артефакта.
/// Из-за этого чат против НОВОГО подключения a038 (id отсутствует в a017) падал с
/// `FOREIGN KEY constraint failed`. SQLite не умеет менять FK на месте, а простой DROP
/// родителя с включёнными FK каскадно удалил бы сообщения чата, поэтому пересобираем таблицы
/// при выключенных FK (вне транзакции — внутри неё `PRAGMA foreign_keys` игнорируется).
async fn ensure_llm_chat_agent_fk_to_a038(pool: &SqlitePool) -> anyhow::Result<()> {
    if !has_table(pool, "a038_llm_connection").await? || !has_table(pool, "a018_llm_chat").await? {
        return Ok(());
    }

    let chat_needs_fix = agent_fk_references(pool, "a018_llm_chat", "a017_llm_agent").await?;
    let artifact_needs_fix = has_table(pool, "a019_llm_artifact").await?
        && agent_fk_references(pool, "a019_llm_artifact", "a017_llm_agent").await?;

    if !chat_needs_fix && !artifact_needs_fix {
        return Ok(());
    }

    // Собираем один пакетный скрипт: FK off → транзакция → пересборка → commit → FK on.
    // `PRAGMA foreign_keys=OFF` действует, так как выполняется до BEGIN на этом соединении.
    let mut script = String::from("PRAGMA foreign_keys=OFF;\nBEGIN;\n");

    if chat_needs_fix {
        script.push_str(
            "CREATE TABLE a018_llm_chat_new (\
                id TEXT PRIMARY KEY, code TEXT NOT NULL UNIQUE, description TEXT NOT NULL, \
                comment TEXT, agent_id TEXT NOT NULL, is_deleted INTEGER NOT NULL DEFAULT 0, \
                is_posted INTEGER NOT NULL DEFAULT 0, created_at TEXT, updated_at TEXT, \
                version INTEGER NOT NULL DEFAULT 1, model_name TEXT NOT NULL DEFAULT 'gpt-4o', \
                rating INTEGER, \
                FOREIGN KEY (agent_id) REFERENCES a038_llm_connection(id));\n\
             INSERT INTO a018_llm_chat_new (id, code, description, comment, agent_id, is_deleted, \
                is_posted, created_at, updated_at, version, model_name, rating) \
                SELECT id, code, description, comment, agent_id, is_deleted, is_posted, \
                created_at, updated_at, version, model_name, rating FROM a018_llm_chat;\n\
             DROP TABLE a018_llm_chat;\n\
             ALTER TABLE a018_llm_chat_new RENAME TO a018_llm_chat;\n\
             CREATE INDEX IF NOT EXISTS idx_a018_llm_chat_code ON a018_llm_chat(code);\n\
             CREATE INDEX IF NOT EXISTS idx_a018_llm_chat_agent_id ON a018_llm_chat(agent_id);\n\
             CREATE INDEX IF NOT EXISTS idx_a018_llm_chat_is_deleted ON a018_llm_chat(is_deleted);\n\
             CREATE INDEX IF NOT EXISTS idx_a018_llm_chat_created_at ON a018_llm_chat(created_at);\n\
             CREATE INDEX IF NOT EXISTS idx_a018_llm_chat_model_name ON a018_llm_chat(model_name);\n",
        );
    }

    if artifact_needs_fix {
        script.push_str(
            "CREATE TABLE a019_llm_artifact_new (\
                id TEXT PRIMARY KEY, code TEXT NOT NULL UNIQUE, description TEXT NOT NULL, \
                comment TEXT, chat_id TEXT NOT NULL, agent_id TEXT NOT NULL, \
                artifact_type TEXT NOT NULL DEFAULT 'sql_query', status TEXT NOT NULL DEFAULT 'active', \
                sql_query TEXT NOT NULL, query_params TEXT, visualization_config TEXT, \
                last_executed_at TEXT, execution_count INTEGER NOT NULL DEFAULT 0, \
                is_deleted INTEGER NOT NULL DEFAULT 0, is_posted INTEGER NOT NULL DEFAULT 0, \
                created_at TEXT, updated_at TEXT, version INTEGER NOT NULL DEFAULT 1, \
                FOREIGN KEY (chat_id) REFERENCES a018_llm_chat(id), \
                FOREIGN KEY (agent_id) REFERENCES a038_llm_connection(id));\n\
             INSERT INTO a019_llm_artifact_new (id, code, description, comment, chat_id, agent_id, \
                artifact_type, status, sql_query, query_params, visualization_config, \
                last_executed_at, execution_count, is_deleted, is_posted, created_at, updated_at, version) \
                SELECT id, code, description, comment, chat_id, agent_id, artifact_type, status, \
                sql_query, query_params, visualization_config, last_executed_at, execution_count, \
                is_deleted, is_posted, created_at, updated_at, version FROM a019_llm_artifact;\n\
             DROP TABLE a019_llm_artifact;\n\
             ALTER TABLE a019_llm_artifact_new RENAME TO a019_llm_artifact;\n\
             CREATE INDEX IF NOT EXISTS idx_a019_artifact_code ON a019_llm_artifact(code);\n\
             CREATE INDEX IF NOT EXISTS idx_a019_artifact_chat_id ON a019_llm_artifact(chat_id);\n\
             CREATE INDEX IF NOT EXISTS idx_a019_artifact_agent_id ON a019_llm_artifact(agent_id);\n\
             CREATE INDEX IF NOT EXISTS idx_a019_artifact_type ON a019_llm_artifact(artifact_type);\n\
             CREATE INDEX IF NOT EXISTS idx_a019_artifact_status ON a019_llm_artifact(status);\n\
             CREATE INDEX IF NOT EXISTS idx_a019_artifact_is_deleted ON a019_llm_artifact(is_deleted);\n\
             CREATE INDEX IF NOT EXISTS idx_a019_artifact_created_at ON a019_llm_artifact(created_at);\n",
        );
    }

    script.push_str("COMMIT;\nPRAGMA foreign_keys=ON;\n");

    let mut conn = pool.acquire().await?;
    match (&mut *conn).execute(script.as_str()).await {
        Ok(_) => {
            tracing::info!(
                "Repointed LLM agent_id FK(s) to a038_llm_connection (chat: {}, artifact: {})",
                chat_needs_fix,
                artifact_needs_fix
            );
            Ok(())
        }
        Err(e) => {
            // Откатываем возможную открытую транзакцию и восстанавливаем enforcement FK.
            let _ = (&mut *conn).execute("ROLLBACK;").await;
            let _ = (&mut *conn).execute("PRAGMA foreign_keys=ON;").await;
            Err(anyhow::anyhow!(
                "Failed to repoint LLM agent_id FK to a038_llm_connection: {e}"
            ))
        }
    }
}
