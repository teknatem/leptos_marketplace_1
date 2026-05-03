//! Единый маппинг ImportProgress (u501–u504) → [`super::progress::TaskProgress`] / [`TaskProgressResponse`].
//! Используется бэкендом (`TaskManager::get_progress`) и фронтом (u504 и др.) без дублирования правил.

use super::progress::{TaskProgress, TaskProgressDetail, TaskProgressResponse, TaskStatus};
use crate::usecases::u501_import_from_ut::progress::{
    AggregateImportStatus as A501, ImportProgress as P501, ImportStatus as S501,
};
use crate::usecases::u502_import_from_ozon::progress::{
    AggregateImportStatus as A502, ImportProgress as P502, ImportStatus as S502,
};
use crate::usecases::u503_import_from_yandex::progress::{
    AggregateImportStatus as A503, ImportProgress as P503, ImportStatus as S503,
};
use crate::usecases::u504_import_from_wildberries::progress::{
    AggregateImportStatus as A504, ImportProgress as P504, ImportStatus as S504,
};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct NormalizedAgg {
    pub index: String,
    pub name: String,
    pub status: AggState,
    pub processed: i32,
    pub total: Option<i32>,
    pub inserted: i32,
    pub updated: i32,
    pub errors: i32,
    pub current_item: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggState {
    Pending,
    Running,
    Completed,
    Failed,
}

fn map501(a: &crate::usecases::u501_import_from_ut::progress::AggregateProgress) -> NormalizedAgg {
    let status = match a.status {
        A501::Pending => AggState::Pending,
        A501::Running => AggState::Running,
        A501::Completed => AggState::Completed,
        A501::Failed => AggState::Failed,
    };
    NormalizedAgg {
        index: a.aggregate_index.clone(),
        name: a.aggregate_name.clone(),
        status,
        processed: a.processed,
        total: a.total,
        inserted: a.inserted,
        updated: a.updated,
        errors: a.errors,
        current_item: a.current_item.clone(),
    }
}

fn map502(
    a: &crate::usecases::u502_import_from_ozon::progress::AggregateProgress,
) -> NormalizedAgg {
    let status = match a.status {
        A502::Pending => AggState::Pending,
        A502::Running => AggState::Running,
        A502::Completed => AggState::Completed,
        A502::Failed => AggState::Failed,
    };
    NormalizedAgg {
        index: a.aggregate_index.clone(),
        name: a.aggregate_name.clone(),
        status,
        processed: a.processed,
        total: a.total,
        inserted: a.inserted,
        updated: a.updated,
        errors: a.errors,
        current_item: a.current_item.clone(),
    }
}

fn map503(
    a: &crate::usecases::u503_import_from_yandex::progress::AggregateProgress,
) -> NormalizedAgg {
    let status = match a.status {
        A503::Pending => AggState::Pending,
        A503::Running => AggState::Running,
        A503::Completed => AggState::Completed,
        A503::Failed => AggState::Failed,
    };
    NormalizedAgg {
        index: a.aggregate_index.clone(),
        name: a.aggregate_name.clone(),
        status,
        processed: a.processed,
        total: a.total,
        inserted: a.inserted,
        updated: a.updated,
        errors: a.errors,
        current_item: a.current_item.clone(),
    }
}

fn map504(
    a: &crate::usecases::u504_import_from_wildberries::progress::AggregateProgress,
) -> NormalizedAgg {
    let status = match a.status {
        A504::Pending => AggState::Pending,
        A504::Running => AggState::Running,
        A504::Completed => AggState::Completed,
        A504::Failed => AggState::Failed,
    };
    NormalizedAgg {
        index: a.aggregate_index.clone(),
        name: a.aggregate_name.clone(),
        status,
        processed: a.processed,
        total: a.total,
        inserted: a.inserted,
        updated: a.updated,
        errors: a.errors,
        current_item: a.current_item.clone(),
    }
}

#[derive(Debug, Clone)]
pub struct NormalizedImportSnapshot {
    pub session_id: String,
    pub status: TaskStatus,
    pub aggregates: Vec<NormalizedAgg>,
    pub total_processed: i32,
    pub total_inserted: i32,
    pub total_updated: i32,
    pub total_errors: i32,
    pub error_messages: Vec<String>,
    pub started_at: Option<DateTime<Utc>>,
    /// Счётчики HTTP (сейчас заполняются для u504 / WB API).
    pub http_request_count: i32,
    pub http_bytes_sent: i64,
    pub http_bytes_received: i64,
}

fn import_status_to_task(s501: S501) -> TaskStatus {
    match s501 {
        S501::Running => TaskStatus::Running,
        S501::Completed => TaskStatus::Completed,
        S501::CompletedWithErrors => TaskStatus::CompletedWithErrors,
        S501::Failed => TaskStatus::Failed,
        S501::Cancelled => TaskStatus::Failed,
    }
}

fn import_status_to_task_502(s: S502) -> TaskStatus {
    match s {
        S502::Running => TaskStatus::Running,
        S502::Completed => TaskStatus::Completed,
        S502::CompletedWithErrors => TaskStatus::CompletedWithErrors,
        S502::Failed => TaskStatus::Failed,
        S502::Cancelled => TaskStatus::Failed,
    }
}

fn import_status_to_task_503(s: S503) -> TaskStatus {
    match s {
        S503::Running => TaskStatus::Running,
        S503::Completed => TaskStatus::Completed,
        S503::CompletedWithErrors => TaskStatus::CompletedWithErrors,
        S503::Failed => TaskStatus::Failed,
        S503::Cancelled => TaskStatus::Failed,
    }
}

fn import_status_to_task_504(s: S504) -> TaskStatus {
    match s {
        S504::Running => TaskStatus::Running,
        S504::Completed => TaskStatus::Completed,
        S504::CompletedWithErrors => TaskStatus::CompletedWithErrors,
        S504::Failed => TaskStatus::Failed,
        S504::Cancelled => TaskStatus::Failed,
    }
}

fn cap_errors<T: Clone>(errors: &[T], limit: usize, fmt: impl Fn(&T) -> String) -> Vec<String> {
    errors.iter().take(limit).map(fmt).collect()
}

/// Полный снимок u501 → нормализованный вид.
pub fn normalize_u501(p: &P501) -> NormalizedImportSnapshot {
    NormalizedImportSnapshot {
        session_id: p.session_id.clone(),
        status: import_status_to_task(p.status.clone()),
        aggregates: p.aggregates.iter().map(map501).collect(),
        total_processed: p.total_processed,
        total_inserted: p.total_inserted,
        total_updated: p.total_updated,
        total_errors: p.total_errors,
        error_messages: cap_errors(&p.errors, 20, |e| {
            format!(
                "{}: {}",
                e.aggregate_index.clone().unwrap_or_default(),
                e.message
            )
        }),
        started_at: Some(p.started_at),
        http_request_count: 0,
        http_bytes_sent: 0,
        http_bytes_received: 0,
    }
}

pub fn normalize_u502(p: &P502) -> NormalizedImportSnapshot {
    NormalizedImportSnapshot {
        session_id: p.session_id.clone(),
        status: import_status_to_task_502(p.status.clone()),
        aggregates: p.aggregates.iter().map(map502).collect(),
        total_processed: p.total_processed,
        total_inserted: p.total_inserted,
        total_updated: p.total_updated,
        total_errors: p.total_errors,
        error_messages: cap_errors(&p.errors, 20, |e| {
            format!(
                "{}: {}",
                e.aggregate_index.clone().unwrap_or_default(),
                e.message
            )
        }),
        started_at: Some(p.started_at),
        http_request_count: 0,
        http_bytes_sent: 0,
        http_bytes_received: 0,
    }
}

pub fn normalize_u503(p: &P503) -> NormalizedImportSnapshot {
    NormalizedImportSnapshot {
        session_id: p.session_id.clone(),
        status: import_status_to_task_503(p.status.clone()),
        aggregates: p.aggregates.iter().map(map503).collect(),
        total_processed: p.total_processed,
        total_inserted: p.total_inserted,
        total_updated: p.total_updated,
        total_errors: p.total_errors,
        error_messages: cap_errors(&p.errors, 20, |e| {
            format!(
                "{}: {}",
                e.aggregate_index.clone().unwrap_or_default(),
                e.message
            )
        }),
        started_at: Some(p.started_at),
        http_request_count: 0,
        http_bytes_sent: 0,
        http_bytes_received: 0,
    }
}

/// `aggregate_filter`: если `Some`, только агрегат с таким `aggregate_index` (как в UI строк u504).
/// Неизвестный ключ: возвращаются все агрегаты и глобальные счётчики.
pub fn normalize_u504(p: &P504, aggregate_filter: Option<&str>) -> NormalizedImportSnapshot {
    let mut aggs: Vec<NormalizedAgg> = match aggregate_filter {
        Some(key) => p
            .aggregates
            .iter()
            .filter(|a| a.aggregate_index == key)
            .map(map504)
            .collect(),
        None => p.aggregates.iter().map(map504).collect(),
    };

    if aggs.is_empty() && aggregate_filter.is_some() {
        aggs = p.aggregates.iter().map(map504).collect();
    }

    let use_row_totals = aggregate_filter.is_some() && aggs.len() == 1;

    let (tp, ti, tu, te) = if use_row_totals {
        let a = &aggs[0];
        (a.processed, a.inserted, a.updated, a.errors)
    } else {
        (
            p.total_processed,
            p.total_inserted,
            p.total_updated,
            p.total_errors,
        )
    };

    NormalizedImportSnapshot {
        session_id: p.session_id.clone(),
        status: import_status_to_task_504(p.status.clone()),
        aggregates: aggs,
        total_processed: tp,
        total_inserted: ti,
        total_updated: tu,
        total_errors: te,
        error_messages: cap_errors(&p.errors, 20, |e| {
            format!(
                "{}: {}",
                e.aggregate_index.clone().unwrap_or_default(),
                e.message
            )
        }),
        started_at: Some(p.started_at),
        http_request_count: p.http_request_count,
        http_bytes_sent: p.http_bytes_sent,
        http_bytes_received: p.http_bytes_received,
    }
}

fn pick_detail(n: &NormalizedImportSnapshot) -> TaskProgressDetail {
    if let Some(a) = n.aggregates.iter().find(|a| a.status == AggState::Running) {
        if let Some(t) = a.total.filter(|t| *t > 0) {
            return TaskProgressDetail::Count {
                current: a.processed.min(t),
                total: t,
                label: Some(a.name.clone()),
            };
        }
        if n.aggregates.len() == 1 {
            if n.total_inserted > 0
                || n.total_updated > 0
                || n.total_errors > 0
                || n.total_processed > 0
            {
                return TaskProgressDetail::DataDelta {
                    inserted: n.total_inserted,
                    updated: n.total_updated,
                    deleted: 0,
                    errors: n.total_errors,
                };
            }
            return TaskProgressDetail::Indeterminate {
                hint: Some(format!("Ожидание ответа сервера — {}", a.name)),
            };
        }
    }

    if n.aggregates.len() > 1 {
        let stages: Vec<String> = n.aggregates.iter().map(|a| a.name.clone()).collect();
        let running_ix = n
            .aggregates
            .iter()
            .position(|a| a.status == AggState::Running);
        let pending_ix = n
            .aggregates
            .iter()
            .position(|a| a.status == AggState::Pending);
        let failed_ix = n
            .aggregates
            .iter()
            .position(|a| a.status == AggState::Failed);

        let (current_index, current_label) = if let Some(i) = running_ix {
            (i, n.aggregates[i].name.clone())
        } else if let Some(i) = pending_ix {
            (i, n.aggregates[i].name.clone())
        } else if let Some(i) = failed_ix {
            (i, n.aggregates[i].name.clone())
        } else {
            let last_done = n
                .aggregates
                .iter()
                .rposition(|a| a.status == AggState::Completed);
            match last_done {
                Some(i) if i + 1 < n.aggregates.len() => (i + 1, n.aggregates[i + 1].name.clone()),
                Some(i) => (i, n.aggregates[i].name.clone()),
                None => (
                    0,
                    n.aggregates
                        .first()
                        .map(|a| a.name.clone())
                        .unwrap_or_default(),
                ),
            }
        };

        return TaskProgressDetail::Pipeline {
            current_index,
            total_stages: n.aggregates.len(),
            current_label,
            stages: Some(stages),
        };
    }

    if n.total_inserted > 0 || n.total_updated > 0 || n.total_errors > 0 || n.total_processed > 0 {
        return TaskProgressDetail::DataDelta {
            inserted: n.total_inserted,
            updated: n.total_updated,
            deleted: 0,
            errors: n.total_errors,
        };
    }

    TaskProgressDetail::Indeterminate {
        hint: Some(if n.status == TaskStatus::Running {
            n.aggregates
                .first()
                .map(|a| format!("Запрос к API — {}", a.name))
                .unwrap_or_else(|| "Ожидание ответа сервера".to_string())
        } else {
            n.aggregates
                .first()
                .map(|a| format!("Ожидание — {}", a.name))
                .unwrap_or_else(|| "Ожидание данных".to_string())
        }),
    }
}

fn denormalize_legacy(
    n: &NormalizedImportSnapshot,
    detail: &TaskProgressDetail,
) -> (Option<i32>, Option<i32>, Option<String>) {
    let (mut total_items, mut processed_items, mut current_item) = (None, None, None);

    match detail {
        TaskProgressDetail::Count {
            current,
            total,
            label: _,
        } => {
            processed_items = Some(*current);
            total_items = Some(*total);
        }
        TaskProgressDetail::Pipeline { .. } => {
            let sum_proc: i32 = n.aggregates.iter().map(|a| a.processed).sum();
            let sum_tot: i32 = n.aggregates.iter().filter_map(|a| a.total).sum();
            processed_items = Some(sum_proc);
            if sum_tot > 0 {
                total_items = Some(sum_tot);
            }
            if let Some(run) = n.aggregates.iter().find(|a| a.status == AggState::Running) {
                current_item = run.current_item.clone();
            }
        }
        TaskProgressDetail::DataDelta { .. } => {
            processed_items = Some(n.total_processed);
            if let Some(run) = n.aggregates.iter().find(|a| a.status == AggState::Running) {
                if let Some(t) = run.total.filter(|t| *t > 0) {
                    total_items = Some(t);
                    processed_items = Some(run.processed.min(t));
                }
                current_item = run.current_item.clone();
            }
        }
        TaskProgressDetail::Percent { value } => {
            processed_items = Some(*value);
            total_items = Some(100);
        }
        TaskProgressDetail::Indeterminate { .. } => {
            if let Some(run) = n.aggregates.iter().find(|a| a.status == AggState::Running) {
                current_item = run.current_item.clone();
            }
        }
    }

    (total_items, processed_items, current_item)
}

/// Строит [`TaskProgress`] с заполненными `detail` и legacy-полями.
pub fn task_progress_from_normalized(
    n: NormalizedImportSnapshot,
    message: impl Into<String>,
) -> TaskProgress {
    let detail = pick_detail(&n);
    let (total_items, processed_items, current_item) = denormalize_legacy(&n, &detail);
    let message = message.into();
    let errors = if n.error_messages.is_empty() {
        None
    } else {
        Some(n.error_messages.clone())
    };

    TaskProgress {
        session_id: n.session_id,
        status: n.status,
        message,
        total_items,
        processed_items,
        errors,
        current_item,
        log_content: None,
        total_inserted: Some(n.total_inserted),
        total_updated: Some(n.total_updated),
        detail: Some(detail),
        started_at: n.started_at,
        http_request_count: Some(n.http_request_count),
        http_bytes_sent: Some(n.http_bytes_sent),
        http_bytes_received: Some(n.http_bytes_received),
    }
}

pub fn task_progress_response_from_normalized(
    n: NormalizedImportSnapshot,
    message: impl Into<String>,
) -> TaskProgressResponse {
    task_progress_from_normalized(n, message).into()
}

pub fn task_progress_response_from_u501(p: &P501) -> TaskProgressResponse {
    task_progress_response_from_normalized(normalize_u501(p), "Импорт из УТ")
}

pub fn task_progress_response_from_u502(p: &P502) -> TaskProgressResponse {
    task_progress_response_from_normalized(normalize_u502(p), "Импорт из Ozon")
}

pub fn task_progress_response_from_u503(p: &P503) -> TaskProgressResponse {
    task_progress_response_from_normalized(normalize_u503(p), "Импорт из Яндекс")
}

pub fn task_progress_response_from_u504(
    p: &P504,
    aggregate_filter: Option<&str>,
) -> TaskProgressResponse {
    task_progress_response_from_normalized(
        normalize_u504(p, aggregate_filter),
        "Импорт из Wildberries",
    )
}

impl From<P501> for TaskProgress {
    fn from(p: P501) -> Self {
        task_progress_from_normalized(normalize_u501(&p), "Импорт из УТ")
    }
}

impl From<&P501> for TaskProgress {
    fn from(p: &P501) -> Self {
        task_progress_from_normalized(normalize_u501(p), "Импорт из УТ")
    }
}

impl From<P502> for TaskProgress {
    fn from(p: P502) -> Self {
        task_progress_from_normalized(normalize_u502(&p), "Импорт из Ozon")
    }
}

impl From<&P502> for TaskProgress {
    fn from(p: &P502) -> Self {
        task_progress_from_normalized(normalize_u502(p), "Импорт из Ozon")
    }
}

impl From<P503> for TaskProgress {
    fn from(p: P503) -> Self {
        task_progress_from_normalized(normalize_u503(&p), "Импорт из Яндекс")
    }
}

impl From<&P503> for TaskProgress {
    fn from(p: &P503) -> Self {
        task_progress_from_normalized(normalize_u503(p), "Импорт из Яндекс")
    }
}

impl From<P504> for TaskProgress {
    fn from(p: P504) -> Self {
        task_progress_from_normalized(normalize_u504(&p, None), "Импорт из Wildberries")
    }
}

impl From<&P504> for TaskProgress {
    fn from(p: &P504) -> Self {
        task_progress_from_normalized(normalize_u504(p, None), "Импорт из Wildberries")
    }
}
