use contracts::usecases::u508_repost_documents::progress::{RepostProgress, RepostStatus};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct ProgressTracker {
    sessions: Arc<RwLock<HashMap<String, RepostProgress>>>,
}

impl ProgressTracker {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn create_session(&self, session_id: String) {
        let mut sessions = self.sessions.write().unwrap();
        sessions.insert(session_id.clone(), RepostProgress::new(session_id));
    }

    pub fn get_progress(&self, session_id: &str) -> Option<RepostProgress> {
        self.sessions.read().unwrap().get(session_id).cloned()
    }

    pub fn set_total(&self, session_id: &str, total: i32) {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(progress) = sessions.get_mut(session_id) {
            progress.total = Some(total);
            progress.updated_at = chrono::Utc::now();
        }
    }

    pub fn set_chunks_total(&self, session_id: &str, total: i32) {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(progress) = sessions.get_mut(session_id) {
            progress.chunks_total = Some(total);
            progress.updated_at = chrono::Utc::now();
        }
    }

    pub fn update_progress(
        &self,
        session_id: &str,
        processed: i32,
        reposted: i32,
        current_item: Option<String>,
    ) {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(progress) = sessions.get_mut(session_id) {
            progress.processed = processed;
            progress.reposted = reposted;
            progress.current_item = current_item;
            progress.updated_at = chrono::Utc::now();
        }
    }

    pub fn update_chunk_progress(
        &self,
        session_id: &str,
        chunks_processed: i32,
        chunk_date: Option<String>,
        chunk_connection_mp_ref: Option<String>,
        chunk_label: Option<String>,
    ) {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(progress) = sessions.get_mut(session_id) {
            progress.chunks_processed = chunks_processed;
            progress.current_chunk_date = chunk_date;
            progress.current_chunk_connection_mp_ref = chunk_connection_mp_ref;
            progress.current_chunk_label = chunk_label;
            progress.updated_at = chrono::Utc::now();
        }
    }

    pub fn add_error(&self, session_id: &str, message: String) {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(progress) = sessions.get_mut(session_id) {
            progress.errors += 1;
            progress.error_messages.push(message);
            progress.updated_at = chrono::Utc::now();
        }
    }

    pub fn complete_session(&self, session_id: &str, status: RepostStatus) {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(progress) = sessions.get_mut(session_id) {
            progress.status = status;
            progress.completed_at = Some(chrono::Utc::now());
            progress.updated_at = chrono::Utc::now();
            progress.current_item = None;
            progress.current_chunk_date = None;
            progress.current_chunk_connection_mp_ref = None;
            progress.current_chunk_label = None;
        }
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}
