use axum::{extract::Path, Json};
use contracts::system::sys_scheduled_task::aggregate::ScheduledTaskId;
use contracts::domain::common::AggregateId;
use contracts::system::sys_scheduled_task::request::{
    CreateScheduledTaskDto, UpdateScheduledTaskDto, ToggleScheduledTaskEnabledDto,
};
use contracts::system::sys_scheduled_task::response::{
    ScheduledTaskResponse, ScheduledTaskListResponse,
};
use contracts::system::sys_scheduled_task::progress::TaskProgressResponse;
use crate::system::sys_scheduled_task::service;
use crate::system::sys_scheduled_task::registry::TaskManagerRegistry;
use crate::system::sys_scheduled_task::logger::TaskLogger;
use std::sync::Arc;
use once_cell::sync::Lazy;

// Note: These should ideally be passed via state, but for simplicity we'll use statics for now
// until we refactor Axum state to include them.
// Actual initialization happens in initialization.rs and main.rs
static TASK_REGISTRY: Lazy<Arc<TaskManagerRegistry>> = Lazy::new(|| {
    // This is a placeholder, actual managers are registered in `initialization.rs`
    Arc::new(TaskManagerRegistry::new())
});

static TASK_LOGGER: Lazy<Arc<TaskLogger>> = Lazy::new(|| {
    Arc::new(TaskLogger::new("./task_logs"))
});

/// GET /api/sys/scheduled_tasks
pub async fn list_scheduled_tasks() -> Result<Json<ScheduledTaskListResponse>, axum::http::StatusCode> {
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

/// GET /api/sys/scheduled_tasks/:id
pub async fn get_scheduled_task(
    Path(id): Path<String>,
) -> Result<Json<ScheduledTaskResponse>, axum::http::StatusCode> {
    let task_id = ScheduledTaskId::from_string(&id)
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    match service::get_by_id(&task_id).await {
        Ok(Some(task)) => Ok(Json(task.into())),
        Ok(None) => Err(axum::http::StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to get scheduled task {}: {}", id, e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// POST /api/sys/scheduled_tasks
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

/// PUT /api/sys/scheduled_tasks/:id
pub async fn update_scheduled_task(
    Path(id): Path<String>,
    Json(dto): Json<UpdateScheduledTaskDto>,
) -> Result<Json<ScheduledTaskResponse>, axum::http::StatusCode> {
    let task_id = ScheduledTaskId::from_string(&id)
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

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

/// DELETE /api/sys/scheduled_tasks/:id
pub async fn delete_scheduled_task(
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, axum::http::StatusCode> {
    let task_id = ScheduledTaskId::from_string(&id)
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    match service::delete(&task_id).await {
        Ok(_) => Ok(axum::http::StatusCode::NO_CONTENT),
        Err(e) => {
            tracing::error!("Failed to delete scheduled task {}: {}", id, e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// POST /api/sys/scheduled_tasks/:id/toggle_enabled
pub async fn toggle_scheduled_task_enabled(
    Path(id): Path<String>,
    Json(dto): Json<ToggleScheduledTaskEnabledDto>,
) -> Result<Json<ScheduledTaskResponse>, axum::http::StatusCode> {
    let task_id = ScheduledTaskId::from_string(&id)
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    match service::toggle_enabled(&task_id, dto.is_enabled).await {
        Ok(_) => match service::get_by_id(&task_id).await {
            Ok(Some(task)) => Ok(Json(task.into())),
            _ => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
        },
        Err(e) => {
            tracing::error!("Failed to toggle scheduled task {} enabled status: {}", id, e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/sys/scheduled_tasks/:id/progress/:session_id
pub async fn get_task_progress(
    Path((task_id_str, session_id)): Path<(String, String)>,
) -> Result<Json<TaskProgressResponse>, axum::http::StatusCode> {
    let task_id = ScheduledTaskId::from_string(&task_id_str)
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    let task = service::get_by_id(&task_id).await
        .map_err(|e| {
            tracing::error!("Failed to get task {}: {}", task_id_str, e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;

    let manager = TASK_REGISTRY.get(&task.task_type);

    match manager {
        Some(mgr) => {
            if let Some(progress) = mgr.get_progress(&session_id) {
                Ok(Json(progress.into()))
            } else {
                // If no live progress, try to read from log file
                match TASK_LOGGER.read_log(&session_id) {
                    Ok(log_content) => {
                        Ok(Json(TaskProgressResponse {
                            session_id: session_id.clone(),
                            status: task.last_run_status.clone().unwrap_or_default(),
                            message: "Log available".to_string(),
                            total_items: None,
                            processed_items: None,
                            errors: None,
                            current_item: None,
                            log_content: Some(log_content),
                        }))
                    }
                    Err(e) => {
                        tracing::warn!("Could not find live progress or log file for session {}: {}", session_id, e);
                        Err(axum::http::StatusCode::NOT_FOUND)
                    }
                }
            }
        }
        None => {
            tracing::warn!("No manager found for task type '{}'", task.task_type);
            Err(axum::http::StatusCode::NOT_IMPLEMENTED)
        }
    }
}

/// GET /api/sys/scheduled_tasks/:id/log/:session_id
pub async fn get_task_log(
    Path((_task_id_str, session_id)): Path<(String, String)>,
) -> Result<String, axum::http::StatusCode> {
    match TASK_LOGGER.read_log(&session_id) {
        Ok(log_content) => Ok(log_content),
        Err(e) => {
            tracing::error!("Failed to read log for session {}: {}", session_id, e);
            Err(axum::http::StatusCode::NOT_FOUND)
        }
    }
}
