use anyhow::Result;
use chrono::Utc;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

use crate::shared::data::db::get_connection;

pub async fn get_setting(key: &str) -> Result<Option<String>> {
    let conn = get_connection();
    let row = conn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT value FROM sys_settings WHERE key = ?",
            vec![key.into()],
        ))
        .await?;

    Ok(row.and_then(|r| r.try_get::<String>("", "value").ok()))
}

pub async fn set_setting(key: &str, value: &str) -> Result<()> {
    let conn = get_connection();
    let now = Utc::now().to_rfc3339();
    conn.execute(Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        "INSERT INTO sys_settings (key, value, created_at, updated_at) VALUES (?, ?, ?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        vec![key.into(), value.into(), now.clone().into(), now.into()],
    ))
    .await?;
    Ok(())
}
