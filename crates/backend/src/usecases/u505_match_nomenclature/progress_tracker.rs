use contracts::usecases::u505_match_nomenclature::progress::{
    MatchError, MatchProgress, MatchStatus,
};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Трекер прогресса сопоставления (in-memory, для real-time мониторинга)
#[derive(Clone)]
pub struct ProgressTracker {
    sessions: Arc<RwLock<HashMap<String, MatchProgress>>>,
}

impl ProgressTracker {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Создать новую сессию сопоставления
    pub fn create_session(&self, session_id: String, total: Option<i32>) {
        let mut sessions = self.sessions.write().unwrap();
        sessions.insert(
            session_id.clone(),
            MatchProgress {
                session_id,
                status: MatchStatus::InProgress,
                total,
                processed: 0,
                matched: 0,
                cleared: 0,
                skipped: 0,
                ambiguous: 0,
                errors: 0,
                error_list: Vec::new(),
                current_item: None,
                started_at: chrono::Utc::now(),
                completed_at: None,
            },
        );
    }

    /// Получить текущий прогресс сессии
    pub fn get_progress(&self, session_id: &str) -> Option<MatchProgress> {
        let sessions = self.sessions.read().unwrap();
        sessions.get(session_id).cloned()
    }

    /// Обновить прогресс
    pub fn update_progress(
        &self,
        session_id: &str,
        processed: i32,
        matched: i32,
        cleared: i32,
        skipped: i32,
        ambiguous: i32,
    ) {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(progress) = sessions.get_mut(session_id) {
            progress.processed = processed;
            progress.matched = matched;
            progress.cleared = cleared;
            progress.skipped = skipped;
            progress.ambiguous = ambiguous;
        }
    }

    /// Установить текущий обрабатываемый товар
    pub fn set_current_item(&self, session_id: &str, label: Option<String>) {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(progress) = sessions.get_mut(session_id) {
            progress.current_item = label;
        }
    }

    /// Добавить ошибку
    pub fn add_error(&self, session_id: &str, message: String, details: Option<String>, article: Option<String>) {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(progress) = sessions.get_mut(session_id) {
            progress.errors += 1;
            progress.error_list.push(MatchError {
                message,
                details,
                article,
            });
        }
    }

    /// Завершить сессию сопоставления
    pub fn complete_session(&self, session_id: &str, status: MatchStatus) {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(progress) = sessions.get_mut(session_id) {
            progress.status = status;
            progress.completed_at = Some(chrono::Utc::now());
            progress.current_item = None;
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
