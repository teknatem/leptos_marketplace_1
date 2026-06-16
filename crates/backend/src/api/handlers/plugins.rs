//! HTTP-обработчики подсистемы Plugins (admin-only).

use axum::body::Bytes;
use axum::http::header;
use axum::response::IntoResponse;
use axum::{
    extract::{Path, Query},
    Json,
};
use serde::Deserialize;
use serde_json::json;

use crate::plugins::service;
use contracts::plugins::{
    PluginBundle, PluginDefinition, PluginError, PluginInvokeRequest, PluginListItem,
    PluginRunBrief, PluginRunContext, PluginStats, PluginUpsert, PluginValidateReport,
};

#[derive(Deserialize)]
pub struct DaysQuery {
    #[serde(default)]
    days: Option<i64>,
}
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
/// Возвращает отчёт с перечнем серверных экспортов и структурированными ошибками.
pub async fn validate(Json(bundle): Json<PluginBundle>) -> Json<PluginValidateReport> {
    Json(service::validate(&bundle).await)
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

/// GET /api/plugin/:id/stats?days=N — статистика запусков плагина (сводка + последние).
pub async fn stats(
    Path(id): Path<String>,
    Query(q): Query<DaysQuery>,
) -> Result<Json<PluginStats>, axum::http::StatusCode> {
    let days = q.days.unwrap_or(7).clamp(1, 365);
    match service::stats(&id, days).await {
        Ok(stats) => Ok(Json(stats)),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /api/plugin/runs/summary?days=N — краткие сводки по всем плагинам (для реестра).
pub async fn runs_summary(
    Query(q): Query<DaysQuery>,
) -> Result<Json<Vec<PluginRunBrief>>, axum::http::StatusCode> {
    let days = q.days.unwrap_or(7).clamp(1, 365);
    match service::runs_summary(days).await {
        Ok(items) => Ok(Json(items)),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /api/plugin/:id/export — скачать плагин как zip-архив переносимого бандла.
pub async fn export(Path(id): Path<String>) -> Result<impl IntoResponse, axum::http::StatusCode> {
    match service::export(&id).await {
        Ok((filename, bytes)) => Ok((
            [
                (header::CONTENT_TYPE, "application/zip".to_string()),
                (
                    header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{filename}\""),
                ),
            ],
            bytes,
        )),
        Err(_) => Err(axum::http::StatusCode::NOT_FOUND),
    }
}

/// POST /api/plugin/import — импортировать плагин из zip-архива (сырые байты в теле).
pub async fn import(
    body: Bytes,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    match service::import(&body).await {
        Ok(outcome) => Ok(Json(json!({
            "ok": outcome.id.is_some(),
            "id": outcome.id,
            "code": outcome.code,
            "validate": outcome.report,
        }))),
        Err(e) => Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )),
    }
}

/// POST /api/plugin/:id/invoke: вызвать экспортированную функцию server_script.
pub async fn invoke(
    Path(id): Path<String>,
    Json(request): Json<PluginInvokeRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    match service::invoke(&id, request).await {
        Ok((value, logs)) => Ok(Json(json!({ "ok": true, "result": value, "logs": logs }))),
        Err(e) => {
            // `error` остаётся строкой (совместимость с фронтендом), `error_detail`
            // несёт структуру { stage, message, stack } для UI-раннера и LLM-агента.
            let detail = e.downcast_ref::<PluginError>().cloned();
            Err((
                axum::http::StatusCode::BAD_REQUEST,
                Json(json!({ "error": e.to_string(), "error_detail": detail })),
            ))
        }
    }
}
