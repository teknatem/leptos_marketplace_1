#[cfg(feature = "server")]
use anyhow::Context;
#[cfg(feature = "server")]
use once_cell::sync::OnceCell;
#[cfg(feature = "server")]
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, Statement};

#[cfg(feature = "server")]
use crate::shared::data::registry::all_tables;
#[cfg(feature = "server")]
use crate::shared::data::schema::{ColumnDefinition, SimpleColumnType, TableDefinition};

#[cfg(feature = "server")]
fn sqlite_type_sql(column_type: &SimpleColumnType) -> &'static str {
    match column_type {
        SimpleColumnType::Integer => "INTEGER",
        SimpleColumnType::BigInteger => "INTEGER",
        SimpleColumnType::Float => "REAL",
        SimpleColumnType::Double => "REAL",
        SimpleColumnType::Decimal { .. } => "NUMERIC",
        SimpleColumnType::String { .. } => "TEXT",
        SimpleColumnType::Text => "TEXT",
        SimpleColumnType::Boolean => "INTEGER",
        SimpleColumnType::DateTime => "TEXT",
        SimpleColumnType::Json => "TEXT",
    }
}

#[cfg(feature = "server")]
fn column_sql(col: &ColumnDefinition) -> String {
    let mut parts: Vec<String> = Vec::new();
    parts.push(format!(
        "{} {}",
        col.name,
        sqlite_type_sql(&col.column_type)
    ));
    if !col.nullable {
        parts.push("NOT NULL".to_string());
    }
    if col.primary_key {
        parts.push("PRIMARY KEY".to_string());
    }
    if col.unique {
        parts.push("UNIQUE".to_string());
    }
    parts.join(" ")
}

#[cfg(feature = "server")]
async fn ensure_table(conn: &DatabaseConnection, table: &TableDefinition) -> anyhow::Result<()> {
    let cols_sql = table
        .columns
        .iter()
        .map(column_sql)
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("CREATE TABLE IF NOT EXISTS {} ({})", table.name, cols_sql);
    conn.execute(Statement::from_string(conn.get_database_backend(), sql))
        .await
        .with_context(|| format!("creating table {}", table.name))?;
    Ok(())
}

#[cfg(feature = "server")]
async fn ensure_columns(conn: &DatabaseConnection, table: &TableDefinition) -> anyhow::Result<()> {
    let pragma_sql = format!("PRAGMA table_info('{}')", table.name);
    let rows = conn
        .query_all(Statement::from_string(
            conn.get_database_backend(),
            pragma_sql,
        ))
        .await
        .with_context(|| format!("reading pragma for {}", table.name))?;

    let mut existing: std::collections::HashSet<String> = std::collections::HashSet::new();
    for row in rows {
        if let Some(name) = row.try_get::<String>("", "name").ok() {
            existing.insert(name);
        }
    }

    for col in &table.columns {
        if !existing.contains(col.name.as_ref()) {
            let sql = format!("ALTER TABLE {} ADD COLUMN {}", table.name, column_sql(col));
            conn.execute(Statement::from_string(conn.get_database_backend(), sql))
                .await
                .with_context(|| format!("adding column {}.{}", table.name, col.name))?;
        }
    }
    Ok(())
}

#[cfg(feature = "server")]
pub async fn initialize_database(db_path: Option<&str>) -> anyhow::Result<()> {
    // Determine the absolute path for the database file
    let path_buf = match db_path {
        Some(path) => {
            let p = std::path::Path::new(path);
            if p.is_absolute() {
                p.to_path_buf()
            } else {
                let cwd = std::env::current_dir()?;
                cwd.join(p)
            }
        }
        None => {
            let cwd = std::env::current_dir()?;
            cwd.join("app.db")
        }
    };

    let final_db_path_str = path_buf
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid database path"))?;

    // Normalize Windows backslashes to forward slashes for URL compatibility
    #[cfg(target_os = "windows")]
    let path_for_url = final_db_path_str.replace('\\', "/");
    #[cfg(not(target_os = "windows"))]
    let path_for_url = final_db_path_str.to_string();

    let db_url = format!("sqlite:///{}", path_for_url);

    // Create the parent directory if it doesn't exist
    if let Some(parent) = path_buf.parent() {
        if !parent.exists() {
            tracing::info!("Creating database directory at: {:?}", parent);
            std::fs::create_dir_all(parent)?;
        }
    }

    // Pre-create the DB file if it does not exist to avoid driver-specific quirks
    if !path_buf.exists() {
        tracing::info!("Creating database file at: {:?}", path_buf);
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&path_buf)?;
    }

    tracing::info!("Connecting to database at: {}", db_url);
    let conn = match Database::connect(&db_url).await {
        Ok(conn) => conn,
        Err(db_err) => {
            tracing::error!(
                "Failed to connect to the database. Detailed Error: {:?}",
                db_err
            );
            return Err(anyhow::Error::new(db_err).context("connect sqlite"));
        }
    };

    // store in global singleton for reuse across handlers
    set_global_connection(conn.clone());

    for table in all_tables() {
        ensure_table(&conn, &table).await?;
        ensure_columns(&conn, &table).await?;
    }
    Ok(())
}

// Global connection singleton for server-side code
#[cfg(feature = "server")]
static GLOBAL_DB: OnceCell<DatabaseConnection> = OnceCell::new();

#[cfg(feature = "server")]
pub fn get_connection() -> &'static DatabaseConnection {
    GLOBAL_DB
        .get()
        .expect("Database not initialized. Call init_data_layer() first")
}

#[cfg(feature = "server")]
fn set_global_connection(conn: DatabaseConnection) {
    let _ = GLOBAL_DB.set(conn);
}
