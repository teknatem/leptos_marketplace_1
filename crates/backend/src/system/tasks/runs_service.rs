use anyhow::Result;
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, Utc};
use contracts::system::tasks::aggregate::ScheduledTaskId;
use contracts::system::tasks::history::{
    TaskHistoryMetric, TaskHistoryPoint, TaskHistoryRequest, TaskHistoryResponse, TaskHistoryScale,
};
use contracts::system::tasks::progress::TaskStatus;
use contracts::system::tasks::runs::TaskRun;
use std::collections::HashSet;

use super::{runs_repository, service as task_service};

const HISTORY_TZ_OFFSET_HOURS: i64 = 3;

/// Метрики выполнения задачи для записи в историю
pub struct RunMetrics {
    pub total_processed: Option<i64>,
    pub total_inserted: Option<i64>,
    pub total_updated: Option<i64>,
    pub total_errors: Option<i64>,
    pub http_request_count: Option<i64>,
    pub http_bytes_sent: Option<i64>,
    pub http_bytes_received: Option<i64>,
}

pub async fn reset_stale_running_runs(reason: &str) -> Result<u64> {
    let models = runs_repository::find_all_running_models()
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))?;
    let n = models.len() as u64;
    let task_ids: Vec<String> = models
        .iter()
        .map(|m| m.task_id.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let reason_owned = reason.to_string();
    for m in models {
        finish_run(
            &m.session_id,
            TaskStatus::Failed,
            None,
            Some(reason_owned.clone()),
        )
        .await?;
    }
    task_service::reset_stale_task_statuses(&task_ids).await?;
    Ok(n)
}

pub async fn create_run(
    task_id: ScheduledTaskId,
    session_id: String,
    triggered_by: String,
) -> Result<()> {
    runs_repository::create_run(task_id, session_id, triggered_by, None)
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))
}

pub async fn create_run_with_log(
    task_id: ScheduledTaskId,
    session_id: String,
    triggered_by: String,
    log_file_path: String,
) -> Result<()> {
    runs_repository::create_run(task_id, session_id, triggered_by, Some(log_file_path))
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))
}

pub async fn finish_run(
    session_id: &str,
    status: TaskStatus,
    metrics: Option<RunMetrics>,
    error_message: Option<String>,
) -> Result<()> {
    let (processed, inserted, updated, errors, http_n, http_up, http_down) = metrics
        .map(|m| {
            (
                m.total_processed,
                m.total_inserted,
                m.total_updated,
                m.total_errors,
                m.http_request_count,
                m.http_bytes_sent,
                m.http_bytes_received,
            )
        })
        .unwrap_or((None, None, None, None, None, None, None));

    runs_repository::finish_run(
        session_id,
        &status.to_string(),
        processed,
        inserted,
        updated,
        errors,
        http_n,
        http_up,
        http_down,
        error_message,
    )
    .await
    .map_err(|e| anyhow::anyhow!("Database error: {}", e))
}

pub async fn get_run_by_session(session_id: &str) -> Result<Option<TaskRun>> {
    runs_repository::find_by_session_id(session_id)
        .await
        .map(|opt| {
            opt.map(|m| TaskRun {
                id: m.id,
                task_id: m.task_id,
                session_id: m.session_id,
                triggered_by: m.triggered_by,
                started_at: m.started_at,
                finished_at: m.finished_at,
                duration_ms: m.duration_ms,
                status: m.status,
                total_processed: m.total_processed,
                total_inserted: m.total_inserted,
                total_updated: m.total_updated,
                total_errors: m.total_errors,
                log_file_path: m.log_file_path,
                error_message: m.error_message,
                http_request_count: m.http_request_count,
                http_bytes_sent: m.http_bytes_sent,
                http_bytes_received: m.http_bytes_received,
                task_code: None,
                task_description: None,
                task_comment: None,
            })
        })
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))
}

pub async fn get_running_for_task(task_id: &str) -> Result<Option<TaskRun>> {
    runs_repository::find_running_for_task_enriched(task_id)
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))
}

pub async fn list_active() -> Result<Vec<TaskRun>> {
    runs_repository::list_active_enriched()
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))
}

pub async fn list_for_task(task_id: &str, limit: u64) -> Result<Vec<TaskRun>> {
    runs_repository::list_for_task(task_id, limit)
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))
}

pub async fn list_recent(limit: u64) -> Result<Vec<TaskRun>> {
    runs_repository::list_recent(limit)
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))
}

