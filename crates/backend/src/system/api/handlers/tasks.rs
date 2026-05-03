use crate::system::tasks::abort_registry;
use crate::system::tasks::logger::get_global_task_logger;
use crate::system::tasks::registry::get_global_registry;
use crate::system::tasks::runs_service;
use crate::system::tasks::service;
use crate::system::tasks::task_session_runner::{spawn_task_session, TaskSessionParams};
use axum::{extract::Path, extract::Query, response::IntoResponse, Json};
use chrono::Utc;
use contracts::domain::common::AggregateId;
use contracts::system::tasks::aggregate::ScheduledTaskId;
use contracts::system::tasks::metadata::TaskMetadataDto;
use contracts::system::tasks::progress::{TaskProgressResponse, TaskStatus};
use contracts::system::tasks::request::{
    CreateScheduledTaskDto, SetWatermarkDto, ToggleScheduledTaskEnabledDto, UpdateScheduledTaskDto,
};
use contracts::system::tasks::response::{ScheduledTaskListResponse, ScheduledTaskResponse};
use contracts::system::tasks::runs::{
    LiveMemoryProgressResponse, RecentRunsResponse, RunTaskResponse, TaskRunListResponse,
};
use serde::Deserialize;
use uuid::Uuid;

/// GET /api/sys/tasks
pub async fn list_scheduled_tasks(
) -> Result<Json<ScheduledTaskListResponse>, axum::http::StatusCode> {
    match service::list_all().await {
        Ok(tasks) => {
            let responses = tasks.into_iter().map(|t| t.into()).collect();
            Ok(Json(ScheduledTaskListResponse { tasks: responses }))
        }
        Err(e) => {
            tracing::error!("Failed to list scheduled tasks: {}", e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/sys/tasks/:id
pub async fn get_scheduled_task(
    Path(id): Path<String>,
) -> Result<Json<ScheduledTaskResponse>, axum::http::StatusCode> {
    let task_id =
        ScheduledTaskId::from_string(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    match service::get_by_id(&task_id).await {
        Ok(Some(task)) => Ok(Json(task.into())),
        Ok(None) => Err(axum::http::StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to get scheduled task {}: {}", id, e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// POST /api/sys/tasks
pub async fn create_scheduled_task(
    Json(dto): Json<CreateScheduledTaskDto>,
) -> Result<Json<ScheduledTaskResponse>, axum::http::StatusCode> {
    match service::create(dto).await {
        Ok(task_id) => match service::get_by_id(&task_id).await {
            Ok(Some(task)) => Ok(Json(task.into())),
            _ => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
        },
        Err(e) => {
            tracing::error!("Failed to create scheduled task: {}", e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// PUT /api/sys/tasks/:id
pub async fn update_scheduled_task(
    Path(id): Path<String>,
    Json(dto): Json<UpdateScheduledTaskDto>,
) -> Result<Json<ScheduledTaskResponse>, axum::http::StatusCode> {
    let task_id =
        ScheduledTaskId::from_string(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    match service::update(&task_id, dto).await {
        Ok(_) => match service::get_by_id(&task_id).await {
            Ok(Some(task)) => Ok(Json(task.into())),
            _ => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
        },
        Err(e) => {
            tracing::error!("Failed to update scheduled task {}: {}", id, e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// DELETE /api/sys/tasks/:id
pub async fn delete_scheduled_task(
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, axum::http::StatusCode> {
    let task_id =
        ScheduledTaskId::from_string(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    match service::delete(&task_id).await {
        Ok(_) => Ok(axum::http::StatusCode::NO_CONTENT),
        Err(e) => {
            tracing::error!("Failed to delete scheduled task {}: {}", id, e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// POST /api/sys/tasks/:id/toggle_enabled
pub async fn toggle_scheduled_task_enabled(
    Path(id): Path<String>,
    Json(dto): Json<ToggleScheduledTaskEnabledDto>,
) -> Result<Json<ScheduledTaskResponse>, axum::http::StatusCode> {
    let task_id =
        ScheduledTaskId::from_string(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    match service::toggle_enabled(&task_id, dto.is_enabled).await {
        Ok(_) => match service::get_by_id(&task_id).await {
            Ok(Some(task)) => Ok(Json(task.into())),
            _ => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
        },
        Err(e) => {
            tracing::error!(
                "Failed to toggle scheduled task {} enabled status: {}",
                id,
                e
            );
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// POST /api/sys/tasks/:id/run — ручной запуск задачи.
/// 409 Conflict + body `TaskRun`, если для задачи уже есть активный запуск.
pub async fn run_task_now(
    Path(id): Path<String>,
) -> Result<axum::response::Response, axum::http::StatusCode> {
    let task_id =
        ScheduledTaskId::from_string(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    let task = service::get_by_id(&task_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get task {}: {}", id, e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;

    if let Ok(Some(existing)) = runs_service::get_running_for_task(&id).await {
        return Ok((axum::http::StatusCode::CONFLICT, Json(existing)).into_response());
    }

    let registry = get_global_registry()
        .ok_or_else(|| {
            tracing::warn!("Task registry not initialized");
            axum::http::StatusCode::SERVICE_UNAVAILABLE
        })?
        .clone();

    // Убеждаемся что тип задачи известен до создания записи в БД.
    if registry.get(&task.task_type).is_none() {
        tracing::warn!("No manager for task type '{}'", task.task_type);
        return Err(axum::http::StatusCode::NOT_IMPLEMENTED);
    }

    let logger = get_global_task_logger();
    let session_id = Uuid::new_v4().to_string();
    let log_file_path = logger.get_log_file_path(&session_id);
    let now = Utc::now();
    let next_run_at = task.next_run_at.unwrap_or(now);

    if let Err(e) = runs_service::create_run_with_log(
        task_id,
        session_id.clone(),
        "Manual".to_string(),
        log_file_path.clone(),
    )
    .await
    {
        tracing::warn!("Failed to record manual run start: {}", e);
    }

    let _ = service::update_run_status(
        &task_id,
        Some(now),
        task.next_run_at,
        Some(log_file_path),
        Some(TaskStatus::Running.to_string()),
        None,
    )
    .await;

    spawn_task_session(TaskSessionParams {
        task,
        session_id: session_id.clone(),
        started_at: now,
        next_run_at,
        logger,
        registry,
    });

    Ok(Json(RunTaskResponse {
        session_id,
        task_id: id,
    })
    .into_response())
}

/// GET /api/sys/tasks/:id/progress/:session_id
pub async fn get_task_progress(
    Path((task_id_str, session_id)): Path<(String, String)>,
) -> Result<Json<TaskProgressResponse>, axum::http::StatusCode> {
    let task_id = ScheduledTaskId::from_string(&task_id_str)
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    let task = service::get_by_id(&task_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get task {}: {}", task_id_str, e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;

    let manager = get_global_registry().and_then(|r| r.get(&task.task_type));

    match manager {
        Some(mgr) => {
            if let Some(progress) = mgr.get_progress(&session_id) {
                Ok(Json(progress.into()))
            } else {
                match get_global_task_logger().read_log(&session_id) {
                    Ok(log_content) => Ok(Json(TaskProgressResponse {
                        session_id: session_id.clone(),
                        status: task.last_run_status.clone().unwrap_or_default(),
                        message: "Log available".to_string(),
                        total_items: None,
                        processed_items: None,
                        errors: None,
                        current_item: None,
                        log_content: Some(log_content),
                        total_inserted: None,
                        total_updated: None,
                        detail: Some(
                            contracts::system::tasks::progress::TaskProgressDetail::Indeterminate {
                                hint: Some("Доступен лог выполнения".to_string()),
                            },
                        ),
                        started_at: None,
                        http_request_count: None,
                        http_bytes_sent: None,
                        http_bytes_received: None,
                    })),
                    Err(e) => {
                        tracing::warn!(
                            "Could not find live progress or log file for session {}: {}",
                            session_id,
                            e
                        );
                        Err(axum::http::StatusCode::NOT_FOUND)
                    }
                }
            }
        }
        None => match get_global_task_logger().read_log(&session_id) {
            Ok(log_content) => Ok(Json(TaskProgressResponse {
                session_id: session_id.clone(),
                status: task.last_run_status.clone().unwrap_or_default(),
                message: "Log available".to_string(),
                total_items: None,
                processed_items: None,
                errors: None,
                current_item: None,
                log_content: Some(log_content),
                total_inserted: None,
                total_updated: None,
                detail: Some(
                    contracts::system::tasks::progress::TaskProgressDetail::Indeterminate {
                        hint: Some("Доступен лог выполнения".to_string()),
                    },
                ),
                started_at: None,
                http_request_count: None,
                http_bytes_sent: None,
                http_bytes_received: None,
            })),
            Err(_) => Err(axum::http::StatusCode::NOT_IMPLEMENTED),
        },
    }
}

/// GET /api/sys/tasks/:id/log/:session_id
pub async fn get_task_log(
    Path((_task_id_str, session_id)): Path<(String, String)>,
) -> Result<String, axum::http::StatusCode> {
    match get_global_task_logger().read_log(&session_id) {
        Ok(log_content) => Ok(log_content),
        Err(e) => {
            tracing::error!("Failed to read log for session {}: {}", session_id, e);
            Err(axum::http::StatusCode::NOT_FOUND)
        }
    }
}

/// GET /api/sys/tasks/:id/runs — история запусков конкретной задачи
#[derive(Deserialize)]
pub struct RunsQuery {
    pub limit: Option<u64>,
}

pub async fn list_task_runs(
    Path(id): Path<String>,
    Query(q): Query<RunsQuery>,
) -> Result<Json<TaskRunListResponse>, axum::http::StatusCode> {
    let limit = q.limit.unwrap_or(50);
    match runs_service::list_for_task(&id, limit).await {
        Ok(runs) => Ok(Json(TaskRunListResponse { task_id: id, runs })),
        Err(e) => {
            tracing::error!("Failed to list runs for task {}: {}", id, e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/sys/tasks/runs/recent — последние запуски всех задач
pub async fn list_recent_runs(
    Query(q): Query<RunsQuery>,
) -> Result<Json<RecentRunsResponse>, axum::http::StatusCode> {
    let limit = q.limit.unwrap_or(100);
    match runs_service::list_recent(limit).await {
        Ok(runs) => Ok(Json(RecentRunsResponse { runs })),
        Err(e) => {
            tracing::error!("Failed to list recent runs: {}", e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/sys/tasks/runs/active — все запуски со статусом Running
pub async fn list_active_runs() -> Result<Json<RecentRunsResponse>, axum::http::StatusCode> {
    match runs_service::list_active().await {
        Ok(runs) => Ok(Json(RecentRunsResponse { runs })),
        Err(e) => {
            tracing::error!("Failed to list active runs: {}", e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/sys/tasks/runs/active/progress — только память: сессии `Running` по всем
/// менеджерам без SQL и без чтения логов с диска (лёгкий мониторинг).
pub async fn list_active_runs_with_progress(
) -> Result<Json<LiveMemoryProgressResponse>, axum::http::StatusCode> {
    let items = match get_global_registry() {
        Some(reg) => reg.snapshot_live_memory_progress(),
        None => Vec::new(),
    };
    Ok(Json(LiveMemoryProgressResponse { items }))
}

/// GET /api/sys/tasks/task_types — метаданные всех зарегистрированных типов задач
pub async fn list_task_types() -> Result<Json<Vec<TaskMetadataDto>>, axum::http::StatusCode> {
    match get_global_registry() {
        Some(registry) => {
            let dtos: Vec<TaskMetadataDto> = registry
                .list_metadata()
                .iter()
                .map(|m| (*m).into())
                .collect();
            Ok(Json(dtos))
        }
        None => Ok(Json(vec![])),
    }
}

/// POST /api/sys/tasks/:id/watermark
///
/// Устанавливает или сбрасывает watermark (last_successful_run_at) задачи.
/// Body: `{ "date": "2026-01-01" }` — установить конкретную дату.
/// Body: `{ "date": null }` — сбросить в NULL (следующий запуск начнёт с work_start_date).
pub async fn set_watermark(
    Path(id): Path<String>,
    Json(dto): Json<SetWatermarkDto>,
) -> Result<axum::http::StatusCode, axum::http::StatusCode> {
    let task_id =
        ScheduledTaskId::from_string(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    match service::set_watermark(&task_id, dto.date.as_deref()).await {
        Ok(_) => Ok(axum::http::StatusCode::OK),
        Err(e) => {
            tracing::error!("Failed to set watermark for task {}: {}", id, e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// POST /api/sys/tasks/runs/{session_id}/abort
///
/// Принудительно прерывает работающую задачу:
/// 1. Вызывает `AbortHandle::abort()` — Tokio отменяет спавненную задачу на ближайшей точке yield.
/// 2. Помечает запуск в БД как Cancelled.
/// 3. Синхронизирует `last_run_status` в `sys_tasks`.
///
/// Возвращает 200, если запись найдена; 404 — если задача уже завершилась или session_id неверен.
pub async fn abort_task_run(
    Path(session_id): Path<String>,
) -> Result<axum::http::StatusCode, axum::http::StatusCode> {
    let aborted = abort_registry::abort(&session_id);
    if !aborted {
        // Задача уже завершилась или session_id неверен — всё равно чистим БД на случай «зависшей» записи.
        tracing::warn!("abort_task_run: no abort handle for session {}", session_id);
    }

    // Отметить запуск как Cancelled в sys_task_runs
    let _ = runs_service::finish_run(
        &session_id,
        contracts::system::tasks::progress::TaskStatus::Cancelled,
        None,
        Some("Прервано пользователем".to_string()),
    )
    .await;

    // Синхронизировать last_run_status в sys_tasks
    if let Ok(Some(run)) = runs_service::get_run_by_session(&session_id).await {
        if let Ok(task_id) = ScheduledTaskId::from_string(&run.task_id) {
            let _ = service::update_run_status(
                &task_id,
                None,
                None,
                None,
                Some("Cancelled".to_string()),
                None,
            )
            .await;
        }
    }

    if aborted {
        Ok(axum::http::StatusCode::OK)
    } else {
        Err(axum::http::StatusCode::NOT_FOUND)
    }
}
