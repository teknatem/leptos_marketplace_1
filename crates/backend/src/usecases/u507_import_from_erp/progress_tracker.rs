use contracts::usecases::u507_import_from_erp::progress::{ImportProgress, ImportStatus};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Трекер прогресса импорта из ERP (in-memory)
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

    pub fn create_session(&self, session_id: String) {
        let mut sessions = self.sessions.write().unwrap();
        sessions.insert(session_id.clone(), ImportProgress::new(session_id));
    }

    pub fn get_progress(&self, session_id: &str) -> Option<ImportProgress> {
        self.sessions.read().unwrap().get(session_id).cloned()
    }

    pub fn set_total(&self, session_id: &str, total: i32) {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(p) = sessions.get_mut(session_id) {
            p.total = Some(total);
            p.updated_at = chrono::Utc::now();
        }
    }

    pub fn update_progress(
        &self,
        session_id: &str,
        processed: i32,
        inserted: i32,
        updated: i32,
        current_item: Option<String>,
    ) {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(p) = sessions.get_mut(session_id) {
            p.processed = processed;
            p.inserted = inserted;
            p.updated = updated;
            p.current_item = current_item;
            p.updated_at = chrono::Utc::now();
        }
    }

    pub fn add_error(&self, session_id: &str, message: String) {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(p) = sessions.get_mut(session_id) {
            p.error_messages.push(message);
            p.errors += 1;
            p.updated_at = chrono::Utc::now();
        }
    }

    pub fn complete_session(&self, session_id: &str, status: ImportStatus) {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(p) = sessions.get_mut(session_id) {
            p.status = status;
            p.completed_at = Some(chrono::Utc::now());
            p.updated_at = chrono::Utc::now();
        }
    }

    pub fn cleanup_old_sessions(&self, max_age_hours: i64) {
        let mut sessions = self.sessions.write().unwrap();
        let now = chrono::Utc::now();
        sessions.retain(|_, p| {
            if let Some(completed_at) = p.completed_at {
                (now - completed_at).num_hours() < max_age_hours
            } else {
                true
            }
        });
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}
