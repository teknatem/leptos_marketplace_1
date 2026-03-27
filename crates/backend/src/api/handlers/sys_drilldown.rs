//! API handlers for Drilldown Session Store (sys_drilldown).
//!
//! POST /api/sys-drilldown           → create session, return {session_id}
//! GET  /api/sys-drilldown/:id       → get session params (increments use_count)
//! GET  /api/sys-drilldown/:id/data  → get params AND execute drilldown query

use axum::{extract::Path, http::StatusCode, response::Json};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::data_view::DataViewRegistry;
use contracts::shared::data_view::ViewContext;
use contracts::shared::drilldown::DrilldownResponse;

// ── Request body for POST ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct DrilldownSessionCreate {
    pub view_id: String,
    pub indicator_id: Option<String>,
    pub indicator_name: Option<String>,
    #[serde(default)]
    pub metric_id: Option<String>,
    /// Multi-resource режим: список выбранных resource id.
    #[serde(default)]
    pub metric_ids: Vec<String>,
    pub group_by: String,
    pub group_by_label: Option<String>,
    pub date_from: String,
    pub date_to: String,
    pub period2_from: Option<String>,
    pub period2_to: Option<String>,
    #[serde(default)]
    pub connection_mp_refs: Vec<String>,
    #[serde(default)]
    pub params: std::collections::HashMap<String, String>,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn get_db() -> &'static sea_orm::DatabaseConnection {
    crate::shared::data::db::get_connection()
}

async fn fetch_session(id: &str) -> Result<Value, (StatusCode, String)> {
    let conn = get_db();
    let sql = format!(
        "SELECT id, view_id, indicator_id, indicator_name, params_json, created_at, last_used_at, use_count \
         FROM sys_drilldown WHERE id = '{}'",
        id.replace('\'', "''")
    );
    let row = conn
        .query_one(Statement::from_string(DatabaseBackend::Sqlite, sql))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Drilldown session not found: {}", id),
            )
        })?;

    let params_json_str: String = row
        .try_get("", "params_json")
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let params: Value = serde_json::from_str(&params_json_str)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(json!({
        "id": row.try_get::<String>("", "id").unwrap_or_default(),
        "view_id": row.try_get::<String>("", "view_id").unwrap_or_default(),
        "indicator_id": row.try_get::<String>("", "indicator_id").unwrap_or_default(),
        "indicator_name": row.try_get::<String>("", "indicator_name").unwrap_or_default(),
        "created_at": row.try_get::<String>("", "created_at").unwrap_or_default(),
        "last_used_at": row.try_get::<Option<String>>("", "last_used_at").unwrap_or_default(),
        "use_count": row.try_get::<i64>("", "use_count").unwrap_or(0),
        "params": params,
    }))
}

async fn touch_session(id: &str) {
    let conn = get_db();
    let sql = format!(
        "UPDATE sys_drilldown \
         SET use_count = use_count + 1, \
             last_used_at = strftime('%Y-%m-%dT%H:%M:%S', 'now') \
         WHERE id = '{}'",
        id.replace('\'', "''")
    );
    let _ = conn
        .execute(Statement::from_string(DatabaseBackend::Sqlite, sql))
        .await;
}

// ── POST /api/sys-drilldown ──────────────────────────────────────────────────

/// Create a new drilldown session.
/// Returns `{session_id}` which becomes the tab key suffix.
pub async fn create(
    Json(body): Json<DrilldownSessionCreate>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let id = Uuid::new_v4().to_string();

    let params = json!({
        "view_id": body.view_id,
        "metric_id": body.metric_id,
        "metric_ids": body.metric_ids,
        "group_by": body.group_by,
        "group_by_label": body.group_by_label.unwrap_or_default(),
        "date_from": body.date_from,
        "date_to": body.date_to,
        "period2_from": body.period2_from,
        "period2_to": body.period2_to,
        "connection_mp_refs": body.connection_mp_refs,
        "params": body.params,
    });
    let params_str = serde_json::to_string(&params)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .replace('\'', "''");

    let indicator_id = body.indicator_id.unwrap_or_default().replace('\'', "''");
    let indicator_name = body.indicator_name.unwrap_or_default().replace('\'', "''");
    let view_id = body.view_id.replace('\'', "''");

    let sql = format!(
        "INSERT INTO sys_drilldown (id, view_id, indicator_id, indicator_name, params_json) \
         VALUES ('{id}', '{view_id}', '{indicator_id}', '{indicator_name}', '{params_str}')"
    );

    get_db()
        .execute(Statement::from_string(DatabaseBackend::Sqlite, sql))
        .await
        .map_err(|e| {
            tracing::error!("sys_drilldown INSERT error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

    Ok(Json(json!({ "session_id": id })))
}

// ── GET /api/sys-drilldown/:id ───────────────────────────────────────────────

/// Return stored session params and increment use_count.
pub async fn get_by_id(Path(id): Path<String>) -> Result<Json<Value>, (StatusCode, String)> {
    let result = fetch_session(&id).await?;
    touch_session(&id).await;
    Ok(Json(result))
}

// ── GET /api/sys-drilldown/:id/data ─────────────────────────────────────────

/// Return stored session params AND execute the drilldown query.
/// Useful for sharing links — caller doesn't need a separate POST.
pub async fn get_data(
    Path(id): Path<String>,
) -> Result<Json<DrilldownResponse>, (StatusCode, String)> {
    let record = fetch_session(&id).await?;
    touch_session(&id).await;

    let params = &record["params"];

    let view_id = params["view_id"].as_str().unwrap_or("").to_string();
    let group_by = params["group_by"].as_str().unwrap_or("").to_string();
    let date_from = params["date_from"].as_str().unwrap_or("").to_string();
    let date_to = params["date_to"].as_str().unwrap_or("").to_string();
    let period2_from = params["period2_from"].as_str().map(String::from);
    let period2_to = params["period2_to"].as_str().map(String::from);
    let metric_id = params["metric_id"].as_str().map(String::from);
    let metric_ids: Vec<String> = params["metric_ids"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let connection_mp_refs: Vec<String> = params["connection_mp_refs"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let mut extra_params: std::collections::HashMap<String, String> = params["params"]
        .as_object()
        .map(|obj| {
            obj.iter()
                .filter_map(|(key, value)| {
                    value.as_str().map(|value| (key.clone(), value.to_string()))
                })
                .collect()
        })
        .unwrap_or_default();
    if let Some(metric_id) = metric_id.filter(|value| !value.trim().is_empty()) {
        extra_params.insert("metric".to_string(), metric_id);
    }

    let registry = DataViewRegistry::new();
    if !registry.has_view(&view_id) {
        return Err((
            StatusCode::NOT_FOUND,
            format!("DataView not found: {}", view_id),
        ));
    }

    let ctx = ViewContext {
        date_from,
        date_to,
        period2_from,
        period2_to,
        connection_mp_refs,
        params: extra_params,
    };

    registry
        .compute_drilldown(&view_id, &ctx, &group_by, &metric_ids)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("sys_drilldown/data error for {}: {}", id, e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })
}
