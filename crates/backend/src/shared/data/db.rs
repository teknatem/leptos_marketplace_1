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
                mp_ref_count INTEGER NOT NULL DEFAULT 0,
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
        // Ensure mp_ref_count column exists; add if missing
        let pragma = format!("PRAGMA table_info('{}');", "a004_nomenclature");
        let cols = conn
            .query_all(Statement::from_string(DatabaseBackend::Sqlite, pragma))
            .await?;
        let mut has_mp_ref_count = false;
        for row in cols {
            let name: String = row.try_get("", "name").unwrap_or_default();
            if name == "mp_ref_count" {
                has_mp_ref_count = true;
                break;
            }
        }
        if !has_mp_ref_count {
            tracing::info!("Adding mp_ref_count column to a004_nomenclature");
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "ALTER TABLE a004_nomenclature ADD COLUMN mp_ref_count INTEGER NOT NULL DEFAULT 0;".to_string(),
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
                marketplace_id TEXT NOT NULL,
                connection_mp_id TEXT NOT NULL DEFAULT '',
                marketplace_sku TEXT NOT NULL,
                barcode TEXT,
                art TEXT NOT NULL,
                product_name TEXT NOT NULL,
                brand TEXT,
                category_id TEXT,
                category_name TEXT,
                price REAL,
                stock INTEGER,
                last_update TEXT,
                marketplace_url TEXT,
                nomenclature_id TEXT,
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
        // Ensure connection_mp_id column exists; add if missing
        let pragma = format!("PRAGMA table_info('{}');", "a007_marketplace_product");
        let cols = conn
            .query_all(Statement::from_string(DatabaseBackend::Sqlite, pragma))
            .await?;
        let mut has_connection_mp_id = false;
        for row in cols {
            let name: String = row.try_get("", "name").unwrap_or_default();
            if name == "connection_mp_id" {
                has_connection_mp_id = true;
            }
        }
        if !has_connection_mp_id {
            tracing::info!("Adding connection_mp_id column to a007_marketplace_product");
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "ALTER TABLE a007_marketplace_product ADD COLUMN connection_mp_id TEXT NOT NULL DEFAULT '';".to_string(),
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

    // document_raw_storage table - для хранения сырых JSON от маркетплейсов
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

        // Создать индекс для быстрого поиска по marketplace + document_type + document_no
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

    // p900_sales_register table - унифицированный регистр продаж
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

        // Создать индексы для быстрого поиска
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
    }

    // a010_ozon_fbs_posting table - документы OZON FBS
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

    // a011_ozon_fbo_posting table - документы OZON FBO
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

    // a012_wb_sales table - документы Wildberries Sales
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
                header_json TEXT NOT NULL,
                line_json TEXT NOT NULL,
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

    // a013_ym_order table - документы Yandex Market Orders
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
