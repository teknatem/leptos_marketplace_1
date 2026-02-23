use crate::shared::config;
use once_cell::sync::OnceCell;
use sea_orm::{ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, Statement};
use std::path::Path;

static DB_CONN: OnceCell<DatabaseConnection> = OnceCell::new();

fn build_sqlite_url(path: &Path) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    let needs_leading_slash = !normalized.starts_with('/') && normalized.contains(':');
    let prefix = if needs_leading_slash { "/" } else { "" };
    format!("sqlite://{}{}?mode=rwc", prefix, normalized)
}

pub async fn initialize_database() -> anyhow::Result<()> {
    if DB_CONN.get().is_some() {
        return Ok(());
    }

    let cfg = config::load_config()?;
    let absolute_path = config::get_database_path(&cfg)?;

    if let Some(parent) = absolute_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db_url = build_sqlite_url(&absolute_path);
    tracing::info!("Connecting to database: {}", db_url);
    let conn = Database::connect(&db_url).await?;

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
