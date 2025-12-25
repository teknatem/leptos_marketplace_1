use contracts::system::sys_scheduled_task::request::{
    CreateScheduledTaskDto, UpdateScheduledTaskDto, ToggleScheduledTaskEnabledDto,
};
use contracts::system::sys_scheduled_task::response::{
    ScheduledTaskResponse, ScheduledTaskListResponse,
};
use contracts::system::sys_scheduled_task::progress::TaskProgressResponse;
use gloo_net::http::Request;
use crate::system::auth::storage;

const API_BASE: &str = "http://localhost:3000";

fn get_auth_header() -> Option<String> {
    storage::get_access_token().map(|token| format!("Bearer {}", token))
}

/// Fetch all scheduled tasks
pub async fn fetch_scheduled_tasks() -> Result<Vec<ScheduledTaskResponse>, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::get(&format!("{}/api/sys/scheduled_tasks", API_BASE))
        .header("Authorization", &auth_header)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to fetch scheduled tasks: {}", response.status()));
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

    let response = Request::get(&format!("{}/api/sys/scheduled_tasks/{}", API_BASE, id))
        .header("Authorization", &auth_header)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to fetch scheduled task: {}", response.status()));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Create new scheduled task
pub async fn create_scheduled_task(dto: CreateScheduledTaskDto) -> Result<ScheduledTaskResponse, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::post(&format!("{}/api/sys/scheduled_tasks", API_BASE))
        .header("Authorization", &auth_header)
        .json(&dto)
        .map_err(|e| format!("Failed to serialize request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to create scheduled task: {}", response.status()));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Update scheduled task
pub async fn update_scheduled_task(id: &str, dto: UpdateScheduledTaskDto) -> Result<ScheduledTaskResponse, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::put(&format!("{}/api/sys/scheduled_tasks/{}", API_BASE, id))
        .header("Authorization", &auth_header)
        .json(&dto)
        .map_err(|e| format!("Failed to serialize request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to update scheduled task: {}", response.status()));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Delete scheduled task
pub async fn delete_scheduled_task(id: &str) -> Result<(), String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::delete(&format!("{}/api/sys/scheduled_tasks/{}", API_BASE, id))
        .header("Authorization", &auth_header)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to delete scheduled task: {}", response.status()));
    }

    Ok(())
}

/// Toggle enabled status
pub async fn toggle_scheduled_task_enabled(id: &str, is_enabled: bool) -> Result<ScheduledTaskResponse, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;
    let dto = ToggleScheduledTaskEnabledDto { is_enabled };

    let response = Request::post(&format!("{}/api/sys/scheduled_tasks/{}/toggle_enabled", API_BASE, id))
        .header("Authorization", &auth_header)
        .json(&dto)
        .map_err(|e| format!("Failed to serialize request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to toggle scheduled task status: {}", response.status()));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Get task progress
pub async fn get_task_progress(task_id: &str, session_id: &str) -> Result<TaskProgressResponse, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::get(&format!(
        "{}/api/sys/scheduled_tasks/{}/progress/{}",
        API_BASE, task_id, session_id
    ))
    .header("Authorization", &auth_header)
    .send()
    .await
    .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to fetch task progress: {}", response.status()));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Get task log
pub async fn get_task_log(task_id: &str, session_id: &str) -> Result<String, String> {
    let auth_header = get_auth_header().ok_or("Not authenticated")?;

    let response = Request::get(&format!(
        "{}/api/sys/scheduled_tasks/{}/log/{}",
        API_BASE, task_id, session_id
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
        .map_err(|e| format!("Failed to parse response: {}", e))
}

