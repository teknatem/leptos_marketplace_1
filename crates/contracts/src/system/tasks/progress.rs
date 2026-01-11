use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

impl ToString for TaskStatus {
    fn to_string(&self) -> String {
        match self {
            TaskStatus::Pending => "Pending".to_string(),
            TaskStatus::Running => "Running".to_string(),
            TaskStatus::Completed => "Completed".to_string(),
            TaskStatus::Failed => "Failed".to_string(),
        }
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
        }
    }
}

// Conversions from UseCase progress types
impl From<crate::usecases::u501_import_from_ut::progress::ImportProgress> for TaskProgress {
    fn from(p: crate::usecases::u501_import_from_ut::progress::ImportProgress) -> Self {
        Self {
            session_id: p.session_id,
            status: match p.status {
                crate::usecases::u501_import_from_ut::progress::ImportStatus::Running => TaskStatus::Running,
                crate::usecases::u501_import_from_ut::progress::ImportStatus::Completed => TaskStatus::Completed,
                crate::usecases::u501_import_from_ut::progress::ImportStatus::CompletedWithErrors => TaskStatus::Completed,
                crate::usecases::u501_import_from_ut::progress::ImportStatus::Failed => TaskStatus::Failed,
                crate::usecases::u501_import_from_ut::progress::ImportStatus::Cancelled => TaskStatus::Failed,
            },
            message: "Import from UT".to_string(),
            total_items: Some(p.total_processed as i32), // Simplified
            processed_items: Some(p.total_processed as i32),
            errors: if p.total_errors > 0 { Some(vec![format!("{} errors", p.total_errors)]) } else { None },
            current_item: None,
            log_content: None,
        }
    }
}

impl From<crate::usecases::u502_import_from_ozon::progress::ImportProgress> for TaskProgress {
    fn from(p: crate::usecases::u502_import_from_ozon::progress::ImportProgress) -> Self {
        Self {
            session_id: p.session_id,
            status: match p.status {
                crate::usecases::u502_import_from_ozon::progress::ImportStatus::Running => TaskStatus::Running,
                crate::usecases::u502_import_from_ozon::progress::ImportStatus::Completed => TaskStatus::Completed,
                crate::usecases::u502_import_from_ozon::progress::ImportStatus::CompletedWithErrors => TaskStatus::Completed,
                crate::usecases::u502_import_from_ozon::progress::ImportStatus::Failed => TaskStatus::Failed,
                crate::usecases::u502_import_from_ozon::progress::ImportStatus::Cancelled => TaskStatus::Failed,
            },
            message: "Import from OZON".to_string(),
            total_items: Some(p.total_processed as i32),
            processed_items: Some(p.total_processed as i32),
            errors: if p.total_errors > 0 { Some(vec![format!("{} errors", p.total_errors)]) } else { None },
            current_item: None,
            log_content: None,
        }
    }
}

impl From<crate::usecases::u503_import_from_yandex::progress::ImportProgress> for TaskProgress {
    fn from(p: crate::usecases::u503_import_from_yandex::progress::ImportProgress) -> Self {
        Self {
            session_id: p.session_id,
            status: match p.status {
                crate::usecases::u503_import_from_yandex::progress::ImportStatus::Running => TaskStatus::Running,
                crate::usecases::u503_import_from_yandex::progress::ImportStatus::Completed => TaskStatus::Completed,
                crate::usecases::u503_import_from_yandex::progress::ImportStatus::CompletedWithErrors => TaskStatus::Completed,
                crate::usecases::u503_import_from_yandex::progress::ImportStatus::Failed => TaskStatus::Failed,
                crate::usecases::u503_import_from_yandex::progress::ImportStatus::Cancelled => TaskStatus::Failed,
            },
            message: "Import from Yandex".to_string(),
            total_items: Some(p.total_processed as i32),
            processed_items: Some(p.total_processed as i32),
            errors: if p.total_errors > 0 { Some(vec![format!("{} errors", p.total_errors)]) } else { None },
            current_item: None,
            log_content: None,
        }
    }
}

impl From<crate::usecases::u504_import_from_wildberries::progress::ImportProgress> for TaskProgress {
    fn from(p: crate::usecases::u504_import_from_wildberries::progress::ImportProgress) -> Self {
        Self {
            session_id: p.session_id,
            status: match p.status {
                crate::usecases::u504_import_from_wildberries::progress::ImportStatus::Running => TaskStatus::Running,
                crate::usecases::u504_import_from_wildberries::progress::ImportStatus::Completed => TaskStatus::Completed,
                crate::usecases::u504_import_from_wildberries::progress::ImportStatus::CompletedWithErrors => TaskStatus::Completed,
                crate::usecases::u504_import_from_wildberries::progress::ImportStatus::Failed => TaskStatus::Failed,
                crate::usecases::u504_import_from_wildberries::progress::ImportStatus::Cancelled => TaskStatus::Failed,
            },
            message: "Import from Wildberries".to_string(),
            total_items: Some(p.total_processed as i32),
            processed_items: Some(p.total_processed as i32),
            errors: if p.total_errors > 0 { Some(vec![format!("{} errors", p.total_errors)]) } else { None },
            current_item: None,
            log_content: None,
        }
    }
}

