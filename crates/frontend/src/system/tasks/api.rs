use crate::shared::api_utils::api_base;
use crate::system::auth::storage;
use contracts::system::ext_api_log::{
    ExtApiHistoryResponse, ExtApiLogListResponse, ExtApiMetric, ExtApiScale, ExtApiSummaryResponse,
};
use contracts::system::tasks::history::{TaskHistoryMetric, TaskHistoryResponse, TaskHistoryScale};
use contracts::system::tasks::metadata::TaskMetadataDto;
use contracts::system::tasks::progress::TaskProgressResponse;
use contracts::system::tasks::request::{
    CreateScheduledTaskDto, SchedulerStatusDto, SetWatermarkDto, ToggleScheduledTaskEnabledDto,
    UpdateScheduledTaskDto,
};
use contracts::system::tasks::response::{ScheduledTaskListResponse, ScheduledTaskResponse};
use contracts::system::tasks::runs::{
    LiveMemoryProgressResponse, RecentRunsResponse, RunTaskResponse, TaskRun, TaskRunListResponse,
};
use gloo_net::http::Request;
use serde_json;

fn get_auth_header() -> Option<String> {
    storage::get_access_token().map(|token| format!("Bearer {}", token))
}

fn history_scale_param(scale: TaskHistoryScale) -> &'static str {
    match scale {
        TaskHistoryScale::Day => "day",
        TaskHistoryScale::Week => "week",
        TaskHistoryScale::Month => "month",
    }
}

fn history_metric_param(metric: TaskHistoryMetric) -> &'static str {
    match metric {
        TaskHistoryMetric::TaskCount => "task_count",
        TaskHistoryMetric::RequestCount => "request_count",
        TaskHistoryMetric::TrafficBytes => "traffic_bytes",
    }
}

fn ext_api_scale_param(scale: ExtApiScale) -> &'static str {
    match scale {
        ExtApiScale::Day => "day",
        ExtApiScale::Week => "week",
        ExtApiScale::Month => "month",
    }
}

fn ext_api_metric_param(metric: ExtApiMetric) -> &'static str {
    match metric {
        ExtApiMetric::RequestCount => "request_count",
        ExtApiMetric::TrafficBytes => "traffic_bytes",
        ExtApiMetric::AvgDurationMs => "avg_duration_ms",
        ExtApiMetric::ErrorCount => "error_count",
    }
}

/// GET-запрос с авторизацией и разбором JSON — общая обвязка для вкладки «Внешний API».
async fn get_json<T: serde::de::DeserializeOwned>(url: String, what: &str) -> Result<T, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;
    let response = Request::get(&url)
        .header("Authorization", &auth_header)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to fetch {}: {}", what, response.status()));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Временной ряд по входящим вызовам внешнего API.
pub async fn fetch_ext_api_history(
    scale: ExtApiScale,
    metric: ExtApiMetric,
    date_from: &str,
) -> Result<ExtApiHistoryResponse, String> {
    get_json(
        format!(
            "{}/api/sys/ext-api/history?scale={}&metric={}&date_from={}",
            api_base(),
            ext_api_scale_param(scale),
            ext_api_metric_param(metric),
            date_from
        ),
        "ext api history",
    )
    .await
}

/// Сводка внешнего API за период: по эндпоинтам и по потребителям.
pub async fn fetch_ext_api_summary(
    scale: ExtApiScale,
    date_from: &str,
) -> Result<ExtApiSummaryResponse, String> {
    get_json(
        format!(
            "{}/api/sys/ext-api/summary?scale={}&date_from={}",
            api_base(),
            ext_api_scale_param(scale),
            date_from
        ),
        "ext api summary",
    )
    .await
}

/// Последние вызовы внешнего API.
pub async fn fetch_ext_api_recent(limit: u32) -> Result<ExtApiLogListResponse, String> {
    get_json(
        format!("{}/api/sys/ext-api/recent?limit={}", api_base(), limit),
        "ext api recent calls",
    )
    .await
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

/// Get task log by run session_id.
pub async fn get_task_log(task_id: &str, session_id: &str) -> Result<String, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::get(&format!(
        "{}/api/sys/tasks/{}/log/{}",
        api_base(),
        task_id,
        session_id
    ))
    .header("Authorization", &auth_header)
    .send()
    .await
    .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to fetch task log: {}", response.status()));
    }

    response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))
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

/// Get aggregated run history for the sys_tasks History tab.
pub async fn fetch_history(
    scale: TaskHistoryScale,
    metric: TaskHistoryMetric,
    date_from: &str,
) -> Result<TaskHistoryResponse, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;
    let url = format!(
        "{}/api/sys/tasks/history?scale={}&metric={}&date_from={}",
        api_base(),
        history_scale_param(scale),
        history_metric_param(metric),
        date_from
    );

    let response = Request::get(&url)
        .header("Authorization", &auth_header)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!(
            "Failed to fetch task history: {}",
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

/// Текущие токены изменений по доменам (легковесный опрос).
pub struct ChangeTokensDto {
    pub sys_tasks: u64,
    pub a027_wb_documents: u64,
    pub a015_wb_orders: u64,
    pub a012_wb_sales: u64,
    pub a013_ym_order: u64,
    pub plugins: u64,
}

pub async fn fetch_change_tokens() -> Result<ChangeTokensDto, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::get(&format!("{}/api/sys/change-tokens", api_base()))
        .header("Authorization", &auth_header)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.ok() {
        return Err(format!(
            "Change tokens request failed: {}",
            response.status()
        ));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(ChangeTokensDto {
        sys_tasks: json["sys_tasks"].as_u64().unwrap_or(0),
        a027_wb_documents: json["a027_wb_documents"].as_u64().unwrap_or(0),
        a015_wb_orders: json["a015_wb_orders"].as_u64().unwrap_or(0),
        a012_wb_sales: json["a012_wb_sales"].as_u64().unwrap_or(0),
        a013_ym_order: json["a013_ym_order"].as_u64().unwrap_or(0),
        plugins: json["plugins"].as_u64().unwrap_or(0),
    })
}

/// Получить статус глобального планировщика (включён / выключен).
pub async fn fetch_scheduler_status() -> Result<SchedulerStatusDto, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::get(&format!("{}/api/sys/scheduler/status", api_base()))
        .header("Authorization", &auth_header)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.ok() {
        return Err(format!(
            "Scheduler status request failed: {}",
            response.status()
        ));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Включить или выключить глобальный планировщик.
pub async fn set_scheduler_status(enabled: bool) -> Result<SchedulerStatusDto, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let dto = SchedulerStatusDto {
        enabled,
        config_enabled: true,
    };
    let response = Request::post(&format!("{}/api/sys/scheduler/status", api_base()))
        .header("Authorization", &auth_header)
        .json(&dto)
        .map_err(|e| format!("Failed to serialize: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.ok() {
        return Err(format!(
            "Set scheduler status failed: {}",
            response.status()
        ));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}
