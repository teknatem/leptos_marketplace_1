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
    let create_connection_1c_table_sql = r#"
        CREATE TABLE IF NOT EXISTS connection_1c_database (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            description TEXT NOT NULL,
            url TEXT NOT NULL,
            comment TEXT,
            login TEXT NOT NULL,
            password TEXT NOT NULL,
            is_primary INTEGER NOT NULL DEFAULT 0,
            is_deleted INTEGER NOT NULL DEFAULT 0,
            created_at TEXT,
            updated_at TEXT
        );
    "#;
    conn.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        create_connection_1c_table_sql.to_string(),
    ))
    .await?;
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
