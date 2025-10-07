use once_cell::sync::OnceCell;
use sea_orm::{ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, Statement};

static DB_CONN: OnceCell<DatabaseConnection> = OnceCell::new();

pub async fn initialize_database(db_path: Option<&str>) -> anyhow::Result<()> {
    let db_file = db_path.unwrap_or("target/db/app.db");
    if let Some(parent) = std::path::Path::new(db_file).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let absolute_path = if std::path::Path::new(db_file).is_absolute() {
        std::path::PathBuf::from(db_file)
    } else {
        std::env::current_dir()?.join(db_file)
    };
    // Normalize path separators and ensure proper URL form on Windows
    let normalized = absolute_path.to_string_lossy().replace('\\', "/");
    let needs_leading_slash = !normalized.starts_with('/') && normalized.contains(':');
    let prefix = if needs_leading_slash { "/" } else { "" };
    let db_url = format!("sqlite://{}{}?mode=rwc", prefix, normalized);
    let conn = Database::connect(&db_url).await?;

    // Ensure required tables exist (minimal schema bootstrap)
    // First, check if old table exists and migrate it
    let check_old_table = r#"
        SELECT name FROM sqlite_master
        WHERE type='table'
        AND (name='connection_1c_database' OR name='a001_connection_1c_database');
    "#;
    let existing_table = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            check_old_table.to_string(),
        ))
        .await?;

    if !existing_table.is_empty() {
        let table_name: String = existing_table[0].try_get("", "name").unwrap_or_default();

        // If old name exists, migrate to new name and schema
        if table_name == "connection_1c_database" {
            tracing::info!(
                "Migrating connection_1c_database to a001_connection_1c_database with new schema"
            );

            // Create new table with new schema
            let create_new_table = r#"
                CREATE TABLE a001_connection_1c_database (
                    id TEXT PRIMARY KEY NOT NULL,
                    code TEXT NOT NULL DEFAULT '',
                    description TEXT NOT NULL,
                    comment TEXT,
                    url TEXT NOT NULL,
                    login TEXT NOT NULL,
                    password TEXT NOT NULL,
                    is_primary INTEGER NOT NULL DEFAULT 0,
                    is_deleted INTEGER NOT NULL DEFAULT 0,
                    is_posted INTEGER NOT NULL DEFAULT 0,
                    created_at TEXT,
                    updated_at TEXT,
                    version INTEGER NOT NULL DEFAULT 0
                );
            "#;
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                create_new_table.to_string(),
            ))
            .await?;

            // Migrate data
            let migrate_data = r#"
                INSERT INTO a001_connection_1c_database
                    (id, code, description, comment, url, login, password, is_primary, is_deleted, is_posted, created_at, updated_at, version)
                SELECT
                    id,
                    'CON-' || substr(id, 1, 8) as code,
                    description,
                    comment,
                    url,
                    login,
                    password,
                    is_primary,
                    is_deleted,
                    0 as is_posted,
                    created_at,
                    updated_at,
                    0 as version
                FROM connection_1c_database;
            "#;
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                migrate_data.to_string(),
            ))
            .await?;

            // Drop old table
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "DROP TABLE connection_1c_database;".to_string(),
            ))
            .await?;

            tracing::info!("Migration to a001_connection_1c_database completed successfully");
        } else if table_name == "a001_connection_1c_database" {
            // New table already exists, check schema
            tracing::info!("Table a001_connection_1c_database already exists");
        }
    } else {
        // Create new table with new schema
        tracing::info!("Creating new a001_connection_1c_database table");
        let create_connection_1c_table_sql = r#"
            CREATE TABLE a001_connection_1c_database (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                comment TEXT,
                url TEXT NOT NULL,
                login TEXT NOT NULL,
                password TEXT NOT NULL,
                is_primary INTEGER NOT NULL DEFAULT 0,
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_posted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT,
                version INTEGER NOT NULL DEFAULT 0
            );
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_connection_1c_table_sql.to_string(),
        ))
        .await?;
    }

    // Create a002_organization table if it doesn't exist
    let check_organization_table = r#"
        SELECT name FROM sqlite_master
        WHERE type='table' AND name='a002_organization';
    "#;
    let org_table_exists = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            check_organization_table.to_string(),
        ))
        .await?;

    if org_table_exists.is_empty() {
        tracing::info!("Creating a002_organization table");
        let create_organization_table_sql = r#"
            CREATE TABLE a002_organization (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                comment TEXT,
                full_name TEXT NOT NULL,
                inn TEXT NOT NULL,
                kpp TEXT NOT NULL DEFAULT '',
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_posted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT,
                version INTEGER NOT NULL DEFAULT 0
            );
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_organization_table_sql.to_string(),
        ))
        .await?;
    }

    // a003_counterparty
    let check_counterparty_table = r#"
        SELECT name FROM sqlite_master WHERE type='table' AND name='a003_counterparty';
    "#;
    let counterparty_table_exists = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            check_counterparty_table.to_string(),
        ))
        .await?;

    if counterparty_table_exists.is_empty() {
        tracing::info!("Creating a003_counterparty table");
        let create_counterparty_table_sql = r#"
            CREATE TABLE a003_counterparty (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                comment TEXT,
                is_folder INTEGER NOT NULL DEFAULT 0,
                parent_id TEXT,
                inn TEXT NOT NULL DEFAULT '',
                kpp TEXT NOT NULL DEFAULT '',
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_posted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT,
                version INTEGER NOT NULL DEFAULT 0
            );
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_counterparty_table_sql.to_string(),
        ))
        .await?;
    } else {
        // Ensure inn and kpp columns exist; add if missing
        let pragma = format!("PRAGMA table_info('{}');", "a003_counterparty");
        let cols = conn
            .query_all(Statement::from_string(DatabaseBackend::Sqlite, pragma))
            .await?;
        let mut has_inn = false;
        let mut has_kpp = false;
        for row in cols {
            let name: String = row.try_get("", "name").unwrap_or_default();
            if name == "inn" {
                has_inn = true;
            }
            if name == "kpp" {
                has_kpp = true;
            }
        }
        if !has_inn {
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "ALTER TABLE a003_counterparty ADD COLUMN inn TEXT NOT NULL DEFAULT '';"
                    .to_string(),
            ))
            .await?;
        }
        if !has_kpp {
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "ALTER TABLE a003_counterparty ADD COLUMN kpp TEXT NOT NULL DEFAULT '';"
                    .to_string(),
            ))
            .await?;
        }
    }

    // a004_nomenclature
    let check_nomenclature_table = r#"
        SELECT name FROM sqlite_master WHERE type='table' AND name='a004_nomenclature';
    "#;
    let nomenclature_table_exists = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            check_nomenclature_table.to_string(),
        ))
        .await?;

    if nomenclature_table_exists.is_empty() {
        tracing::info!("Creating a004_nomenclature table");
        let create_nomenclature_table_sql = r#"
            CREATE TABLE a004_nomenclature (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                full_description TEXT NOT NULL DEFAULT '',
                comment TEXT,
                is_folder INTEGER NOT NULL DEFAULT 0,
                parent_id TEXT,
                article TEXT NOT NULL DEFAULT '',
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_posted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT,
                version INTEGER NOT NULL DEFAULT 0
            );
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_nomenclature_table_sql.to_string(),
        ))
        .await?;
    }

    // a005_marketplace
    let check_marketplace_table = r#"
        SELECT name FROM sqlite_master WHERE type='table' AND name='a005_marketplace';
    "#;
    let marketplace_table_exists = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            check_marketplace_table.to_string(),
        ))
        .await?;

    if marketplace_table_exists.is_empty() {
        tracing::info!("Creating a005_marketplace table");
        let create_marketplace_table_sql = r#"
            CREATE TABLE a005_marketplace (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                comment TEXT,
                url TEXT NOT NULL,
                logo_path TEXT,
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_posted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT,
                version INTEGER NOT NULL DEFAULT 0
            );
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_marketplace_table_sql.to_string(),
        ))
        .await?;
    } else {
        // Ensure logo_path column exists; add if missing
        let pragma = format!("PRAGMA table_info('{}');", "a005_marketplace");
        let cols = conn
            .query_all(Statement::from_string(DatabaseBackend::Sqlite, pragma))
            .await?;
        let mut has_logo_path = false;
        for row in cols {
            let name: String = row.try_get("", "name").unwrap_or_default();
            if name == "logo_path" {
                has_logo_path = true;
            }
        }
        if !has_logo_path {
            tracing::info!("Adding logo_path column to a005_marketplace");
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "ALTER TABLE a005_marketplace ADD COLUMN logo_path TEXT;".to_string(),
            ))
            .await?;
        }
    }

    // a006_connection_mp table
    let check_connection_mp = r#"
        SELECT name FROM sqlite_master
        WHERE type='table' AND name='a006_connection_mp';
    "#;
    let connection_mp_exists = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            check_connection_mp.to_string(),
        ))
        .await?;

    if connection_mp_exists.is_empty() {
        tracing::info!("Creating a006_connection_mp table");
        let create_connection_mp_table_sql = r#"
            CREATE TABLE a006_connection_mp (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                comment TEXT,
                marketplace TEXT NOT NULL,
                organization TEXT NOT NULL,
                api_key TEXT NOT NULL,
                supplier_id TEXT,
                application_id TEXT,
                is_used INTEGER NOT NULL DEFAULT 0,
                business_account_id TEXT,
                api_key_stats TEXT,
                test_mode INTEGER NOT NULL DEFAULT 0,
                authorization_type TEXT NOT NULL DEFAULT 'API Key',
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_posted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT,
                version INTEGER NOT NULL DEFAULT 0
            );
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_connection_mp_table_sql.to_string(),
        ))
        .await?;
    }

    DB_CONN
        .set(conn)
        .map_err(|_| anyhow::anyhow!("Failed to set DB_CONN"))?;
    Ok(())
}

pub fn get_connection() -> &'static DatabaseConnection {
    DB_CONN
        .get()
        .expect("Database connection has not been initialized")
}