pub async fn query_history(req: TaskHistoryRequest) -> Result<TaskHistoryResponse> {
    let date_from = NaiveDate::parse_from_str(&req.date_from, "%Y-%m-%d")
        .map_err(|e| anyhow::anyhow!("Invalid date_from '{}': {}", req.date_from, e))?;
    let date_to = period_end(date_from, req.scale);
    let local_start = date_from
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| anyhow::anyhow!("Invalid start date"))?;
    let local_end = date_to
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| anyhow::anyhow!("Invalid end date"))?;
    let utc_start = local_start - Duration::hours(HISTORY_TZ_OFFSET_HOURS);
    let utc_end = local_end - Duration::hours(HISTORY_TZ_OFFSET_HOURS);
    let date_from_sql = format!("{}Z", utc_start.format("%Y-%m-%dT%H:%M:%S"));
    let date_to_sql = format!("{}Z", utc_end.format("%Y-%m-%dT%H:%M:%S"));
    let bucket_count = bucket_count(local_start, date_to, req.scale)?;
    let bucket_size_seconds = bucket_size_seconds(req.scale);
    let mut values = vec![0.0_f64; bucket_count as usize];

    let rows =
        runs_repository::query_history_runs(&date_from_sql, &date_to_sql, req.task_ids.as_deref())
            .await
            .map_err(|e| anyhow::anyhow!("Database error: {}", e))?;

    let now = Utc::now();
    for row in rows {
        let started_local = row.started_at.naive_utc() + Duration::hours(HISTORY_TZ_OFFSET_HOURS);
        let finished_local =
            row.finished_at.unwrap_or(now).naive_utc() + Duration::hours(HISTORY_TZ_OFFSET_HOURS);
        let interval_start = started_local.max(local_start);
        let interval_end = finished_local.min(local_end);
        if interval_end < local_start || interval_start >= local_end {
            continue;
        }

        let start_seconds = (interval_start - local_start).num_seconds().max(0);
        let end_seconds = (interval_end - local_start)
            .num_seconds()
            .max(start_seconds);
        let first_bucket = (start_seconds / bucket_size_seconds).max(0) as u32;
        let last_bucket = if end_seconds <= start_seconds {
            first_bucket
        } else {
            ((end_seconds - 1) / bucket_size_seconds).max(0) as u32
        }
        .min(bucket_count.saturating_sub(1));

        if first_bucket >= bucket_count {
            continue;
        }

        let active_bucket_count = (last_bucket.saturating_sub(first_bucket) + 1).max(1) as f64;
        let value_per_bucket = match req.metric {
            TaskHistoryMetric::TaskCount => 1.0,
            TaskHistoryMetric::RequestCount => {
                row.http_request_count.unwrap_or(0).max(0) as f64 / active_bucket_count
            }
            TaskHistoryMetric::TrafficBytes => {
                let sent = row.http_bytes_sent.unwrap_or(0).max(0);
                let received = row.http_bytes_received.unwrap_or(0).max(0);
                (sent + received) as f64 / active_bucket_count
            }
        };

        for offset in first_bucket..=last_bucket {
            if let Some(slot) = values.get_mut(offset as usize) {
                *slot += value_per_bucket;
            }
        }
    }

    let points = values
        .into_iter()
        .enumerate()
        .filter_map(|(offset, value)| {
            if value <= 0.0 {
                return None;
            }
            let bucket = local_start + Duration::seconds(offset as i64 * bucket_size_seconds);
            Some(TaskHistoryPoint {
                bucket: format!("{} MSK", bucket.format("%Y-%m-%dT%H:%M:%S")),
                value,
                offset: offset as u32,
            })
        })
        .collect();

    Ok(TaskHistoryResponse {
        points,
        bucket_count,
        date_from: req.date_from,
    })
}

fn bucket_size_seconds(scale: TaskHistoryScale) -> i64 {
    match scale {
        TaskHistoryScale::Day => 60,
        TaskHistoryScale::Week => 5 * 60,
        TaskHistoryScale::Month => 60 * 60,
    }
}

fn period_end(date_from: NaiveDate, scale: TaskHistoryScale) -> NaiveDate {
    match scale {
        TaskHistoryScale::Day => date_from + Duration::days(1),
        TaskHistoryScale::Week => date_from + Duration::days(7),
        TaskHistoryScale::Month => add_one_month(date_from),
    }
}

fn add_one_month(date: NaiveDate) -> NaiveDate {
    let (year, month) = if date.month() == 12 {
        (date.year() + 1, 1)
    } else {
        (date.year(), date.month() + 1)
    };
    let last_day = last_day_of_month(year, month);
    NaiveDate::from_ymd_opt(year, month, date.day().min(last_day)).unwrap_or(date)
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
    let next_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    };
    next_month
        .map(|date| (date - Duration::days(1)).day())
        .unwrap_or(31)
}

fn bucket_count(
    start_dt: NaiveDateTime,
    date_to: NaiveDate,
    scale: TaskHistoryScale,
) -> Result<u32> {
    let end_dt = date_to
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| anyhow::anyhow!("Invalid end date"))?;
    let minutes = (end_dt - start_dt).num_minutes().max(0);
    Ok(match scale {
        TaskHistoryScale::Day => minutes as u32,
        TaskHistoryScale::Week => (minutes / 5) as u32,
        TaskHistoryScale::Month => (minutes / 60) as u32,
    })
}
