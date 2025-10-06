use contracts::usecases::u501_import_from_ut::progress::{
    AggregateImportStatus, AggregateProgress, ImportProgress, ImportStatus,
};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Трекер прогресса импорта (in-memory, для real-time мониторинга)
#[derive(Clone)]
pub struct ProgressTracker {
    sessions: Arc<RwLock<HashMap<String, ImportProgress>>>,
}

impl ProgressTracker {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Создать новую сессию импорта
    pub fn create_session(&self, session_id: String) {
        let mut sessions = self.sessions.write().unwrap();
        sessions.insert(session_id.clone(), ImportProgress::new(session_id));
    }

    /// Получить текущий прогресс сессии
    pub fn get_progress(&self, session_id: &str) -> Option<ImportProgress> {
        let sessions = self.sessions.read().unwrap();
        sessions.get(session_id).cloned()
    }

    /// Добавить агрегат для отслеживания
    pub fn add_aggregate(&self, session_id: &str, aggregate_index: String, aggregate_name: String) {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(progress) = sessions.get_mut(session_id) {
            progress.aggregates.push(AggregateProgress {
                aggregate_index,
                aggregate_name,
                status: AggregateImportStatus::Pending,
                processed: 0,
                total: None,
                inserted: 0,
                updated: 0,
                errors: 0,
                current_item: None,
            });
            progress.updated_at = chrono::Utc::now();
        }
    }

    /// Обновить прогресс агрегата
    pub fn update_aggregate(
        &self,
        session_id: &str,
        aggregate_index: &str,
        processed: i32,
        total: Option<i32>,
        inserted: i32,
        updated: i32,
    ) {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(progress) = sessions.get_mut(session_id) {
            if let Some(agg) = progress
                .aggregates
                .iter_mut()
                .find(|a| a.aggregate_index == aggregate_index)
            {
                agg.status = AggregateImportStatus::Running;
                agg.processed = processed;
                agg.total = total;
                agg.inserted = inserted;
                agg.updated = updated;

                // Обновить общую статистику
                progress.total_processed = progress.aggregates.iter().map(|a| a.processed).sum();
                progress.total_inserted = progress.aggregates.iter().map(|a| a.inserted).sum();
                progress.total_updated = progress.aggregates.iter().map(|a| a.updated).sum();
                progress.updated_at = chrono::Utc::now();
            }
        }
    }

    /// Установить текущий обрабатываемый элемент для агрегата
    pub fn set_current_item(&self, session_id: &str, aggregate_index: &str, label: Option<String>) {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(progress) = sessions.get_mut(session_id) {
            if let Some(agg) = progress
                .aggregates
                .iter_mut()
                .find(|a| a.aggregate_index == aggregate_index)
            {
                agg.current_item = label;
                progress.updated_at = chrono::Utc::now();
            }
        }
    }

    /// Отметить агрегат как завершенный
    pub fn complete_aggregate(&self, session_id: &str, aggregate_index: &str) {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(progress) = sessions.get_mut(session_id) {
            if let Some(agg) = progress
                .aggregates
                .iter_mut()
                .find(|a| a.aggregate_index == aggregate_index)
            {
                agg.status = AggregateImportStatus::Completed;
                progress.updated_at = chrono::Utc::now();
            }
        }
    }

    /// Отметить агрегат как проваленный
    pub fn fail_aggregate(&self, session_id: &str, aggregate_index: &str, error: String) {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(progress) = sessions.get_mut(session_id) {
            if let Some(agg) = progress
                .aggregates
                .iter_mut()
                .find(|a| a.aggregate_index == aggregate_index)
            {
                agg.status = AggregateImportStatus::Failed;
                agg.errors += 1;
            }
            progress.add_error(Some(aggregate_index.to_string()), error, None);
            progress.updated_at = chrono::Utc::now();
        }
    }

    /// Добавить ошибку
    pub fn add_error(
        &self,
        session_id: &str,
        aggregate_index: Option<String>,
        message: String,
        details: Option<String>,
    ) {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(progress) = sessions.get_mut(session_id) {
            progress.add_error(aggregate_index, message, details);
            progress.updated_at = chrono::Utc::now();
        }
    }

    /// Завершить сессию импорта
    pub fn complete_session(&self, session_id: &str, status: ImportStatus) {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(progress) = sessions.get_mut(session_id) {
            progress.status = status;
            progress.completed_at = Some(chrono::Utc::now());
            progress.updated_at = chrono::Utc::now();
        }
    }

    /// Удалить старые сессии (для очистки памяти)
    pub fn cleanup_old_sessions(&self, max_age_hours: i64) {
        let mut sessions = self.sessions.write().unwrap();
        let now = chrono::Utc::now();
        sessions.retain(|_, progress| {
            if let Some(completed_at) = progress.completed_at {
                (now - completed_at).num_hours() < max_age_hours
            } else {
                true // Не удаляем активные сессии
            }
        });
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}
