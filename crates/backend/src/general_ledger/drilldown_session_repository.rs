use anyhow::Result;
use sea_orm::{ConnectionTrait, Statement, Value};
use uuid::Uuid;

use crate::shared::data::db::get_connection;
use contracts::general_ledger::{
    GlDrilldownQuery, GlDrilldownSessionCreate, GlDrilldownSessionCreateResponse,
    GlDrilldownSessionRecord,
};

fn conn() -> &'static sea_orm::DatabaseConnection {
    get_connection()
}

fn string_value(value: impl Into<String>) -> Value {
    Value::String(Some(Box::new(value.into())))
}

pub async fn create_session(
    body: &GlDrilldownSessionCreate,
) -> Result<GlDrilldownSessionCreateResponse> {
    let session_id = Uuid::new_v4().to_string();
    let params_json = serde_json::to_string(&body.query)?;
    let stmt = Statement::from_sql_and_values(
        conn().get_database_backend(),
        r#"
        INSERT INTO sys_gl_drilldown (id, title, params_json)
        VALUES (?, ?, ?)
        "#,
        vec![
            string_value(session_id.clone()),
            string_value(body.title.clone().unwrap_or_default()),
            string_value(params_json),
        ],
    );

    conn().execute(stmt).await?;

    Ok(GlDrilldownSessionCreateResponse { session_id })
}

pub async fn get_session(id: &str) -> Result<Option<GlDrilldownSessionRecord>> {
    let stmt = Statement::from_sql_and_values(
        conn().get_database_backend(),
        r#"
        SELECT id, title, params_json, created_at, last_used_at, use_count
        FROM sys_gl_drilldown
        WHERE id = ?
        "#,
        vec![string_value(id.to_string())],
    );

    let Some(row) = conn().query_one(stmt).await? else {
        return Ok(None);
    };

    let params_json: String = row.try_get("", "params_json")?;
    let query: GlDrilldownQuery = serde_json::from_str(&params_json)?;

    Ok(Some(GlDrilldownSessionRecord {
        id: row.try_get("", "id")?,
        title: row.try_get("", "title").unwrap_or_default(),
        created_at: row.try_get("", "created_at").unwrap_or_default(),
        last_used_at: row.try_get("", "last_used_at").ok(),
        use_count: row.try_get("", "use_count").unwrap_or(0),
        query,
    }))
}

pub async fn touch_session(id: &str) -> Result<()> {
    let stmt = Statement::from_sql_and_values(
        conn().get_database_backend(),
        r#"
        UPDATE sys_gl_drilldown
        SET use_count = use_count + 1,
            last_used_at = strftime('%Y-%m-%dT%H:%M:%S', 'now')
        WHERE id = ?
        "#,
        vec![string_value(id.to_string())],
    );

    conn().execute(stmt).await?;
    Ok(())
}
