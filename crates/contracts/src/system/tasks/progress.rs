use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    /// Завершено, но были частичные ошибки (watermark не двигается, если так решил менеджер).
    CompletedWithErrors,
    Failed,
    Cancelled,
}

impl ToString for TaskStatus {
    fn to_string(&self) -> String {
        match self {
            TaskStatus::Pending => "Pending".to_string(),
            TaskStatus::Running => "Running".to_string(),
            TaskStatus::Completed => "Completed".to_string(),
            TaskStatus::CompletedWithErrors => "CompletedWithErrors".to_string(),
            TaskStatus::Failed => "Failed".to_string(),
            TaskStatus::Cancelled => "Cancelled".to_string(),
        }
    }
}

/// Тегированное представление прогресса для UI (регламент и usecase-страницы).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TaskProgressDetail {
    Count {
        current: i32,
        total: i32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    },
    Percent {
        /// 0–100
        value: i32,
    },
    DataDelta {
        inserted: i32,
        updated: i32,
        #[serde(default)]
        deleted: i32,
        #[serde(default)]
        errors: i32,
    },
    Pipeline {
        current_index: usize,
        total_stages: usize,
        current_label: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stages: Option<Vec<String>>,
    },
    Indeterminate {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hint: Option<String>,
    },
}

/// Краткая подпись для UI (список задач, подсказки).
pub fn task_progress_detail_caption_ru(d: &TaskProgressDetail) -> String {
    match d {
        TaskProgressDetail::Count {
            current,
            total,
            label,
        } => {
            let scope = label
                .as_ref()
                .map(|s| format!(" — {s}"))
                .unwrap_or_default();
            format!("{current}/{total}{scope}")
        }
        TaskProgressDetail::Percent { value } => format!("{value}%"),
        TaskProgressDetail::DataDelta {
            inserted,
            updated,
            deleted: _,
            errors,
        } => {
            if *inserted == 0 && *updated == 0 && *errors == 0 {
                return "Обработка данных".to_string();
            }
            let mut parts = Vec::with_capacity(3);
            if *inserted > 0 {
                parts.push(format!("+{inserted} нов"));
            }
            if *updated > 0 {
                parts.push(format!("={updated} изм"));
            }
            if *errors > 0 {
                parts.push(format!("{errors} ош"));
            }
            parts.join(" / ")
        }
        TaskProgressDetail::Pipeline {
            current_index,
            total_stages,
            current_label,
            ..
        } => {
            format!(
                "Шаг {}/{}: {}",
                (current_index + 1).min(*total_stages),
                total_stages,
                current_label
            )
        }
        TaskProgressDetail::Indeterminate { hint } => hint
            .as_ref()
            .cloned()
            .unwrap_or_else(|| "Выполняется".to_string()),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgress {
    pub session_id: String,
    pub status: TaskStatus,
    pub message: String,
    pub total_items: Option<i32>,
    pub processed_items: Option<i32>,
    pub errors: Option<Vec<String>>,
    pub current_item: Option<String>,
    pub log_content: Option<String>,
    /// Итоги для записи в `sys_task_runs` (импорты и др.).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_inserted: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_updated: Option<i32>,
    /// Расширенное отображение; при отсутствии UI использует legacy-поля выше.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<TaskProgressDetail>,
    /// Момент создания сессии в трекере (UTC).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    /// Число HTTP-запросов к внешнему API (WB и др.) за сессию.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_request_count: Option<i32>,
    /// Суммарный размер отправленных тел запросов, байт.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_bytes_sent: Option<i64>,
    /// Суммарный размер полученных тел ответов, байт.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_bytes_received: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgressResponse {
    pub session_id: String,
    pub status: String,
    pub message: String,
    pub total_items: Option<i32>,
    pub processed_items: Option<i32>,
    pub errors: Option<Vec<String>>,
    pub current_item: Option<String>,
    pub log_content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_inserted: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_updated: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<TaskProgressDetail>,
    /// Момент создания сессии (UTC); присутствует для записей из in-memory трекера.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_request_count: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_bytes_sent: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_bytes_received: Option<i64>,
}

impl Default for TaskProgressResponse {
    fn default() -> Self {
        Self {
            session_id: String::new(),
            status: "Running".to_string(),
            message: String::new(),
            total_items: None,
            processed_items: None,
            errors: None,
            current_item: None,
            log_content: None,
            total_inserted: None,
            total_updated: None,
            detail: None,
            started_at: None,
            http_request_count: None,
            http_bytes_sent: None,
            http_bytes_received: None,
        }
    }
}

impl From<TaskProgress> for TaskProgressResponse {
    fn from(p: TaskProgress) -> Self {
        Self {
            session_id: p.session_id,
            status: p.status.to_string(),
            message: p.message,
            total_items: p.total_items,
            processed_items: p.processed_items,
            errors: p.errors,
            current_item: p.current_item,
            log_content: p.log_content,
            total_inserted: p.total_inserted,
            total_updated: p.total_updated,
            detail: p.detail,
            started_at: p.started_at,
            http_request_count: p.http_request_count,
            http_bytes_sent: p.http_bytes_sent,
            http_bytes_received: p.http_bytes_received,
        }
    }
}
