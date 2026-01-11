use once_cell::sync::OnceCell;
use sea_orm::{ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, Statement};
use crate::shared::config;

static DB_CONN: OnceCell<DatabaseConnection> = OnceCell::new();

pub async fn initialize_database() -> anyhow::Result<()> {
    // Load configuration from config.toml
    let cfg = config::load_config()?;
    
    // Get database path from configuration
    let absolute_path = config::get_database_path(&cfg)?;
    
    tracing::info!("Database path: {}", absolute_path.display());
    
    // Create parent directory if it doesn't exist
    if let Some(parent) = absolute_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    // Normalize path separators and ensure proper URL form on Windows
    let normalized = absolute_path.to_string_lossy().replace('\\', "/");
    let needs_leading_slash = !normalized.starts_with('/') && normalized.contains(':');
    let prefix = if needs_leading_slash { "/" } else { "" };
    let db_url = format!("sqlite://{}{}?mode=rwc", prefix, normalized);
    
    tracing::info!("Connecting to database: {}", db_url);
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
                mp_ref_count INTEGER NOT NULL DEFAULT 0,
                dim1_category TEXT NOT NULL DEFAULT '',
                dim2_line TEXT NOT NULL DEFAULT '',
                dim3_model TEXT NOT NULL DEFAULT '',
                dim4_format TEXT NOT NULL DEFAULT '',
                dim5_sink TEXT NOT NULL DEFAULT '',
                dim6_size TEXT NOT NULL DEFAULT '',
                is_assembly INTEGER NOT NULL DEFAULT 0,
                base_nomenclature_ref TEXT,
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
    } else {
        // Ensure columns exist; add if missing
        let pragma = format!("PRAGMA table_info('{}');", "a004_nomenclature");
        let cols = conn
            .query_all(Statement::from_string(DatabaseBackend::Sqlite, pragma))
            .await?;

        let mut has_mp_ref_count = false;
        let mut has_dim1_category = false;
        let mut has_dim2_line = false;
        let mut has_dim3_model = false;
        let mut has_dim4_format = false;
        let mut has_dim5_sink = false;
        let mut has_dim6_size = false;
        let mut has_is_assembly = false;
        let mut has_base_nomenclature_ref = false;

        for row in cols {
            let name: String = row.try_get("", "name").unwrap_or_default();
            match name.as_str() {
                "mp_ref_count" => has_mp_ref_count = true,
                "dim1_category" => has_dim1_category = true,
                "dim2_line" => has_dim2_line = true,
                "dim3_model" => has_dim3_model = true,
                "dim4_format" => has_dim4_format = true,
                "dim5_sink" => has_dim5_sink = true,
                "dim6_size" => has_dim6_size = true,
                "is_assembly" => has_is_assembly = true,
                "base_nomenclature_ref" => has_base_nomenclature_ref = true,
                _ => {}
            }
        }

        if !has_mp_ref_count {
            tracing::info!("Adding mp_ref_count column to a004_nomenclature");
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "ALTER TABLE a004_nomenclature ADD COLUMN mp_ref_count INTEGER NOT NULL DEFAULT 0;"
                    .to_string(),
            ))
            .await?;
        }

        if !has_dim1_category {
            tracing::info!("Adding dim1_category column to a004_nomenclature");
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "ALTER TABLE a004_nomenclature ADD COLUMN dim1_category TEXT NOT NULL DEFAULT '';"
                    .to_string(),
            ))
            .await?;
        }

        if !has_dim2_line {
            tracing::info!("Adding dim2_line column to a004_nomenclature");
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "ALTER TABLE a004_nomenclature ADD COLUMN dim2_line TEXT NOT NULL DEFAULT '';"
                    .to_string(),
            ))
            .await?;
        }

        if !has_dim3_model {
            tracing::info!("Adding dim3_model column to a004_nomenclature");
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "ALTER TABLE a004_nomenclature ADD COLUMN dim3_model TEXT NOT NULL DEFAULT '';"
                    .to_string(),
            ))
            .await?;
        }

        if !has_dim4_format {
            tracing::info!("Adding dim4_format column to a004_nomenclature");
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "ALTER TABLE a004_nomenclature ADD COLUMN dim4_format TEXT NOT NULL DEFAULT '';"
                    .to_string(),
            ))
            .await?;
        }

        if !has_dim5_sink {
            tracing::info!("Adding dim5_sink column to a004_nomenclature");
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "ALTER TABLE a004_nomenclature ADD COLUMN dim5_sink TEXT NOT NULL DEFAULT '';"
                    .to_string(),
            ))
            .await?;
        }

        if !has_dim6_size {
            tracing::info!("Adding dim6_size column to a004_nomenclature");
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "ALTER TABLE a004_nomenclature ADD COLUMN dim6_size TEXT NOT NULL DEFAULT '';"
                    .to_string(),
            ))
            .await?;
        }

        if !has_is_assembly {
            tracing::info!("Adding is_assembly column to a004_nomenclature");
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "ALTER TABLE a004_nomenclature ADD COLUMN is_assembly INTEGER NOT NULL DEFAULT 0;"
                    .to_string(),
            ))
            .await?;
        }

        if !has_base_nomenclature_ref {
            tracing::info!("Adding base_nomenclature_ref column to a004_nomenclature");
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "ALTER TABLE a004_nomenclature ADD COLUMN base_nomenclature_ref TEXT;".to_string(),
            ))
            .await?;
        }
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
                marketplace_type TEXT,
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
        // Ensure logo_path and marketplace_type columns exist; add if missing
        let pragma = format!("PRAGMA table_info('{}');", "a005_marketplace");
        let cols = conn
            .query_all(Statement::from_string(DatabaseBackend::Sqlite, pragma))
            .await?;
        let mut has_logo_path = false;
        let mut has_marketplace_type = false;
        for row in cols {
            let name: String = row.try_get("", "name").unwrap_or_default();
            if name == "logo_path" {
                has_logo_path = true;
            }
            if name == "marketplace_type" {
                has_marketplace_type = true;
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
        if !has_marketplace_type {
            tracing::info!("Adding marketplace_type column to a005_marketplace");
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "ALTER TABLE a005_marketplace ADD COLUMN marketplace_type TEXT;".to_string(),
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

    // a007_marketplace_product table
    let check_marketplace_product = r#"
        SELECT name FROM sqlite_master
        WHERE type='table' AND name='a007_marketplace_product';
    "#;
    let marketplace_product_exists = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            check_marketplace_product.to_string(),
        ))
        .await?;

    if marketplace_product_exists.is_empty() {
        tracing::info!("Creating a007_marketplace_product table");
        let create_marketplace_product_table_sql = r#"
            CREATE TABLE a007_marketplace_product (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                comment TEXT,
                marketplace_ref TEXT NOT NULL,
                connection_mp_ref TEXT NOT NULL DEFAULT '',
                marketplace_sku TEXT NOT NULL,
                barcode TEXT,
                article TEXT NOT NULL,
                brand TEXT,
                category_id TEXT,
                category_name TEXT,
                last_update TEXT,
                nomenclature_ref TEXT,
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_posted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT,
                version INTEGER NOT NULL DEFAULT 0
            );
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_marketplace_product_table_sql.to_string(),
        ))
        .await?;
    } else {
        // Ensure connection_mp_ref column exists; add if missing
        let pragma = format!("PRAGMA table_info('{}');", "a007_marketplace_product");
        let cols = conn
            .query_all(Statement::from_string(DatabaseBackend::Sqlite, pragma))
            .await?;
        let mut has_connection_mp_ref = false;
        for row in cols {
            let name: String = row.try_get("", "name").unwrap_or_default();
            if name == "connection_mp_ref" {
                has_connection_mp_ref = true;
            }
        }
        if !has_connection_mp_ref {
            tracing::info!("Adding connection_mp_ref column to a007_marketplace_product");
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "ALTER TABLE a007_marketplace_product ADD COLUMN connection_mp_ref TEXT NOT NULL DEFAULT '';".to_string(),
            ))
            .await?;
            // Delete existing records as they are test data
            tracing::info!("Deleting existing records from a007_marketplace_product");
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "DELETE FROM a007_marketplace_product;".to_string(),
            ))
            .await?;
        }
    }

    // a008_marketplace_sales table
    let check_marketplace_sales = r#"
        SELECT name FROM sqlite_master
        WHERE type='table' AND name='a008_marketplace_sales';
    "#;
    let marketplace_sales_exists = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            check_marketplace_sales.to_string(),
        ))
        .await?;

    if marketplace_sales_exists.is_empty() {
        tracing::info!("Creating a008_marketplace_sales table");
        let create_marketplace_sales_table_sql = r#"
            CREATE TABLE a008_marketplace_sales (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                comment TEXT,
                connection_id TEXT NOT NULL,
                organization_id TEXT NOT NULL,
                marketplace_id TEXT NOT NULL,
                accrual_date TEXT NOT NULL,
                product_id TEXT NOT NULL,
                quantity INTEGER NOT NULL,
                revenue REAL NOT NULL,
                operation_type TEXT NOT NULL DEFAULT '',
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_posted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT,
                version INTEGER NOT NULL DEFAULT 0
            );
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_marketplace_sales_table_sql.to_string(),
        ))
        .await?;

        // Unique index on (connection_id, product_id, accrual_date, operation_type)
        let create_idx_sql = r#"
            CREATE UNIQUE INDEX IF NOT EXISTS idx_a008_sales_unique
            ON a008_marketplace_sales (connection_id, product_id, accrual_date, operation_type);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_idx_sql.to_string(),
        ))
        .await?;
    } else {
        // Ensure operation_type column and new unique index exist
        let pragma = format!("PRAGMA table_info('{}');", "a008_marketplace_sales");
        let cols = conn
            .query_all(Statement::from_string(DatabaseBackend::Sqlite, pragma))
            .await?;
        let mut has_operation_type = false;
        for row in cols {
            let name: String = row.try_get("", "name").unwrap_or_default();
            if name == "operation_type" {
                has_operation_type = true;
                break;
            }
        }
        if !has_operation_type {
            tracing::info!("Adding operation_type column to a008_marketplace_sales");
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "ALTER TABLE a008_marketplace_sales ADD COLUMN operation_type TEXT NOT NULL DEFAULT '';".to_string(),
            ))
            .await?;
        }

        // Recreate unique index with operation_type if the old one exists
        let drop_old_idx = r#"
            DROP INDEX IF EXISTS idx_a008_sales_unique;
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            drop_old_idx.to_string(),
        ))
        .await?;
        let create_new_idx = r#"
            CREATE UNIQUE INDEX IF NOT EXISTS idx_a008_sales_unique
            ON a008_marketplace_sales (connection_id, product_id, accrual_date, operation_type);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_new_idx.to_string(),
        ))
        .await?;
    }

    // a009_ozon_returns table
    let check_ozon_returns = r#"
        SELECT name FROM sqlite_master
        WHERE type='table' AND name='a009_ozon_returns';
    "#;
    let ozon_returns_exists = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            check_ozon_returns.to_string(),
        ))
        .await?;

    if ozon_returns_exists.is_empty() {
        tracing::info!("Creating a009_ozon_returns table");
        let create_ozon_returns_table_sql = r#"
            CREATE TABLE a009_ozon_returns (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                comment TEXT,
                connection_id TEXT NOT NULL,
                organization_id TEXT NOT NULL,
                marketplace_id TEXT NOT NULL,
                return_id TEXT NOT NULL,
                return_date TEXT NOT NULL,
                return_reason_name TEXT NOT NULL,
                return_type TEXT NOT NULL,
                order_id TEXT NOT NULL,
                order_number TEXT NOT NULL,
                sku TEXT NOT NULL,
                product_name TEXT NOT NULL,
                price REAL NOT NULL,
                quantity INTEGER NOT NULL,
                posting_number TEXT NOT NULL,
                clearing_id TEXT,
                return_clearing_id TEXT,
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_posted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT,
                version INTEGER NOT NULL DEFAULT 0
            );
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_ozon_returns_table_sql.to_string(),
        ))
        .await?;

        // Unique index on (connection_id, return_id, sku) to prevent duplicates
        let create_idx_sql = r#"
            CREATE UNIQUE INDEX IF NOT EXISTS idx_a009_returns_unique
            ON a009_ozon_returns (connection_id, return_id, sku);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_idx_sql.to_string(),
        ))
        .await?;
    }

    // system_log table
    let check_system_log = r#"
        SELECT name FROM sqlite_master
        WHERE type='table' AND name='system_log';
    "#;
    let system_log_exists = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            check_system_log.to_string(),
        ))
        .await?;

    if system_log_exists.is_empty() {
        tracing::info!("Creating system_log table");
        let create_system_log_table_sql = r#"
            CREATE TABLE system_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                source TEXT NOT NULL,
                category TEXT NOT NULL,
                message TEXT NOT NULL
            );
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_system_log_table_sql.to_string(),
        ))
        .await?;
    }

    // document_raw_storage table - РґР»СЏ С…СЂР°РЅРµРЅРёСЏ СЃС‹СЂС‹С… JSON РѕС‚ РјР°СЂРєРµС‚РїР»РµР№СЃРѕРІ
    let check_raw_storage = r#"
        SELECT name FROM sqlite_master
        WHERE type='table' AND name='document_raw_storage';
    "#;
    let raw_storage_exists = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            check_raw_storage.to_string(),
        ))
        .await?;

    if raw_storage_exists.is_empty() {
        tracing::info!("Creating document_raw_storage table");
        let create_raw_storage_table_sql = r#"
            CREATE TABLE document_raw_storage (
                id TEXT PRIMARY KEY NOT NULL,
                marketplace TEXT NOT NULL,
                document_type TEXT NOT NULL,
                document_no TEXT NOT NULL,
                raw_json TEXT NOT NULL,
                fetched_at TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_raw_storage_table_sql.to_string(),
        ))
        .await?;

        // РЎРѕР·РґР°С‚СЊ РёРЅРґРµРєСЃ РґР»СЏ Р±С‹СЃС‚СЂРѕРіРѕ РїРѕРёСЃРєР° РїРѕ marketplace + document_type + document_no
        let create_raw_storage_idx = r#"
            CREATE INDEX IF NOT EXISTS idx_raw_storage_lookup
            ON document_raw_storage (marketplace, document_type, document_no);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_raw_storage_idx.to_string(),
        ))
        .await?;
    }

    // p900_sales_register table - СѓРЅРёС„РёС†РёСЂРѕРІР°РЅРЅС‹Р№ СЂРµРіРёСЃС‚СЂ РїСЂРѕРґР°Р¶
    let check_sales_register = r#"
        SELECT name FROM sqlite_master
        WHERE type='table' AND name='p900_sales_register';
    "#;
    let sales_register_exists = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            check_sales_register.to_string(),
        ))
        .await?;

    if sales_register_exists.is_empty() {
        tracing::info!("Creating p900_sales_register table");
        let create_sales_register_table_sql = r#"
            CREATE TABLE p900_sales_register (
                -- NK (Natural Key)
                marketplace TEXT NOT NULL,
                document_no TEXT NOT NULL,
                line_id TEXT NOT NULL,

                -- Metadata
                scheme TEXT,
                document_type TEXT NOT NULL,
                document_version INTEGER NOT NULL DEFAULT 1,

                -- References to aggregates (UUID)
                connection_mp_ref TEXT NOT NULL,
                organization_ref TEXT NOT NULL,
                marketplace_product_ref TEXT,
                nomenclature_ref TEXT,
                registrator_ref TEXT NOT NULL,
                
                -- Timestamps and status
                event_time_source TEXT NOT NULL,
                sale_date TEXT NOT NULL,
                source_updated_at TEXT,
                status_source TEXT NOT NULL,
                status_norm TEXT NOT NULL,
                
                -- Product identification
                seller_sku TEXT,
                mp_item_id TEXT NOT NULL,
                barcode TEXT,
                title TEXT,
                
                -- Quantities and money
                qty REAL NOT NULL,
                price_list REAL,
                discount_total REAL,
                price_effective REAL,
                amount_line REAL,
                currency_code TEXT,
                
                -- Technical fields
                loaded_at_utc TEXT NOT NULL,
                payload_version INTEGER NOT NULL DEFAULT 1,
                extra TEXT,
                
                PRIMARY KEY (marketplace, document_no, line_id)
            );
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_sales_register_table_sql.to_string(),
        ))
        .await?;

        // РЎРѕР·РґР°С‚СЊ РёРЅРґРµРєСЃС‹ РґР»СЏ Р±С‹СЃС‚СЂРѕРіРѕ РїРѕРёСЃРєР°
        let create_register_idx1 = r#"
            CREATE INDEX IF NOT EXISTS idx_sales_register_sale_date
            ON p900_sales_register (sale_date);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_register_idx1.to_string(),
        ))
        .await?;

        let create_register_idx2 = r#"
            CREATE INDEX IF NOT EXISTS idx_sales_register_event_time
            ON p900_sales_register (event_time_source);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_register_idx2.to_string(),
        ))
        .await?;

        let create_register_idx3 = r#"
            CREATE INDEX IF NOT EXISTS idx_sales_register_connection_mp
            ON p900_sales_register (connection_mp_ref);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_register_idx3.to_string(),
        ))
        .await?;

        let create_register_idx4 = r#"
            CREATE INDEX IF NOT EXISTS idx_sales_register_organization
            ON p900_sales_register (organization_ref);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_register_idx4.to_string(),
        ))
        .await?;

        let create_register_idx5 = r#"
            CREATE INDEX IF NOT EXISTS idx_sales_register_product
            ON p900_sales_register (marketplace_product_ref);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_register_idx5.to_string(),
        ))
        .await?;

        let create_register_idx6 = r#"
            CREATE INDEX IF NOT EXISTS idx_sales_register_seller_sku
            ON p900_sales_register (seller_sku);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_register_idx6.to_string(),
        ))
        .await?;

        let create_register_idx7 = r#"
            CREATE INDEX IF NOT EXISTS idx_sales_register_mp_item_id
            ON p900_sales_register (mp_item_id);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_register_idx7.to_string(),
        ))
        .await?;

        let create_register_idx8 = r#"
            CREATE INDEX IF NOT EXISTS idx_sales_register_status_norm
            ON p900_sales_register (status_norm);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_register_idx8.to_string(),
        ))
        .await?;
    } else {
        // РўР°Р±Р»РёС†Р° СЃСѓС‰РµСЃС‚РІСѓРµС‚, РїСЂРѕРІРµСЂСЏРµРј РЅР°Р»РёС‡РёРµ РїРѕР»СЏ nomenclature_ref
        let check_nomenclature_ref = r#"
            PRAGMA table_info(p900_sales_register);
        "#;
        let cols = conn
            .query_all(Statement::from_string(
                DatabaseBackend::Sqlite,
                check_nomenclature_ref.to_string(),
            ))
            .await?;

        let mut has_nomenclature_ref = false;
        for row in cols {
            let name: String = row.try_get("", "name").unwrap_or_default();
            if name == "nomenclature_ref" {
                has_nomenclature_ref = true;
                break;
            }
        }

        if !has_nomenclature_ref {
            tracing::info!("Adding nomenclature_ref column to p900_sales_register");
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "ALTER TABLE p900_sales_register ADD COLUMN nomenclature_ref TEXT;".to_string(),
            ))
            .await?;
        }
    }

    // p901_nomenclature_barcodes table - С€С‚СЂРёС…РєРѕРґС‹ РЅРѕРјРµРЅРєР»Р°С‚СѓСЂС‹
    let check_barcodes_table = r#"
        SELECT name FROM sqlite_master
        WHERE type='table' AND name='p901_nomenclature_barcodes';
    "#;
    let barcodes_table_exists = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            check_barcodes_table.to_string(),
        ))
        .await?;

    if barcodes_table_exists.is_empty() {
        tracing::info!("Creating p901_nomenclature_barcodes table with composite key");
        let create_barcodes_table_sql = r#"
            CREATE TABLE p901_nomenclature_barcodes (
                barcode TEXT NOT NULL,
                source TEXT NOT NULL,
                nomenclature_ref TEXT,
                article TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
                PRIMARY KEY (barcode, source)
            );
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_barcodes_table_sql.to_string(),
        ))
        .await?;

        // РЎРѕР·РґР°С‚СЊ РёРЅРґРµРєСЃ РґР»СЏ Р±С‹СЃС‚СЂРѕРіРѕ РїРѕРёСЃРєР° РїРѕ nomenclature_ref
        let create_barcodes_idx1 = r#"
            CREATE INDEX IF NOT EXISTS idx_barcodes_nomenclature_ref
            ON p901_nomenclature_barcodes (nomenclature_ref);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_barcodes_idx1.to_string(),
        ))
        .await?;

        // РЎРѕР·РґР°С‚СЊ РёРЅРґРµРєСЃ РґР»СЏ РїРѕРёСЃРєР° РїРѕ Р°СЂС‚РёРєСѓР»Сѓ
        let create_barcodes_idx2 = r#"
            CREATE INDEX IF NOT EXISTS idx_barcodes_article
            ON p901_nomenclature_barcodes (article);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_barcodes_idx2.to_string(),
        ))
        .await?;

        // РЎРѕР·РґР°С‚СЊ РёРЅРґРµРєСЃ РґР»СЏ С„РёР»СЊС‚СЂР°С†РёРё РїРѕ is_active
        let create_barcodes_idx3 = r#"
            CREATE INDEX IF NOT EXISTS idx_barcodes_is_active
            ON p901_nomenclature_barcodes (is_active);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_barcodes_idx3.to_string(),
        ))
        .await?;

        // РЎРѕР·РґР°С‚СЊ РёРЅРґРµРєСЃ РїРѕ source РґР»СЏ Р±С‹СЃС‚СЂРѕР№ С„РёР»СЊС‚СЂР°С†РёРё
        let create_barcodes_idx4 = r#"
            CREATE INDEX IF NOT EXISTS idx_barcodes_source
            ON p901_nomenclature_barcodes (source);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_barcodes_idx4.to_string(),
        ))
        .await?;
    } else {
        // РњРёРіСЂР°С†РёСЏ: РїСЂРѕРІРµСЂРёС‚СЊ, РёСЃРїРѕР»СЊР·СѓРµС‚ Р»Рё С‚Р°Р±Р»РёС†Р° СЃС‚Р°СЂСѓСЋ СЃС…РµРјСѓ (single primary key)
        tracing::info!("Checking p901_nomenclature_barcodes schema for migration");

        let check_old_schema = r#"
            SELECT sql FROM sqlite_master
            WHERE type='table' AND name='p901_nomenclature_barcodes'
            AND sql LIKE '%barcode TEXT PRIMARY KEY%';
        "#;
        let old_schema_exists = conn
            .query_all(Statement::from_string(
                DatabaseBackend::Sqlite,
                check_old_schema.to_string(),
            ))
            .await?;

        if !old_schema_exists.is_empty() {
            tracing::warn!(
                "Old p901_nomenclature_barcodes schema detected. Performing migration..."
            );

            // 1. РџРµСЂРµРёРјРµРЅРѕРІР°С‚СЊ СЃС‚Р°СЂСѓСЋ С‚Р°Р±Р»РёС†Сѓ
            let rename_old_table = r#"
                ALTER TABLE p901_nomenclature_barcodes
                RENAME TO p901_nomenclature_barcodes_backup;
            "#;
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                rename_old_table.to_string(),
            ))
            .await?;

            // 2. РЎРѕР·РґР°С‚СЊ РЅРѕРІСѓСЋ С‚Р°Р±Р»РёС†Сѓ СЃ composite key
            let create_new_table = r#"
                CREATE TABLE p901_nomenclature_barcodes (
                    barcode TEXT NOT NULL,
                    source TEXT NOT NULL,
                    nomenclature_ref TEXT,
                    article TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    is_active INTEGER NOT NULL DEFAULT 1,
                    PRIMARY KEY (barcode, source)
                );
            "#;
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                create_new_table.to_string(),
            ))
            .await?;

            // 3. РЎРєРѕРїРёСЂРѕРІР°С‚СЊ РґР°РЅРЅС‹Рµ РёР· backup (РІСЃРµ СЃС‚Р°СЂС‹Рµ Р·Р°РїРёСЃРё СЃС‡РёС‚Р°СЋС‚СЃСЏ source='1C')
            let migrate_data = r#"
                INSERT INTO p901_nomenclature_barcodes
                    (barcode, source, nomenclature_ref, article, created_at, updated_at, is_active)
                SELECT
                    barcode,
                    COALESCE(source, '1C') as source,
                    nomenclature_ref,
                    article,
                    created_at,
                    updated_at,
                    is_active
                FROM p901_nomenclature_barcodes_backup;
            "#;
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                migrate_data.to_string(),
            ))
            .await?;

            // 4. РЎРѕР·РґР°С‚СЊ РёРЅРґРµРєСЃС‹ РґР»СЏ РЅРѕРІРѕР№ С‚Р°Р±Р»РёС†С‹
            let create_indexes = vec![
                r#"CREATE INDEX IF NOT EXISTS idx_barcodes_nomenclature_ref ON p901_nomenclature_barcodes (nomenclature_ref);"#,
                r#"CREATE INDEX IF NOT EXISTS idx_barcodes_article ON p901_nomenclature_barcodes (article);"#,
                r#"CREATE INDEX IF NOT EXISTS idx_barcodes_is_active ON p901_nomenclature_barcodes (is_active);"#,
                r#"CREATE INDEX IF NOT EXISTS idx_barcodes_source ON p901_nomenclature_barcodes (source);"#,
            ];

            for idx_sql in create_indexes {
                conn.execute(Statement::from_string(
                    DatabaseBackend::Sqlite,
                    idx_sql.to_string(),
                ))
                .await?;
            }

            // 5. РЈРґР°Р»РёС‚СЊ backup С‚Р°Р±Р»РёС†Сѓ
            let drop_backup = r#"DROP TABLE p901_nomenclature_barcodes_backup;"#;
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                drop_backup.to_string(),
            ))
            .await?;

            tracing::info!("Migration of p901_nomenclature_barcodes completed successfully");
        }
    }

    // a010_ozon_fbs_posting table - РґРѕРєСѓРјРµРЅС‚С‹ OZON FBS
    let check_ozon_fbs = r#"
        SELECT name FROM sqlite_master
        WHERE type='table' AND name='a010_ozon_fbs_posting';
    "#;
    let ozon_fbs_exists = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            check_ozon_fbs.to_string(),
        ))
        .await?;

    if ozon_fbs_exists.is_empty() {
        tracing::info!("Creating a010_ozon_fbs_posting table");
        let create_table_sql = r#"
            CREATE TABLE a010_ozon_fbs_posting (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                comment TEXT,
                document_no TEXT NOT NULL UNIQUE,
                header_json TEXT NOT NULL,
                lines_json TEXT NOT NULL,
                state_json TEXT NOT NULL,
                source_meta_json TEXT NOT NULL,
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_posted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT,
                version INTEGER NOT NULL DEFAULT 0
            );
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_table_sql.to_string(),
        ))
        .await?;
    }

    // a011_ozon_fbo_posting table - РґРѕРєСѓРјРµРЅС‚С‹ OZON FBO
    let check_ozon_fbo = r#"
        SELECT name FROM sqlite_master
        WHERE type='table' AND name='a011_ozon_fbo_posting';
    "#;
    let ozon_fbo_exists = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            check_ozon_fbo.to_string(),
        ))
        .await?;

    if ozon_fbo_exists.is_empty() {
        tracing::info!("Creating a011_ozon_fbo_posting table");
        let create_table_sql = r#"
            CREATE TABLE a011_ozon_fbo_posting (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                comment TEXT,
                document_no TEXT NOT NULL UNIQUE,
                status_norm TEXT NOT NULL DEFAULT '',
                substatus_raw TEXT,
                created_at_source TEXT,
                header_json TEXT NOT NULL,
                lines_json TEXT NOT NULL,
                state_json TEXT NOT NULL,
                source_meta_json TEXT NOT NULL,
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_posted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT,
                version INTEGER NOT NULL DEFAULT 0
            );
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_table_sql.to_string(),
        ))
        .await?;
    } else {
        // РњРёРіСЂР°С†РёСЏ: РґРѕР±Р°РІР»РµРЅРёРµ РїРѕР»РµР№ status_norm Рё substatus_raw РµСЃР»Рё РёС… РЅРµС‚
        let check_status_norm = r#"
            SELECT COUNT(*) as cnt FROM pragma_table_info('a011_ozon_fbo_posting')
            WHERE name='status_norm';
        "#;
        let has_status_norm = conn
            .query_one(Statement::from_string(
                DatabaseBackend::Sqlite,
                check_status_norm.to_string(),
            ))
            .await?
            .map(|row| row.try_get::<i32>("", "cnt").unwrap_or(0) > 0)
            .unwrap_or(false);

        if !has_status_norm {
            tracing::info!("Adding status_norm and substatus_raw columns to a011_ozon_fbo_posting");
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "ALTER TABLE a011_ozon_fbo_posting ADD COLUMN status_norm TEXT NOT NULL DEFAULT '';".to_string(),
            ))
            .await?;
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "ALTER TABLE a011_ozon_fbo_posting ADD COLUMN substatus_raw TEXT;".to_string(),
            ))
            .await?;
        }

        // РњРёРіСЂР°С†РёСЏ: РґРѕР±Р°РІР»РµРЅРёРµ РїРѕР»СЏ created_at_source РµСЃР»Рё РµРіРѕ РЅРµС‚
        let check_created_at_source = r#"
            SELECT COUNT(*) as cnt FROM pragma_table_info('a011_ozon_fbo_posting')
            WHERE name='created_at_source';
        "#;
        let has_created_at_source = conn
            .query_one(Statement::from_string(
                DatabaseBackend::Sqlite,
                check_created_at_source.to_string(),
            ))
            .await?
            .map(|row| row.try_get::<i32>("", "cnt").unwrap_or(0) > 0)
            .unwrap_or(false);

        if !has_created_at_source {
            tracing::info!("Adding created_at_source column to a011_ozon_fbo_posting");
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "ALTER TABLE a011_ozon_fbo_posting ADD COLUMN created_at_source TEXT;".to_string(),
            ))
            .await?;
        }
    }

    // a012_wb_sales table - РґРѕРєСѓРјРµРЅС‚С‹ Wildberries Sales
    let check_wb_sales = r#"
        SELECT name FROM sqlite_master
        WHERE type='table' AND name='a012_wb_sales';
    "#;
    let wb_sales_exists = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            check_wb_sales.to_string(),
        ))
        .await?;

    if wb_sales_exists.is_empty() {
        tracing::info!("Creating a012_wb_sales table");
        let create_table_sql = r#"
            CREATE TABLE a012_wb_sales (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                comment TEXT,
                document_no TEXT NOT NULL UNIQUE,
                sale_id TEXT,
                -- Denormalized fields from JSON for fast queries
                sale_date TEXT,
                organization_id TEXT,
                connection_id TEXT,
                supplier_article TEXT,
                nm_id INTEGER,
                barcode TEXT,
                product_name TEXT,
                qty REAL,
                amount_line REAL,
                total_price REAL,
                finished_price REAL,
                event_type TEXT,
                -- JSON storage (kept for backward compatibility and full data)
                header_json TEXT NOT NULL,
                line_json TEXT NOT NULL,
                state_json TEXT NOT NULL,
                warehouse_json TEXT,
                source_meta_json TEXT NOT NULL,
                marketplace_product_ref TEXT,
                nomenclature_ref TEXT,
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_posted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT,
                version INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_a012_sale_date ON a012_wb_sales(sale_date);
            CREATE INDEX IF NOT EXISTS idx_a012_organization ON a012_wb_sales(organization_id);
            CREATE INDEX IF NOT EXISTS idx_a012_sale_id ON a012_wb_sales(sale_id);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_table_sql.to_string(),
        ))
        .await?;
    } else {
        // Migration: add new denormalized columns if not exist
        let new_columns = vec![
            ("sale_id", "TEXT"),
            ("sale_date", "TEXT"),
            ("organization_id", "TEXT"),
            ("connection_id", "TEXT"),
            ("supplier_article", "TEXT"),
            ("nm_id", "INTEGER"),
            ("barcode", "TEXT"),
            ("product_name", "TEXT"),
            ("qty", "REAL"),
            ("amount_line", "REAL"),
            ("total_price", "REAL"),
            ("finished_price", "REAL"),
            ("event_type", "TEXT"),
            ("warehouse_json", "TEXT"),
        ];

        for (col_name, col_type) in new_columns {
            let check_sql = format!(
                "SELECT COUNT(*) as cnt FROM pragma_table_info('a012_wb_sales') WHERE name='{}';",
                col_name
            );
            let has_column = conn
                .query_one(Statement::from_string(DatabaseBackend::Sqlite, check_sql))
                .await?
                .map(|row| row.try_get::<i32>("", "cnt").unwrap_or(0) > 0)
                .unwrap_or(false);

            if !has_column {
                tracing::info!("Adding {} column to a012_wb_sales", col_name);
                let alter_sql = format!(
                    "ALTER TABLE a012_wb_sales ADD COLUMN {} {};",
                    col_name, col_type
                );
                conn.execute(Statement::from_string(DatabaseBackend::Sqlite, alter_sql))
                    .await?;
            }
        }

        // Create indexes if not exist
        let indexes = vec![
            "CREATE INDEX IF NOT EXISTS idx_a012_sale_date ON a012_wb_sales(sale_date);",
            "CREATE INDEX IF NOT EXISTS idx_a012_organization ON a012_wb_sales(organization_id);",
            "CREATE INDEX IF NOT EXISTS idx_a012_sale_id ON a012_wb_sales(sale_id);",
        ];
        for idx_sql in indexes {
            let _ = conn
                .execute(Statement::from_string(
                    DatabaseBackend::Sqlite,
                    idx_sql.to_string(),
                ))
                .await;
        }
    }

    // a013_ym_order table - РґРѕРєСѓРјРµРЅС‚С‹ Yandex Market Orders
    let check_ym_order = r#"
        SELECT name FROM sqlite_master
        WHERE type='table' AND name='a013_ym_order';
    "#;
    let ym_order_exists = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            check_ym_order.to_string(),
        ))
        .await?;

    if ym_order_exists.is_empty() {
        tracing::info!("Creating a013_ym_order table");
        let create_table_sql = r#"
            CREATE TABLE a013_ym_order (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                comment TEXT,
                document_no TEXT NOT NULL UNIQUE,
                header_json TEXT NOT NULL,
                lines_json TEXT NOT NULL,
                state_json TEXT NOT NULL,
                source_meta_json TEXT NOT NULL,
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_posted INTEGER NOT NULL DEFAULT 0,
                is_error INTEGER NOT NULL DEFAULT 0,
                -- Р”РµРЅРѕСЂРјР°Р»РёР·РѕРІР°РЅРЅС‹Рµ РїРѕР»СЏ РґР»СЏ Р±С‹СЃС‚СЂС‹С… Р·Р°РїСЂРѕСЃРѕРІ СЃРїРёСЃРєР°
                status_changed_at TEXT,
                creation_date TEXT,
                delivery_date TEXT,
                campaign_id TEXT,
                status_norm TEXT,
                total_qty REAL DEFAULT 0,
                total_amount REAL DEFAULT 0,
                total_amount_api REAL,
                lines_count INTEGER DEFAULT 0,
                delivery_total REAL,
                subsidies_total REAL DEFAULT 0,
                organization_id TEXT,
                connection_id TEXT,
                created_at TEXT,
                updated_at TEXT,
                version INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX idx_a013_delivery_date ON a013_ym_order(delivery_date);
            CREATE INDEX idx_a013_status_norm ON a013_ym_order(status_norm);
            CREATE INDEX idx_a013_organization_id ON a013_ym_order(organization_id);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_table_sql.to_string(),
        ))
        .await?;
    } else {
        // РњРёРіСЂР°С†РёСЏ: РґРѕР±Р°РІР»СЏРµРј РґРµРЅРѕСЂРјР°Р»РёР·РѕРІР°РЅРЅС‹Рµ РєРѕР»РѕРЅРєРё РµСЃР»Рё РёС… РЅРµС‚
        let columns_to_add = vec![
            ("is_error", "INTEGER NOT NULL DEFAULT 0"),
            ("status_changed_at", "TEXT"),
            ("creation_date", "TEXT"),
            ("delivery_date", "TEXT"),
            ("campaign_id", "TEXT"),
            ("status_norm", "TEXT"),
            ("total_qty", "REAL DEFAULT 0"),
            ("total_amount", "REAL DEFAULT 0"),
            ("total_amount_api", "REAL"),
            ("lines_count", "INTEGER DEFAULT 0"),
            ("delivery_total", "REAL"),
            ("subsidies_total", "REAL DEFAULT 0"),
            ("organization_id", "TEXT"),
            ("connection_id", "TEXT"),
        ];

        for (col_name, col_type) in columns_to_add {
            let check_col = format!(
                "SELECT COUNT(*) as cnt FROM pragma_table_info('a013_ym_order') WHERE name='{}'",
                col_name
            );
            let col_exists = conn
                .query_one(Statement::from_string(
                    DatabaseBackend::Sqlite,
                    check_col,
                ))
                .await?;
            if let Some(row) = col_exists {
                let cnt: i32 = row.try_get("", "cnt").unwrap_or(0);
                if cnt == 0 {
                    tracing::info!("Adding {} column to a013_ym_order", col_name);
                    let alter_sql = format!(
                        "ALTER TABLE a013_ym_order ADD COLUMN {} {}",
                        col_name, col_type
                    );
                    conn.execute(Statement::from_string(
                        DatabaseBackend::Sqlite,
                        alter_sql,
                    ))
                    .await?;
                }
            }
        }
    }

    // a013_ym_order_items table - С‚Р°Р±Р»РёС‡РЅР°СЏ С‡Р°СЃС‚СЊ Р·Р°РєР°Р·РѕРІ YM
    let check_ym_order_items = r#"
        SELECT name FROM sqlite_master
        WHERE type='table' AND name='a013_ym_order_items';
    "#;
    let ym_order_items_exists = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            check_ym_order_items.to_string(),
        ))
        .await?;

    if ym_order_items_exists.is_empty() {
        tracing::info!("Creating a013_ym_order_items table");
        let create_items_sql = r#"
            CREATE TABLE a013_ym_order_items (
                id TEXT PRIMARY KEY NOT NULL,
                order_id TEXT NOT NULL,
                line_id TEXT NOT NULL,
                shop_sku TEXT NOT NULL,
                offer_id TEXT NOT NULL,
                name TEXT NOT NULL DEFAULT '',
                qty REAL NOT NULL DEFAULT 1.0,
                price_list REAL,
                discount_total REAL,
                price_effective REAL,
                amount_line REAL,
                price_plan REAL DEFAULT 0,
                marketplace_product_ref TEXT,
                nomenclature_ref TEXT,
                currency_code TEXT,
                buyer_price REAL,
                subsidies_json TEXT,
                status TEXT,
                FOREIGN KEY (order_id) REFERENCES a013_ym_order(id)
            );
            CREATE INDEX idx_a013_items_order_id ON a013_ym_order_items(order_id);
            CREATE INDEX idx_a013_items_shop_sku ON a013_ym_order_items(shop_sku);
            CREATE INDEX idx_a013_items_nomenclature_ref ON a013_ym_order_items(nomenclature_ref);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_items_sql.to_string(),
        ))
        .await?;
    }

    // a014_ozon_transactions table - С‚СЂР°РЅР·Р°РєС†РёРё OZON
    let check_ozon_transactions = r#"
        SELECT name FROM sqlite_master
        WHERE type='table' AND name='a014_ozon_transactions';
    "#;
    let ozon_transactions_exists = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            check_ozon_transactions.to_string(),
        ))
        .await?;

    if ozon_transactions_exists.is_empty() {
        tracing::info!("Creating a014_ozon_transactions table");
        let create_table_sql = r#"
            CREATE TABLE a014_ozon_transactions (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                comment TEXT,
                operation_id INTEGER NOT NULL UNIQUE,
                posting_number TEXT NOT NULL,
                header_json TEXT NOT NULL,
                posting_json TEXT NOT NULL,
                items_json TEXT NOT NULL,
                services_json TEXT NOT NULL,
                source_meta_json TEXT NOT NULL,
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_posted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT,
                version INTEGER NOT NULL DEFAULT 0
            );
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_table_sql.to_string(),
        ))
        .await?;
    }

    // a016_ym_returns table - РІРѕР·РІСЂР°С‚С‹ Рё РЅРµРІС‹РєСѓРїС‹ Yandex Market
    let check_ym_returns = r#"
        SELECT name FROM sqlite_master
        WHERE type='table' AND name='a016_ym_returns';
    "#;
    let ym_returns_exists = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            check_ym_returns.to_string(),
        ))
        .await?;

    if ym_returns_exists.is_empty() {
        tracing::info!("Creating a016_ym_returns table");
        let create_table_sql = r#"
            CREATE TABLE a016_ym_returns (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                comment TEXT,
                return_id INTEGER NOT NULL UNIQUE,
                order_id INTEGER NOT NULL,
                header_json TEXT NOT NULL,
                lines_json TEXT NOT NULL,
                state_json TEXT NOT NULL,
                source_meta_json TEXT NOT NULL,
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_posted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT,
                version INTEGER NOT NULL DEFAULT 0
            );
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_table_sql.to_string(),
        ))
        .await?;

        // Index on order_id for queries by order
        let create_idx_order = r#"
            CREATE INDEX IF NOT EXISTS idx_a016_order_id ON a016_ym_returns(order_id);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_idx_order.to_string(),
        ))
        .await?;
    }

    // p902_ozon_finance_realization table - С„РёРЅР°РЅСЃРѕРІС‹Рµ РґР°РЅРЅС‹Рµ СЂРµР°Р»РёР·Р°С†РёРё OZON
    let check_p902 = r#"
        SELECT name FROM sqlite_master
        WHERE type='table' AND name='p902_ozon_finance_realization';
    "#;
    let p902_exists = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            check_p902.to_string(),
        ))
        .await?;

    // p903_wb_finance_report table - С„РёРЅР°РЅСЃРѕРІС‹Рµ РѕС‚С‡РµС‚С‹ Wildberries
    let check_p903 = r#"
        SELECT name FROM sqlite_master
        WHERE type='table' AND name='p903_wb_finance_report';
    "#;
    let p903_exists = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            check_p903.to_string(),
        ))
        .await?;

    if p902_exists.is_empty() {
        tracing::info!("Creating p902_ozon_finance_realization table");
    } else {
        // РџСЂРѕРІРµСЂСЏРµРј, РЅСѓР¶РЅР° Р»Рё РјРёРіСЂР°С†РёСЏ (РµСЃС‚СЊ Р»Рё РєРѕР»РѕРЅРєР° is_return)
        let check_is_return_column = Statement::from_string(
            DatabaseBackend::Sqlite,
            "PRAGMA table_info(p902_ozon_finance_realization);".to_string(),
        );

        let columns = conn.query_all(check_is_return_column).await?;
        let has_is_return = columns.iter().any(|row| {
            row.try_get::<String>("", "name")
                .map(|name| name == "is_return")
                .unwrap_or(false)
        });

        if !has_is_return {
            tracing::warn!("Migrating p902_ozon_finance_realization table - adding is_return column and updating PRIMARY KEY");

            // Р’С‹РїРѕР»РЅСЏРµРј РјРёРіСЂР°С†РёСЋ
            let migration_sql = r#"
                -- РЎРѕР·РґР°РµРј РІСЂРµРјРµРЅРЅСѓСЋ С‚Р°Р±Р»РёС†Сѓ СЃ РЅРѕРІРѕР№ СЃС‚СЂСѓРєС‚СѓСЂРѕР№
                CREATE TABLE p902_ozon_finance_realization_new (
                    posting_number TEXT NOT NULL,
                    sku TEXT NOT NULL,
                    document_type TEXT NOT NULL,
                    registrator_ref TEXT NOT NULL,
                    connection_mp_ref TEXT NOT NULL,
                    organization_ref TEXT NOT NULL,
                    posting_ref TEXT,
                    accrual_date TEXT NOT NULL,
                    operation_date TEXT,
                    delivery_date TEXT,
                    delivery_schema TEXT,
                    delivery_region TEXT,
                    delivery_city TEXT,
                    quantity REAL NOT NULL,
                    price REAL,
                    amount REAL NOT NULL,
                    commission_amount REAL,
                    commission_percent REAL,
                    services_amount REAL,
                    payout_amount REAL,
                    operation_type TEXT NOT NULL,
                    operation_type_name TEXT,
                    is_return INTEGER NOT NULL DEFAULT 0,
                    currency_code TEXT,
                    loaded_at_utc TEXT NOT NULL,
                    payload_version INTEGER NOT NULL DEFAULT 1,
                    extra TEXT,
                    PRIMARY KEY (posting_number, sku, operation_type)
                );
            "#;

            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                migration_sql.to_string(),
            ))
            .await?;

            // РљРѕРїРёСЂСѓРµРј РґР°РЅРЅС‹Рµ РёР· СЃС‚Р°СЂРѕР№ С‚Р°Р±Р»РёС†С‹
            let copy_data_sql = r#"
                INSERT INTO p902_ozon_finance_realization_new (
                    posting_number, sku, document_type, registrator_ref,
                    connection_mp_ref, organization_ref, posting_ref,
                    accrual_date, operation_date, delivery_date,
                    delivery_schema, delivery_region, delivery_city,
                    quantity, price, amount, commission_amount, commission_percent,
                    services_amount, payout_amount,
                    operation_type, operation_type_name, is_return,
                    currency_code, loaded_at_utc, payload_version, extra
                )
                SELECT
                    posting_number, sku, document_type, registrator_ref,
                    connection_mp_ref, organization_ref, posting_ref,
                    accrual_date, operation_date, delivery_date,
                    delivery_schema, delivery_region, delivery_city,
                    quantity, price, amount, commission_amount, commission_percent,
                    services_amount, payout_amount,
                    operation_type, operation_type_name, 0 as is_return,
                    currency_code, loaded_at_utc, payload_version, extra
                FROM p902_ozon_finance_realization;
            "#;

            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                copy_data_sql.to_string(),
            ))
            .await?;

            // РЈРґР°Р»СЏРµРј СЃС‚Р°СЂСѓСЋ С‚Р°Р±Р»РёС†Сѓ
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "DROP TABLE p902_ozon_finance_realization;".to_string(),
            ))
            .await?;

            // РџРµСЂРµРёРјРµРЅРѕРІС‹РІР°РµРј РЅРѕРІСѓСЋ С‚Р°Р±Р»РёС†Сѓ
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "ALTER TABLE p902_ozon_finance_realization_new RENAME TO p902_ozon_finance_realization;".to_string(),
            )).await?;

            // РЎРѕР·РґР°РµРј РёРЅРґРµРєСЃС‹ Р·Р°РЅРѕРІРѕ
            let create_idx1 = "CREATE INDEX IF NOT EXISTS idx_p902_accrual_date ON p902_ozon_finance_realization (accrual_date);";
            let create_idx2 = "CREATE INDEX IF NOT EXISTS idx_p902_posting_number ON p902_ozon_finance_realization (posting_number);";
            let create_idx3 = "CREATE INDEX IF NOT EXISTS idx_p902_connection_mp_ref ON p902_ozon_finance_realization (connection_mp_ref);";
            let create_idx4 = "CREATE INDEX IF NOT EXISTS idx_p902_posting_ref ON p902_ozon_finance_realization (posting_ref);";

            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                create_idx1.to_string(),
            ))
            .await?;
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                create_idx2.to_string(),
            ))
            .await?;
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                create_idx3.to_string(),
            ))
            .await?;
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                create_idx4.to_string(),
            ))
            .await?;

            tracing::info!("Migration of p902_ozon_finance_realization completed successfully");
        }
    }

    if p902_exists.is_empty() {
        let create_p902_table_sql = r#"
            CREATE TABLE p902_ozon_finance_realization (
                -- Composite Key (posting_number + sku + operation_type)
                posting_number TEXT NOT NULL,
                sku TEXT NOT NULL,

                -- Metadata
                document_type TEXT NOT NULL,
                registrator_ref TEXT NOT NULL,

                -- References
                connection_mp_ref TEXT NOT NULL,
                organization_ref TEXT NOT NULL,
                posting_ref TEXT,

                -- Р”Р°С‚С‹
                accrual_date TEXT NOT NULL,
                operation_date TEXT,
                delivery_date TEXT,

                -- РРЅС„РѕСЂРјР°С†РёСЏ Рѕ РґРѕСЃС‚Р°РІРєРµ
                delivery_schema TEXT,
                delivery_region TEXT,
                delivery_city TEXT,

                -- РљРѕР»РёС‡РµСЃС‚РІРѕ Рё СЃСѓРјРјС‹
                quantity REAL NOT NULL,
                price REAL,
                amount REAL NOT NULL,
                commission_amount REAL,
                commission_percent REAL,
                services_amount REAL,
                payout_amount REAL,

                -- РўРёРї РѕРїРµСЂР°С†РёРё
                operation_type TEXT NOT NULL,
                operation_type_name TEXT,
                is_return INTEGER NOT NULL DEFAULT 0,

                -- Р’Р°Р»СЋС‚Р°
                currency_code TEXT,

                -- РўРµС…РЅРёС‡РµСЃРєРёРµ РїРѕР»СЏ
                loaded_at_utc TEXT NOT NULL,
                payload_version INTEGER NOT NULL DEFAULT 1,
                extra TEXT,

                PRIMARY KEY (posting_number, sku, operation_type)
            );
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_p902_table_sql.to_string(),
        ))
        .await?;

        // РЎРѕР·РґР°С‚СЊ РёРЅРґРµРєСЃС‹ РґР»СЏ Р±С‹СЃС‚СЂРѕРіРѕ РїРѕРёСЃРєР°
        let create_p902_idx1 = r#"
            CREATE INDEX IF NOT EXISTS idx_p902_accrual_date
            ON p902_ozon_finance_realization (accrual_date);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_p902_idx1.to_string(),
        ))
        .await?;

        let create_p902_idx2 = r#"
            CREATE INDEX IF NOT EXISTS idx_p902_posting_number
            ON p902_ozon_finance_realization (posting_number);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_p902_idx2.to_string(),
        ))
        .await?;

        let create_p902_idx3 = r#"
            CREATE INDEX IF NOT EXISTS idx_p902_connection_mp_ref
            ON p902_ozon_finance_realization (connection_mp_ref);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_p902_idx3.to_string(),
        ))
        .await?;

        let create_p902_idx4 = r#"
            CREATE INDEX IF NOT EXISTS idx_p902_posting_ref
            ON p902_ozon_finance_realization (posting_ref);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_p902_idx4.to_string(),
        ))
        .await?;

        tracing::info!("Created p902_ozon_finance_realization table with indexes");
    }

    // ============================================================
    // P903: Wildberries Finance Report
    // ============================================================
    if p903_exists.is_empty() {
        let create_p903_table_sql = r#"
            CREATE TABLE p903_wb_finance_report (
                -- Composite Primary Key
                rr_dt TEXT NOT NULL,              -- Р”Р°С‚Р° СЃС‚СЂРѕРєРё С„РёРЅР°РЅСЃРѕРІРѕРіРѕ РѕС‚С‡С‘С‚Р°
                rrd_id INTEGER NOT NULL,          -- Р’РЅСѓС‚СЂРµРЅРЅРёР№ ID СЃС‚СЂРѕРєРё РѕС‚С‡РµС‚Р°

                -- Metadata
                connection_mp_ref TEXT NOT NULL,
                organization_ref TEXT NOT NULL,

                -- Main Fields (22 specified fields)
                acquiring_fee REAL,               -- РљРѕРјРёСЃСЃРёСЏ Р·Р° СЌРєРІР°Р№СЂРёРЅРі
                acquiring_percent REAL,           -- РџСЂРѕС†РµРЅС‚ РєРѕРјРёСЃСЃРёРё Р·Р° СЌРєРІР°Р№СЂРёРЅРі
                additional_payment REAL,          -- Р”РѕРїРѕР»РЅРёС‚РµР»СЊРЅС‹Рµ РІС‹РїР»Р°С‚С‹
                bonus_type_name TEXT,             -- РўРёРї Р±РѕРЅСѓСЃР° РёР»Рё С€С‚СЂР°С„Р°
                commission_percent REAL,          -- РџСЂРѕС†РµРЅС‚ РєРѕРјРёСЃСЃРёРё WB
                delivery_amount REAL,             -- РЎСѓРјРјР° Р·Р° РґРѕСЃС‚Р°РІРєСѓ РѕС‚ РїРѕРєСѓРїР°С‚РµР»СЏ
                delivery_rub REAL,                -- РЎС‚РѕРёРјРѕСЃС‚СЊ РґРѕСЃС‚Р°РІРєРё РґР»СЏ РїСЂРѕРґР°РІС†Р°
                nm_id INTEGER,                    -- РђСЂС‚РёРєСѓР» WB
                penalty REAL,                     -- РЁС‚СЂР°С„
                ppvz_vw REAL,                     -- РЈСЃР»СѓРіР° РІРѕР·РІСЂР°С‚Р° СЃСЂРµРґСЃС‚РІ
                ppvz_vw_nds REAL,                 -- РќР”РЎ РїРѕ СѓСЃР»СѓРіРµ РІРѕР·РІСЂР°С‚Р°
                ppvz_sales_commission REAL,       -- РљРѕРјРёСЃСЃРёСЏ WB Р·Р° РїСЂРѕРґР°Р¶Сѓ
                quantity INTEGER,                 -- РљРѕР»РёС‡РµСЃС‚РІРѕ С‚РѕРІР°СЂРѕРІ
                rebill_logistic_cost REAL,        -- Р Р°СЃС…РѕРґС‹ РЅР° Р»РѕРіРёСЃС‚РёРєСѓ
                retail_amount REAL,               -- РћР±С‰Р°СЏ СЃСѓРјРјР° РїСЂРѕРґР°Р¶Рё
                retail_price REAL,                -- Р РѕР·РЅРёС‡РЅР°СЏ С†РµРЅР° Р·Р° РµРґРёРЅРёС†Сѓ
                retail_price_withdisc_rub REAL,   -- Р¦РµРЅР° СЃ СѓС‡РµС‚РѕРј СЃРєРёРґРѕРє
                return_amount REAL,               -- РЎСѓРјРјР° РІРѕР·РІСЂР°С‚Р°
                sa_name TEXT,                     -- РђСЂС‚РёРєСѓР» РїСЂРѕРґР°РІС†Р°
                storage_fee REAL,                 -- РџР»Р°С‚Р° Р·Р° С…СЂР°РЅРµРЅРёРµ
                subject_name TEXT,                -- РљР°С‚РµРіРѕСЂРёСЏ С‚РѕРІР°СЂР°
                supplier_oper_name TEXT,          -- РўРёРї РѕРїРµСЂР°С†РёРё
                cashback_amount REAL,             -- РЎСѓРјРјР° РєСЌС€Р±СЌРєР°
                ppvz_for_pay REAL,                -- Рљ РїРµСЂРµС‡РёСЃР»РµРЅРёСЋ Р·Р° С‚РѕРІР°СЂ
                ppvz_kvw_prc REAL,                -- РџСЂРѕС†РµРЅС‚ РєРѕРјРёСЃСЃРёРё
                ppvz_kvw_prc_base REAL,           -- Р‘Р°Р·РѕРІС‹Р№ РїСЂРѕС†РµРЅС‚ РєРѕРјРёСЃСЃРёРё
                srv_dbs INTEGER,                  -- Р”РѕСЃС‚Р°РІРєР° СЃРёР»Р°РјРё РїСЂРѕРґР°РІС†Р° (0/1)

                -- Technical fields
                loaded_at_utc TEXT NOT NULL,
                payload_version INTEGER NOT NULL DEFAULT 1,
                extra TEXT,                       -- Full JSON from API

                PRIMARY KEY (rr_dt, rrd_id)
            );
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_p903_table_sql.to_string(),
        ))
        .await?;

        // Create indexes for fast search
        let create_p903_idx1 = r#"
            CREATE INDEX IF NOT EXISTS idx_p903_rr_dt
            ON p903_wb_finance_report (rr_dt);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_p903_idx1.to_string(),
        ))
        .await?;

        let create_p903_idx2 = r#"
            CREATE INDEX IF NOT EXISTS idx_p903_nm_id
            ON p903_wb_finance_report (nm_id);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_p903_idx2.to_string(),
        ))
        .await?;

        let create_p903_idx3 = r#"
            CREATE INDEX IF NOT EXISTS idx_p903_connection_mp_ref
            ON p903_wb_finance_report (connection_mp_ref);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_p903_idx3.to_string(),
        ))
        .await?;

        tracing::info!("Created p903_wb_finance_report table with indexes");
    } else {
        // РњРёРіСЂР°С†РёСЏ: РґРѕР±Р°РІРёС‚СЊ РїРѕР»Рµ ppvz_sales_commission РµСЃР»Рё РµРіРѕ РЅРµС‚
        let check_column = conn
            .query_all(Statement::from_string(
                DatabaseBackend::Sqlite,
                "PRAGMA table_info(p903_wb_finance_report);".to_string(),
            ))
            .await?;

        let has_ppvz_sales_commission = check_column.iter().any(|row| {
            row.try_get::<String>("", "name")
                .ok()
                .map(|name| name == "ppvz_sales_commission")
                .unwrap_or(false)
        });

        if !has_ppvz_sales_commission {
            tracing::info!("Migrating p903_wb_finance_report: adding ppvz_sales_commission column");
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "ALTER TABLE p903_wb_finance_report ADD COLUMN ppvz_sales_commission REAL;"
                    .to_string(),
            ))
            .await?;
            tracing::info!("Migration of p903_wb_finance_report completed successfully");
        }

        // РњРёРіСЂР°С†РёСЏ: РґРѕР±Р°РІРёС‚СЊ РЅРѕРІС‹Рµ РїРѕР»СЏ РµСЃР»Рё РёС… РЅРµС‚
        let new_fields = vec![
            ("cashback_amount", "REAL"),
            ("ppvz_for_pay", "REAL"),
            ("ppvz_kvw_prc", "REAL"),
            ("ppvz_kvw_prc_base", "REAL"),
            ("srv_dbs", "INTEGER"),
            ("srid", "TEXT"),
        ];

        for (field_name, field_type) in new_fields {
            let has_field = check_column.iter().any(|row| {
                row.try_get::<String>("", "name")
                    .ok()
                    .map(|name| name == field_name)
                    .unwrap_or(false)
            });

            if !has_field {
                tracing::info!(
                    "Migrating p903_wb_finance_report: adding {} column",
                    field_name
                );
                conn.execute(Statement::from_string(
                    DatabaseBackend::Sqlite,
                    format!(
                        "ALTER TABLE p903_wb_finance_report ADD COLUMN {} {};",
                        field_name, field_type
                    ),
                ))
                .await?;
            }
        }
    }

    // ============================================================
    // P904: Sales Data
    // ============================================================
    let check_p904_sales_data = r#"
        SELECT name FROM sqlite_master
        WHERE type='table' AND name='p904_sales_data';
    "#;
    let p904_sales_data_exists = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            check_p904_sales_data.to_string(),
        ))
        .await?;

    if p904_sales_data_exists.is_empty() {
        tracing::info!("Creating p904_sales_data table");
        let create_p904_sales_data_table_sql = r#"
            CREATE TABLE p904_sales_data (
                -- Technical fields
                registrator_ref TEXT NOT NULL,
                registrator_type TEXT NOT NULL DEFAULT '',
                
                -- Dimensions
                date TEXT NOT NULL,
                connection_mp_ref TEXT NOT NULL,
                nomenclature_ref TEXT NOT NULL,
                marketplace_product_ref TEXT NOT NULL,
                
                -- Sums
                customer_in REAL NOT NULL DEFAULT 0,
                customer_out REAL NOT NULL DEFAULT 0,
                coinvest_in REAL NOT NULL DEFAULT 0,
                commission_out REAL NOT NULL DEFAULT 0,
                acquiring_out REAL NOT NULL DEFAULT 0,
                penalty_out REAL NOT NULL DEFAULT 0,
                logistics_out REAL NOT NULL DEFAULT 0,
                seller_out REAL NOT NULL DEFAULT 0,
                price_full REAL NOT NULL DEFAULT 0,
                price_list REAL NOT NULL DEFAULT 0,
                price_return REAL NOT NULL DEFAULT 0,
                commission_percent REAL NOT NULL DEFAULT 0,
                coinvest_persent REAL NOT NULL DEFAULT 0,
                total REAL NOT NULL DEFAULT 0,
                
                -- Info fields
                document_no TEXT NOT NULL,
                article TEXT NOT NULL,
                posted_at TEXT NOT NULL,
                
                -- Primary Key
                id TEXT PRIMARY KEY NOT NULL
            );
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_p904_sales_data_table_sql.to_string(),
        ))
        .await?;

        // Index on registrator_ref for fast deletion
        let create_idx_registrator = r#"
            CREATE INDEX IF NOT EXISTS idx_p904_registrator
            ON p904_sales_data (registrator_ref);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_idx_registrator.to_string(),
        ))
        .await?;
    } else {
        // Migration: add registrator_type if not exists
        let check_column = conn
            .query_all(Statement::from_string(
                DatabaseBackend::Sqlite,
                "PRAGMA table_info(p904_sales_data);".to_string(),
            ))
            .await?;

        let has_registrator_type = check_column.iter().any(|row| {
            row.try_get::<String>("", "name")
                .ok()
                .map(|name| name == "registrator_type")
                .unwrap_or(false)
        });

        if !has_registrator_type {
            tracing::info!("Migrating p904_sales_data: adding registrator_type column");
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "ALTER TABLE p904_sales_data ADD COLUMN registrator_type TEXT NOT NULL DEFAULT '';"
                    .to_string(),
            ))
            .await?;
        }
    }

    // ============================================================
    // P905: WB Commission History
    // ============================================================
    let check_p905_wb_commission = r#"
        SELECT name FROM sqlite_master
        WHERE type='table' AND name='p905_wb_commission_history';
    "#;
    let p905_wb_commission_exists = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            check_p905_wb_commission.to_string(),
        ))
        .await?;

    if p905_wb_commission_exists.is_empty() {
        tracing::info!("Creating p905_wb_commission_history table");
        let create_p905_wb_commission_table_sql = r#"
            CREATE TABLE p905_wb_commission_history (
                id TEXT PRIMARY KEY NOT NULL,
                date TEXT NOT NULL,
                subject_id INTEGER NOT NULL,
                subject_name TEXT NOT NULL,
                parent_id INTEGER NOT NULL,
                parent_name TEXT NOT NULL,
                kgvp_booking REAL NOT NULL,
                kgvp_marketplace REAL NOT NULL,
                kgvp_pickup REAL NOT NULL,
                kgvp_supplier REAL NOT NULL,
                kgvp_supplier_express REAL NOT NULL,
                paid_storage_kgvp REAL NOT NULL,
                raw_json TEXT NOT NULL,
                loaded_at_utc TEXT NOT NULL,
                payload_version INTEGER NOT NULL DEFAULT 1
            );
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_p905_wb_commission_table_sql.to_string(),
        ))
        .await?;

        // Create unique index to prevent duplicates per date and subject
        let create_idx_date_subject = r#"
            CREATE UNIQUE INDEX IF NOT EXISTS idx_p905_date_subject 
            ON p905_wb_commission_history(date, subject_id);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_idx_date_subject.to_string(),
        ))
        .await?;

        // Create index for date range queries
        let create_idx_date = r#"
            CREATE INDEX IF NOT EXISTS idx_p905_date 
            ON p905_wb_commission_history(date);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_idx_date.to_string(),
        ))
        .await?;

        // Create index for subject_id lookups
        let create_idx_subject = r#"
            CREATE INDEX IF NOT EXISTS idx_p905_subject_id 
            ON p905_wb_commission_history(subject_id);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_idx_subject.to_string(),
        ))
        .await?;

        tracing::info!("Successfully created p905_wb_commission_history table with indexes");
    }

    // ============================================================
    // P906: Nomenclature Prices (1C)
    // ============================================================
    let check_p906_nomenclature_prices = r#"
        SELECT name FROM sqlite_master
        WHERE type='table' AND name='p906_nomenclature_prices';
    "#;
    let p906_nomenclature_prices_exists = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            check_p906_nomenclature_prices.to_string(),
        ))
        .await?;

    if p906_nomenclature_prices_exists.is_empty() {
        tracing::info!("Creating p906_nomenclature_prices table");
        let create_p906_nomenclature_prices_table_sql = r#"
            CREATE TABLE p906_nomenclature_prices (
                id TEXT PRIMARY KEY NOT NULL,
                period TEXT NOT NULL,
                nomenclature_ref TEXT NOT NULL,
                price REAL NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_p906_nomenclature_prices_table_sql.to_string(),
        ))
        .await?;

        // Create unique index on period + nomenclature_ref to prevent duplicates
        let create_idx_period_nomenclature = r#"
            CREATE UNIQUE INDEX IF NOT EXISTS idx_p906_period_nomenclature 
            ON p906_nomenclature_prices(period, nomenclature_ref);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_idx_period_nomenclature.to_string(),
        ))
        .await?;

        // Create index for period queries
        let create_idx_period = r#"
            CREATE INDEX IF NOT EXISTS idx_p906_period 
            ON p906_nomenclature_prices(period);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_idx_period.to_string(),
        ))
        .await?;

        // Create index for nomenclature_ref lookups
        let create_idx_nomenclature = r#"
            CREATE INDEX IF NOT EXISTS idx_p906_nomenclature_ref 
            ON p906_nomenclature_prices(nomenclature_ref);
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_idx_nomenclature.to_string(),
        ))
        .await?;

        tracing::info!("Successfully created p906_nomenclature_prices table with indexes");
    }

    // ============================================================
    // User Form Settings
    // ============================================================
    let check_user_form_settings = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT name FROM sqlite_master WHERE type='table' AND name='user_form_settings';"
                .to_string(),
        ))
        .await?;

    if check_user_form_settings.is_empty() {
        tracing::info!("Creating user_form_settings table");
        let create_user_form_settings_sql = r#"
            CREATE TABLE user_form_settings (
                form_key TEXT PRIMARY KEY NOT NULL,
                settings_json TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_user_form_settings_sql.to_string(),
        ))
        .await?;
    }

    // ============================================================
    // System: Tasks (Scheduled Tasks)
    // ============================================================
    let check_tasks = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT name FROM sqlite_master WHERE type='table' AND name='sys_tasks';"
                .to_string(),
        ))
        .await?;

    if check_tasks.is_empty() {
        tracing::info!("Creating sys_tasks table");
        let create_tasks_sql = r#"
            CREATE TABLE sys_tasks (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL UNIQUE,
                description TEXT,
                task_type TEXT NOT NULL,
                schedule_cron TEXT,
                config_json TEXT,
                is_enabled INTEGER NOT NULL DEFAULT 1,
                last_run_at TEXT,
                next_run_at TEXT,
                last_run_status TEXT,
                last_run_log_file TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                is_deleted INTEGER NOT NULL DEFAULT 0
            );
        "#;
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            create_tasks_sql.to_string(),
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

/// Migrate existing WB Sales documents to fill denormalized columns from JSON
/// This function should be called once to backfill all denormalized fields
pub async fn migrate_wb_sales_denormalize() -> anyhow::Result<u64> {
    let conn = get_connection();

    // Get all WB Sales documents that need migration (sale_date is null means not migrated)
    let sql = r#"
        SELECT id, header_json, line_json, state_json, source_meta_json 
        FROM a012_wb_sales 
        WHERE sale_date IS NULL
    "#;

    let rows = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            sql.to_string(),
        ))
        .await?;

    let total = rows.len();
    if total == 0 {
        tracing::info!("No WB Sales documents need denormalization migration");
        return Ok(0);
    }

    tracing::info!(
        "Found {} WB Sales documents to denormalize, starting migration...",
        total
    );

    let mut updated = 0u64;
    for row in rows {
        let id: String = row.try_get("", "id")?;
        let header_json: String = row.try_get("", "header_json")?;
        let line_json: String = row.try_get("", "line_json")?;
        let state_json: String = row.try_get("", "state_json")?;
        let source_meta_json: String = row.try_get("", "source_meta_json")?;

        // Parse JSON fields
        let header: serde_json::Value = serde_json::from_str(&header_json).unwrap_or_default();
        let line: serde_json::Value = serde_json::from_str(&line_json).unwrap_or_default();
        let state: serde_json::Value = serde_json::from_str(&state_json).unwrap_or_default();
        let source_meta: serde_json::Value =
            serde_json::from_str(&source_meta_json).unwrap_or_default();

        // Extract values
        let sale_id = header
            .get("sale_id")
            .and_then(|v| v.as_str())
            .map(|s| s.replace("'", "''"));
        let organization_id = header
            .get("organization_id")
            .and_then(|v| v.as_str())
            .map(|s| s.replace("'", "''"));
        let connection_id = header
            .get("connection_id")
            .and_then(|v| v.as_str())
            .map(|s| s.replace("'", "''"));

        let supplier_article = line
            .get("supplier_article")
            .and_then(|v| v.as_str())
            .map(|s| s.replace("'", "''"));
        let nm_id = line.get("nm_id").and_then(|v| v.as_i64());
        let barcode = line
            .get("barcode")
            .and_then(|v| v.as_str())
            .map(|s| s.replace("'", "''"));
        let product_name = line
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.replace("'", "''"));
        let qty = line.get("qty").and_then(|v| v.as_f64());
        let amount_line = line.get("amount_line").and_then(|v| v.as_f64());
        let total_price = line.get("total_price").and_then(|v| v.as_f64());
        let finished_price = line.get("finished_price").and_then(|v| v.as_f64());

        let event_type = state
            .get("event_type")
            .and_then(|v| v.as_str())
            .map(|s| s.replace("'", "''"));
        let sale_dt = state
            .get("sale_dt")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Try to get sale_id from raw JSON if not in header
        let sale_id = if sale_id.is_none() {
            let raw_payload_ref = source_meta.get("raw_payload_ref").and_then(|v| v.as_str());
            if let Some(ref_id) = raw_payload_ref {
                let raw_sql = format!(
                    "SELECT raw_json FROM document_raw_storage WHERE id = '{}'",
                    ref_id
                );
                let raw_result = conn
                    .query_one(Statement::from_string(DatabaseBackend::Sqlite, raw_sql))
                    .await
                    .ok()
                    .flatten();

                if let Some(raw_row) = raw_result {
                    if let Ok(raw_json_str) = raw_row.try_get::<String>("", "raw_json") {
                        let raw: serde_json::Value =
                            serde_json::from_str(&raw_json_str).unwrap_or_default();
                        raw.get("saleID")
                            .and_then(|v| v.as_str())
                            .map(|s| s.replace("'", "''"))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            sale_id
        };

        // Build UPDATE statement
        let mut sets = Vec::new();

        if let Some(v) = sale_id {
            sets.push(format!("sale_id = '{}'", v));
        }
        if let Some(v) = sale_dt {
            sets.push(format!("sale_date = '{}'", v));
        }
        if let Some(v) = organization_id {
            sets.push(format!("organization_id = '{}'", v));
        }
        if let Some(v) = connection_id {
            sets.push(format!("connection_id = '{}'", v));
        }
        if let Some(v) = supplier_article {
            sets.push(format!("supplier_article = '{}'", v));
        }
        if let Some(v) = nm_id {
            sets.push(format!("nm_id = {}", v));
        }
        if let Some(v) = barcode {
            sets.push(format!("barcode = '{}'", v));
        }
        if let Some(v) = product_name {
            sets.push(format!("product_name = '{}'", v));
        }
        if let Some(v) = qty {
            sets.push(format!("qty = {}", v));
        }
        if let Some(v) = amount_line {
            sets.push(format!("amount_line = {}", v));
        }
        if let Some(v) = total_price {
            sets.push(format!("total_price = {}", v));
        }
        if let Some(v) = finished_price {
            sets.push(format!("finished_price = {}", v));
        }
        if let Some(v) = event_type {
            sets.push(format!("event_type = '{}'", v));
        }

        if sets.is_empty() {
            continue;
        }

        let update_sql = format!(
            "UPDATE a012_wb_sales SET {} WHERE id = '{}'",
            sets.join(", "),
            id
        );

        conn.execute(Statement::from_string(DatabaseBackend::Sqlite, update_sql))
            .await?;

        updated += 1;

        if updated % 100 == 0 {
            tracing::info!(
                "Denormalization progress: {}/{} documents updated",
                updated,
                total
            );
        }
    }

    tracing::info!(
        "WB Sales denormalization completed: {} documents updated",
        updated
    );
    Ok(updated)
}
