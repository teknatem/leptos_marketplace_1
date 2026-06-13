//! HTTP-обработчики подсистемы Plugins (admin-only).

use axum::{extract::Path, Json};
use serde_json::json;

use crate::plugins::service;
use contracts::plugins::{
    PluginBundle, PluginDefinition, PluginListItem, PluginRunContext, PluginUpsert,
};
use contracts::shared::drilldown::DrilldownResponse;

/// GET /api/plugin — список включённых плагинов (для меню/навигатора).
pub async fn list() -> Result<Json<Vec<PluginListItem>>, axum::http::StatusCode> {
    match service::list_enabled().await {
        Ok(defs) => Ok(Json(defs.iter().map(PluginListItem::from).collect())),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /api/plugin/all — все плагины (страница управления).
pub async fn list_all() -> Result<Json<Vec<PluginListItem>>, axum::http::StatusCode> {
    match service::list_all().await {
        Ok(defs) => Ok(Json(defs.iter().map(PluginListItem::from).collect())),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /api/plugin/:id — полное определение/бандл (включая исходники).
pub async fn get_by_id(
    Path(id): Path<String>,
) -> Result<Json<PluginDefinition>, axum::http::StatusCode> {
    match service::get_by_id(&id).await {
        Ok(Some(def)) => Ok(Json(def)),
        Ok(None) => Err(axum::http::StatusCode::NOT_FOUND),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /api/plugin — создать или обновить плагин.
pub async fn upsert(
    Json(dto): Json<PluginUpsert>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    match service::upsert(dto).await {
        Ok(id) => Ok(Json(json!({ "id": id }))),
        Err(e) => Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )),
    }
}

/// DELETE /api/plugin/:id — мягкое удаление.
pub async fn delete(Path(id): Path<String>) -> Result<(), axum::http::StatusCode> {
    match service::delete(&id).await {
        Ok(()) => Ok(()),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /api/plugin/validate — проверка бандла без сохранения.
pub async fn validate(Json(bundle): Json<PluginBundle>) -> Json<serde_json::Value> {
    match service::validate(&bundle) {
        Ok(()) => Json(json!({ "ok": true })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

/// POST /api/plugin/testdata — вставить демонстрационный плагин.
pub async fn testdata() -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    match service::insert_test_data().await {
        Ok(()) => Ok(Json(json!({ "ok": true }))),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /api/plugin/:id/data — выполнить декларативную привязку данных (DataView).
pub async fn run_data(
    Path(id): Path<String>,
    Json(ctx): Json<PluginRunContext>,
) -> Result<Json<DrilldownResponse>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    match service::run_data(&id, &ctx).await {
        Ok(resp) => Ok(Json(resp)),
        Err(e) => Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )),
    }
}

/// POST /api/plugin/:id/run — исполнить server_script плагина (Rhai на сервере).
pub async fn run_script(
    Path(id): Path<String>,
    Json(ctx): Json<PluginRunContext>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    match service::run_script(&id, &ctx).await {
        Ok((value, logs)) => Ok(Json(json!({ "ok": true, "result": value, "logs": logs }))),
        Err(e) => Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )),
    }
}
