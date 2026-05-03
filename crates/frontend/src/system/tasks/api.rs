use crate::shared::api_utils::api_base;
use crate::system::auth::storage;
use contracts::system::tasks::metadata::TaskMetadataDto;
use contracts::system::tasks::progress::TaskProgressResponse;
use contracts::system::tasks::request::{
    CreateScheduledTaskDto, SetWatermarkDto, ToggleScheduledTaskEnabledDto, UpdateScheduledTaskDto,
};
use contracts::system::tasks::response::{ScheduledTaskListResponse, ScheduledTaskResponse};
use contracts::system::tasks::runs::{
    LiveMemoryProgressResponse, RecentRunsResponse, RunTaskResponse, TaskRun, TaskRunListResponse,
};
use gloo_net::http::Request;

fn get_auth_header() -> Option<String> {
    storage::get_access_token().map(|token| format!("Bearer {}", token))
}

/// Fetch all scheduled tasks
pub async fn fetch_scheduled_tasks() -> Result<Vec<ScheduledTaskResponse>, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::get(&format!("{}/api/sys/tasks", api_base()))
        .header("Authorization", &auth_header)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!(
            "Failed to fetch scheduled tasks: {}",
            response.status()
        ));
    }

    let result: ScheduledTaskListResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(result.tasks)
}

/// Get scheduled task by ID
pub async fn get_scheduled_task(id: &str) -> Result<ScheduledTaskResponse, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::get(&format!("{}/api/sys/tasks/{}", api_base(), id))
        .header("Authorization", &auth_header)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!(
            "Failed to fetch scheduled task: {}",
            response.status()
        ));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Create new scheduled task
pub async fn create_scheduled_task(
    dto: CreateScheduledTaskDto,
) -> Result<ScheduledTaskResponse, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::post(&format!("{}/api/sys/tasks", api_base()))
        .header("Authorization", &auth_header)
        .json(&dto)
        .map_err(|e| format!("Failed to serialize request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!(
            "Failed to create scheduled task: {}",
            response.status()
        ));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Update scheduled task
pub async fn update_scheduled_task(
    id: &str,
    dto: UpdateScheduledTaskDto,
) -> Result<ScheduledTaskResponse, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::put(&format!("{}/api/sys/tasks/{}", api_base(), id))
        .header("Authorization", &auth_header)
        .json(&dto)
        .map_err(|e| format!("Failed to serialize request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!(
            "Failed to update scheduled task: {}",
            response.status()
        ));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Toggle enabled status
pub async fn toggle_scheduled_task_enabled(
    id: &str,
    is_enabled: bool,
) -> Result<ScheduledTaskResponse, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;
    let dto = ToggleScheduledTaskEnabledDto { is_enabled };

    let response = Request::post(&format!(
        "{}/api/sys/tasks/{}/toggle_enabled",
        api_base(),
        id
    ))
    .header("Authorization", &auth_header)
    .json(&dto)
    .map_err(|e| format!("Failed to serialize request: {}", e))?
    .send()
    .await
    .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!(
            "Failed to toggle scheduled task status: {}",
            response.status()
        ));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Get task progress
pub async fn get_task_progress(
    task_id: &str,
    session_id: &str,
) -> Result<TaskProgressResponse, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::get(&format!(
        "{}/api/sys/tasks/{}/progress/{}",
        api_base(),
        task_id,
        session_id
    ))
    .header("Authorization", &auth_header)
    .send()
    .await
    .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!(
            "Failed to fetch task progress: {}",
            response.status()
        ));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Результат ручного запуска: новая сессия или конфликт (уже выполняется).
#[derive(Debug, Clone)]
pub enum RunTaskNowOutcome {
    Started(RunTaskResponse),
    /// HTTP 409 — в теле текущий `TaskRun` со статусом Running
    AlreadyRunning(TaskRun),
}

/// Run task manually
pub async fn run_task_now(task_id: &str) -> Result<RunTaskNowOutcome, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::post(&format!("{}/api/sys/tasks/{}/run", api_base(), task_id))
        .header("Authorization", &auth_header)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    let status = response.status();

    if status == 409 {
        let run: TaskRun = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse conflict body: {}", e))?;
        return Ok(RunTaskNowOutcome::AlreadyRunning(run));
    }

    if !response.ok() {
        return Err(format!("Failed to run task: {}", status));
    }

    let body: RunTaskResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    Ok(RunTaskNowOutcome::Started(body))
}

/// Сессии Running в памяти трекеров (без БД и без чтения логов)
pub async fn get_active_runs_with_progress() -> Result<LiveMemoryProgressResponse, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::get(&format!(
        "{}/api/sys/tasks/runs/active/progress",
        api_base()
    ))
    .header("Authorization", &auth_header)
    .send()
    .await
    .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!(
            "Failed to fetch active runs with progress: {}",
            response.status()
        ));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Get run history for a specific task
pub async fn get_task_runs(
    task_id: &str,
    limit: Option<u32>,
) -> Result<TaskRunListResponse, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;
    let url = match limit {
        Some(l) => format!("{}/api/sys/tasks/{}/runs?limit={}", api_base(), task_id, l),
        None => format!("{}/api/sys/tasks/{}/runs", api_base(), task_id),
    };

    let response = Request::get(&url)
        .header("Authorization", &auth_header)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to fetch task runs: {}", response.status()));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Get recent runs across all tasks
pub async fn get_recent_runs(limit: Option<u32>) -> Result<RecentRunsResponse, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;
    let url = match limit {
        Some(l) => format!("{}/api/sys/tasks/runs/recent?limit={}", api_base(), l),
        None => format!("{}/api/sys/tasks/runs/recent", api_base()),
    };

    let response = Request::get(&url)
        .header("Authorization", &auth_header)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!(
            "Failed to fetch recent runs: {}",
            response.status()
        ));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Get task type metadata
pub async fn get_task_types() -> Result<Vec<TaskMetadataDto>, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::get(&format!("{}/api/sys/tasks/task_types", api_base()))
        .header("Authorization", &auth_header)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to fetch task types: {}", response.status()));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Abort a running task by session_id
pub async fn abort_task_run(session_id: &str) -> Result<(), String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::post(&format!(
        "{}/api/sys/tasks/runs/{}/abort",
        api_base(),
        session_id
    ))
    .header("Authorization", &auth_header)
    .send()
    .await
    .map_err(|e| format!("Failed to send request: {}", e))?;

    if response.status() == 404 {
        return Err("Задача не найдена (уже завершилась?)".to_string());
    }
    if !response.ok() {
        return Err(format!("Ошибка прерывания: {}", response.status()));
    }
    Ok(())
}

/// Set or reset the watermark (last_successful_run_at) for a task.
/// `date` = "YYYY-MM-DD" to set a specific date, `None` to reset to NULL.
pub async fn set_watermark(id: &str, date: Option<String>) -> Result<(), String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let dto = SetWatermarkDto { date };
    let response = Request::post(&format!("{}/api/sys/tasks/{}/watermark", api_base(), id))
        .header("Authorization", &auth_header)
        .json(&dto)
        .map_err(|e| format!("Failed to serialize: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if response.ok() {
        Ok(())
    } else {
        Err(format!("Set watermark failed: {}", response.status()))
    }
}
