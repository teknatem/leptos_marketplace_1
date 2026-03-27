use contracts::domain::a018_llm_chat::aggregate::LlmChatMessage;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub enum LlmJobStatus {
    Pending,
    Done(LlmChatMessage),
    Error(String),
}

type Store = Arc<RwLock<HashMap<String, LlmJobStatus>>>;

static JOB_STORE: Lazy<Store> = Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// Register a new job as Pending.
pub async fn register(job_id: &str) {
    JOB_STORE
        .write()
        .await
        .insert(job_id.to_string(), LlmJobStatus::Pending);
}

/// Mark job as Done.
pub async fn complete(job_id: &str, msg: LlmChatMessage) {
    JOB_STORE
        .write()
        .await
        .insert(job_id.to_string(), LlmJobStatus::Done(msg));
}

/// Mark job as Error.
pub async fn fail(job_id: &str, error: String) {
    JOB_STORE
        .write()
        .await
        .insert(job_id.to_string(), LlmJobStatus::Error(error));
}

/// Read current status. Removes the entry once it reaches a terminal state (Done/Error).
pub async fn take(job_id: &str) -> Option<LlmJobStatus> {
    let mut store = JOB_STORE.write().await;
    let status = store.get(job_id)?.clone();
    if matches!(status, LlmJobStatus::Done(_) | LlmJobStatus::Error(_)) {
        store.remove(job_id);
    }
    Some(status)
}
